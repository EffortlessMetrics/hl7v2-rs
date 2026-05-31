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
    segment_repetition: Option<usize>,
    field_index: usize,
    canonical_path: String,
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
    let mut policy: SafeAnalysisPolicy = toml::from_str(policy_text)
        .map_err(|error| format!("redaction policy is invalid TOML: {error}"))?;
    if policy.rules.is_empty() {
        return Err("redaction policy must contain at least one rule".to_string());
    }

    let mut seen_paths = BTreeSet::new();
    for rule in &mut policy.rules {
        let parsed_path = parse_redaction_path(&rule.path)?;
        rule.path = parsed_path.canonical_path;
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
    let located = hl7v2::parse_located_path(path).map_err(|error| {
        if !path.contains('.') && !path.contains('-') {
            format!("redaction path '{path}' must use SEG.field or SEG-FIELD syntax")
        } else {
            format!("redaction path '{path}' is invalid: {error}")
        }
    })?;

    if located.path.component.is_some() || located.path.subcomponent.is_some() {
        return Err(format!(
            "redaction path '{path}' must target a field, not a component"
        ));
    }
    if located.path.repetition.is_some() {
        return Err(format!(
            "redaction path '{path}' must target a field, not a field repetition"
        ));
    }
    if located.path.segment == "MSH" && located.path.field < 3 {
        return Err(format!(
            "redaction path '{path}' targets MSH.1/MSH.2, which are delimiter metadata and not redacted by this command"
        ));
    }

    let canonical_path = located.to_path_string();

    Ok(ParsedRedactionPath {
        segment_id: located.path.segment,
        segment_repetition: located.segment_repetition,
        field_index: located.path.field,
        canonical_path,
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
