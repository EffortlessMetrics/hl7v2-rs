use crate::model::{Field, Message, Segment};

use super::path::parse_segment_field_path;
use super::types::RedactionConfig;

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
