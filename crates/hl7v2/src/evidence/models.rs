use crate::conformance::validation::ValidationReport;
use crate::redact::{RedactionAction, RedactionError};

/// Machine-readable summary returned after writing an evidence bundle.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EvidenceBundleSummary {
    /// Evidence bundle contract version.
    pub bundle_version: String,
    /// Bundle output directory label. This is `.` for local Python-created bundles.
    pub output_dir: String,
    /// Message type from the raw input message.
    pub message_type: String,
    /// Whether validation passed after redaction.
    pub validation_valid: bool,
    /// Number of validation issues generated from the redacted message.
    pub validation_issue_count: usize,
    /// Whether the redaction policy removed or hashed PHI-bearing fields.
    pub redaction_phi_removed: bool,
    /// Bundle-relative artifact names written by this helper.
    pub artifacts: Vec<String>,
}

impl EvidenceBundleSummary {
    /// Convert this v1 bundle summary into the explicit v2 evidence contract shape.
    ///
    /// This preserves the default serialized form of [`EvidenceBundleSummary`].
    /// Producers opt into v2 when they are ready to emit embedded provenance.
    #[must_use]
    pub fn to_v2(
        &self,
        tool_name: impl Into<String>,
        tool_version: impl Into<String>,
    ) -> EvidenceBundleSummaryV2 {
        EvidenceBundleSummaryV2 {
            schema_version: "2".to_string(),
            tool_name: tool_name.into(),
            tool_version: tool_version.into(),
            summary: self.clone(),
        }
    }
}

/// Evidence bundle summary v2 with embedded evidence provenance.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EvidenceBundleSummaryV2 {
    /// Evidence artifact schema version.
    pub schema_version: String,
    /// Producer surface that generated this bundle summary.
    pub tool_name: String,
    /// Producer package version.
    pub tool_version: String,
    /// V1 bundle summary fields.
    #[serde(flatten)]
    pub summary: EvidenceBundleSummary,
}

/// Evidence bundle manifest written inside the bundle directory.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
    /// Convert this v1 bundle manifest into the explicit v2 evidence contract shape.
    #[must_use]
    pub fn to_v2(&self) -> EvidenceBundleManifestV2 {
        EvidenceBundleManifestV2 {
            schema_version: "2".to_string(),
            manifest: self.clone(),
        }
    }
}

/// Evidence bundle manifest v2 with embedded schema provenance.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EvidenceBundleManifestV2 {
    /// Evidence artifact schema version.
    pub schema_version: String,
    /// V1 bundle manifest fields.
    #[serde(flatten)]
    pub manifest: EvidenceBundleManifest,
}

/// Evidence bundle manifest artifact entry.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EvidenceBundleManifestArtifact {
    /// Bundle-relative artifact path.
    pub path: String,
    /// Stable artifact role.
    pub role: String,
    /// SHA-256 digest of the artifact bytes.
    pub sha256: String,
}

/// Environment metadata written inside an evidence bundle.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
    /// Convert this v1 bundle environment into the explicit v2 evidence contract shape.
    #[must_use]
    pub fn to_v2(&self) -> EvidenceBundleEnvironmentV2 {
        EvidenceBundleEnvironmentV2 {
            schema_version: "2".to_string(),
            environment: self.clone(),
        }
    }
}

/// Evidence bundle environment v2 with embedded schema provenance.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EvidenceBundleEnvironmentV2 {
    /// Evidence artifact schema version.
    pub schema_version: String,
    /// V1 bundle environment fields.
    #[serde(flatten)]
    pub environment: EvidenceBundleEnvironment,
}

/// Field-path trace written inside an evidence bundle.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FieldPathTraceReport {
    /// HL7 trigger event from `MSH.9`, such as `ADT^A01`.
    pub message_type: String,
    /// Number of field entries included in the trace.
    pub field_count: usize,
    /// Field path trace records.
    pub fields: Vec<FieldPathTrace>,
}

impl FieldPathTraceReport {
    /// Convert this v1 field-path trace into the explicit v2 evidence contract shape.
    #[must_use]
    pub fn to_v2(
        &self,
        tool_name: impl Into<String>,
        tool_version: impl Into<String>,
    ) -> FieldPathTraceReportV2 {
        FieldPathTraceReportV2 {
            schema_version: "2".to_string(),
            tool_name: tool_name.into(),
            tool_version: tool_version.into(),
            trace: self.clone(),
        }
    }
}

/// Field-path trace v2 with embedded schema and tool provenance.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FieldPathTraceReportV2 {
    /// Evidence artifact schema version.
    pub schema_version: String,
    /// Producer surface that generated this trace.
    pub tool_name: String,
    /// Producer package version.
    pub tool_version: String,
    /// V1 field-path trace fields.
    #[serde(flatten)]
    pub trace: FieldPathTraceReport,
}

/// Field path trace record.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldValueShape {
    /// Empty field after redaction or original content.
    Empty,
    /// Non-empty value not matching a known redaction marker.
    Present,
    /// SHA-256 redaction marker.
    HashedSha256,
}

/// Machine-readable report returned by evidence bundle replay.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EvidenceReplayReport {
    /// Evidence replay contract version.
    pub replay_version: String,
    /// Bundle version from the manifest or environment, when available.
    pub bundle_version: Option<String>,
    /// Tool that generated this replay report.
    pub tool_name: String,
    /// Tool version that generated this replay report.
    pub tool_version: String,
    /// Message type inferred from replay artifacts.
    pub message_type: Option<String>,
    /// Whether every replay verification check passed.
    pub reproduced: bool,
    /// Whether regenerated validation passed, when regeneration was possible.
    pub validation_valid: Option<bool>,
    /// Number of regenerated validation issues, when regeneration was possible.
    pub validation_issue_count: Option<usize>,
    /// Replay verification checks.
    pub checks: Vec<EvidenceReplayCheck>,
    /// Regenerated validation report when replay reached validation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_report: Option<ValidationReport>,
}

impl EvidenceReplayReport {
    /// Convert this v1 replay report into the explicit v2 evidence contract shape.
    ///
    /// This preserves the default serialized form of [`EvidenceReplayReport`].
    /// Producers opt into v2 when they are ready to emit embedded provenance.
    #[must_use]
    pub fn to_v2(&self) -> EvidenceReplayReportV2 {
        EvidenceReplayReportV2 {
            schema_version: "2".to_string(),
            report: self.clone(),
        }
    }
}

/// Evidence replay report v2 with embedded evidence schema version.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EvidenceReplayReportV2 {
    /// Evidence artifact schema version.
    pub schema_version: String,
    /// V1 replay report fields.
    #[serde(flatten)]
    pub report: EvidenceReplayReport,
}

/// One replay verification check.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EvidenceReplayCheck {
    /// Stable check name.
    pub name: String,
    /// Check status.
    pub status: EvidenceReplayCheckStatus,
    /// Human-readable check result.
    pub message: String,
}

/// Evidence replay check status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceReplayCheckStatus {
    /// Check passed.
    Pass,
    /// Check failed.
    Fail,
}

/// Evidence bundle write or replay error.
#[derive(Debug, thiserror::Error)]
pub enum EvidenceError {
    /// Input message could not be parsed.
    #[error("parse error: {0}")]
    Parse(String),
    /// Profile YAML could not be loaded.
    #[error("profile load error: {0}")]
    Profile(String),
    /// Safe-analysis redaction failed.
    #[error("redaction error: {0}")]
    Redaction(#[from] RedactionError),
    /// Redacted output could not be parsed.
    #[error("redacted message parse error: {0}")]
    RedactedParse(String),
    /// Bundle request or producer option is invalid.
    #[error("invalid evidence bundle input: {0}")]
    InvalidInput(String),
    /// Bundle output already exists.
    #[error("bundle output directory already exists")]
    OutputExists,
    /// Filesystem failure.
    #[error("bundle IO error: {0}")]
    Io(String),
    /// JSON serialization or parsing failure.
    #[error("bundle JSON error: {0}")]
    Json(String),
}
