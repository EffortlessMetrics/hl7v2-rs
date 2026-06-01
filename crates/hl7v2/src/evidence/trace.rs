use super::models::{FieldPathTrace, FieldPathTraceReport, FieldValueShape};
use crate::model::{Atom, Field, Message};
use crate::redact::{RedactionAction, RedactionReceipt};
use std::collections::BTreeMap;

pub(crate) fn build_field_path_trace(
    message: &Message,
    receipt: &RedactionReceipt,
) -> FieldPathTraceReport {
    let redaction_actions: Vec<(&str, RedactionAction)> = receipt
        .actions
        .iter()
        .map(|action| (action.path.as_str(), action.action))
        .collect();
    let mut fields = Vec::new();
    let mut segment_occurrences = BTreeMap::<String, usize>::new();

    for (segment_position, segment) in message.segments.iter().enumerate() {
        let segment_index = segment_position.saturating_add(1);
        let segment_occurrence = {
            let count = segment_occurrences
                .entry(segment.id_str().to_string())
                .or_insert(0);
            *count = count.saturating_add(1);
            *count
        };
        for (modeled_index, field) in segment.fields.iter().enumerate() {
            let field_index = hl7_field_index(segment.id_str(), modeled_index);
            let canonical_path = format!("{}.{}", segment.id_str(), field_index);
            let occurrence_path = format!(
                "{}[{}].{}",
                segment.id_str(),
                segment_occurrence,
                field_index
            );
            let field_text = field_to_text(field, &message.delims);
            fields.push(FieldPathTrace {
                path: occurrence_path.clone(),
                canonical_path: canonical_path.clone(),
                segment_index,
                field_index,
                present: !field_text.is_empty(),
                value_shape: field_value_shape(&field_text),
                redaction_action: redaction_action_for_field(
                    &redaction_actions,
                    &occurrence_path,
                    &canonical_path,
                ),
            });
        }
    }

    FieldPathTraceReport {
        message_type: message_type(message),
        field_count: fields.len(),
        fields,
    }
}

fn redaction_action_for_field(
    actions: &[(&str, RedactionAction)],
    occurrence_path: &str,
    canonical_path: &str,
) -> Option<RedactionAction> {
    actions.iter().find_map(|(action_path, action)| {
        (path_targets_field(action_path, occurrence_path)
            || path_targets_field(action_path, canonical_path))
        .then_some(*action)
    })
}

fn path_targets_field(action_path: &str, field_path: &str) -> bool {
    if action_path == field_path {
        return true;
    }

    action_path
        .strip_prefix(field_path)
        .is_some_and(|suffix| suffix.starts_with('.') || suffix.starts_with('['))
}

fn message_type(message: &Message) -> String {
    let message_code = crate::get(message, "MSH.9.1")
        .or_else(|| crate::get(message, "MSH.9"))
        .unwrap_or("unknown");
    let trigger_event = crate::get(message, "MSH.9.2");

    trigger_event.map_or_else(
        || message_code.to_string(),
        |event| format!("{message_code}^{event}"),
    )
}

fn hl7_field_index(segment_id: &str, modeled_index: usize) -> usize {
    if segment_id == "MSH" {
        modeled_index.saturating_add(2)
    } else {
        modeled_index.saturating_add(1)
    }
}

fn field_value_shape(field_text: &str) -> FieldValueShape {
    if field_text.is_empty() {
        FieldValueShape::Empty
    } else if field_text.starts_with("hash:sha256:") {
        FieldValueShape::HashedSha256
    } else {
        FieldValueShape::Present
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
