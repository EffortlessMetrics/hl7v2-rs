//! PHI redaction for HL7 messages.
//!
//! This module provides functionality for identifying and redacting
//! Personally Identifiable Information (PII) and Protected Health
//! Information (PHI) from HL7 v2 messages.

use crate::model::{Atom, Field, Message, Segment};
use crate::parser::parse;
use crate::writer::write;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

/// Configuration for redaction.
#[derive(Debug, Clone, Default)]
pub struct RedactionConfig {
    /// Replacement string for redacted fields.
    pub replacement: String,
    /// List of field paths to redact, for example `PID.5` or `PID.7`.
    pub fields: Vec<String>,
}

impl RedactionConfig {
    /// Create a new redaction configuration with default HIPAA-oriented fields.
    pub fn hipaa_defaults() -> Self {
        Self {
            replacement: "[REDACTED]".to_string(),
            fields: vec![
                "PID.5".to_string(),  // Patient Name
                "PID.7".to_string(),  // Date/Time of Birth
                "PID.11".to_string(), // Patient Address
                "PID.13".to_string(), // Phone Number - Home
                "PID.14".to_string(), // Phone Number - Business
                "PID.19".to_string(), // SSN Number - Patient
                "NK1.2".to_string(),  // Name
                "NK1.4".to_string(),  // Address
                "NK1.5".to_string(),  // Phone Number
            ],
        }
    }
}

/// Redact PHI from a message based on configuration.
pub fn redact(message: &mut Message, config: &RedactionConfig) {
    for path in &config.fields {
        let Some((segment_id, field_index)) = parse_segment_field_path(path) else {
            continue;
        };

        for segment in &mut message.segments {
            if std::str::from_utf8(&segment.id) == Ok(segment_id) {
                redact_field(segment, field_index, &config.replacement);
            }
        }
    }
}

/// Output from applying a safe-analysis redaction policy to raw HL7.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SafeAnalysisRedactionOutput {
    /// SHA-256 digest of the original input message.
    pub input_sha256: String,
    /// SHA-256 digest of the policy TOML.
    pub policy_sha256: String,
    /// Message type from `MSH.9`, such as `ADT^A01`.
    pub message_type: String,
    /// Redacted HL7 message.
    pub redacted_hl7: String,
    /// Receipt describing the redaction actions applied.
    pub receipt: RedactionReceipt,
}

/// Redaction receipt compatible with safe-analysis evidence artifacts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactionReceipt {
    /// Whether any configured PHI-bearing field was removed or hashed.
    pub phi_removed: bool,
    /// Hash algorithm used by hash redaction actions.
    pub hash_algorithm: String,
    /// Per-rule redaction receipts.
    pub actions: Vec<RedactionActionReceipt>,
}

/// Per-rule redaction action receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactionActionReceipt {
    /// HL7 path covered by this policy action.
    pub path: String,
    /// Policy action applied to this path.
    pub action: RedactionAction,
    /// Policy reason for the action.
    pub reason: String,
    /// Number of matching values affected by this action.
    pub matched_count: usize,
    /// Whether missing matches are acceptable.
    pub optional: bool,
    /// Action status.
    pub status: RedactionActionStatus,
}

/// Safe-analysis redaction action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RedactionAction {
    /// Replace a field with a deterministic SHA-256 hash marker.
    Hash,
    /// Clear the field value.
    Drop,
    /// Keep a non-sensitive field unchanged.
    Retain,
}

/// Redaction action status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RedactionActionStatus {
    /// Action was applied to at least one field.
    Applied,
    /// Retain action matched at least one field.
    Retained,
    /// Optional action did not match a field.
    NotFound,
}

/// Parsed safe-analysis redaction policy.
#[derive(Debug, Clone, Deserialize)]
pub struct SafeAnalysisPolicy {
    rules: Vec<SafeAnalysisPolicyRule>,
}

/// One rule in a safe-analysis redaction policy.
#[derive(Debug, Clone, Deserialize)]
pub struct SafeAnalysisPolicyRule {
    path: String,
    action: RedactionAction,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    optional: bool,
}

#[derive(Debug)]
struct ParsedRedactionPath {
    segment_id: String,
    field_index: usize,
}

/// Safe-analysis redaction error.
#[derive(Debug, thiserror::Error)]
pub enum RedactionError {
    /// Input message could not be parsed.
    #[error("parse error: {0}")]
    Parse(String),
    /// Redacted output could not be encoded as UTF-8.
    #[error("redacted message was not UTF-8: {0}")]
    Utf8(String),
    /// Policy TOML or policy semantics were invalid.
    #[error("{0}")]
    Policy(String),
}

/// Apply a safe-analysis policy to raw HL7 and return redacted evidence output.
///
/// This function fails closed when the policy is malformed, contains duplicate
/// paths, tries to retain built-in sensitive fields, omits present built-in
/// sensitive fields, or has a non-optional redaction rule that matches nothing.
///
/// # Errors
///
/// Returns [`RedactionError`] when the input message cannot parse, the policy
/// cannot be loaded, the policy does not protect present sensitive fields, or
/// the redacted message cannot be encoded as UTF-8.
pub fn redact_hl7_safe_analysis(
    content: impl AsRef<[u8]>,
    policy_text: &str,
) -> Result<SafeAnalysisRedactionOutput, RedactionError> {
    let content = content.as_ref();
    let mut message = parse(content).map_err(|error| RedactionError::Parse(error.to_string()))?;
    let message_type = message_type(&message);
    let receipt = redact_message_safe_analysis(&mut message, policy_text)?;
    let redacted_hl7 = String::from_utf8(write(&message))
        .map_err(|error| RedactionError::Utf8(error.to_string()))?;

    Ok(SafeAnalysisRedactionOutput {
        input_sha256: compute_sha256_bytes(content),
        policy_sha256: compute_sha256(policy_text),
        message_type,
        redacted_hl7,
        receipt,
    })
}

/// Apply a safe-analysis policy to a parsed message.
///
/// # Errors
///
/// Returns [`RedactionError`] if the policy is invalid or does not protect
/// present built-in sensitive fields.
pub fn redact_message_safe_analysis(
    message: &mut Message,
    policy_text: &str,
) -> Result<RedactionReceipt, RedactionError> {
    let policy = load_safe_analysis_policy(policy_text)?;
    apply_safe_analysis_policy(message, &policy)
}

/// Load and validate a safe-analysis policy from TOML.
///
/// # Errors
///
/// Returns [`RedactionError::Policy`] when TOML parsing fails or the policy is
/// structurally unsafe.
pub fn load_safe_analysis_policy(policy_text: &str) -> Result<SafeAnalysisPolicy, RedactionError> {
    let policy: SafeAnalysisPolicy = toml::from_str(policy_text).map_err(|error| {
        RedactionError::Policy(format!("redaction policy is invalid TOML: {error}"))
    })?;
    if policy.rules.is_empty() {
        return Err(RedactionError::Policy(
            "redaction policy must contain at least one rule".to_string(),
        ));
    }

    let mut seen_paths = BTreeSet::new();
    for rule in &policy.rules {
        parse_redaction_path(&rule.path).map_err(RedactionError::Policy)?;
        if !seen_paths.insert(rule.path.clone()) {
            return Err(RedactionError::Policy(format!(
                "redaction policy contains duplicate rule for {}",
                rule.path
            )));
        }
        if rule.reason.as_deref().unwrap_or("").trim().is_empty() {
            return Err(RedactionError::Policy(format!(
                "redaction rule {} must include a reason",
                rule.path
            )));
        }
        if safe_analysis_sensitive_paths().contains(rule.path.as_str())
            && rule.action == RedactionAction::Retain
        {
            return Err(RedactionError::Policy(format!(
                "redaction rule {} cannot retain a built-in sensitive field",
                rule.path
            )));
        }
    }

    Ok(policy)
}

fn apply_safe_analysis_policy(
    message: &mut Message,
    policy: &SafeAnalysisPolicy,
) -> Result<RedactionReceipt, RedactionError> {
    validate_safe_analysis_policy_covers_sensitive_fields(message, policy)?;

    let mut actions = Vec::new();
    let mut phi_removed = false;
    let mut errors = Vec::new();

    for rule in &policy.rules {
        let parsed_path = parse_redaction_path(&rule.path).map_err(RedactionError::Policy)?;
        let mut matched_count = 0_usize;

        for segment in &mut message.segments {
            if segment.id_str() != parsed_path.segment_id {
                continue;
            }

            let Some(field_index) =
                modeled_field_index(&parsed_path.segment_id, parsed_path.field_index)
            else {
                continue;
            };
            let Some(field) = segment.fields.get_mut(field_index) else {
                continue;
            };

            matched_count = matched_count.saturating_add(1);
            match rule.action {
                RedactionAction::Hash => {
                    let value = field_to_text(field, &message.delims);
                    *field = Field::from_text(format!("hash:sha256:{}", compute_sha256(&value)));
                    phi_removed = true;
                }
                RedactionAction::Drop => {
                    *field = Field::new();
                    phi_removed = true;
                }
                RedactionAction::Retain => {}
            }
        }

        let status = match (matched_count, rule.action) {
            (0, _) => RedactionActionStatus::NotFound,
            (_, RedactionAction::Retain) => RedactionActionStatus::Retained,
            _ => RedactionActionStatus::Applied,
        };

        if matched_count == 0 && !rule.optional && rule.action != RedactionAction::Retain {
            errors.push(format!(
                "redaction rule {} matched no fields; mark optional=true if absence is expected",
                rule.path
            ));
        }

        actions.push(RedactionActionReceipt {
            path: rule.path.clone(),
            action: rule.action,
            reason: rule.reason.clone().unwrap_or_default(),
            matched_count,
            optional: rule.optional,
            status,
        });
    }

    if !errors.is_empty() {
        return Err(RedactionError::Policy(errors.join("; ")));
    }

    Ok(RedactionReceipt {
        phi_removed,
        hash_algorithm: "sha256".to_string(),
        actions,
    })
}

fn validate_safe_analysis_policy_covers_sensitive_fields(
    message: &Message,
    policy: &SafeAnalysisPolicy,
) -> Result<(), RedactionError> {
    let protected_paths: BTreeSet<&str> = policy
        .rules
        .iter()
        .filter(|rule| rule.action != RedactionAction::Retain)
        .map(|rule| rule.path.as_str())
        .collect();
    let present_sensitive_paths = present_sensitive_paths(message);
    let missing_paths: Vec<&str> = present_sensitive_paths
        .iter()
        .copied()
        .filter(|path| !protected_paths.contains(path))
        .collect();

    if missing_paths.is_empty() {
        return Ok(());
    }

    Err(RedactionError::Policy(format!(
        "redaction policy does not protect present sensitive field(s): {}",
        missing_paths.join(", ")
    )))
}

fn present_sensitive_paths(message: &Message) -> BTreeSet<&'static str> {
    safe_analysis_sensitive_paths()
        .iter()
        .copied()
        .filter(|path| {
            parse_redaction_path(path).ok().is_some_and(|parsed| {
                message_has_nonempty_field(message, &parsed.segment_id, parsed.field_index)
            })
        })
        .collect()
}

fn safe_analysis_sensitive_paths() -> BTreeSet<&'static str> {
    [
        "PID.3", "PID.5", "PID.7", "PID.11", "PID.13", "PID.14", "PID.19", "NK1.2", "NK1.4",
        "NK1.5",
    ]
    .into_iter()
    .collect()
}

fn parse_segment_field_path(path: &str) -> Option<(&str, usize)> {
    let (segment_id, field_part) = path.split_once('.')?;
    if segment_id.is_empty() || field_part.contains('.') {
        return None;
    }

    field_part
        .parse::<usize>()
        .ok()
        .map(|field_index| (segment_id, field_index))
}

fn parse_redaction_path(path: &str) -> Result<ParsedRedactionPath, String> {
    let (segment_id, field_part) = path
        .split_once('.')
        .ok_or_else(|| format!("redaction path '{path}' must use SEG.field syntax"))?;
    if segment_id.len() != 3
        || !segment_id
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit())
    {
        return Err(format!(
            "redaction path '{path}' must start with a three-character uppercase segment id"
        ));
    }
    if field_part.contains('.') {
        return Err(format!(
            "redaction path '{path}' must target a field, not a component"
        ));
    }

    let field_index = field_part.parse::<usize>().map_err(|_err| {
        format!("redaction path '{path}' must use a positive numeric field index")
    })?;
    if field_index == 0 {
        return Err(format!(
            "redaction path '{path}' must use a one-based field index"
        ));
    }
    if segment_id == "MSH" && field_index < 3 {
        return Err(format!(
            "redaction path '{path}' targets MSH.1/MSH.2, which are delimiter metadata and not redacted by this command"
        ));
    }

    Ok(ParsedRedactionPath {
        segment_id: segment_id.to_string(),
        field_index,
    })
}

fn message_has_nonempty_field(message: &Message, segment_id: &str, field_index: usize) -> bool {
    let Some(field_index) = modeled_field_index(segment_id, field_index) else {
        return false;
    };

    message
        .segments
        .iter()
        .filter(|segment| segment.id_str() == segment_id)
        .filter_map(|segment| segment.fields.get(field_index))
        .any(|field| !field_to_text(field, &message.delims).is_empty())
}

fn modeled_field_index(segment_id: &str, field_index: usize) -> Option<usize> {
    if segment_id == "MSH" {
        field_index.checked_sub(2)
    } else {
        field_index.checked_sub(1)
    }
}

fn field_to_text(field: &Field, delims: &crate::Delims) -> String {
    field
        .reps
        .iter()
        .map(|rep| {
            rep.comps
                .iter()
                .map(|comp| {
                    comp.subs
                        .iter()
                        .map(|atom| match atom {
                            Atom::Text(text) => text.as_str(),
                            Atom::Null => "\"\"",
                        })
                        .collect::<Vec<_>>()
                        .join(&delims.sub.to_string())
                })
                .collect::<Vec<_>>()
                .join(&delims.comp.to_string())
        })
        .collect::<Vec<_>>()
        .join(&delims.rep.to_string())
}

fn message_type(message: &Message) -> String {
    message
        .segments
        .iter()
        .find(|segment| segment.id_str() == "MSH")
        .and_then(|segment| segment.fields.get(7))
        .map(|field| field_to_text(field, &message.delims))
        .filter(|message_type| !message_type.is_empty())
        .unwrap_or_else(|| "UNKNOWN".to_string())
}

fn compute_sha256(value: &str) -> String {
    compute_sha256_bytes(value.as_bytes())
}

fn compute_sha256_bytes(value: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value);
    format!("{:x}", hasher.finalize())
}

fn redact_field(segment: &mut Segment, field_index: usize, replacement: &str) {
    if field_index == 0 {
        return;
    }

    let Some(zero_based_index) = field_index.checked_sub(1) else {
        return;
    };
    let Some(field) = segment.fields.get_mut(zero_based_index) else {
        return;
    };

    *field = Field::from_text(replacement);
}

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
