//! Request and response models for the HTTP API.
//!
//! These models follow JSON:API conventions where appropriate and align
//! with the OpenAPI specification in `schemas/openapi/hl7v2-api.yaml`.

use serde::{Deserialize, Serialize};

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
    /// Profile to validate against (path or name)
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
    /// Validation errors
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub errors: Vec<ValidationError>,
    /// Validation warnings
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub warnings: Vec<ValidationWarning>,
    /// Message metadata
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
