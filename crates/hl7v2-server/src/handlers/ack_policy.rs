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
