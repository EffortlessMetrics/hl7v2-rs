use crate::handlers::error::AppError;
use crate::models::{
    AckPolicyAcceptOn, AckPolicyConfig, AckPolicyDecision, AckPolicyMode, AckPolicyOutcome,
    AckPolicyReason, AckPolicyRejectCondition, AckRequestCode,
};

pub(super) fn parse_msh_for_ack_policy(
    message_bytes: &[u8],
    mllp_framed: bool,
) -> Result<hl7v2::Message, AppError> {
    let input = if mllp_framed {
        hl7v2::unwrap_mllp(message_bytes)
            .map_err(|e| AppError::Parse(format!("MLLP parse error: {}", e)))?
    } else {
        message_bytes
    };

    let first_segment_end = input
        .iter()
        .position(|byte| matches!(byte, b'\r' | b'\n'))
        .unwrap_or(input.len());
    let msh = &input[..first_segment_end];
    if !msh.starts_with(b"MSH") {
        return Err(AppError::Parse(
            "Parse error: message did not contain a usable MSH segment".to_string(),
        ));
    }

    let mut buffer = msh.to_vec();
    buffer.push(b'\r');
    hl7v2::parse(&buffer)
        .map_err(|e| AppError::Parse(format!("MSH parse error for ACK policy: {}", e)))
}

pub(super) fn map_ack_code(code: AckRequestCode) -> hl7v2::AckCode {
    match code {
        AckRequestCode::Aa => hl7v2::AckCode::AA,
        AckRequestCode::Ae => hl7v2::AckCode::AE,
        AckRequestCode::Ar => hl7v2::AckCode::AR,
        AckRequestCode::Ca => hl7v2::AckCode::CA,
        AckRequestCode::Ce => hl7v2::AckCode::CE,
        AckRequestCode::Cr => hl7v2::AckCode::CR,
    }
}

pub(super) fn ack_policy_decision_for_validation(
    policy: &AckPolicyConfig,
    report: &hl7v2::ValidationReport,
) -> Result<AckPolicyDecision, AppError> {
    if report.valid && policy.accept_on == AckPolicyAcceptOn::Valid {
        let ack_code = match policy.mode {
            AckPolicyMode::Original => AckRequestCode::Aa,
            AckPolicyMode::Enhanced => AckRequestCode::Ca,
        };
        return Ok(AckPolicyDecision {
            mode: policy.mode,
            outcome: AckPolicyOutcome::Accepted,
            reason: AckPolicyReason::Valid,
            ack_code: ack_code.as_str().to_string(),
            include_error_text: false,
            error_text: None,
        });
    }

    if policy.rejects(AckPolicyRejectCondition::ValidationError) {
        return Ok(ack_policy_reject_decision(
            policy,
            AckPolicyReason::ValidationError,
            report.issue_count,
        ));
    }

    Err(AppError::Validation(
        "ACK policy did not define a decision for validation failure".to_string(),
    ))
}

pub(super) fn ack_policy_reject_decision(
    policy: &AckPolicyConfig,
    reason: AckPolicyReason,
    issue_count: usize,
) -> AckPolicyDecision {
    let ack_code = match policy.mode {
        AckPolicyMode::Original => AckRequestCode::Ar,
        AckPolicyMode::Enhanced => AckRequestCode::Cr,
    };
    let error_text = policy
        .include_error_text
        .then(|| ack_policy_error_text(reason, issue_count));

    AckPolicyDecision {
        mode: policy.mode,
        outcome: AckPolicyOutcome::Rejected,
        reason,
        ack_code: ack_code.as_str().to_string(),
        include_error_text: policy.include_error_text,
        error_text,
    }
}

pub(super) fn ack_policy_error_text(reason: AckPolicyReason, issue_count: usize) -> String {
    match reason {
        AckPolicyReason::Valid => "message accepted".to_string(),
        AckPolicyReason::ParseError => "message parsing failed".to_string(),
        AckPolicyReason::ValidationError => {
            format!("message validation failed with {issue_count} issue(s)")
        }
    }
}

pub(super) fn ack_code_from_policy_decision(
    decision: &AckPolicyDecision,
) -> Result<hl7v2::AckCode, AppError> {
    match decision.ack_code.as_str() {
        "AA" => Ok(hl7v2::AckCode::AA),
        "AR" => Ok(hl7v2::AckCode::AR),
        "CA" => Ok(hl7v2::AckCode::CA),
        "CR" => Ok(hl7v2::AckCode::CR),
        code => Err(AppError::Internal(format!(
            "ACK policy produced unsupported ACK code: {code}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_MESSAGE: &str = "MSH|^~\\&|SENDAPP|SENDFAC|RECVAPP|RECVFAC|202605030101||ADT^A01|CTRL123|P|2.5\rPID|1||123456^^^HOSP^MR||Doe^John\r";

    fn empty_report(valid: bool, issue_count: usize) -> hl7v2::ValidationReport {
        hl7v2::ValidationReport {
            valid,
            message_type: "ADT^A01".to_string(),
            profile: None,
            segment_count: 0,
            issue_count,
            issues: Vec::new(),
        }
    }

    fn decision_with_code(code: &str) -> AckPolicyDecision {
        AckPolicyDecision {
            mode: AckPolicyMode::Original,
            outcome: AckPolicyOutcome::Accepted,
            reason: AckPolicyReason::Valid,
            ack_code: code.to_string(),
            include_error_text: false,
            error_text: None,
        }
    }

    #[test]
    fn map_ack_code_covers_all_request_codes() {
        assert_eq!(map_ack_code(AckRequestCode::Aa), hl7v2::AckCode::AA);
        assert_eq!(map_ack_code(AckRequestCode::Ae), hl7v2::AckCode::AE);
        assert_eq!(map_ack_code(AckRequestCode::Ar), hl7v2::AckCode::AR);
        assert_eq!(map_ack_code(AckRequestCode::Ca), hl7v2::AckCode::CA);
        assert_eq!(map_ack_code(AckRequestCode::Ce), hl7v2::AckCode::CE);
        assert_eq!(map_ack_code(AckRequestCode::Cr), hl7v2::AckCode::CR);
    }

    #[test]
    fn parse_msh_for_ack_policy_accepts_plain_message() {
        let parsed = parse_msh_for_ack_policy(SAMPLE_MESSAGE.as_bytes(), false)
            .expect("plain message should parse");
        assert_eq!(parsed.segments[0].id_str(), "MSH");
    }

    #[test]
    fn parse_msh_for_ack_policy_accepts_mllp_framed_message() {
        let framed = hl7v2::wrap_mllp(SAMPLE_MESSAGE.as_bytes());
        let parsed =
            parse_msh_for_ack_policy(&framed, true).expect("MLLP framed message should parse");
        assert_eq!(parsed.segments[0].id_str(), "MSH");
    }

    #[test]
    fn parse_msh_for_ack_policy_rejects_message_without_msh() {
        let err = parse_msh_for_ack_policy(b"PID|1||123^^^HOSP^MR\r", false)
            .expect_err("message without MSH must fail");
        assert!(
            matches!(&err, AppError::Parse(m) if m.contains("MSH")),
            "expected AppError::Parse mentioning MSH, got {err:?}"
        );
    }

    #[test]
    fn parse_msh_for_ack_policy_uses_only_first_segment_when_terminator_is_lf() {
        // The helper splits on \r or \n. Confirm \n terminated input is accepted.
        let input = "MSH|^~\\&|S|F|R|R|202605030101||ADT^A01|C1|P|2.5\nPID|1\n";
        let parsed = parse_msh_for_ack_policy(input.as_bytes(), false)
            .expect("LF-terminated MSH should parse");
        assert_eq!(parsed.segments[0].id_str(), "MSH");
    }

    #[test]
    fn ack_policy_decision_for_validation_accepts_valid_report_in_original_mode() {
        let policy = AckPolicyConfig::default();
        let report = empty_report(true, 0);

        let decision =
            ack_policy_decision_for_validation(&policy, &report).expect("valid report accepts");
        assert_eq!(decision.outcome, AckPolicyOutcome::Accepted);
        assert_eq!(decision.reason, AckPolicyReason::Valid);
        assert_eq!(decision.ack_code, "AA");
        assert!(!decision.include_error_text);
        assert!(decision.error_text.is_none());
    }

    #[test]
    fn ack_policy_decision_for_validation_accepts_valid_report_in_enhanced_mode() {
        let policy = AckPolicyConfig {
            mode: AckPolicyMode::Enhanced,
            ..AckPolicyConfig::default()
        };
        let report = empty_report(true, 0);

        let decision =
            ack_policy_decision_for_validation(&policy, &report).expect("valid accepts enhanced");
        assert_eq!(decision.ack_code, "CA");
        assert_eq!(decision.outcome, AckPolicyOutcome::Accepted);
    }

    #[test]
    fn ack_policy_decision_for_validation_rejects_invalid_report() {
        let policy = AckPolicyConfig::default();
        let report = empty_report(false, 3);

        let decision =
            ack_policy_decision_for_validation(&policy, &report).expect("invalid report rejects");
        assert_eq!(decision.outcome, AckPolicyOutcome::Rejected);
        assert_eq!(decision.reason, AckPolicyReason::ValidationError);
        assert_eq!(decision.ack_code, "AR");
        assert!(decision.include_error_text);
        let text = decision
            .error_text
            .as_deref()
            .expect("error text should be populated");
        assert!(
            text.contains("3"),
            "error text should mention issue count: {text}"
        );
    }

    #[test]
    fn ack_policy_decision_for_validation_returns_error_when_policy_has_no_path() {
        // Build a policy whose accept_on does not match a valid report and whose
        // reject_on does not include ValidationError, leaving the helper with no
        // decision for an invalid-report case.
        let policy = AckPolicyConfig {
            mode: AckPolicyMode::Original,
            accept_on: AckPolicyAcceptOn::Valid,
            reject_on: vec![AckPolicyRejectCondition::ParseError],
            include_error_text: false,
        };
        let report = empty_report(false, 1);

        let err = ack_policy_decision_for_validation(&policy, &report)
            .expect_err("no rejection path should error");
        assert!(
            matches!(&err, AppError::Validation(m) if m.contains("ACK policy")),
            "expected AppError::Validation mentioning ACK policy, got {err:?}"
        );
    }

    #[test]
    fn ack_policy_reject_decision_original_mode_uses_ar_and_includes_error_text() {
        let policy = AckPolicyConfig::default();
        let decision = ack_policy_reject_decision(&policy, AckPolicyReason::ParseError, 0);
        assert_eq!(decision.ack_code, "AR");
        assert_eq!(decision.outcome, AckPolicyOutcome::Rejected);
        assert_eq!(decision.reason, AckPolicyReason::ParseError);
        assert!(decision.include_error_text);
        assert_eq!(
            decision.error_text.as_deref(),
            Some("message parsing failed")
        );
    }

    #[test]
    fn ack_policy_reject_decision_enhanced_mode_uses_cr() {
        let policy = AckPolicyConfig {
            mode: AckPolicyMode::Enhanced,
            include_error_text: false,
            ..AckPolicyConfig::default()
        };
        let decision = ack_policy_reject_decision(&policy, AckPolicyReason::ValidationError, 2);
        assert_eq!(decision.ack_code, "CR");
        assert_eq!(decision.outcome, AckPolicyOutcome::Rejected);
        assert!(!decision.include_error_text);
        assert!(decision.error_text.is_none());
    }

    #[test]
    fn ack_policy_error_text_covers_every_reason() {
        assert_eq!(
            ack_policy_error_text(AckPolicyReason::Valid, 0),
            "message accepted"
        );
        assert_eq!(
            ack_policy_error_text(AckPolicyReason::ParseError, 7),
            "message parsing failed"
        );
        assert_eq!(
            ack_policy_error_text(AckPolicyReason::ValidationError, 4),
            "message validation failed with 4 issue(s)"
        );
    }

    #[test]
    fn ack_code_from_policy_decision_maps_known_codes() {
        assert!(matches!(
            ack_code_from_policy_decision(&decision_with_code("AA")),
            Ok(hl7v2::AckCode::AA)
        ));
        assert!(matches!(
            ack_code_from_policy_decision(&decision_with_code("AR")),
            Ok(hl7v2::AckCode::AR)
        ));
        assert!(matches!(
            ack_code_from_policy_decision(&decision_with_code("CA")),
            Ok(hl7v2::AckCode::CA)
        ));
        assert!(matches!(
            ack_code_from_policy_decision(&decision_with_code("CR")),
            Ok(hl7v2::AckCode::CR)
        ));
    }

    #[test]
    fn ack_code_from_policy_decision_rejects_unknown_code() {
        let err = ack_code_from_policy_decision(&decision_with_code("ZZ"))
            .expect_err("unsupported ack code must error");
        assert!(
            matches!(&err, AppError::Internal(m) if m.contains("ZZ")),
            "expected AppError::Internal mentioning ZZ, got {err:?}"
        );
    }
}
