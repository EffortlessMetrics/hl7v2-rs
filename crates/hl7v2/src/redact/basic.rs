use crate::model::Message;

use super::path::{modeled_field_index, parse_redaction_path};
use super::target::replace_redaction_target;
use super::types::RedactionConfig;

/// Redact PHI from a message based on configuration.
pub fn redact(message: &mut Message, config: &RedactionConfig) {
    for path in &config.fields {
        let Ok(parsed_path) = parse_redaction_path(path) else {
            continue;
        };

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
            if let Some(field_index) =
                modeled_field_index(&parsed_path.segment_id, parsed_path.field_index)
                && let Some(field) = segment.fields.get_mut(field_index)
            {
                replace_redaction_target(field, &parsed_path, &config.replacement);
            }
        }
    }
}
