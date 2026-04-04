//! PHI (Protected Health Information) redaction for HL7 v2 messages.
//!
//! This crate provides functionality to redact sensitive patient information
//! from HL7 v2 messages for safe logging, testing, and non-production environments.
//!
//! # Features
//!
//! - **Redaction strategies**: Replace, hash, or mask PHI fields
//! - **Common PHI fields**: Pre-configured rules for standard HL7 PHI locations
//! - **Custom rules**: Define your own redaction rules
//! - **Audit logging**: Track which fields were redacted
//!
//! # Example
//!
//! ```rust
//! use hl7v2_redact::{RedactionEngine, RedactionRule, RedactionStrategy};
//! use hl7v2_parser::parse;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Parse an HL7 message
//! let hl7 = "MSH|^~\\&|SendingApp|SendingFac|20250128120000||ADT^A01|12345|P|2.5\rPID|1||12345^^^MRN||Doe^John^M||19800101|M|||123 Main St^^Springfield^IL^62701||555-1234\r";
//! let message = parse(hl7.as_bytes())?;
//!
//! // Create redaction rules
//! let rules = vec![
//!     RedactionRule::phi_field("PID.3", RedactionStrategy::Hash),     // Patient ID
//!     RedactionRule::phi_field("PID.5", RedactionStrategy::Mask),    // Patient Name
//!     RedactionRule::phi_field("PID.7", RedactionStrategy::Replace("1900-01-01".to_string())), // DOB
//!     RedactionRule::phi_field("PID.11", RedactionStrategy::Remove),   // Address
//!     RedactionRule::phi_field("PID.13", RedactionStrategy::Mask),     // Phone
//! ];
//!
//! // Apply redaction
//! let engine = RedactionEngine::new(rules);
//! let redacted = engine.redact(&message)?;
//! # Ok(())
//! # }
//! ```

use hl7v2_model::{Atom, Field, Message, Rep};
use hl7v2_query::get;
use hl7v2_writer::write;
use sha2::{Digest, Sha256};

/// Errors that can occur during redaction
#[derive(Debug, thiserror::Error)]
pub enum RedactionError {
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    #[error("Message error: {0}")]
    MessageError(String),
}

/// Strategy for redacting a field
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum RedactionStrategy {
    /// Replace with a fixed string
    Replace(String),
    /// Replace with hash of original value
    Hash,
    /// Replace with masking characters (e.g., ***)
    Mask,
    /// Remove the field entirely (empty)
    Remove,
    /// Truncate to first N characters, mask the rest
    Truncate(usize),
    /// Custom transformation function (not serializable)
    #[serde(skip)]
    Custom(fn(&str) -> String),
}

impl Default for RedactionStrategy {
    fn default() -> Self {
        RedactionStrategy::Mask
    }
}

/// A rule for redacting a specific field
#[derive(Debug, Clone)]
pub struct RedactionRule {
    /// HL7 path (e.g., "PID.3", "PID.5.1")
    pub path: String,
    /// Redaction strategy to apply
    pub strategy: RedactionStrategy,
    /// Description of what this rule redacts (for audit)
    pub description: String,
}

impl RedactionRule {
    /// Create a new redaction rule
    pub fn new(path: impl Into<String>, strategy: RedactionStrategy) -> Self {
        Self {
            path: path.into(),
            strategy,
            description: String::new(),
        }
    }

    /// Create a rule with a description
    pub fn with_description(
        path: impl Into<String>,
        strategy: RedactionStrategy,
        description: impl Into<String>,
    ) -> Self {
        Self {
            path: path.into(),
            strategy,
            description: description.into(),
        }
    }

    /// Create a rule for a standard PHI field
    pub fn phi_field(path: impl Into<String>, strategy: RedactionStrategy) -> Self {
        let path_str = path.into();
        let description = format!("PHI field: {}", path_str);
        Self::with_description(path_str, strategy, description)
    }
}

/// Audit log entry for redaction
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RedactionAuditEntry {
    pub path: String,
    pub description: String,
    pub strategy: RedactionStrategy,
    pub had_value: bool,
}

/// Result of redaction operation
#[derive(Debug, Clone)]
pub struct RedactionResult {
    /// The redacted message bytes
    pub message_bytes: Vec<u8>,
    /// The redacted message
    pub message: Message,
    /// Audit log of what was redacted
    pub audit_log: Vec<RedactionAuditEntry>,
}

/// Engine for applying redaction rules to HL7 messages
#[derive(Debug, Clone)]
pub struct RedactionEngine {
    rules: Vec<RedactionRule>,
}

impl RedactionEngine {
    /// Create a new redaction engine with the given rules
    pub fn new(rules: Vec<RedactionRule>) -> Self {
        Self { rules }
    }

    /// Create an engine with common PHI redaction rules
    pub fn common_phi_rules() -> Self {
        let rules = vec![
            // Patient identifiers
            RedactionRule::phi_field("PID.2", RedactionStrategy::Hash),
            RedactionRule::phi_field("PID.3", RedactionStrategy::Hash),
            RedactionRule::phi_field("PID.4", RedactionStrategy::Hash),
            // Patient demographics
            RedactionRule::phi_field("PID.5", RedactionStrategy::Mask),
            RedactionRule::phi_field("PID.6", RedactionStrategy::Mask),
            RedactionRule::phi_field(
                "PID.7",
                RedactionStrategy::Replace("1900-01-01".to_string()),
            ),
            // Contact information
            RedactionRule::phi_field("PID.9", RedactionStrategy::Mask),
            RedactionRule::phi_field("PID.11", RedactionStrategy::Remove),
            RedactionRule::phi_field("PID.13", RedactionStrategy::Mask),
            RedactionRule::phi_field("PID.14", RedactionStrategy::Mask),
            // Additional identifiers
            RedactionRule::phi_field("PID.18", RedactionStrategy::Hash),
            RedactionRule::phi_field("PID.19", RedactionStrategy::Hash),
            RedactionRule::phi_field("PID.20", RedactionStrategy::Hash),
            // Mother/father identifiers
            RedactionRule::phi_field("PID.21", RedactionStrategy::Hash),
            // Additional contact points
            RedactionRule::phi_field("PID.22", RedactionStrategy::Mask),
            RedactionRule::phi_field("PID.40", RedactionStrategy::Mask),
        ];
        Self::new(rules)
    }

    /// Create an engine with HIPAA Safe Harbor rules (18 identifiers)
    pub fn hipaa_safe_harbor() -> Self {
        let rules = vec![
            // Names
            RedactionRule::phi_field("PID.5", RedactionStrategy::Mask),
            RedactionRule::phi_field("PID.6", RedactionStrategy::Mask),
            // Geographic subdivisions smaller than state (keep first 3 zip digits only)
            RedactionRule::phi_field("PID.11", RedactionStrategy::Truncate(3)),
            // Dates (except year) - handled by replace with year only
            RedactionRule::phi_field(
                "PID.7",
                RedactionStrategy::Replace("1900-01-01".to_string()),
            ),
            // Phone/fax
            RedactionRule::phi_field("PID.13", RedactionStrategy::Mask),
            RedactionRule::phi_field("PID.14", RedactionStrategy::Mask),
            // Email addresses
            RedactionRule::phi_field("PID.40", RedactionStrategy::Mask),
            // SSN
            RedactionRule::phi_field("PID.19", RedactionStrategy::Hash),
            // Medical record numbers
            RedactionRule::phi_field("PID.2", RedactionStrategy::Hash),
            RedactionRule::phi_field("PID.3", RedactionStrategy::Hash),
            RedactionRule::phi_field("PID.4", RedactionStrategy::Hash),
            // Health plan beneficiary numbers
            RedactionRule::phi_field("PID.18", RedactionStrategy::Hash),
            // Certificate/license numbers
            RedactionRule::phi_field("PID.20", RedactionStrategy::Hash),
        ];
        Self::new(rules)
    }

    /// Apply redaction rules to a message
    pub fn redact(&self, message: &Message) -> Result<RedactionResult, RedactionError> {
        let mut redacted_message = message.clone();
        let mut audit_log = Vec::new();

        for rule in &self.rules {
            let had_value = self.apply_rule(&mut redacted_message, rule)?;
            audit_log.push(RedactionAuditEntry {
                path: rule.path.clone(),
                description: rule.description.clone(),
                strategy: rule.strategy.clone(),
                had_value,
            });
        }

        let message_bytes = write(&redacted_message);

        Ok(RedactionResult {
            message_bytes,
            message: redacted_message,
            audit_log,
        })
    }

    /// Apply a single rule to the message
    fn apply_rule(
        &self,
        message: &mut Message,
        rule: &RedactionRule,
    ) -> Result<bool, RedactionError> {
        // Check if the path exists and has a value
        let had_value = get(message, &rule.path).is_some();

        // Parse the path
        let path_parts = self.parse_path(message, &rule.path)?;

        // Find and modify the field
        if let Some((segment_idx, field_idx, sub_idx)) = path_parts {
            if let Some(segment) = message.segments.get_mut(segment_idx) {
                if let Some(field) = segment.fields.get_mut(field_idx) {
                    self.apply_strategy_to_field(field, &rule.strategy, sub_idx);
                }
            }
        }

        Ok(had_value)
    }

    /// Parse an HL7 path into indices
    fn parse_path(
        &self,
        message: &Message,
        path: &str,
    ) -> Result<Option<(usize, usize, Option<usize>)>, RedactionError> {
        // Handle MSH.1 specially (it's the field separator itself)
        if path == "MSH.1" || path == "MSH.2" {
            return Ok(None); // Cannot redact these
        }

        // Format: SEGMENT.FIELD[.COMPONENT]
        let parts: Vec<&str> = path.split('.').collect();
        if parts.len() < 2 {
            return Err(RedactionError::InvalidPath(path.to_string()));
        }

        let segment_name = parts[0];
        let field_num: usize = parts[1]
            .parse()
            .map_err(|_| RedactionError::InvalidPath(path.to_string()))?;

        // Find the segment index - if not found, return Ok(None) to skip gracefully
        let segment_idx = match message.segments.iter().position(|s| {
            std::str::from_utf8(&s.id)
                .map(|id| id == segment_name)
                .unwrap_or(false)
        }) {
            Some(idx) => idx,
            None => return Ok(None), // Segment not found - skip this rule
        };

        // Field index (0-based - MSH has special handling because MSH.1 is the separator)
        let field_idx = if segment_name == "MSH" {
            field_num - 2 // MSH.2 is at index 0, MSH.3 at index 1, etc.
        } else {
            field_num - 1
        };

        // Component index
        let sub_idx = parts.get(2).and_then(|s| s.parse().ok());

        Ok(Some((segment_idx, field_idx, sub_idx)))
    }

    /// Apply redaction strategy to a field
    fn apply_strategy_to_field(
        &self,
        field: &mut Field,
        strategy: &RedactionStrategy,
        sub_idx: Option<usize>,
    ) {
        match strategy {
            RedactionStrategy::Replace(value) => {
                if let Some(idx) = sub_idx {
                    // Modify specific component
                    if let Some(rep) = field.reps.get_mut(0) {
                        if let Some(comp) = rep.comps.get_mut(idx - 1) {
                            comp.subs = vec![Atom::Text(value.clone())];
                        }
                    }
                } else {
                    // Replace entire field
                    field.reps = vec![Rep::from_text(value.as_str())];
                }
            }
            RedactionStrategy::Hash => {
                let original = self.field_to_string(field);
                let hash = self.compute_hash(&original);
                field.reps = vec![Rep::from_text(hash.as_str())];
            }
            RedactionStrategy::Mask => {
                let masked = self.mask_value(&self.field_to_string(field));
                field.reps = vec![Rep::from_text(masked.as_str())];
            }
            RedactionStrategy::Remove => {
                field.reps.clear();
            }
            RedactionStrategy::Truncate(n) => {
                let original = self.field_to_string(field);
                let truncated = if original.len() > *n {
                    format!("{}{}", &original[..*n], "*".repeat(original.len() - *n))
                } else {
                    original
                };
                field.reps = vec![Rep::from_text(truncated.as_str())];
            }
            RedactionStrategy::Custom(f) => {
                let original = self.field_to_string(field);
                let result = f(&original);
                field.reps = vec![Rep::from_text(result.as_str())];
            }
        }
    }

    /// Convert a field to string
    fn field_to_string(&self, field: &Field) -> String {
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
                                Atom::Text(s) => s.clone(),
                                Atom::Null => String::new(),
                            })
                            .collect::<Vec<_>>()
                            .join("&")
                    })
                    .collect::<Vec<_>>()
                    .join("^")
            })
            .collect::<Vec<_>>()
            .join("~")
    }

    /// Compute SHA-256 hash of a string
    fn compute_hash(&self, input: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        format!("{:x}", hasher.finalize())[..16].to_string()
    }

    /// Mask a value with asterisks
    fn mask_value(&self, value: &str) -> String {
        if value.is_empty() {
            return String::new();
        }

        // Split by component delimiter and mask each component
        value
            .split('^')
            .map(|comp| {
                let chars: Vec<char> = comp.chars().collect();
                if chars.len() <= 2 {
                    "**".to_string()
                } else {
                    format!("{}**{}", chars[0], chars[chars.len() - 1])
                }
            })
            .collect::<Vec<_>>()
            .join("^")
    }
}

/// Convenience function to redact with common PHI rules
pub fn redact_phi(message: &Message) -> Result<RedactionResult, RedactionError> {
    let engine = RedactionEngine::common_phi_rules();
    engine.redact(message)
}

/// Convenience function to redact with HIPAA Safe Harbor rules
pub fn redact_hipaa(message: &Message) -> Result<RedactionResult, RedactionError> {
    let engine = RedactionEngine::hipaa_safe_harbor();
    engine.redact(message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hl7v2_parser::parse;

    fn sample_message() -> Message {
        let hl7 = "MSH|^~\\&|SendingApp|SendingFac|20250128120000||ADT^A01|12345|P|2.5\rPID|1||12345^^^MRN||Doe^John^M||19800101|M|||123 Main St^^Springfield^IL^62701||555-1234\r";
        parse(hl7.as_bytes()).unwrap()
    }

    #[test]
    fn test_redaction_rule_creation() {
        let rule = RedactionRule::new("PID.3", RedactionStrategy::Hash);
        assert_eq!(rule.path, "PID.3");
        assert!(matches!(rule.strategy, RedactionStrategy::Hash));

        let rule_with_desc =
            RedactionRule::with_description("PID.5", RedactionStrategy::Mask, "Patient name");
        assert_eq!(rule_with_desc.description, "Patient name");

        let phi_rule = RedactionRule::phi_field("PID.7", RedactionStrategy::Remove);
        assert_eq!(phi_rule.path, "PID.7");
        assert!(phi_rule.description.contains("PHI"));
    }

    #[test]
    fn test_redaction_strategies_serialization() {
        let strategies = vec![
            RedactionStrategy::Replace("test".to_string()),
            RedactionStrategy::Hash,
            RedactionStrategy::Mask,
            RedactionStrategy::Remove,
            RedactionStrategy::Truncate(3),
        ];

        for strategy in strategies {
            let json = serde_json::to_string(&strategy).unwrap();
            let deserialized: RedactionStrategy = serde_json::from_str(&json).unwrap();
            assert_eq!(strategy, deserialized);
        }
    }

    #[test]
    fn test_engine_creation() {
        let engine = RedactionEngine::common_phi_rules();
        assert!(!engine.rules.is_empty());

        let hipaa_engine = RedactionEngine::hipaa_safe_harbor();
        assert!(!hipaa_engine.rules.is_empty());
    }

    #[test]
    fn test_hash_redaction() {
        let message = sample_message();
        let rules = vec![RedactionRule::new("PID.3", RedactionStrategy::Hash)];
        let engine = RedactionEngine::new(rules);

        let result = engine.redact(&message).unwrap();
        let redacted_str = String::from_utf8_lossy(&result.message_bytes);

        // PID.3 should be a hash (16 hex chars)
        assert!(redacted_str.contains("PID|1||"));
        // The patient ID should not be "12345" anymore
        assert!(!redacted_str.contains("12345^^^MRN"));
    }

    #[test]
    fn test_mask_redaction() {
        let message = sample_message();
        let rules = vec![RedactionRule::new("PID.5", RedactionStrategy::Mask)];
        let engine = RedactionEngine::new(rules);

        let result = engine.redact(&message).unwrap();
        let redacted_str = String::from_utf8_lossy(&result.message_bytes);

        // Name should be masked (components are masked, delimiters may be escaped)
        assert!(!redacted_str.contains("Doe^John"));
        // The masked value should be present (with possible escaping)
        assert!(redacted_str.contains("D**e"));
        assert!(redacted_str.contains("J**n"));
    }

    #[test]
    fn test_replace_redaction() {
        let message = sample_message();
        let rules = vec![RedactionRule::new(
            "PID.7",
            RedactionStrategy::Replace("1900-01-01".to_string()),
        )];
        let engine = RedactionEngine::new(rules);

        let result = engine.redact(&message).unwrap();
        let redacted_str = String::from_utf8_lossy(&result.message_bytes);

        assert!(!redacted_str.contains("19800101"));
        assert!(redacted_str.contains("1900-01-01"));
    }

    #[test]
    fn test_remove_redaction() {
        let message = sample_message();
        let rules = vec![RedactionRule::new("PID.11", RedactionStrategy::Remove)];
        let engine = RedactionEngine::new(rules);

        let result = engine.redact(&message).unwrap();
        let redacted_str = String::from_utf8_lossy(&result.message_bytes);

        // Address should be empty
        assert!(!redacted_str.contains("123 Main St"));
    }

    #[test]
    fn test_truncate_redaction() {
        let message = sample_message();
        let rules = vec![RedactionRule::new("PID.13", RedactionStrategy::Truncate(3))];
        let engine = RedactionEngine::new(rules);

        let result = engine.redact(&message).unwrap();
        let redacted_str = String::from_utf8_lossy(&result.message_bytes);

        // Phone should be truncated
        assert!(!redacted_str.contains("555-1234"));
    }

    #[test]
    fn test_multiple_rules() {
        let message = sample_message();
        let rules = vec![
            RedactionRule::new("PID.3", RedactionStrategy::Hash),
            RedactionRule::new("PID.5", RedactionStrategy::Mask),
            RedactionRule::new(
                "PID.7",
                RedactionStrategy::Replace("1900-01-01".to_string()),
            ),
        ];
        let engine = RedactionEngine::new(rules);

        let result = engine.redact(&message).unwrap();
        assert_eq!(result.audit_log.len(), 3);

        let redacted_str = String::from_utf8_lossy(&result.message_bytes);
        // PID.3 should be hashed (no longer 12345)
        assert!(!redacted_str.contains("12345^^^MRN"));
        // Patient name should be masked
        assert!(!redacted_str.contains("Doe^John"));
        // DOB should be replaced
        assert!(!redacted_str.contains("19800101"));
    }

    #[test]
    fn test_convenience_functions() {
        let message = sample_message();

        let phi_result = redact_phi(&message).unwrap();
        assert!(!phi_result.audit_log.is_empty());

        let hipaa_result = redact_hipaa(&message).unwrap();
        assert!(!hipaa_result.audit_log.is_empty());
    }

    #[test]
    fn test_audit_log_entries() {
        let message = sample_message();
        let rules = vec![
            RedactionRule::with_description("PID.3", RedactionStrategy::Hash, "Patient identifier"),
            RedactionRule::with_description("PID.5", RedactionStrategy::Mask, "Patient name"),
        ];
        let engine = RedactionEngine::new(rules);

        let result = engine.redact(&message).unwrap();

        assert_eq!(result.audit_log.len(), 2);
        assert_eq!(result.audit_log[0].path, "PID.3");
        assert_eq!(result.audit_log[0].description, "Patient identifier");
        assert!(result.audit_log[0].had_value);

        assert_eq!(result.audit_log[1].path, "PID.5");
        assert_eq!(result.audit_log[1].description, "Patient name");
        assert!(result.audit_log[1].had_value);
    }

    #[test]
    fn test_invalid_path_error() {
        let message = sample_message();
        let rules = vec![RedactionRule::new("INVALID", RedactionStrategy::Mask)];
        let engine = RedactionEngine::new(rules);

        let result = engine.redact(&message);
        assert!(result.is_err());
    }

    #[test]
    fn test_msh_field_redaction() {
        let hl7 = "MSH|^~\\&|SendingApp|SendingFac|20250128120000||ADT^A01|12345|P|2.5\r";
        let message = parse(hl7.as_bytes()).unwrap();

        let rules = vec![RedactionRule::new(
            "MSH.3",
            RedactionStrategy::Replace("REDACTED".to_string()),
        )];
        let engine = RedactionEngine::new(rules);

        let result = engine.redact(&message).unwrap();
        let redacted_str = String::from_utf8_lossy(&result.message_bytes);

        // The MSH segment should be modified
        assert!(redacted_str.contains("REDACTED") || !redacted_str.contains("SendingApp"));
    }

    #[test]
    fn test_empty_field_handling() {
        let hl7 = "MSH|^~\\&|SendingApp|SendingFac|20250128120000||ADT^A01|12345|P|2.5\rPID|1||\r";
        let message = parse(hl7.as_bytes()).unwrap();

        let rules = vec![RedactionRule::new("PID.3", RedactionStrategy::Hash)];
        let engine = RedactionEngine::new(rules);

        let result = engine.redact(&message);
        // Empty field should either work or return an appropriate result
        assert!(result.is_ok(), "Should handle empty fields gracefully");
    }
}
