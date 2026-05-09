//! Request and response models for the HTTP API.
//!
//! These models follow JSON:API conventions where appropriate and align
//! with the OpenAPI specification in `api/openapi/hl7v2-api-v1.yaml`.

use serde::{Deserialize, Serialize};

use hl7v2::{ValidationReport, ValidationReportIssue};

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Service status
    pub status: HealthStatus,
    /// Service version
    pub version: String,
    /// Uptime in seconds
    pub uptime_seconds: u64,
}

/// Readiness check response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadyResponse {
    /// Whether the service is ready to receive traffic.
    pub ready: bool,
    /// Overall readiness status.
    pub status: ReadinessStatus,
    /// Server package version.
    pub version: String,
    /// Individual readiness checks.
    pub checks: Vec<ReadinessCheck>,
}

impl ReadyResponse {
    /// Build a readiness response from individual checks.
    pub fn from_checks(checks: Vec<ReadinessCheck>) -> Self {
        let ready = checks
            .iter()
            .all(|check| check.status == ReadinessCheckStatus::Pass);

        Self {
            ready,
            status: if ready {
                ReadinessStatus::Ready
            } else {
                ReadinessStatus::NotReady
            },
            version: env!("CARGO_PKG_VERSION").to_string(),
            checks,
        }
    }
}

/// Overall readiness status.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessStatus {
    /// All required checks passed.
    Ready,
    /// At least one required check failed.
    NotReady,
}

/// Individual readiness check.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReadinessCheck {
    /// Stable check name.
    pub name: String,
    /// Check status.
    pub status: ReadinessCheckStatus,
    /// Human-readable diagnostic message.
    pub message: String,
}

impl ReadinessCheck {
    /// Build a passing readiness check.
    pub fn pass(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: ReadinessCheckStatus::Pass,
            message: message.into(),
        }
    }

    /// Build a failing readiness check.
    pub fn fail(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: ReadinessCheckStatus::Fail,
            message: message.into(),
        }
    }
}

/// Individual readiness check status.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReadinessCheckStatus {
    /// Check passed.
    Pass,
    /// Check failed.
    Fail,
}

/// Health status enum
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// Service is healthy
    Healthy,
    /// Service is degraded but functional
    Degraded,
    /// Service is unhealthy
    Unhealthy,
}

/// Parse request body
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseRequest {
    /// Raw HL7 message content (can be MLLP framed or plain)
    pub message: String,
    /// Whether the message is MLLP framed
    #[serde(default)]
    pub mllp_framed: bool,
    /// Options for parsing
    #[serde(default)]
    pub options: ParseOptions,
}

/// Parse options
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParseOptions {
    /// Return JSON representation of message
    #[serde(default = "default_true")]
    pub include_json: bool,
    /// Validate structure (segment IDs, delimiters)
    #[serde(default = "default_true")]
    pub validate_structure: bool,
}

fn default_true() -> bool {
    true
}

/// Parse response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseResponse {
    /// Parsed message in JSON format
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<serde_json::Value>,
    /// Message metadata
    pub metadata: MessageMetadata,
    /// Parsing warnings (if any)
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub warnings: Vec<String>,
}

/// Message metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMetadata {
    /// Message type (e.g., "ADT^A01")
    pub message_type: String,
    /// HL7 version (e.g., "2.5")
    pub version: String,
    /// Sending application
    pub sending_application: String,
    /// Sending facility
    pub sending_facility: String,
    /// Message control ID
    pub message_control_id: String,
    /// Number of segments
    pub segment_count: usize,
    /// Character sets used
    pub charsets: Vec<String>,
}

/// Validate request body
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateRequest {
    /// Raw HL7 message content
    pub message: String,
    /// Inline profile YAML content to validate against
    pub profile: String,
    /// Whether the message is MLLP framed
    #[serde(default)]
    pub mllp_framed: bool,
}

/// Validate response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateResponse {
    /// Whether validation passed
    pub valid: bool,
    /// HL7 trigger event from `MSH.9`, such as `ADT^A01`.
    pub message_type: String,
    /// Profile identifier, usually the loaded profile message structure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    /// Number of parsed message segments.
    pub segment_count: usize,
    /// Number of reported validation issues.
    pub issue_count: usize,
    /// Stable validation issue records.
    pub issues: Vec<ValidationReportIssue>,
    /// Validation errors
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub errors: Vec<ValidationError>,
    /// Validation warnings
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub warnings: Vec<ValidationWarning>,
    /// Message metadata
    pub metadata: MessageMetadata,
}

/// Validate and redact request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateRedactedRequest {
    /// Raw HL7 message content.
    pub message: String,
    /// Inline profile YAML content to validate against.
    pub profile: String,
    /// Inline safe-analysis redaction policy in TOML format.
    pub redaction_policy: String,
    /// Whether the message is MLLP framed.
    #[serde(default)]
    pub mllp_framed: bool,
    /// Whether to include the redacted HL7 payload in the response.
    #[serde(default)]
    pub include_redacted_hl7: bool,
}

/// Validate and redact response body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateRedactedResponse {
    /// Validation report generated from the redacted message.
    pub validation_report: ValidationReport,
    /// Receipt describing redaction actions applied before validation.
    pub redaction_receipt: RedactionReceipt,
    /// Redacted HL7 payload, included only when requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redacted_hl7: Option<String>,
}

/// Redaction receipt compatible with evidence redaction receipts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RedactionReceipt {
    /// Whether any configured PHI-bearing field was removed or hashed.
    pub phi_removed: bool,
    /// Hash algorithm used by hash redaction actions.
    pub hash_algorithm: String,
    /// Per-rule redaction receipts.
    pub actions: Vec<RedactionActionReceipt>,
}

/// Per-rule redaction action receipt.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
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
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RedactionActionStatus {
    /// Action was applied to at least one field.
    Applied,
    /// Retain action matched at least one field.
    Retained,
    /// Optional action did not match a field.
    NotFound,
}

/// ACK generation request body
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AckRequest {
    /// Raw HL7 message content
    pub message: String,
    /// ACK code to generate
    pub code: AckRequestCode,
    /// Optional error text for ERR segment generation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    /// Whether the input message is MLLP framed
    #[serde(default)]
    pub mllp_framed: bool,
    /// Whether to MLLP frame the ACK response
    #[serde(default)]
    pub mllp_frame: bool,
}

/// HTTP ACK codes.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AckRequestCode {
    /// Application accept
    #[serde(rename = "AA")]
    Aa,
    /// Application error
    #[serde(rename = "AE")]
    Ae,
    /// Application reject
    #[serde(rename = "AR")]
    Ar,
    /// Commit accept
    #[serde(rename = "CA")]
    Ca,
    /// Commit error
    #[serde(rename = "CE")]
    Ce,
    /// Commit reject
    #[serde(rename = "CR")]
    Cr,
}

impl AckRequestCode {
    /// Return the HL7 code string.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Aa => "AA",
            Self::Ae => "AE",
            Self::Ar => "AR",
            Self::Ca => "CA",
            Self::Ce => "CE",
            Self::Cr => "CR",
        }
    }
}

/// ACK generation response body
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AckResponse {
    /// Generated ACK message, optionally MLLP framed
    pub ack_message: String,
    /// Generated ACK code
    pub ack_code: String,
    /// Metadata extracted from the generated ACK message
    pub metadata: MessageMetadata,
}

/// Normalize request body
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizeRequest {
    /// Raw HL7 message content
    pub message: String,
    /// Whether the input message is MLLP framed
    #[serde(default)]
    pub mllp_framed: bool,
    /// Normalization options
    #[serde(default)]
    pub options: NormalizeOptions,
}

/// Normalize options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizeOptions {
    /// Rewrite delimiters to canonical `|^~\&`
    #[serde(default = "default_true")]
    pub canonical_delimiters: bool,
    /// MLLP frame the normalized response
    #[serde(default)]
    pub mllp_frame: bool,
}

impl Default for NormalizeOptions {
    fn default() -> Self {
        Self {
            canonical_delimiters: true,
            mllp_frame: false,
        }
    }
}

/// Normalize response body
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizeResponse {
    /// Normalized HL7 message, optionally MLLP framed
    pub normalized_message: String,
    /// Metadata extracted from the normalized message
    pub metadata: MessageMetadata,
}

/// Validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// Error code (e.g., "V_RequiredField")
    pub code: String,
    /// Human-readable error message
    pub message: String,
    /// Location in message (e.g., `PID.5[1].1`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    /// Severity level
    pub severity: ErrorSeverity,
}

/// Validation warning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    /// Warning code
    pub code: String,
    /// Human-readable warning message
    pub message: String,
    /// Location in message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
}

/// Error severity
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ErrorSeverity {
    /// Fatal error, message cannot be processed
    Error,
    /// Warning, message can be processed but may have issues
    Warning,
    /// Informational, no action required
    Info,
}

/// Standard error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Error code
    pub code: String,
    /// Human-readable error message
    pub message: String,
    /// Additional error details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ErrorResponse {
    /// Create a new error response
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    /// Add details to the error response
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}
