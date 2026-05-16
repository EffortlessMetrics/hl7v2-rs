//! Safe-analysis redaction helpers for HTTP evidence endpoints.

use crate::models::{
    RedactionAction, RedactionActionReceipt, RedactionActionStatus, RedactionReceipt,
};
use hl7v2::{Atom, Field, Message};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

#[derive(Debug, Deserialize)]
struct SafeAnalysisPolicy {
    rules: Vec<SafeAnalysisPolicyRule>,
}

#[derive(Debug, Deserialize)]
struct SafeAnalysisPolicyRule {
    path: String,
    action: RedactionAction,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    optional: bool,
}

struct ParsedRedactionPath {
    segment_id: String,
    field_index: usize,
}

/// Apply a safe-analysis policy to a message and return a redaction receipt.
pub fn redact_message(
    message: &mut Message,
    policy_text: &str,
) -> Result<RedactionReceipt, String> {
    let policy = load_safe_analysis_policy(policy_text)?;
    apply_safe_analysis_policy(message, &policy)
}

fn load_safe_analysis_policy(policy_text: &str) -> Result<SafeAnalysisPolicy, String> {
    let policy: SafeAnalysisPolicy = toml::from_str(policy_text)
        .map_err(|error| format!("redaction policy is invalid TOML: {error}"))?;
    if policy.rules.is_empty() {
        return Err("redaction policy must contain at least one rule".to_string());
    }

    let mut seen_paths = BTreeSet::new();
    for rule in &policy.rules {
        parse_redaction_path(&rule.path)?;
        if !seen_paths.insert(rule.path.clone()) {
            return Err(format!(
                "redaction policy contains duplicate rule for {}",
                rule.path
            ));
        }
        if rule.reason.as_deref().unwrap_or("").trim().is_empty() {
            return Err(format!(
                "redaction rule {} must include a reason",
                rule.path
            ));
        }
        if safe_analysis_sensitive_paths().contains(rule.path.as_str())
            && rule.action == RedactionAction::Retain
        {
            return Err(format!(
                "redaction rule {} cannot retain a built-in sensitive field",
                rule.path
            ));
        }
    }

    Ok(policy)
}

fn apply_safe_analysis_policy(
    message: &mut Message,
    policy: &SafeAnalysisPolicy,
) -> Result<RedactionReceipt, String> {
    validate_safe_analysis_policy_covers_sensitive_fields(message, policy)?;

    let mut actions = Vec::new();
    let mut phi_removed = false;
    let mut errors = Vec::new();

    for rule in &policy.rules {
        let parsed_path = parse_redaction_path(&rule.path)?;
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
        return Err(errors.join("; "));
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
) -> Result<(), String> {
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

    Err(format!(
        "redaction policy does not protect present sensitive field(s): {}",
        missing_paths.join(", ")
    ))
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

fn field_to_text(field: &Field, delims: &hl7v2::Delims) -> String {
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

fn compute_sha256(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FULL_SENSITIVE_MESSAGE: &str = "MSH|^~\\&|LAB|L|EHR|E|202605030101||ADT^A01|CTRL999|P|2.5\rPID|1||MRN-1^^^HOSP^MR||Doe^John||19700101\r";

    const NON_SENSITIVE_MESSAGE: &str =
        "MSH|^~\\&|LAB|L|EHR|E|202605030101||ADT^A01|CTRL000|P|2.5\rPID|1||||||||M\r";

    const FULL_SENTINEL_POLICY: &str = r#"
[[rules]]
path = "PID.3"
action = "hash"
reason = "patient identifier"

[[rules]]
path = "PID.5"
action = "drop"
reason = "patient name"

[[rules]]
path = "PID.7"
action = "drop"
reason = "date of birth"
"#;

    fn parse_message(text: &str) -> Message {
        hl7v2::parse(text.as_bytes()).expect("test fixture must parse")
    }

    fn render_message(message: &Message) -> String {
        String::from_utf8(hl7v2::write(message)).expect("rendered message must be UTF-8")
    }

    fn action_for<'a>(receipt: &'a RedactionReceipt, path: &str) -> &'a RedactionActionReceipt {
        receipt
            .actions
            .iter()
            .find(|action| action.path == path)
            .expect("receipt must contain an action for the queried path")
    }

    #[test]
    fn empty_policy_text_is_rejected_as_invalid_toml_or_empty_rules() {
        let mut message = parse_message(NON_SENSITIVE_MESSAGE);
        let error = redact_message(&mut message, "").expect_err("empty policy must error");
        assert!(
            error.contains("at least one rule") || error.contains("invalid TOML"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn malformed_toml_returns_invalid_toml_error() {
        let mut message = parse_message(NON_SENSITIVE_MESSAGE);
        let bogus = "this is = = not toml [[";
        let error = redact_message(&mut message, bogus).expect_err("malformed TOML must error");
        assert!(error.contains("invalid TOML"), "unexpected error: {error}");
    }

    #[test]
    fn unknown_action_in_policy_is_rejected_during_parsing() {
        let mut message = parse_message(NON_SENSITIVE_MESSAGE);
        let policy = r#"
[[rules]]
path = "PID.8"
action = "obliterate"
reason = "made up action"
"#;
        let error = redact_message(&mut message, policy).expect_err("unknown action must error");
        assert!(error.contains("invalid TOML"), "unexpected error: {error}");
    }

    #[test]
    fn rule_without_reason_is_rejected() {
        let mut message = parse_message(NON_SENSITIVE_MESSAGE);
        let policy = r#"
[[rules]]
path = "PID.8"
action = "drop"
"#;
        let error =
            redact_message(&mut message, policy).expect_err("missing reason must be rejected");
        assert!(
            error.contains("must include a reason"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn duplicate_rule_paths_are_rejected() {
        let mut message = parse_message(NON_SENSITIVE_MESSAGE);
        let policy = r#"
[[rules]]
path = "PID.8"
action = "drop"
reason = "first"

[[rules]]
path = "PID.8"
action = "drop"
reason = "second"
"#;
        let error = redact_message(&mut message, policy).expect_err("duplicate path must error");
        assert!(
            error.contains("duplicate rule"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn malformed_path_without_dot_returns_typed_error() {
        let mut message = parse_message(NON_SENSITIVE_MESSAGE);
        let policy = r#"
[[rules]]
path = "PID"
action = "drop"
reason = "no field index"
"#;
        let error = redact_message(&mut message, policy).expect_err("missing dot must error");
        assert!(error.contains("SEG.field"), "unexpected error: {error}");
    }

    #[test]
    fn malformed_path_with_empty_field_index_returns_typed_error() {
        let mut message = parse_message(NON_SENSITIVE_MESSAGE);
        let policy = r#"
[[rules]]
path = "PID."
action = "drop"
reason = "trailing dot"
"#;
        let error = redact_message(&mut message, policy).expect_err("empty index must error");
        assert!(
            error.contains("positive numeric field index"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn path_targeting_component_is_rejected() {
        let mut message = parse_message(NON_SENSITIVE_MESSAGE);
        let policy = r#"
[[rules]]
path = "PID.5.1"
action = "drop"
reason = "components not allowed"
"#;
        let error =
            redact_message(&mut message, policy).expect_err("component path must be rejected");
        assert!(
            error.contains("must target a field"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn zero_field_index_is_rejected() {
        let mut message = parse_message(NON_SENSITIVE_MESSAGE);
        let policy = r#"
[[rules]]
path = "PID.0"
action = "drop"
reason = "zero is not one-based"
"#;
        let error = redact_message(&mut message, policy).expect_err("zero index must error");
        assert!(error.contains("one-based"), "unexpected error: {error}");
    }

    #[test]
    fn msh_delimiter_field_paths_are_rejected() {
        let mut message = parse_message(NON_SENSITIVE_MESSAGE);
        let policy = r#"
[[rules]]
path = "MSH.1"
action = "drop"
reason = "delimiters cannot be redacted"
"#;
        let error = redact_message(&mut message, policy).expect_err("MSH.1 must be rejected");
        assert!(
            error.contains("delimiter metadata"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn lowercase_segment_id_is_rejected() {
        let mut message = parse_message(NON_SENSITIVE_MESSAGE);
        let policy = r#"
[[rules]]
path = "pid.5"
action = "drop"
reason = "must be uppercase"
"#;
        let error =
            redact_message(&mut message, policy).expect_err("lowercase segment must be rejected");
        assert!(
            error.contains("three-character uppercase"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn retain_on_built_in_sensitive_field_is_rejected() {
        let mut message = parse_message(NON_SENSITIVE_MESSAGE);
        let policy = r#"
[[rules]]
path = "PID.5"
action = "retain"
reason = "cannot retain sensitive"
"#;
        let error = redact_message(&mut message, policy)
            .expect_err("retaining sensitive field must be rejected");
        assert!(
            error.contains("cannot retain a built-in sensitive field"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn drop_action_clears_pid5_and_leaves_other_fields_intact() {
        let mut message = parse_message(FULL_SENSITIVE_MESSAGE);
        let receipt = redact_message(&mut message, FULL_SENTINEL_POLICY)
            .expect("policy fully covers sentinel message");
        let rendered = render_message(&message);

        assert!(receipt.phi_removed);
        assert_eq!(receipt.hash_algorithm, "sha256");
        assert!(!rendered.contains("Doe^John"));

        let pid5 = action_for(&receipt, "PID.5");
        assert_eq!(pid5.action, RedactionAction::Drop);
        assert_eq!(pid5.status, RedactionActionStatus::Applied);
        assert_eq!(pid5.matched_count, 1);
        assert_eq!(pid5.reason, "patient name");

        let control_id = hl7v2::get(&message, "MSH.10").expect("MSH.10 must survive");
        assert_eq!(control_id, "CTRL999");
    }

    #[test]
    fn hash_action_produces_deterministic_sha256_marker() {
        let mut first = parse_message(FULL_SENSITIVE_MESSAGE);
        let mut second = parse_message(FULL_SENSITIVE_MESSAGE);

        redact_message(&mut first, FULL_SENTINEL_POLICY).expect("first redaction succeeds");
        redact_message(&mut second, FULL_SENTINEL_POLICY).expect("second redaction succeeds");

        let first_pid3 = hl7v2::get(&first, "PID.3").expect("PID.3 hash marker present");
        let second_pid3 = hl7v2::get(&second, "PID.3").expect("PID.3 hash marker present");

        assert!(first_pid3.starts_with("hash:sha256:"));
        assert_eq!(first_pid3, second_pid3, "hash must be deterministic");

        let alt_raw = "MSH|^~\\&|LAB|L|EHR|E|202605030101||ADT^A01|CTRL999|P|2.5\rPID|1||MRN-2^^^HOSP^MR||Doe^John||19700101\r";
        let mut alt = parse_message(alt_raw);
        redact_message(&mut alt, FULL_SENTINEL_POLICY).expect("alt redaction succeeds");
        let alt_pid3 = hl7v2::get(&alt, "PID.3").expect("PID.3 hash marker present");
        assert_ne!(
            first_pid3, alt_pid3,
            "different input must produce different digest"
        );
    }

    #[test]
    fn missing_optional_field_is_a_no_op_with_not_found_status() {
        let mut message = parse_message(NON_SENSITIVE_MESSAGE);
        let policy = r#"
[[rules]]
path = "ZZZ.4"
action = "drop"
reason = "absent segment is fine when optional"
optional = true
"#;
        let receipt =
            redact_message(&mut message, policy).expect("optional absence is not an error");

        assert!(!receipt.phi_removed);
        let entry = action_for(&receipt, "ZZZ.4");
        assert_eq!(entry.matched_count, 0);
        assert_eq!(entry.status, RedactionActionStatus::NotFound);
        assert!(entry.optional);
    }

    #[test]
    fn missing_required_field_returns_error_with_guidance() {
        let mut message = parse_message(NON_SENSITIVE_MESSAGE);
        let policy = r#"
[[rules]]
path = "ZZZ.4"
action = "drop"
reason = "must be marked optional if absence is expected"
"#;
        let error =
            redact_message(&mut message, policy).expect_err("absent non-optional path must error");
        assert!(
            error.contains("matched no fields"),
            "unexpected error: {error}"
        );
        assert!(error.contains("optional=true"), "unexpected error: {error}");
    }

    #[test]
    fn redaction_reason_is_preserved_in_receipt_action() {
        let mut message = parse_message(FULL_SENSITIVE_MESSAGE);
        let receipt = redact_message(&mut message, FULL_SENTINEL_POLICY)
            .expect("policy fully covers sentinel message");

        let pid3 = action_for(&receipt, "PID.3");
        let pid7 = action_for(&receipt, "PID.7");
        assert_eq!(pid3.reason, "patient identifier");
        assert_eq!(pid7.reason, "date of birth");
        assert_eq!(pid3.action, RedactionAction::Hash);
    }

    #[test]
    fn repeating_segments_have_rule_applied_to_each_occurrence() {
        let multi_pid = "MSH|^~\\&|LAB|L|EHR|E|202605030101||ADT^A01|CTRL333|P|2.5\rPID|1||MRN-A^^^HOSP^MR||Smith^Alice||19700101\rPID|2||MRN-B^^^HOSP^MR||Jones^Bob||19800202\r";
        let mut message = parse_message(multi_pid);
        let receipt = redact_message(&mut message, FULL_SENTINEL_POLICY)
            .expect("policy fully covers sentinel message");

        let pid5 = action_for(&receipt, "PID.5");
        assert_eq!(pid5.matched_count, 2);
        let rendered = render_message(&message);
        assert!(!rendered.contains("Smith^Alice"));
        assert!(!rendered.contains("Jones^Bob"));
    }

    #[test]
    fn closed_policy_fails_when_present_sensitive_field_is_not_covered() {
        let mut message = parse_message(FULL_SENSITIVE_MESSAGE);
        let incomplete = r#"
[[rules]]
path = "PID.3"
action = "hash"
reason = "patient identifier"
"#;
        let error =
            redact_message(&mut message, incomplete).expect_err("incomplete policy must error");
        assert!(error.contains("PID.5"), "unexpected error: {error}");
        assert!(
            error.contains("does not protect"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn unicode_field_value_is_preserved_when_not_targeted() {
        let unicode_marker = "héllo-世界";
        let raw = format!(
            "MSH|^~\\&|LAB|L|EHR|E|202605030101||ADT^A01|CTRL555|P|2.5\rPID|1||MRN-9^^^HOSP^MR||Doe^Jane||19700101|{unicode_marker}\r"
        );
        let mut message = parse_message(&raw);
        let receipt = redact_message(&mut message, FULL_SENTINEL_POLICY)
            .expect("policy fully covers sentinel message");

        assert!(receipt.phi_removed);
        let pid8 = hl7v2::get(&message, "PID.8").expect("PID.8 unicode marker survives");
        assert_eq!(pid8, unicode_marker);

        let rendered = render_message(&message);
        assert!(rendered.contains(unicode_marker));
        assert!(!rendered.contains("Doe^Jane"));
        assert!(!rendered.contains("19700101"));
    }
}
