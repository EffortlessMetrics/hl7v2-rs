//! Request and response models for the HTTP API.
//!
//! These models follow JSON:API conventions where appropriate and align
//! with the OpenAPI specification in `api/openapi/hl7v2-api-v1.yaml`.

use serde::{Deserialize, Serialize};
use hl7v2::{ValidationReport, ValidationReportIssue, ValidationReportV2};

use super::{
    QuarantineOutputSummary, QuarantineOutputSummaryV2, RedactionReceipt, RedactionReceiptV2,
    ValidationError, ValidationWarning,
};

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
    /// Optional validation report schema version.
    ///
    /// Omitted or `1` preserves the existing response shape. `2` adds the
    /// nested `validation_report_v2` field with embedded evidence provenance.
    #[serde(default)]
    pub report_schema_version: Option<u8>,
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
    /// Opt-in validation report v2 artifact with embedded provenance.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_report_v2: Option<ValidationReportV2>,
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
    /// Optional validation report schema version.
    ///
    /// Omitted or `1` preserves the existing response shape. `2` adds the
    /// nested `validation_report_v2` field with embedded evidence provenance.
    #[serde(default)]
    pub report_schema_version: Option<u8>,
    /// Optional redaction receipt schema version.
    ///
    /// Omitted or `1` preserves the existing `redaction_receipt` field. `2`
    /// adds the nested `redaction_receipt_v2` field with embedded evidence
    /// provenance.
    #[serde(default)]
    pub redaction_receipt_schema_version: Option<u8>,
    /// Optional quarantine output summary schema version.
    ///
    /// Omitted or `1` preserves the existing `quarantine` field. `2` adds the
    /// nested `quarantine_v2` field with embedded evidence provenance when
    /// quarantine output is written.
    #[serde(default)]
    pub quarantine_schema_version: Option<u8>,
}

/// Validate and redact response body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateRedactedResponse {
    /// Validation report generated from the redacted message.
    pub validation_report: ValidationReport,
    /// Opt-in validation report v2 artifact with embedded provenance.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_report_v2: Option<ValidationReportV2>,
    /// Receipt describing redaction actions applied before validation.
    pub redaction_receipt: RedactionReceipt,
    /// Opt-in redaction receipt v2 artifact with embedded provenance.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redaction_receipt_v2: Option<RedactionReceiptV2>,
    /// Quarantine output written when configured and validation failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quarantine: Option<QuarantineOutputSummary>,
    /// Opt-in quarantine output v2 artifact with embedded provenance.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quarantine_v2: Option<QuarantineOutputSummaryV2>,
    /// Redacted HL7 payload, included only when requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redacted_hl7: Option<String>,
}

