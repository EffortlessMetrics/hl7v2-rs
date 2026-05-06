//! PHI redaction for HL7 messages.
//!
//! This module provides functionality for identifying and redacting
//! Personally Identifiable Information (PII) and Protected Health
//! Information (PHI) from HL7 v2 messages.

use crate::model::{Field, Message, Segment};

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
    use super::{RedactionConfig, parse_segment_field_path, redact};
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
}
