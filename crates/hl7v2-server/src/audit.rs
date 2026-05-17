//! Redacted structured audit log helpers for server evidence workflows.

use crate::hash::compute_sha256;
use crate::models::{AckPolicyOutcome, AckPolicyReason};

/// Structured event emitted when `/hl7/parse` completes.
pub(crate) const EVENT_PARSE: &str = "hl7v2.parse.completed";
/// Structured event emitted when `/hl7/validate` completes.
pub(crate) const EVENT_VALIDATE: &str = "hl7v2.validate.completed";
/// Structured event emitted when `/hl7/validate-redacted` completes.
pub(crate) const EVENT_VALIDATE_REDACTED: &str = "hl7v2.validate_redacted.completed";
/// Structured event emitted when `/hl7/bundle` writes an evidence bundle.
pub(crate) const EVENT_BUNDLE: &str = "hl7v2.bundle.completed";
/// Structured event emitted when `/hl7/replay` verifies a bundle.
pub(crate) const EVENT_REPLAY: &str = "hl7v2.replay.completed";
/// Structured event emitted when `/hl7/ack` generates an ACK.
pub(crate) const EVENT_ACK: &str = "hl7v2.ack.completed";
/// Structured event emitted when `/hl7/ack-policy` makes an ACK policy decision.
pub(crate) const EVENT_ACK_POLICY: &str = "hl7v2.ack_policy.completed";
/// Structured event emitted when `/hl7/normalize` completes.
pub(crate) const EVENT_NORMALIZE: &str = "hl7v2.normalize.completed";
/// Structured event emitted when a request returns an application error.
pub(crate) const EVENT_ERROR: &str = "hl7v2.request.failed";

/// Message context safe to put in structured logs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MessageLogContext {
    /// Message type from `MSH.9`, or `UNKNOWN` when unavailable.
    pub message_type: String,
    /// SHA-256 hash of `MSH.10`, or `missing` when no control ID is present.
    pub message_control_id_hash: String,
    /// Correlation identifier derived from the control ID hash.
    pub correlation_id: String,
}

impl MessageLogContext {
    /// Build log context without retaining raw message identifiers.
    pub(crate) fn from_message(message: &hl7v2::Message) -> Self {
        let message_type =
            joined_components(message, "MSH.9").unwrap_or_else(|| "UNKNOWN".to_string());
        let message_control_id = hl7v2::get(message, "MSH.10").unwrap_or("");
        let message_control_id_hash = hash_or_missing(message_control_id);

        Self {
            message_type,
            correlation_id: message_control_id_hash.clone(),
            message_control_id_hash,
        }
    }
}

/// Hash a caller-supplied identifier before logging it.
pub(crate) fn hash_identifier(identifier: &str) -> String {
    hash_or_missing(identifier)
}

/// Stable validation status string for structured logs.
pub(crate) fn validation_status(valid: bool) -> &'static str {
    if valid { "valid" } else { "invalid" }
}

/// Stable redaction status string for structured logs.
pub(crate) fn redaction_status(phi_removed: bool) -> &'static str {
    if phi_removed {
        "phi_removed"
    } else {
        "no_phi_removed"
    }
}

/// Stable ACK policy outcome label for structured logs.
pub(crate) fn ack_outcome_label(outcome: AckPolicyOutcome) -> &'static str {
    match outcome {
        AckPolicyOutcome::Accepted => "accepted",
        AckPolicyOutcome::Rejected => "rejected",
    }
}

/// Stable ACK policy reason label for structured logs.
pub(crate) fn ack_reason_label(reason: AckPolicyReason) -> &'static str {
    match reason {
        AckPolicyReason::Valid => "valid",
        AckPolicyReason::ParseError => "parse_error",
        AckPolicyReason::ValidationError => "validation_error",
    }
}

fn hash_or_missing(value: &str) -> String {
    if value.is_empty() {
        "missing".to_string()
    } else {
        compute_sha256(value)
    }
}

fn joined_components(message: &hl7v2::Message, path: &str) -> Option<String> {
    let mut components = Vec::new();

    for index in 1.. {
        let component_path = format!("{path}.{index}");
        match hl7v2::get(message, &component_path) {
            Some(value) if !value.is_empty() => components.push(value.to_string()),
            Some(_) => {}
            None => break,
        }
    }

    if components.is_empty() {
        hl7v2::get(message, path).map(str::to_string)
    } else {
        Some(components.join("^"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hl7v2_test_utils::{PHI_LEAK_SENTINEL_MESSAGE, assert_no_phi_leak_sentinels};

    const SAMPLE_MESSAGE: &str = "MSH|^~\\&|SENDAPP|SENDFAC|RECVAPP|RECVFAC|202605030101||ADT^A01|CTRL123|P|2.5\rPID|1||123456^^^HOSP^MR||Doe^John\r";
    const MISSING_CONTROL_ID: &str =
        "MSH|^~\\&|SENDAPP|SENDFAC|RECVAPP|RECVFAC|202605030101||ORU^R01||P|2.5\r";

    #[test]
    fn message_log_context_hashes_control_id_without_echoing_it() {
        let message = hl7v2::parse(SAMPLE_MESSAGE.as_bytes()).expect("message should parse");
        let context = MessageLogContext::from_message(&message);

        assert_eq!(context.message_type, "ADT^A01");
        assert_eq!(context.message_control_id_hash.len(), 64);
        assert_eq!(context.correlation_id, context.message_control_id_hash);
        assert!(!context.message_control_id_hash.contains("CTRL123"));
    }

    #[test]
    fn message_log_context_reports_missing_control_id_without_hashing_phi() {
        let message = hl7v2::parse(MISSING_CONTROL_ID.as_bytes()).expect("message should parse");
        let context = MessageLogContext::from_message(&message);

        assert_eq!(context.message_type, "ORU^R01");
        assert_eq!(context.message_control_id_hash, "missing");
        assert_eq!(context.correlation_id, "missing");
    }

    #[test]
    fn hash_identifier_never_echoes_caller_supplied_identifier() {
        let hash = hash_identifier("case-Doe-123456");

        assert_eq!(hash.len(), 64);
        assert!(!hash.contains("Doe"));
        assert!(!hash.contains("123456"));
    }

    #[test]
    fn message_log_context_does_not_echo_phi_leak_sentinels() {
        let message = hl7v2::parse(PHI_LEAK_SENTINEL_MESSAGE.as_bytes())
            .expect("sentinel message should parse");
        let context = MessageLogContext::from_message(&message);

        assert_eq!(context.message_type, "ADT^A01");
        assert_eq!(context.message_control_id_hash.len(), 64);
        assert_no_phi_leak_sentinels("message log context", &format!("{context:?}"));
    }

    #[test]
    fn stable_status_labels_are_machine_friendly() {
        assert_eq!(validation_status(true), "valid");
        assert_eq!(validation_status(false), "invalid");
        assert_eq!(redaction_status(true), "phi_removed");
        assert_eq!(redaction_status(false), "no_phi_removed");
        assert_eq!(ack_outcome_label(AckPolicyOutcome::Accepted), "accepted");
        assert_eq!(ack_outcome_label(AckPolicyOutcome::Rejected), "rejected");
        assert_eq!(ack_reason_label(AckPolicyReason::Valid), "valid");
        assert_eq!(ack_reason_label(AckPolicyReason::ParseError), "parse_error");
        assert_eq!(
            ack_reason_label(AckPolicyReason::ValidationError),
            "validation_error"
        );
    }
}
