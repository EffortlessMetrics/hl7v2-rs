//! PHI redaction for HL7 messages.
//!
//! This module provides functionality for identifying and redacting
//! Personally Identifiable Information (PII) and Protected Health
//! Information (PHI) from HL7 v2 messages.

mod basic;
mod digest;
mod path;
mod policy;
mod safe_analysis;
mod text;
mod types;

pub use basic::redact;
pub use policy::{load_safe_analysis_policy, redact_message_safe_analysis};
pub use safe_analysis::redact_hl7_safe_analysis;
pub use types::{
    RedactionAction, RedactionActionReceipt, RedactionActionStatus, RedactionConfig,
    RedactionError, RedactionReceipt, RedactionReceiptV2, SafeAnalysisPolicy,
    SafeAnalysisPolicyRule, SafeAnalysisRedactionOutput, SafeAnalysisRedactionOutputV2,
};

#[cfg(test)]
pub(crate) use path::parse_segment_field_path;

#[cfg(test)]
mod tests {
    use super::{
        RedactionAction, RedactionActionStatus, RedactionConfig, load_safe_analysis_policy,
        parse_segment_field_path, redact, redact_hl7_safe_analysis,
    };
    use crate::{Delims, Field, Message, Segment};

    fn test_message_with_pid_names(names: &[&str]) -> Message {
        Message {
            delims: Delims::default(),
            segments: names
                .iter()
                .map(|name| Segment {
                    id: *b"PID",
                    fields: vec![
                        Field::from_text("1"),
                        Field::from_text(""),
                        Field::from_text("123456^^^HOSP^MR"),
                        Field::from_text(""),
                        Field::from_text(*name),
                    ],
                })
                .collect(),
            charsets: vec![],
        }
    }

    #[test]
    fn redacts_configured_segment_field() {
        let mut message = test_message_with_pid_names(&["Doe^John"]);

        let mut config = RedactionConfig::default();
        config.fields.push("PID.5".to_string());
        config.replacement = "XXX".to_string();

        redact(&mut message, &config);

        let redacted_value = message
            .segments
            .iter()
            .find(|segment| segment.id == *b"PID")
            .and_then(|segment| segment.fields.get(4))
            .and_then(Field::first_text);

        assert_eq!(redacted_value, Some("XXX"));
    }

    #[test]
    fn hipaa_defaults_include_expected_fields() {
        let config = RedactionConfig::hipaa_defaults();

        assert_eq!(config.replacement, "[REDACTED]");
        assert_eq!(config.fields.len(), 9);
        assert!(config.fields.iter().any(|field| field == "PID.5"));
        assert!(config.fields.iter().any(|field| field == "NK1.5"));
    }

    #[test]
    fn parse_segment_field_path_rejects_invalid_paths() {
        assert_eq!(parse_segment_field_path("PID.5"), Some(("PID", 5)));
        assert_eq!(parse_segment_field_path("PID"), None);
        assert_eq!(parse_segment_field_path(".5"), None);
        assert_eq!(parse_segment_field_path("PID.5.1"), None);
        assert_eq!(parse_segment_field_path("PID.name"), None);
    }

    #[test]
    fn ignores_invalid_or_missing_redaction_paths() {
        let mut message = test_message_with_pid_names(&["Doe^John"]);
        let config = RedactionConfig {
            replacement: "XXX".to_string(),
            fields: vec![
                "PID".to_string(),
                ".5".to_string(),
                "PID.5.1".to_string(),
                "PID.name".to_string(),
                "PID.0".to_string(),
                "PID.99".to_string(),
                "NK1.5".to_string(),
            ],
        };

        redact(&mut message, &config);

        let value = message
            .segments
            .iter()
            .find(|segment| segment.id == *b"PID")
            .and_then(|segment| segment.fields.get(4))
            .and_then(Field::first_text);

        assert_eq!(value, Some("Doe^John"));
    }

    #[test]
    fn redacts_all_matching_segments() {
        let mut message = test_message_with_pid_names(&["Doe^John", "Smith^Jane"]);
        let config = RedactionConfig {
            replacement: "XXX".to_string(),
            fields: vec!["PID.5".to_string()],
        };

        redact(&mut message, &config);

        let redacted_count = message
            .segments
            .iter()
            .filter(|segment| segment.fields.get(4).and_then(Field::first_text) == Some("XXX"))
            .count();

        assert_eq!(redacted_count, 2);
    }

    fn safe_analysis_policy() -> &'static str {
        r#"
[[rules]]
path = "PID.3"
action = "hash"
reason = "Patient identifier"

[[rules]]
path = "PID.5"
action = "drop"
reason = "Patient name"

[[rules]]
path = "PID.7"
action = "drop"
reason = "Date of birth"

[[rules]]
path = "PID.11"
action = "drop"
reason = "Address"

[[rules]]
path = "PID.13"
action = "drop"
reason = "Phone"
optional = true
"#
    }

    fn safe_analysis_message() -> &'static str {
        "MSH|^~\\&|SEND|FAC|RECV|FAC|202605090101||ADT^A01|CTRL1|P|2.5\rPID|1||123456^^^HOSP^MR||Doe^John||19700101|M|||123 Main^^Boston^MA||555-1212"
    }

    fn ensure(condition: bool, message: &'static str) -> Result<(), Box<dyn std::error::Error>> {
        if condition {
            Ok(())
        } else {
            Err(std::io::Error::other(message).into())
        }
    }

    #[test]
    fn safe_analysis_redacts_hashes_and_receipts_without_raw_phi()
    -> Result<(), Box<dyn std::error::Error>> {
        let output = redact_hl7_safe_analysis(safe_analysis_message(), safe_analysis_policy())?;

        ensure(output.message_type == "ADT^A01", "expected ADT^A01")?;
        ensure(output.input_sha256.len() == 64, "expected input SHA-256")?;
        ensure(output.policy_sha256.len() == 64, "expected policy SHA-256")?;
        ensure(output.receipt.phi_removed, "expected PHI removal receipt")?;
        ensure(
            output.receipt.hash_algorithm == "sha256",
            "expected SHA-256 receipt",
        )?;
        ensure(
            !output.redacted_hl7.contains("Doe^John"),
            "redacted HL7 leaked patient name",
        )?;
        ensure(
            !output.redacted_hl7.contains("123456"),
            "redacted HL7 leaked patient identifier",
        )?;
        ensure(
            !output.redacted_hl7.contains("19700101"),
            "redacted HL7 leaked date of birth",
        )?;
        ensure(
            !output.redacted_hl7.contains("123 Main"),
            "redacted HL7 leaked address",
        )?;
        ensure(
            output.redacted_hl7.contains("hash:sha256:"),
            "expected hash marker",
        )?;

        let receipt_json = serde_json::to_string(&output.receipt)?;
        ensure(!receipt_json.contains("Doe"), "receipt leaked patient name")?;
        ensure(
            !receipt_json.contains("123456"),
            "receipt leaked patient identifier",
        )?;
        ensure(
            !receipt_json.contains("19700101"),
            "receipt leaked date of birth",
        )?;

        let pid3 = output
            .receipt
            .actions
            .iter()
            .find(|action| action.path == "PID.3")
            .ok_or_else(|| std::io::Error::other("expected PID.3 receipt action"))?;
        ensure(pid3.action == RedactionAction::Hash, "expected PID.3 hash")?;
        ensure(
            pid3.status == RedactionActionStatus::Applied,
            "expected PID.3 applied status",
        )?;
        ensure(pid3.matched_count == 1, "expected one PID.3 match")?;
        Ok(())
    }

    #[test]
    fn redaction_receipt_v2_embeds_tool_provenance() -> Result<(), Box<dyn std::error::Error>> {
        let output = redact_hl7_safe_analysis(safe_analysis_message(), safe_analysis_policy())?;
        let receipt_v2 = output.receipt.to_v2("hl7v2", "1.3.0");

        ensure(receipt_v2.schema_version == "2", "expected v2 schema")?;
        ensure(receipt_v2.tool_name == "hl7v2", "expected tool name")?;
        ensure(receipt_v2.tool_version == "1.3.0", "expected tool version")?;
        ensure(receipt_v2.phi_removed, "expected PHI removal")?;
        ensure(
            receipt_v2.hash_algorithm == "sha256",
            "expected SHA-256 receipt",
        )?;
        ensure(
            receipt_v2
                .actions
                .iter()
                .any(|action| action.path == "PID.3" && action.action == RedactionAction::Hash),
            "expected PID.3 hash action",
        )?;
        Ok(())
    }

    #[test]
    fn safe_analysis_output_v2_embeds_tool_provenance() -> Result<(), Box<dyn std::error::Error>> {
        let output = redact_hl7_safe_analysis(safe_analysis_message(), safe_analysis_policy())?;
        let output_v2 = output.to_v2("hl7v2-cli", "1.3.0");

        ensure(output_v2.schema_version == "2", "expected v2 schema")?;
        ensure(output_v2.tool_name == "hl7v2-cli", "expected tool name")?;
        ensure(output_v2.tool_version == "1.3.0", "expected tool version")?;
        ensure(
            output_v2.receipt.schema_version == "2",
            "expected nested receipt v2 schema",
        )?;
        ensure(
            output_v2.receipt.tool_name == "hl7v2-cli",
            "expected nested receipt tool name",
        )?;
        ensure(
            output_v2.receipt.tool_version == "1.3.0",
            "expected nested receipt tool version",
        )?;
        ensure(output_v2.receipt.phi_removed, "expected PHI removal")?;
        ensure(
            !output_v2.redacted_hl7.contains("Doe^John"),
            "redacted HL7 leaked patient name",
        )?;
        Ok(())
    }

    #[test]
    fn safe_analysis_reports_original_message_type_even_if_redacted()
    -> Result<(), Box<dyn std::error::Error>> {
        let policy = r#"
[[rules]]
path = "MSH.9"
action = "drop"
reason = "Test message type redaction"
"#;
        let output = redact_hl7_safe_analysis(
            "MSH|^~\\&|SEND|FAC|RECV|FAC|202605090101||ADT^A01|CTRL1|P|2.5",
            policy,
        )?;

        ensure(
            output.message_type == "ADT^A01",
            "expected original message type",
        )?;
        ensure(
            !output.redacted_hl7.contains("ADT^A01"),
            "expected redacted message type field",
        )?;
        Ok(())
    }

    #[test]
    fn safe_analysis_fails_closed_when_policy_omits_present_sensitive_field()
    -> Result<(), Box<dyn std::error::Error>> {
        let policy = r#"
[[rules]]
path = "PID.3"
action = "hash"
reason = "Patient identifier"
"#;

        let Err(error) = redact_hl7_safe_analysis(safe_analysis_message(), policy) else {
            return Err(std::io::Error::other(
                "expected incomplete sensitive-field policy to fail",
            )
            .into());
        };
        ensure(
            error
                .to_string()
                .contains("redaction policy does not protect present sensitive field(s)"),
            "expected sensitive-field coverage error",
        )?;
        ensure(
            error.to_string().contains("PID.5"),
            "expected PID.5 in coverage error",
        )?;
        Ok(())
    }

    #[test]
    fn safe_analysis_rejects_retaining_builtin_sensitive_field()
    -> Result<(), Box<dyn std::error::Error>> {
        let policy = r#"
[[rules]]
path = "PID.5"
action = "retain"
reason = "Unsafe"
"#;

        let Err(error) = load_safe_analysis_policy(policy) else {
            return Err(std::io::Error::other(
                "expected retaining a built-in sensitive field to fail",
            )
            .into());
        };
        ensure(
            error
                .to_string()
                .contains("redaction rule PID.5 cannot retain a built-in sensitive field"),
            "expected retain-sensitive-field error",
        )?;
        Ok(())
    }

    #[test]
    fn safe_analysis_requires_non_optional_matches() -> Result<(), Box<dyn std::error::Error>> {
        let policy = r#"
[[rules]]
path = "PID.3"
action = "hash"
reason = "Patient identifier"

[[rules]]
path = "PID.5"
action = "drop"
reason = "Patient name"

[[rules]]
path = "PID.7"
action = "drop"
reason = "Date of birth"

[[rules]]
path = "PID.11"
action = "drop"
reason = "Address"

[[rules]]
path = "PID.13"
action = "drop"
reason = "Phone"

[[rules]]
path = "PID.19"
action = "drop"
reason = "SSN"
"#;

        let Err(error) = redact_hl7_safe_analysis(safe_analysis_message(), policy) else {
            return Err(
                std::io::Error::other("expected non-optional missing match to fail").into(),
            );
        };
        ensure(
            error
                .to_string()
                .contains("redaction rule PID.19 matched no fields"),
            "expected non-optional missing match error",
        )?;
        Ok(())
    }
}
