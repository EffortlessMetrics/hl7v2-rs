use std::collections::BTreeSet;

use crate::model::Message;

use super::path::{modeled_field_index, parse_redaction_path};
use super::target::apply_redaction_target;
use super::text::field_to_text;
use super::types::{
    RedactionAction, RedactionActionReceipt, RedactionActionStatus, RedactionError,
    RedactionReceipt, SafeAnalysisPolicy,
};

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
    let mut policy: SafeAnalysisPolicy = toml::from_str(policy_text).map_err(|error| {
        RedactionError::Policy(format!("redaction policy is invalid TOML: {error}"))
    })?;
    if policy.rules.is_empty() {
        return Err(RedactionError::Policy(
            "redaction policy must contain at least one rule".to_string(),
        ));
    }

    let sensitive_paths = safe_analysis_sensitive_paths();
    let mut seen_paths = BTreeSet::new();
    for rule in &mut policy.rules {
        let parsed_path = parse_redaction_path(&rule.path).map_err(RedactionError::Policy)?;
        rule.path = parsed_path.canonical_path;
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
        if sensitive_paths.contains(rule.path.as_str()) && rule.action == RedactionAction::Retain {
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
        let mut segment_match_count = 0_usize;

        for segment in &mut message.segments {
            if segment.id_str() != parsed_path.segment_id {
                continue;
            }
            segment_match_count = segment_match_count.saturating_add(1);
            if let Some(segment_repetition) = parsed_path.segment_repetition
                && segment_match_count != segment_repetition
            {
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

            if apply_redaction_target(field, &parsed_path, rule.action, &message.delims) {
                matched_count = matched_count.saturating_add(1);
                if rule.action != RedactionAction::Retain {
                    phi_removed = true;
                }
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
