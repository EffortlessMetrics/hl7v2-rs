//! PHI redaction for HL7 messages.
//!
//! This crate provides functionality for identifying and redacting
//! Personally Identifiable Information (PII) and Protected Health
//! Information (PHI) from HL7 v2 messages.

use hl7v2_core::{Message, Segment};
use hl7v2_model::{Delims, Field};

/// Configuration for redaction
#[derive(Debug, Clone, Default)]
pub struct RedactionConfig {
    /// Replacement string for redacted fields
    pub replacement: String,
    /// List of field paths to redact (e.g., "PID.5", "PID.7")
    pub fields: Vec<String>,
}

impl RedactionConfig {
    /// Create a new redaction configuration with default HIPAA fields
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

/// Redact PHI from a message based on configuration
pub fn redact(message: &mut Message, config: &RedactionConfig) {
    for path in &config.fields {
        // Simple path parsing (only segment.field for now)
        let parts: Vec<&str> = path.split('.').collect();
        if parts.len() == 2 {
            let segment_id = parts[0];
            if let Ok(field_idx) = parts[1].parse::<usize>() {
                for segment in &mut message.segments {
                    if String::from_utf8_lossy(&segment.id) == segment_id {
                        redact_field(segment, field_idx, &config.replacement, &message.delims);
                    }
                }
            }
        }
    }
}

fn redact_field(segment: &mut Segment, field_idx: usize, replacement: &str, _delims: &Delims) {
    if field_idx > 0 && field_idx <= segment.fields.len() {
        segment.fields[field_idx - 1] = Field::from_text(replacement);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hl7v2_core::parse;

    #[test]
    fn test_redaction() {
        let hl7 = b"MSH|^~\\&|SENDER|FACILITY\rPID|1||123456^^^HOSP^MR||Doe^John||19800101|M\r";
        let mut message = parse(hl7).unwrap();

        let mut config = RedactionConfig::default();
        config.fields.push("PID.5".to_string());
        config.replacement = "XXX".to_string();

        redact(&mut message, &config);

        let pid = message.segments.iter().find(|s| &s.id == b"PID").unwrap();
        assert_eq!(pid.fields[4].first_text(), Some("XXX"));
    }
}
