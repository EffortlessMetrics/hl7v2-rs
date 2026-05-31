//! Safe-analysis redaction helpers for HTTP evidence endpoints.

use crate::models::{
    RedactionAction, RedactionActionReceipt, RedactionActionStatus, RedactionReceipt,
};
use hl7v2::{Atom, Comp, Field, Message, Rep};
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
    field_repetition: Option<usize>,
    component: Option<usize>,
    subcomponent: Option<usize>,
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
        field_repetition: located.path.repetition,
        component: located.path.component,
        subcomponent: located.path.subcomponent,
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

fn apply_redaction_target(
    field: &mut Field,
    path: &ParsedRedactionPath,
    action: RedactionAction,
    delims: &hl7v2::Delims,
) -> bool {
    let Some(target) = select_target(field, path) else {
        return false;
    };

    match action {
        RedactionAction::Hash => target.hash(delims),
        RedactionAction::Drop => target.replace_with_text(String::new()),
        RedactionAction::Retain => {}
    }

    true
}

enum RedactionTarget<'a> {
    Field(&'a mut Field),
    Rep(&'a mut Rep),
    Comp(&'a mut Comp),
    Atom(&'a mut Atom),
}

impl RedactionTarget<'_> {
    fn hash(self, delims: &hl7v2::Delims) {
        let value = match &self {
            Self::Field(field) => field_to_text(field, delims),
            Self::Rep(rep) => rep_to_text(rep, delims),
            Self::Comp(comp) => comp_to_text(comp, delims),
            Self::Atom(atom) => atom_to_text(atom).to_string(),
        };
        self.replace_with_text(format!("hash:sha256:{}", compute_sha256(&value)));
    }

    fn replace_with_text(self, replacement: String) {
        match self {
            Self::Field(field) => {
                *field = Field::from_text(replacement);
            }
            Self::Rep(rep) => {
                *rep = Rep::from_text(replacement);
            }
            Self::Comp(comp) => {
                *comp = Comp::from_text(replacement);
            }
            Self::Atom(atom) => {
                *atom = Atom::Text(replacement);
            }
        }
    }
}

fn select_target<'a>(
    field: &'a mut Field,
    path: &ParsedRedactionPath,
) -> Option<RedactionTarget<'a>> {
    if path.field_repetition.is_none() && path.component.is_none() {
        return Some(RedactionTarget::Field(field));
    }

    let rep_index = path.field_repetition.unwrap_or(1).checked_sub(1)?;
    let rep = field.reps.get_mut(rep_index)?;
    let Some(component) = path.component else {
        return Some(RedactionTarget::Rep(rep));
    };

    let component_index = component.checked_sub(1)?;
    let comp = rep.comps.get_mut(component_index)?;
    let Some(subcomponent) = path.subcomponent else {
        return Some(RedactionTarget::Comp(comp));
    };

    let subcomponent_index = subcomponent.checked_sub(1)?;
    comp.subs
        .get_mut(subcomponent_index)
        .map(RedactionTarget::Atom)
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
        .map(|rep| rep_to_text(rep, delims))
        .collect::<Vec<_>>()
        .join(&delims.rep.to_string())
}

fn rep_to_text(rep: &Rep, delims: &hl7v2::Delims) -> String {
    rep.comps
        .iter()
        .map(|comp| comp_to_text(comp, delims))
        .collect::<Vec<_>>()
        .join(&delims.comp.to_string())
}

fn comp_to_text(comp: &Comp, delims: &hl7v2::Delims) -> String {
    comp.subs
        .iter()
        .map(atom_to_text)
        .collect::<Vec<_>>()
        .join(&delims.sub.to_string())
}

fn atom_to_text(atom: &Atom) -> &str {
    match atom {
        Atom::Text(text) => text.as_str(),
        Atom::Null => "\"\"",
    }
}

fn compute_sha256(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("{:x}", hasher.finalize())
}
