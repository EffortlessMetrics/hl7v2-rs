//! Request and response models for the HTTP API.
//!
//! These models follow JSON:API conventions where appropriate and align
//! with the OpenAPI specification in `api/openapi/hl7v2-api-v1.yaml`.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use hl7v2::{ValidationReport, ValidationReportIssue, ValidationReportV2};

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

/// Server evidence bundle creation request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleRequest {
    /// Raw HL7 message content.
    pub message: String,
    /// Inline profile YAML content to validate against.
    pub profile: String,
    /// Inline safe-analysis redaction policy in TOML format.
    pub redaction_policy: String,
    /// Caller-supplied bundle identifier relative to the configured bundle root.
    pub bundle_id: String,
    /// Whether the message is MLLP framed.
    #[serde(default)]
    pub mllp_framed: bool,
    /// Optional bundle-internal artifact schema version.
    ///
    /// Version 1 preserves the default bundle artifact shapes. Version 2 writes
    /// `manifest.json`, `environment.json`, `field-paths.json`, and
    /// `redaction-receipt.json` with embedded evidence provenance.
    #[serde(default)]
    pub bundle_artifact_schema_version: Option<u8>,
}

/// Server evidence bundle replay request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayRequest {
    /// Caller-supplied bundle identifier relative to the configured bundle root.
    pub bundle_id: String,
    /// Optional evidence replay report schema version.
    ///
    /// Omitted or `1` preserves the default replay report shape. `2` returns
    /// the v2 replay report shape with embedded evidence provenance.
    #[serde(default)]
    pub replay_report_schema_version: Option<u8>,
}

/// Inline corpus message supplied to corpus evidence endpoints.
///
/// `id` is a label used in parse-error reports. It is not interpreted as a
/// filesystem path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusMessageInput {
    /// Optional caller-facing message label.
    #[serde(default)]
    pub id: Option<String>,
    /// Raw HL7 message content. MLLP framing is detected automatically.
    pub message: String,
}

/// Server inline corpus summary request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusSummaryRequest {
    /// Inline HL7 messages to summarize.
    pub messages: Vec<CorpusMessageInput>,
    /// Optional corpus summary schema version.
    ///
    /// Omitted or `1` preserves the default summary shape. `2` returns the v2
    /// summary shape with embedded evidence provenance.
    #[serde(default)]
    pub summary_schema_version: Option<u8>,
}

/// Server inline corpus fingerprint request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusFingerprintRequest {
    /// Inline HL7 messages to fingerprint.
    pub messages: Vec<CorpusMessageInput>,
    /// Optional inline profile YAML content for validation issue-code counts.
    #[serde(default)]
    pub profile: Option<String>,
    /// Optional corpus fingerprint schema version.
    ///
    /// Omitted or `1` preserves the default fingerprint shape. `2` returns the
    /// v2 fingerprint shape with embedded evidence provenance.
    #[serde(default)]
    pub fingerprint_schema_version: Option<u8>,
}

/// Server inline corpus diff request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusDiffRequest {
    /// Inline before-corpus messages.
    pub before: Vec<CorpusMessageInput>,
    /// Inline after-corpus messages.
    pub after: Vec<CorpusMessageInput>,
    /// Optional inline profile YAML content for validation issue-code deltas.
    #[serde(default)]
    pub profile: Option<String>,
    /// Optional corpus diff schema version.
    ///
    /// Omitted or `1` preserves the default diff report shape. `2` returns the
    /// v2 diff report shape with embedded evidence provenance.
    #[serde(default)]
    pub diff_schema_version: Option<u8>,
}

/// Evidence bundle summary response body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceBundleSummary {
    /// Evidence bundle contract version.
    pub bundle_version: String,
    /// Bundle output directory relative to the configured server bundle root.
    pub output_dir: String,
    /// HL7 trigger event from `MSH.9`, such as `ADT^A01`.
    pub message_type: String,
    /// Whether validation passed after redaction.
    pub validation_valid: bool,
    /// Number of validation issues generated from the redacted message.
    pub validation_issue_count: usize,
    /// Whether configured PHI paths were removed or hashed.
    pub redaction_phi_removed: bool,
    /// Bundle-relative artifact names written by the server.
    pub artifacts: Vec<String>,
}

/// Server quarantine output configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QuarantineConfig {
    /// Whether failed redacted validation should write quarantine output.
    #[serde(default)]
    pub enabled: bool,
    /// Filesystem root used for generated quarantine output.
    #[serde(default)]
    pub path: Option<PathBuf>,
    /// Whether to write `message.redacted.hl7` when not writing a full bundle.
    #[serde(default = "default_true")]
    pub write_redacted: bool,
    /// Whether to write `validation-report.json` when not writing a full bundle.
    #[serde(default = "default_true")]
    pub write_report: bool,
    /// Whether to write a full replayable evidence bundle.
    #[serde(default = "default_true")]
    pub write_bundle: bool,
}

impl Default for QuarantineConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            path: None,
            write_redacted: true,
            write_report: true,
            write_bundle: true,
        }
    }
}

/// Sanitized quarantine configuration for diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublicQuarantineConfig {
    /// Whether quarantine output is enabled.
    pub enabled: bool,
    /// Whether a quarantine output path is configured without exposing the path.
    pub path_configured: bool,
    /// Whether redacted HL7 artifacts are configured for quarantine output.
    pub write_redacted: bool,
    /// Whether validation report artifacts are configured for quarantine output.
    pub write_report: bool,
    /// Whether replayable bundle output is configured for quarantine output.
    pub write_bundle: bool,
}

/// Summary of quarantine output written by the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuarantineOutputSummary {
    /// Quarantine output contract version.
    pub quarantine_version: String,
    /// Output directory relative to the configured quarantine root.
    pub output_dir: String,
    /// Stable reason for quarantine output.
    pub reason: QuarantineReason,
    /// Number of validation issues that triggered the quarantine write.
    pub validation_issue_count: usize,
    /// Quarantine-relative artifact names written by the server.
    pub artifacts: Vec<String>,
}

impl QuarantineOutputSummary {
    /// Convert this v1 quarantine summary into the explicit v2 evidence contract shape.
    ///
    /// The default serialized server response remains v1-compatible. Callers
    /// opt into this additive v2 shape with `quarantine_schema_version = 2`.
    #[must_use]
    pub fn to_v2(
        &self,
        tool_name: impl Into<String>,
        tool_version: impl Into<String>,
    ) -> QuarantineOutputSummaryV2 {
        QuarantineOutputSummaryV2 {
            schema_version: "2".to_string(),
            tool_name: tool_name.into(),
            tool_version: tool_version.into(),
            summary: self.clone(),
        }
    }
}

/// Quarantine output summary v2 with embedded evidence provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuarantineOutputSummaryV2 {
    /// Evidence artifact schema version.
    pub schema_version: String,
    /// Producer surface that generated this quarantine summary.
    pub tool_name: String,
    /// Producer package version.
    pub tool_version: String,
    /// V1 quarantine summary fields.
    #[serde(flatten)]
    pub summary: QuarantineOutputSummary,
}

/// Reason that caused quarantine output.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QuarantineReason {
    /// Validation report was invalid after redaction.
    ValidationError,
}

/// Evidence bundle manifest written inside the bundle directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceBundleManifest {
    /// Evidence bundle contract version.
    pub bundle_version: String,
    /// Tool that generated this bundle.
    pub tool_name: String,
    /// Tool version that generated this bundle.
    pub tool_version: String,
    /// Bundle-relative artifact entries.
    pub artifacts: Vec<EvidenceBundleManifestArtifact>,
}

impl EvidenceBundleManifest {
    /// Convert this manifest to the v2 evidence contract with server provenance.
    pub fn to_v2(&self) -> EvidenceBundleManifestV2 {
        EvidenceBundleManifestV2 {
            schema_version: "2".to_string(),
            bundle_version: self.bundle_version.clone(),
            tool_name: self.tool_name.clone(),
            tool_version: self.tool_version.clone(),
            artifacts: self.artifacts.clone(),
        }
    }
}

/// Evidence bundle manifest v2 with embedded schema provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceBundleManifestV2 {
    /// Evidence artifact schema version.
    pub schema_version: String,
    /// Evidence bundle contract version.
    pub bundle_version: String,
    /// Tool that generated this bundle.
    pub tool_name: String,
    /// Tool version that generated this bundle.
    pub tool_version: String,
    /// Bundle-relative artifact entries.
    pub artifacts: Vec<EvidenceBundleManifestArtifact>,
}

/// Evidence bundle manifest artifact entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceBundleManifestArtifact {
    /// Bundle-relative artifact path.
    pub path: String,
    /// Stable artifact role.
    pub role: String,
    /// SHA-256 digest of the artifact bytes.
    pub sha256: String,
}

/// Environment metadata written inside an evidence bundle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceBundleEnvironment {
    /// Evidence bundle contract version.
    pub bundle_version: String,
    /// Tool that generated this bundle.
    pub tool_name: String,
    /// Tool version that generated this bundle.
    pub tool_version: String,
    /// Message type from the raw message.
    pub message_type: String,
    /// SHA-256 digest of the raw input message.
    pub input_sha256: String,
    /// SHA-256 digest of the profile YAML.
    pub profile_sha256: String,
    /// SHA-256 digest of the redaction policy TOML.
    pub redaction_policy_sha256: String,
    /// Whether validation passed after redaction.
    pub validation_valid: bool,
    /// Number of validation issues generated from the redacted message.
    pub validation_issue_count: usize,
    /// Replay command for validating the bundled artifacts.
    pub replay_command: String,
}

impl EvidenceBundleEnvironment {
    /// Convert this environment artifact to the v2 evidence contract.
    pub fn to_v2(&self) -> EvidenceBundleEnvironmentV2 {
        EvidenceBundleEnvironmentV2 {
            schema_version: "2".to_string(),
            bundle_version: self.bundle_version.clone(),
            tool_name: self.tool_name.clone(),
            tool_version: self.tool_version.clone(),
            message_type: self.message_type.clone(),
            input_sha256: self.input_sha256.clone(),
            profile_sha256: self.profile_sha256.clone(),
            redaction_policy_sha256: self.redaction_policy_sha256.clone(),
            validation_valid: self.validation_valid,
            validation_issue_count: self.validation_issue_count,
            replay_command: self.replay_command.clone(),
        }
    }
}

/// Evidence bundle environment v2 with embedded schema provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceBundleEnvironmentV2 {
    /// Evidence artifact schema version.
    pub schema_version: String,
    /// Evidence bundle contract version.
    pub bundle_version: String,
    /// Tool that generated this bundle.
    pub tool_name: String,
    /// Tool version that generated this bundle.
    pub tool_version: String,
    /// Message type from the raw message.
    pub message_type: String,
    /// SHA-256 digest of the raw input message.
    pub input_sha256: String,
    /// SHA-256 digest of the profile YAML.
    pub profile_sha256: String,
    /// SHA-256 digest of the redaction policy TOML.
    pub redaction_policy_sha256: String,
    /// Whether validation passed after redaction.
    pub validation_valid: bool,
    /// Number of validation issues generated from the redacted message.
    pub validation_issue_count: usize,
    /// Replay command for validating the bundled artifacts.
    pub replay_command: String,
}

/// Field-path trace written inside an evidence bundle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldPathTraceReport {
    /// HL7 trigger event from `MSH.9`, such as `ADT^A01`.
    pub message_type: String,
    /// Number of field entries included in the trace.
    pub field_count: usize,
    /// Field path trace records.
    pub fields: Vec<FieldPathTrace>,
}

impl FieldPathTraceReport {
    /// Convert this field-path trace to the v2 evidence contract.
    pub fn to_v2(&self) -> FieldPathTraceReportV2 {
        FieldPathTraceReportV2 {
            schema_version: "2".to_string(),
            tool_name: "hl7v2-server".to_string(),
            tool_version: env!("CARGO_PKG_VERSION").to_string(),
            message_type: self.message_type.clone(),
            field_count: self.field_count,
            fields: self.fields.clone(),
        }
    }
}

/// Field-path trace v2 with embedded schema and tool provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldPathTraceReportV2 {
    /// Evidence artifact schema version.
    pub schema_version: String,
    /// Tool that generated this trace.
    pub tool_name: String,
    /// Tool version that generated this trace.
    pub tool_version: String,
    /// HL7 trigger event from `MSH.9`, such as `ADT^A01`.
    pub message_type: String,
    /// Number of field entries included in the trace.
    pub field_count: usize,
    /// Field path trace records.
    pub fields: Vec<FieldPathTrace>,
}

/// Field path trace record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldPathTrace {
    /// Segment-position-qualified path.
    pub path: String,
    /// Segment and HL7 field path, such as `PID.3`.
    pub canonical_path: String,
    /// One-based segment index.
    pub segment_index: usize,
    /// One-based HL7 field index.
    pub field_index: usize,
    /// Whether the field value is present after redaction.
    pub present: bool,
    /// Shape of the redacted field value.
    pub value_shape: FieldValueShape,
    /// Redaction action associated with this path, when configured.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redaction_action: Option<RedactionAction>,
}

/// Redacted field value shape.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FieldValueShape {
    /// Empty field after redaction or original content.
    Empty,
    /// Non-empty value not matching a known redaction marker.
    Present,
    /// SHA-256 redaction marker.
    HashedSha256,
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

impl RedactionReceipt {
    /// Convert this receipt to the v2 evidence contract with server provenance.
    pub fn to_v2(&self) -> RedactionReceiptV2 {
        RedactionReceiptV2 {
            schema_version: "2".to_string(),
            tool_name: "hl7v2-server".to_string(),
            tool_version: env!("CARGO_PKG_VERSION").to_string(),
            phi_removed: self.phi_removed,
            hash_algorithm: self.hash_algorithm.clone(),
            actions: self.actions.clone(),
        }
    }
}

/// Redaction receipt v2 with embedded evidence provenance.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

/// Configurable server ACK policy.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AckPolicyConfig {
    /// ACK mode to use when choosing generated ACK/NAK codes.
    #[serde(default)]
    pub mode: AckPolicyMode,
    /// Condition that causes an accept ACK.
    #[serde(default)]
    pub accept_on: AckPolicyAcceptOn,
    /// Conditions that cause reject ACKs.
    #[serde(default = "default_ack_reject_on")]
    pub reject_on: Vec<AckPolicyRejectCondition>,
    /// Whether generated NAKs should include non-PHI error text in `ERR`.
    #[serde(default = "default_include_error_text")]
    pub include_error_text: bool,
}

impl Default for AckPolicyConfig {
    fn default() -> Self {
        Self {
            mode: AckPolicyMode::Original,
            accept_on: AckPolicyAcceptOn::Valid,
            reject_on: default_ack_reject_on(),
            include_error_text: true,
        }
    }
}

impl AckPolicyConfig {
    /// Return whether the policy rejects the supplied condition.
    pub fn rejects(&self, condition: AckPolicyRejectCondition) -> bool {
        self.reject_on.contains(&condition)
    }
}

fn default_ack_reject_on() -> Vec<AckPolicyRejectCondition> {
    vec![
        AckPolicyRejectCondition::ParseError,
        AckPolicyRejectCondition::ValidationError,
    ]
}

fn default_include_error_text() -> bool {
    true
}

/// ACK policy mode.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AckPolicyMode {
    /// Use original mode application ACK codes: `AA` and `AR`.
    #[default]
    Original,
    /// Use enhanced mode commit ACK codes: `CA` and `CR`.
    Enhanced,
}

/// ACK policy accept condition.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AckPolicyAcceptOn {
    /// Accept only after the message validates against the supplied profile.
    #[default]
    Valid,
}

/// ACK policy reject condition.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AckPolicyRejectCondition {
    /// Reject when the inbound message cannot be parsed enough to validate.
    ParseError,
    /// Reject when validation against the supplied profile fails.
    ValidationError,
}

/// Policy-driven ACK request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AckPolicyRequest {
    /// Raw HL7 message content.
    pub message: String,
    /// Inline profile YAML content used for validation before deciding ACK/NAK.
    pub profile: String,
    /// Whether the input message is MLLP framed.
    #[serde(default)]
    pub mllp_framed: bool,
    /// Whether to MLLP frame the ACK response.
    #[serde(default)]
    pub mllp_frame: bool,
}

/// Policy-driven ACK response body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AckPolicyResponse {
    /// Generated ACK or NAK message, optionally MLLP framed.
    pub ack_message: String,
    /// Generated ACK code.
    pub ack_code: String,
    /// Decision details used to choose the ACK code.
    pub decision: AckPolicyDecision,
    /// Validation report used for the decision, when the message parsed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_report: Option<hl7v2::ValidationReport>,
    /// Metadata extracted from the generated ACK message.
    pub metadata: MessageMetadata,
}

/// ACK policy decision details.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AckPolicyDecision {
    /// Configured ACK mode.
    pub mode: AckPolicyMode,
    /// Decision outcome.
    pub outcome: AckPolicyOutcome,
    /// Reason for the decision.
    pub reason: AckPolicyReason,
    /// Generated ACK code.
    pub ack_code: String,
    /// Whether non-PHI error text was included in the ACK.
    pub include_error_text: bool,
    /// Error text included in the ACK, when configured.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_text: Option<String>,
}

/// ACK policy decision outcome.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AckPolicyOutcome {
    /// Message was accepted.
    Accepted,
    /// Message was rejected.
    Rejected,
}

/// ACK policy decision reason.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AckPolicyReason {
    /// Message parsed and validated successfully.
    Valid,
    /// Message could not be parsed.
    ParseError,
    /// Message parsed but failed profile validation.
    ValidationError,
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
