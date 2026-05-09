//! Evidence bundle and replay helpers.
//!
//! These helpers back evidence-grade workflows that combine redaction,
//! profile validation, artifact manifests, and replay verification.

use crate::conformance::profile::{load_profile_checked, validate};
use crate::conformance::validation::ValidationReport;
use crate::model::{Atom, Field, Message};
use crate::parser::parse;
use crate::redact::{RedactionAction, RedactionError, RedactionReceipt, redact_hl7_safe_analysis};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

const BUNDLE_VERSION: &str = "1";
const REPLAY_VERSION: &str = "1";
const REPLAY_COMMAND: &str = "hl7v2 replay . --format json";
const BUNDLE_ARTIFACT_SPECS: [(&str, &str); 9] = [
    ("message.redacted.hl7", "redacted_message"),
    ("validation-report.json", "validation_report"),
    ("field-paths.json", "field_path_trace"),
    ("profile.yaml", "profile"),
    ("redaction-receipt.json", "redaction_receipt"),
    ("environment.json", "environment"),
    ("replay.sh", "replay_shell_script"),
    ("replay.ps1", "replay_powershell_script"),
    ("README.md", "bundle_readme"),
];

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

/// Write a redacted evidence bundle from raw HL7, profile YAML, and policy TOML.
///
/// The bundle contains only redacted HL7 plus structured evidence artifacts. It
/// intentionally writes `profile.yaml` as supplied by the caller; callers should
/// review profile content before sharing a bundle.
///
/// # Errors
///
/// Returns [`EvidenceError`] when parsing, profile loading, redaction,
/// validation setup, JSON serialization, or filesystem writes fail. Existing
/// output directories are rejected.
pub fn write_safe_analysis_bundle(
    content: impl AsRef<[u8]>,
    profile_yaml: &str,
    policy_text: &str,
    out: impl AsRef<Path>,
    tool_name: &str,
) -> Result<EvidenceBundleSummary, EvidenceError> {
    let content = content.as_ref();
    let out = out.as_ref();
    if out.exists() {
        return Err(EvidenceError::OutputExists);
    }

    let redaction_output = redact_hl7_safe_analysis(content, policy_text)?;
    let redacted_message = parse(redaction_output.redacted_hl7.as_bytes())
        .map_err(|error| EvidenceError::RedactedParse(error.to_string()))?;
    let profile = load_profile_checked(profile_yaml)
        .map_err(|error| EvidenceError::Profile(error.to_string()))?;
    let validation_report = ValidationReport::from_issues(
        &redacted_message,
        Some("profile.yaml".to_string()),
        validate(&redacted_message, &profile),
    );
    let field_trace = build_field_path_trace(&redacted_message, &redaction_output.receipt);
    let environment = EvidenceBundleEnvironment {
        bundle_version: BUNDLE_VERSION.to_string(),
        tool_name: tool_name.to_string(),
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        message_type: validation_report.message_type.clone(),
        input_sha256: redaction_output.input_sha256,
        profile_sha256: compute_sha256(profile_yaml),
        redaction_policy_sha256: redaction_output.policy_sha256,
        validation_valid: validation_report.valid,
        validation_issue_count: validation_report.issue_count,
        replay_command: REPLAY_COMMAND.to_string(),
    };

    fs::create_dir(out).map_err(|error| EvidenceError::Io(error.to_string()))?;
    fs::write(
        out.join("message.redacted.hl7"),
        redaction_output.redacted_hl7,
    )
    .map_err(|error| EvidenceError::Io(error.to_string()))?;
    fs::write(out.join("profile.yaml"), profile_yaml)
        .map_err(|error| EvidenceError::Io(error.to_string()))?;
    write_json_file(&out.join("validation-report.json"), &validation_report)?;
    write_json_file(
        &out.join("redaction-receipt.json"),
        &redaction_output.receipt,
    )?;
    write_json_file(&out.join("field-paths.json"), &field_trace)?;
    write_json_file(&out.join("environment.json"), &environment)?;
    fs::write(out.join("replay.sh"), replay_shell_script())
        .map_err(|error| EvidenceError::Io(error.to_string()))?;
    fs::write(out.join("replay.ps1"), replay_powershell_script())
        .map_err(|error| EvidenceError::Io(error.to_string()))?;
    fs::write(out.join("README.md"), bundle_readme(tool_name))
        .map_err(|error| EvidenceError::Io(error.to_string()))?;

    let manifest = EvidenceBundleManifest {
        bundle_version: BUNDLE_VERSION.to_string(),
        tool_name: tool_name.to_string(),
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        artifacts: BUNDLE_ARTIFACT_SPECS
            .iter()
            .map(|(path, role)| bundle_manifest_artifact(out, path, role))
            .collect::<Result<_, _>>()?,
    };
    write_json_file(&out.join("manifest.json"), &manifest)?;

    let mut artifacts = BUNDLE_ARTIFACT_SPECS
        .iter()
        .map(|(path, _role)| (*path).to_string())
        .collect::<Vec<_>>();
    artifacts.push("manifest.json".to_string());

    Ok(EvidenceBundleSummary {
        bundle_version: BUNDLE_VERSION.to_string(),
        output_dir: ".".to_string(),
        message_type: validation_report.message_type.clone(),
        validation_valid: validation_report.valid,
        validation_issue_count: validation_report.issue_count,
        redaction_phi_removed: redaction_output.receipt.phi_removed,
        artifacts,
    })
}

/// Replay and verify an evidence bundle directory.
///
/// Replay reports fail closed: malformed manifests, unsafe manifest paths,
/// missing artifacts, hash mismatches, parse failures, profile failures, and
/// validation-report drift all produce a report with `reproduced = false`.
pub fn replay_evidence_bundle(bundle: impl AsRef<Path>, tool_name: &str) -> EvidenceReplayReport {
    build_replay_report(bundle.as_ref(), tool_name)
}

fn build_replay_report(bundle: &Path, tool_name: &str) -> EvidenceReplayReport {
    let mut checks = Vec::new();
    let required_artifacts = [
        "manifest.json",
        "message.redacted.hl7",
        "validation-report.json",
        "field-paths.json",
        "profile.yaml",
        "redaction-receipt.json",
        "environment.json",
        "replay.sh",
        "replay.ps1",
    ];

    let missing_artifacts: Vec<&str> = required_artifacts
        .iter()
        .copied()
        .filter(|artifact| !bundle.join(artifact).is_file())
        .collect();
    if missing_artifacts.is_empty() {
        checks.push(replay_check(
            "bundle-layout",
            EvidenceReplayCheckStatus::Pass,
            "all expected bundle artifacts are present",
        ));
    } else {
        checks.push(replay_check(
            "bundle-layout",
            EvidenceReplayCheckStatus::Fail,
            format!(
                "missing expected bundle artifact(s): {}",
                missing_artifacts.join(", ")
            ),
        ));
    }

    let manifest = match read_bundle_manifest(bundle) {
        Ok(manifest) => {
            checks.push(replay_check(
                "manifest",
                EvidenceReplayCheckStatus::Pass,
                "manifest.json parsed",
            ));
            Some(manifest)
        }
        Err(error) => {
            checks.push(replay_check(
                "manifest",
                EvidenceReplayCheckStatus::Fail,
                error,
            ));
            None
        }
    };
    let manifest_bundle_version = manifest
        .as_ref()
        .map(|manifest| manifest.bundle_version.clone());
    let manifest_catalog_ok = manifest
        .as_ref()
        .is_some_and(|manifest| verify_bundle_manifest_catalog(manifest, &mut checks));
    let manifest_hashes_ok = manifest_catalog_ok
        && manifest
            .as_ref()
            .is_some_and(|manifest| verify_bundle_manifest_hashes(bundle, manifest, &mut checks));

    if !manifest_hashes_ok {
        return EvidenceReplayReport {
            replay_version: REPLAY_VERSION.to_string(),
            bundle_version: manifest_bundle_version,
            tool_name: tool_name.to_string(),
            tool_version: env!("CARGO_PKG_VERSION").to_string(),
            message_type: None,
            reproduced: false,
            validation_valid: None,
            validation_issue_count: None,
            checks,
            validation_report: None,
        };
    }

    let environment = match read_bundle_json_value(bundle, "environment.json") {
        Ok(environment) => {
            checks.push(replay_check(
                "environment",
                EvidenceReplayCheckStatus::Pass,
                "environment.json parsed",
            ));
            Some(environment)
        }
        Err(error) => {
            checks.push(replay_check(
                "environment",
                EvidenceReplayCheckStatus::Fail,
                error,
            ));
            None
        }
    };

    let stored_report = match read_bundle_validation_report(bundle, "validation-report.json") {
        Ok(report) => {
            checks.push(replay_check(
                "stored-validation-report",
                EvidenceReplayCheckStatus::Pass,
                "validation-report.json parsed",
            ));
            Some(report)
        }
        Err(error) => {
            checks.push(replay_check(
                "stored-validation-report",
                EvidenceReplayCheckStatus::Fail,
                error,
            ));
            None
        }
    };

    let redacted_message = match read_bundle_artifact(bundle, "message.redacted.hl7") {
        Ok(contents) => match parse(&contents) {
            Ok(message) => {
                checks.push(replay_check(
                    "parse-redacted-message",
                    EvidenceReplayCheckStatus::Pass,
                    "message.redacted.hl7 parsed",
                ));
                Some(message)
            }
            Err(error) => {
                checks.push(replay_check(
                    "parse-redacted-message",
                    EvidenceReplayCheckStatus::Fail,
                    format!("message.redacted.hl7 did not parse: {error}"),
                ));
                None
            }
        },
        Err(error) => {
            checks.push(replay_check(
                "parse-redacted-message",
                EvidenceReplayCheckStatus::Fail,
                error,
            ));
            None
        }
    };

    let loaded_profile = match read_bundle_string(bundle, "profile.yaml") {
        Ok(profile_yaml) => match load_profile_checked(&profile_yaml) {
            Ok(profile) => {
                checks.push(replay_check(
                    "load-profile",
                    EvidenceReplayCheckStatus::Pass,
                    "profile.yaml loaded",
                ));
                Some(profile)
            }
            Err(error) => {
                checks.push(replay_check(
                    "load-profile",
                    EvidenceReplayCheckStatus::Fail,
                    format!("profile.yaml did not load: {error}"),
                ));
                None
            }
        },
        Err(error) => {
            checks.push(replay_check(
                "load-profile",
                EvidenceReplayCheckStatus::Fail,
                error,
            ));
            None
        }
    };

    let actual_report = match (redacted_message.as_ref(), loaded_profile.as_ref()) {
        (Some(message), Some(profile)) => {
            let report = ValidationReport::from_issues(
                message,
                Some("profile.yaml".to_string()),
                validate(message, profile),
            );
            checks.push(replay_check(
                "generate-validation-report",
                EvidenceReplayCheckStatus::Pass,
                "validation report regenerated from bundled message and profile",
            ));
            Some(report)
        }
        _ => {
            checks.push(replay_check(
                "generate-validation-report",
                EvidenceReplayCheckStatus::Fail,
                "validation report could not be regenerated",
            ));
            None
        }
    };

    match (actual_report.as_ref(), stored_report.as_ref()) {
        (Some(actual), Some(stored)) if actual == stored => checks.push(replay_check(
            "report-match",
            EvidenceReplayCheckStatus::Pass,
            "regenerated validation report matches validation-report.json",
        )),
        (Some(_), Some(_)) => checks.push(replay_check(
            "report-match",
            EvidenceReplayCheckStatus::Fail,
            "regenerated validation report differs from validation-report.json",
        )),
        _ => checks.push(replay_check(
            "report-match",
            EvidenceReplayCheckStatus::Fail,
            "validation report comparison could not be completed",
        )),
    }

    if let (Some(environment), Some(actual)) = (environment.as_ref(), actual_report.as_ref()) {
        let mut mismatches = Vec::new();
        if json_string(environment, "message_type").as_deref() != Some(actual.message_type.as_str())
        {
            mismatches.push("message_type");
        }
        if json_bool(environment, "validation_valid") != Some(actual.valid) {
            mismatches.push("validation_valid");
        }
        if json_usize(environment, "validation_issue_count") != Some(actual.issue_count) {
            mismatches.push("validation_issue_count");
        }

        if mismatches.is_empty() {
            checks.push(replay_check(
                "environment-match",
                EvidenceReplayCheckStatus::Pass,
                "environment metadata matches regenerated validation report",
            ));
        } else {
            checks.push(replay_check(
                "environment-match",
                EvidenceReplayCheckStatus::Fail,
                format!("environment metadata mismatch: {}", mismatches.join(", ")),
            ));
        }
    } else {
        checks.push(replay_check(
            "environment-match",
            EvidenceReplayCheckStatus::Fail,
            "environment metadata comparison could not be completed",
        ));
    }

    let reproduced = checks
        .iter()
        .all(|check| check.status == EvidenceReplayCheckStatus::Pass);
    let bundle_version = environment
        .as_ref()
        .and_then(|value| json_string(value, "bundle_version"))
        .or(manifest_bundle_version);
    let message_type = actual_report
        .as_ref()
        .map(|report| report.message_type.clone())
        .or_else(|| {
            stored_report
                .as_ref()
                .map(|report| report.message_type.clone())
        })
        .or_else(|| {
            environment
                .as_ref()
                .and_then(|value| json_string(value, "message_type"))
        });
    let validation_valid = actual_report.as_ref().map(|report| report.valid);
    let validation_issue_count = actual_report.as_ref().map(|report| report.issue_count);

    EvidenceReplayReport {
        replay_version: REPLAY_VERSION.to_string(),
        bundle_version,
        tool_name: tool_name.to_string(),
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        message_type,
        reproduced,
        validation_valid,
        validation_issue_count,
        checks,
        validation_report: actual_report,
    }
}

fn write_json_file<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), EvidenceError> {
    let bytes =
        serde_json::to_vec_pretty(value).map_err(|error| EvidenceError::Json(error.to_string()))?;
    fs::write(path, bytes).map_err(|error| EvidenceError::Io(error.to_string()))
}

fn bundle_manifest_artifact(
    bundle_dir: &Path,
    path: &str,
    role: &str,
) -> Result<EvidenceBundleManifestArtifact, EvidenceError> {
    let bytes =
        fs::read(bundle_dir.join(path)).map_err(|error| EvidenceError::Io(error.to_string()))?;
    Ok(EvidenceBundleManifestArtifact {
        path: path.to_string(),
        role: role.to_string(),
        sha256: compute_sha256_bytes(&bytes),
    })
}

fn build_field_path_trace(message: &Message, receipt: &RedactionReceipt) -> FieldPathTraceReport {
    let redaction_actions: BTreeMap<&str, RedactionAction> = receipt
        .actions
        .iter()
        .map(|action| (action.path.as_str(), action.action))
        .collect();
    let mut fields = Vec::new();

    for (segment_position, segment) in message.segments.iter().enumerate() {
        let segment_index = segment_position.saturating_add(1);
        for (modeled_index, field) in segment.fields.iter().enumerate() {
            let field_index = hl7_field_index(segment.id_str(), modeled_index);
            let canonical_path = format!("{}.{}", segment.id_str(), field_index);
            let field_text = field_to_text(field, &message.delims);
            fields.push(FieldPathTrace {
                path: format!("{}[{}].{}", segment.id_str(), segment_index, field_index),
                canonical_path: canonical_path.clone(),
                segment_index,
                field_index,
                present: !field_text.is_empty(),
                value_shape: field_value_shape(&field_text),
                redaction_action: redaction_actions.get(canonical_path.as_str()).copied(),
            });
        }
    }

    FieldPathTraceReport {
        message_type: message_type(message),
        field_count: fields.len(),
        fields,
    }
}

fn message_type(message: &Message) -> String {
    crate::get(message, "MSH.9")
        .unwrap_or("unknown")
        .to_string()
}

fn hl7_field_index(segment_id: &str, modeled_index: usize) -> usize {
    if segment_id == "MSH" {
        modeled_index.saturating_add(2)
    } else {
        modeled_index.saturating_add(1)
    }
}

fn field_value_shape(field_text: &str) -> FieldValueShape {
    if field_text.is_empty() {
        FieldValueShape::Empty
    } else if field_text.starts_with("hash:sha256:") {
        FieldValueShape::HashedSha256
    } else {
        FieldValueShape::Present
    }
}

fn field_to_text(field: &Field, delims: &crate::Delims) -> String {
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
                            Atom::Text(text) => text.as_str(),
                            Atom::Null => "\"\"",
                        })
                        .collect::<Vec<_>>()
                        .join(&delims.sub.to_string())
                })
                .collect::<Vec<_>>()
                .join(&delims.comp.to_string())
        })
        .collect::<Vec<_>>()
        .join(&delims.rep.to_string())
}

fn read_bundle_manifest(bundle: &Path) -> Result<EvidenceBundleManifest, String> {
    let contents = read_bundle_string(bundle, "manifest.json")?;
    serde_json::from_str(&contents)
        .map_err(|error| format!("manifest.json is invalid JSON: {error}"))
}

fn verify_bundle_manifest_catalog(
    manifest: &EvidenceBundleManifest,
    checks: &mut Vec<EvidenceReplayCheck>,
) -> bool {
    let expected = BUNDLE_ARTIFACT_SPECS;
    let mut errors = Vec::new();
    let mut seen_paths = BTreeSet::new();

    for artifact in &manifest.artifacts {
        if !seen_paths.insert(artifact.path.clone()) {
            errors.push("duplicate artifact path".to_string());
        }
        if safe_bundle_relative_path(&artifact.path).is_err() {
            errors.push("unsafe artifact path".to_string());
            continue;
        }
        if !is_lower_sha256_hex(&artifact.sha256) {
            errors.push(format!("{} has invalid sha256", artifact.path));
        }
        if !expected
            .iter()
            .any(|(path, role)| *path == artifact.path.as_str() && *role == artifact.role.as_str())
        {
            errors.push(format!(
                "{} has unexpected role {}",
                artifact.path, artifact.role
            ));
        }
    }

    for (expected_path, expected_role) in expected {
        if !manifest
            .artifacts
            .iter()
            .any(|artifact| artifact.path == expected_path && artifact.role == expected_role)
        {
            errors.push(format!("missing manifest entry for {expected_path}"));
        }
    }

    if errors.is_empty() {
        checks.push(replay_check(
            "manifest-artifacts",
            EvidenceReplayCheckStatus::Pass,
            "manifest lists expected bundle artifacts",
        ));
        true
    } else {
        checks.push(replay_check(
            "manifest-artifacts",
            EvidenceReplayCheckStatus::Fail,
            format!("manifest artifact catalog invalid: {}", errors.join(", ")),
        ));
        false
    }
}

fn verify_bundle_manifest_hashes(
    bundle: &Path,
    manifest: &EvidenceBundleManifest,
    checks: &mut Vec<EvidenceReplayCheck>,
) -> bool {
    let mut errors = Vec::new();

    for artifact in &manifest.artifacts {
        let relative_path = match safe_bundle_relative_path(&artifact.path) {
            Ok(relative_path) => relative_path,
            Err(error) => {
                errors.push(error);
                continue;
            }
        };
        match fs::read(bundle.join(relative_path)) {
            Ok(bytes) => {
                let actual = compute_sha256_bytes(&bytes);
                if actual != artifact.sha256 {
                    errors.push(format!("{} hash mismatch", artifact.path));
                }
            }
            Err(error) => {
                errors.push(format!("could not read {}: {error}", artifact.path));
            }
        }
    }

    if errors.is_empty() {
        checks.push(replay_check(
            "manifest-hashes",
            EvidenceReplayCheckStatus::Pass,
            "manifest artifact hashes match bundle contents",
        ));
        true
    } else {
        checks.push(replay_check(
            "manifest-hashes",
            EvidenceReplayCheckStatus::Fail,
            format!("manifest hash verification failed: {}", errors.join(", ")),
        ));
        false
    }
}

fn safe_bundle_relative_path(path: &str) -> Result<PathBuf, String> {
    if path.is_empty() || path.contains('\\') {
        return Err("manifest artifact path must be bundle-relative".to_string());
    }

    let relative_path = Path::new(path);
    if relative_path.is_absolute()
        || relative_path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::Prefix(_) | Component::RootDir
            )
        })
    {
        return Err("manifest artifact path must be bundle-relative".to_string());
    }

    Ok(relative_path.to_path_buf())
}

fn is_lower_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

fn read_bundle_artifact(bundle: &Path, artifact: &str) -> Result<Vec<u8>, String> {
    fs::read(bundle.join(artifact)).map_err(|error| format!("could not read {artifact}: {error}"))
}

fn read_bundle_string(bundle: &Path, artifact: &str) -> Result<String, String> {
    fs::read_to_string(bundle.join(artifact))
        .map_err(|error| format!("could not read {artifact}: {error}"))
}

fn read_bundle_json_value(bundle: &Path, artifact: &str) -> Result<serde_json::Value, String> {
    let contents = read_bundle_string(bundle, artifact)?;
    serde_json::from_str(&contents).map_err(|error| format!("{artifact} is invalid JSON: {error}"))
}

fn read_bundle_validation_report(
    bundle: &Path,
    artifact: &str,
) -> Result<ValidationReport, String> {
    let contents = read_bundle_string(bundle, artifact)?;
    serde_json::from_str(&contents).map_err(|error| format!("{artifact} is invalid JSON: {error}"))
}

fn json_string(value: &serde_json::Value, key: &str) -> Option<String> {
    value.get(key)?.as_str().map(ToOwned::to_owned)
}

fn json_bool(value: &serde_json::Value, key: &str) -> Option<bool> {
    value.get(key)?.as_bool()
}

fn json_usize(value: &serde_json::Value, key: &str) -> Option<usize> {
    value
        .get(key)?
        .as_u64()
        .and_then(|count| usize::try_from(count).ok())
}

fn replay_check(
    name: impl Into<String>,
    status: EvidenceReplayCheckStatus,
    message: impl Into<String>,
) -> EvidenceReplayCheck {
    EvidenceReplayCheck {
        name: name.into(),
        status,
        message: message.into(),
    }
}

fn compute_sha256(value: &str) -> String {
    compute_sha256_bytes(value.as_bytes())
}

fn compute_sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn replay_shell_script() -> &'static str {
    "#!/usr/bin/env sh\nset -eu\ncd \"$(dirname \"$0\")\"\nhl7v2 replay . --format json > replay-report.json\n"
}

fn replay_powershell_script() -> &'static str {
    "$ErrorActionPreference = 'Stop'\nSet-Location $PSScriptRoot\nhl7v2 replay . --format json > .\\replay-report.json\n"
}

fn bundle_readme(tool_name: &str) -> String {
    format!(
        "# HL7v2 Evidence Bundle\n\n\
This directory contains a redacted, replayable evidence packet generated by `{tool_name}`.\n\n\
## Contents\n\n\
- `message.redacted.hl7`: redacted HL7 message used for replay.\n\
- `validation-report.json`: validation report generated from the redacted message.\n\
- `field-paths.json`: field-path trace and redaction action metadata.\n\
- `profile.yaml`: profile used for replay validation.\n\
- `redaction-receipt.json`: receipt describing retained, hashed, dropped, or missing fields.\n\
- `environment.json`: tool version, bundle metadata, and input/profile/policy hashes.\n\
- `manifest.json`: bundle-relative artifact paths, roles, and SHA-256 hashes.\n\
- `replay.sh` and `replay.ps1`: shell helpers that replay the bundle.\n\n\
## Replay\n\n\
Run `hl7v2 replay . --format json` from this directory, or run the generated script for your shell.\n\n\
## Safety Notes\n\n\
This bundle is intended for support and debugging after safe-analysis redaction. It should not contain raw message PHI in reports, receipts, traces, manifests, or replay output. The profile is user-authored and included as supplied; review it before sharing. Redaction receipts prove configured actions were applied, but they are not a general PHI detector.\n"
    )
}

#[cfg(test)]
mod tests {
    use super::{EvidenceReplayCheckStatus, replay_evidence_bundle, write_safe_analysis_bundle};

    fn raw_message() -> &'static str {
        "MSH|^~\\&|SENDAPP|SENDFAC|RECVAPP|RECVFAC|202605080101||ADT^A01^ADT_A01|CTRL123|P|2.5\rPID|1||123456^^^HOSP^MR||Doe^John||19700101|M"
    }

    fn profile_yaml() -> &'static str {
        r#"
message_structure: ADT_A01
version: "2.5.1"
segments:
  - id: MSH
  - id: PID
constraints:
  - path: MSH.9
    required: true
  - path: PID.3
    required: true
"#
    }

    fn policy_toml() -> &'static str {
        r#"
[[rules]]
path = "PID.3"
action = "hash"
reason = "Patient identifier"

[[rules]]
path = "PID.5"
action = "drop"
reason = "Patient name"

[[rules]]
path = "PID.7"
action = "drop"
reason = "Date of birth"
"#
    }

    fn ensure(condition: bool, message: &'static str) -> Result<(), Box<dyn std::error::Error>> {
        if condition {
            Ok(())
        } else {
            Err(std::io::Error::other(message).into())
        }
    }

    #[test]
    fn bundle_and_replay_keep_phi_out_of_reports() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let bundle_dir = temp.path().join("bundle");
        let summary = write_safe_analysis_bundle(
            raw_message(),
            profile_yaml(),
            policy_toml(),
            &bundle_dir,
            "hl7v2-python",
        )?;

        ensure(summary.bundle_version == "1", "expected bundle version")?;
        ensure(summary.output_dir == ".", "expected local output label")?;
        ensure(summary.message_type == "ADT^A01", "expected message type")?;
        ensure(summary.validation_valid, "expected valid redacted message")?;
        ensure(summary.redaction_phi_removed, "expected PHI removal")?;
        ensure(
            summary
                .artifacts
                .iter()
                .any(|artifact| artifact == "manifest.json"),
            "expected manifest artifact",
        )?;

        let replay = replay_evidence_bundle(&bundle_dir, "hl7v2-python");
        ensure(replay.reproduced, "expected replay to reproduce")?;
        ensure(
            replay.tool_name == "hl7v2-python",
            "expected Python replay tool name",
        )?;
        ensure(
            replay
                .checks
                .iter()
                .all(|check| check.status == EvidenceReplayCheckStatus::Pass),
            "expected all replay checks to pass",
        )?;

        let mut artifact_text = String::new();
        for artifact in [
            "validation-report.json",
            "field-paths.json",
            "redaction-receipt.json",
            "environment.json",
            "manifest.json",
        ] {
            artifact_text.push_str(&std::fs::read_to_string(bundle_dir.join(artifact))?);
        }
        let replay_json = serde_json::to_string(&replay)?;
        artifact_text.push_str(&replay_json);
        for sentinel in ["Doe^John", "123456", "19700101"] {
            ensure(
                !artifact_text.contains(sentinel),
                "raw PHI sentinel leaked into evidence artifacts",
            )?;
        }

        Ok(())
    }

    #[test]
    fn replay_fails_closed_when_manifest_hash_is_wrong() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let bundle_dir = temp.path().join("bundle");
        write_safe_analysis_bundle(
            raw_message(),
            profile_yaml(),
            policy_toml(),
            &bundle_dir,
            "hl7v2-python",
        )?;
        std::fs::write(
            bundle_dir.join("message.redacted.hl7"),
            "MSH|^~\\&|SEND|FAC|RECV|FAC|202605080101||ADT^A01|TAMPER|P|2.5",
        )?;

        let replay = replay_evidence_bundle(&bundle_dir, "hl7v2-python");
        ensure(
            !replay.reproduced,
            "expected tampered bundle to fail replay",
        )?;
        ensure(
            replay.checks.iter().any(|check| {
                check.name == "manifest-hashes" && check.status == EvidenceReplayCheckStatus::Fail
            }),
            "expected manifest hash failure",
        )?;
        Ok(())
    }
}
