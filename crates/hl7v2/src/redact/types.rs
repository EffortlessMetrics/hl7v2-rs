use serde::{Deserialize, Serialize};

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

/// Output from applying a safe-analysis redaction policy to raw HL7.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SafeAnalysisRedactionOutput {
    /// SHA-256 digest of the original input message.
    pub input_sha256: String,
    /// SHA-256 digest of the policy TOML.
    pub policy_sha256: String,
    /// Message type from `MSH.9`, such as `ADT^A01`.
    pub message_type: String,
    /// Redacted HL7 message.
    pub redacted_hl7: String,
    /// Receipt describing the redaction actions applied.
    pub receipt: RedactionReceipt,
}

impl SafeAnalysisRedactionOutput {
    /// Convert this redaction output to the v2 evidence contract with embedded
    /// tool provenance.
    #[must_use]
    pub fn to_v2(
        &self,
        tool_name: impl Into<String>,
        tool_version: impl Into<String>,
    ) -> SafeAnalysisRedactionOutputV2 {
        let tool_name = tool_name.into();
        let tool_version = tool_version.into();
        SafeAnalysisRedactionOutputV2 {
            schema_version: "2".to_string(),
            tool_name: tool_name.clone(),
            tool_version: tool_version.clone(),
            input_sha256: self.input_sha256.clone(),
            policy_sha256: self.policy_sha256.clone(),
            message_type: self.message_type.clone(),
            redacted_hl7: self.redacted_hl7.clone(),
            receipt: self.receipt.to_v2(tool_name, tool_version),
        }
    }
}

/// Safe-analysis redaction output v2 with embedded evidence provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SafeAnalysisRedactionOutputV2 {
    /// Evidence schema version.
    pub schema_version: String,
    /// Tool or binding that produced the redaction output.
    pub tool_name: String,
    /// Producer package version.
    pub tool_version: String,
    /// SHA-256 digest of the original input message.
    pub input_sha256: String,
    /// SHA-256 digest of the policy TOML.
    pub policy_sha256: String,
    /// Message type from `MSH.9`, such as `ADT^A01`.
    pub message_type: String,
    /// Redacted HL7 message.
    pub redacted_hl7: String,
    /// Receipt describing the redaction actions applied.
    pub receipt: RedactionReceiptV2,
}

/// Redaction receipt compatible with safe-analysis evidence artifacts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactionReceipt {
    /// Whether any configured PHI-bearing field was removed or hashed.
    pub phi_removed: bool,
    /// Hash algorithm used by hash redaction actions.
    pub hash_algorithm: String,
    /// Per-rule redaction receipts.
    pub actions: Vec<RedactionActionReceipt>,
}

impl RedactionReceipt {
    /// Convert this receipt to the v2 evidence contract with embedded tool
    /// provenance.
    #[must_use]
    pub fn to_v2(
        &self,
        tool_name: impl Into<String>,
        tool_version: impl Into<String>,
    ) -> RedactionReceiptV2 {
        RedactionReceiptV2 {
            schema_version: "2".to_string(),
            tool_name: tool_name.into(),
            tool_version: tool_version.into(),
            phi_removed: self.phi_removed,
            hash_algorithm: self.hash_algorithm.clone(),
            actions: self.actions.clone(),
        }
    }
}

/// Redaction receipt v2 with embedded evidence provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactionReceiptV2 {
    /// Evidence schema version.
    pub schema_version: String,
    /// Tool or binding that produced the receipt.
    pub tool_name: String,
    /// Producer package version.
    pub tool_version: String,
    /// Whether any configured PHI-bearing field was removed or hashed.
    pub phi_removed: bool,
    /// Hash algorithm used by hash redaction actions.
    pub hash_algorithm: String,
    /// Per-rule redaction receipts.
    pub actions: Vec<RedactionActionReceipt>,
}

/// Per-rule redaction action receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactionActionReceipt {
    /// HL7 path covered by this policy action.
    pub path: String,
    /// Policy action applied to this path.
    pub action: RedactionAction,
    /// Policy reason for the action.
    pub reason: String,
    /// Number of matching values affected by this action.
    pub matched_count: usize,
    /// Whether missing matches are acceptable.
    pub optional: bool,
    /// Action status.
    pub status: RedactionActionStatus,
}

/// Safe-analysis redaction action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RedactionAction {
    /// Replace a field with a deterministic SHA-256 hash marker.
    Hash,
    /// Clear the field value.
    Drop,
    /// Keep a non-sensitive field unchanged.
    Retain,
}

/// Redaction action status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RedactionActionStatus {
    /// Action was applied to at least one field.
    Applied,
    /// Retain action matched at least one field.
    Retained,
    /// Optional action did not match a field.
    NotFound,
}

/// Parsed safe-analysis redaction policy.
#[derive(Debug, Clone, Deserialize)]
pub struct SafeAnalysisPolicy {
    pub(crate) rules: Vec<SafeAnalysisPolicyRule>,
}

/// One rule in a safe-analysis redaction policy.
#[derive(Debug, Clone, Deserialize)]
pub struct SafeAnalysisPolicyRule {
    pub(crate) path: String,
    pub(crate) action: RedactionAction,
    #[serde(default)]
    pub(crate) reason: Option<String>,
    #[serde(default)]
    pub(crate) optional: bool,
}

/// Safe-analysis redaction error.
#[derive(Debug, thiserror::Error)]
pub enum RedactionError {
    /// Input message could not be parsed.
    #[error("parse error: {0}")]
    Parse(String),
    /// Redacted output could not be encoded as UTF-8.
    #[error("redacted message was not UTF-8: {0}")]
    Utf8(String),
    /// Policy TOML or policy semantics were invalid.
    #[error("{0}")]
    Policy(String),
}
