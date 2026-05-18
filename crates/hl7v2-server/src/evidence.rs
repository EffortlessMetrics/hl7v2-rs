//! Evidence bundle helpers for HTTP endpoints.

use crate::models::{
    EvidenceBundleEnvironment, EvidenceBundleManifest, EvidenceBundleManifestArtifact,
    EvidenceBundleSummary, FieldPathTrace, FieldPathTraceReport, FieldValueShape, QuarantineConfig,
    QuarantineOutputSummary, QuarantineReason, RedactionAction, RedactionReceipt,
};
use hl7v2::{Atom, Field, Message, ValidationReport};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

const BUNDLE_VERSION: &str = "1";
const QUARANTINE_VERSION: &str = "1";
const TOOL_NAME: &str = "hl7v2-server";
const REPLAY_COMMAND: &str = "hl7v2 replay . --format json";

const BUNDLE_ARTIFACT_SPECS: [(&str, &str); 10] = [
    ("message.redacted.hl7", "redacted_message"),
    ("validation-report.json", "validation_report"),
    ("field-paths.json", "field_path_trace"),
    ("profile.yaml", "profile"),
    ("redaction-receipt.json", "redaction_receipt"),
    ("environment.json", "environment"),
    ("replay.sh", "replay_shell_script"),
    ("replay.ps1", "replay_powershell_script"),
    ("README.md", "bundle_readme"),
    ("SAFE-SHARING.md", "safe_sharing_checklist"),
];

/// Evidence bundle write failure.
#[derive(Debug)]
pub enum EvidenceBundleError {
    /// Request data cannot produce a safe bundle.
    InvalidRequest(String),
    /// Bundle output already exists.
    Conflict(String),
    /// Filesystem or serialization failure.
    Io(String),
}

impl fmt::Display for EvidenceBundleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest(message) | Self::Conflict(message) | Self::Io(message) => {
                f.write_str(message)
            }
        }
    }
}

impl std::error::Error for EvidenceBundleError {}

/// Inputs needed to write a redacted evidence bundle.
pub struct EvidenceBundleWriteRequest<'a> {
    /// Configured server bundle root.
    pub root: &'a Path,
    /// Caller-supplied safe bundle identifier.
    pub bundle_id: &'a str,
    /// Public output label returned in response summaries.
    pub public_output_dir: Option<&'a str>,
    /// Original request message bytes.
    pub raw_input: &'a [u8],
    /// Profile YAML included in the bundle.
    pub profile_yaml: &'a str,
    /// Safe-analysis policy TOML included by hash in the bundle.
    pub policy_text: &'a str,
    /// Redacted parsed message used for field traces.
    pub redacted_message: &'a Message,
    /// Redacted HL7 wire payload written to the bundle.
    pub redacted_hl7: &'a str,
    /// Redaction receipt written to the bundle.
    pub redaction_receipt: &'a RedactionReceipt,
    /// Validation report written to the bundle.
    pub validation_report: &'a ValidationReport,
    /// Bundle-internal artifact schema version.
    pub artifact_schema_version: u8,
}

/// Inputs needed to write configured quarantine output.
pub struct QuarantineOutputWriteRequest<'a> {
    /// Configured quarantine output root.
    pub root: &'a Path,
    /// Generated safe quarantine output identifier.
    pub output_id: &'a str,
    /// Quarantine output policy.
    pub config: &'a QuarantineConfig,
    /// Original request message bytes.
    pub raw_input: &'a [u8],
    /// Profile YAML used for validation.
    pub profile_yaml: &'a str,
    /// Safe-analysis policy TOML used for redaction.
    pub policy_text: &'a str,
    /// Redacted parsed message used for field traces.
    pub redacted_message: &'a Message,
    /// Redacted HL7 wire payload.
    pub redacted_hl7: &'a str,
    /// Redaction receipt generated before validation.
    pub redaction_receipt: &'a RedactionReceipt,
    /// Validation report that triggered quarantine output.
    pub validation_report: &'a ValidationReport,
}

/// Write a redacted evidence bundle under a configured server root.
pub fn write_evidence_bundle(
    request: EvidenceBundleWriteRequest<'_>,
) -> Result<EvidenceBundleSummary, EvidenceBundleError> {
    let EvidenceBundleWriteRequest {
        root,
        bundle_id,
        public_output_dir,
        raw_input,
        profile_yaml,
        policy_text,
        redacted_message,
        redacted_hl7,
        redaction_receipt,
        validation_report,
        artifact_schema_version,
    } = request;

    if !matches!(artifact_schema_version, 1 | 2) {
        return Err(EvidenceBundleError::InvalidRequest(
            "evidence bundle artifact schema version must be 1 or 2".to_string(),
        ));
    }

    validate_bundle_id(bundle_id)?;

    let bundle_dir = bundle_path_for_id(root, bundle_id)?;
    fs::create_dir(&bundle_dir).map_err(|error| {
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            EvidenceBundleError::Conflict("bundle output directory already exists".to_string())
        } else {
            EvidenceBundleError::Io(format!("could not create bundle output directory: {error}"))
        }
    })?;

    let message_type = validation_report.message_type.clone();
    let field_trace = build_field_path_trace(redacted_message, redaction_receipt);
    let environment = EvidenceBundleEnvironment {
        bundle_version: BUNDLE_VERSION.to_string(),
        tool_name: TOOL_NAME.to_string(),
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        message_type: message_type.clone(),
        input_sha256: compute_sha256_bytes(raw_input),
        profile_sha256: compute_sha256(profile_yaml),
        redaction_policy_sha256: compute_sha256(policy_text),
        validation_valid: validation_report.valid,
        validation_issue_count: validation_report.issue_count,
        replay_command: REPLAY_COMMAND.to_string(),
    };

    fs::write(bundle_dir.join("message.redacted.hl7"), redacted_hl7).map_err(|error| {
        EvidenceBundleError::Io(format!(
            "could not write redacted message artifact: {error}"
        ))
    })?;
    fs::write(bundle_dir.join("profile.yaml"), profile_yaml).map_err(|error| {
        EvidenceBundleError::Io(format!("could not write profile artifact: {error}"))
    })?;
    write_json_file(
        &bundle_dir.join("validation-report.json"),
        validation_report,
    )?;
    if artifact_schema_version == 2 {
        write_json_file(
            &bundle_dir.join("redaction-receipt.json"),
            &redaction_receipt.to_v2(),
        )?;
        write_json_file(&bundle_dir.join("field-paths.json"), &field_trace.to_v2())?;
        write_json_file(&bundle_dir.join("environment.json"), &environment.to_v2())?;
    } else {
        write_json_file(
            &bundle_dir.join("redaction-receipt.json"),
            redaction_receipt,
        )?;
        write_json_file(&bundle_dir.join("field-paths.json"), &field_trace)?;
        write_json_file(&bundle_dir.join("environment.json"), &environment)?;
    }
    fs::write(bundle_dir.join("replay.sh"), replay_shell_script()).map_err(|error| {
        EvidenceBundleError::Io(format!("could not write replay shell script: {error}"))
    })?;
    fs::write(bundle_dir.join("replay.ps1"), replay_powershell_script()).map_err(|error| {
        EvidenceBundleError::Io(format!("could not write replay PowerShell script: {error}"))
    })?;
    fs::write(bundle_dir.join("README.md"), bundle_readme()).map_err(|error| {
        EvidenceBundleError::Io(format!("could not write bundle README: {error}"))
    })?;
    fs::write(bundle_dir.join("SAFE-SHARING.md"), safe_sharing_checklist()).map_err(|error| {
        EvidenceBundleError::Io(format!("could not write safe-sharing checklist: {error}"))
    })?;

    let manifest = EvidenceBundleManifest {
        bundle_version: BUNDLE_VERSION.to_string(),
        tool_name: TOOL_NAME.to_string(),
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        artifacts: BUNDLE_ARTIFACT_SPECS
            .iter()
            .map(|(path, role)| bundle_manifest_artifact(&bundle_dir, path, role))
            .collect::<Result<_, _>>()?,
    };
    if artifact_schema_version == 2 {
        write_json_file(&bundle_dir.join("manifest.json"), &manifest.to_v2())?;
    } else {
        write_json_file(&bundle_dir.join("manifest.json"), &manifest)?;
    }

    let mut artifacts = BUNDLE_ARTIFACT_SPECS
        .iter()
        .map(|(path, _role)| (*path).to_string())
        .collect::<Vec<_>>();
    artifacts.push("manifest.json".to_string());

    Ok(EvidenceBundleSummary {
        bundle_version: BUNDLE_VERSION.to_string(),
        output_dir: public_output_dir.unwrap_or(bundle_id).to_string(),
        message_type,
        validation_valid: validation_report.valid,
        validation_issue_count: validation_report.issue_count,
        redaction_phi_removed: redaction_receipt.phi_removed,
        artifacts,
    })
}

/// Write quarantine output under a configured quarantine root.
pub fn write_quarantine_output(
    request: QuarantineOutputWriteRequest<'_>,
) -> Result<QuarantineOutputSummary, EvidenceBundleError> {
    let QuarantineOutputWriteRequest {
        root,
        output_id,
        config,
        raw_input,
        profile_yaml,
        policy_text,
        redacted_message,
        redacted_hl7,
        redaction_receipt,
        validation_report,
    } = request;

    if !config.write_bundle && !config.write_report && !config.write_redacted {
        return Err(EvidenceBundleError::InvalidRequest(
            "quarantine output must enable at least one artifact writer".to_string(),
        ));
    }

    if config.write_bundle {
        let bundle = write_evidence_bundle(EvidenceBundleWriteRequest {
            root,
            bundle_id: output_id,
            public_output_dir: None,
            raw_input,
            profile_yaml,
            policy_text,
            redacted_message,
            redacted_hl7,
            redaction_receipt,
            validation_report,
            artifact_schema_version: 1,
        })?;

        return Ok(QuarantineOutputSummary {
            quarantine_version: QUARANTINE_VERSION.to_string(),
            output_dir: bundle.output_dir,
            reason: QuarantineReason::ValidationError,
            validation_issue_count: bundle.validation_issue_count,
            artifacts: bundle.artifacts,
        });
    }

    validate_bundle_id(output_id)?;

    let output_dir = root.join(output_id);
    fs::create_dir(&output_dir).map_err(|error| {
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            EvidenceBundleError::Conflict(format!(
                "quarantine output directory already exists for output_id '{output_id}'"
            ))
        } else {
            EvidenceBundleError::Io(format!(
                "could not create quarantine output directory for output_id '{output_id}': {error}"
            ))
        }
    })?;

    let mut artifacts = Vec::new();
    if config.write_report {
        write_json_file(
            &output_dir.join("validation-report.json"),
            validation_report,
        )?;
        artifacts.push("validation-report.json".to_string());
    }
    if config.write_redacted {
        fs::write(output_dir.join("message.redacted.hl7"), redacted_hl7).map_err(|error| {
            EvidenceBundleError::Io(format!(
                "could not write quarantine redacted message artifact: {error}"
            ))
        })?;
        write_json_file(
            &output_dir.join("redaction-receipt.json"),
            redaction_receipt,
        )?;
        artifacts.push("message.redacted.hl7".to_string());
        artifacts.push("redaction-receipt.json".to_string());
    }

    Ok(QuarantineOutputSummary {
        quarantine_version: QUARANTINE_VERSION.to_string(),
        output_dir: output_id.to_string(),
        reason: QuarantineReason::ValidationError,
        validation_issue_count: validation_report.issue_count,
        artifacts,
    })
}

fn validate_bundle_id(bundle_id: &str) -> Result<(), EvidenceBundleError> {
    let trimmed = bundle_id.trim();
    if trimmed.is_empty() {
        return Err(EvidenceBundleError::InvalidRequest(
            "bundle_id must not be empty".to_string(),
        ));
    }
    if trimmed != bundle_id {
        return Err(EvidenceBundleError::InvalidRequest(
            "bundle_id must not include leading or trailing whitespace".to_string(),
        ));
    }
    if trimmed == "." || trimmed == ".." {
        return Err(EvidenceBundleError::InvalidRequest(
            "bundle_id must be a single safe path segment".to_string(),
        ));
    }
    if trimmed.len() > 128 {
        return Err(EvidenceBundleError::InvalidRequest(
            "bundle_id must be 128 characters or fewer".to_string(),
        ));
    }
    if !trimmed
        .bytes()
        .all(|byte| matches!(byte, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.'))
    {
        return Err(EvidenceBundleError::InvalidRequest(
            "bundle_id may contain only ASCII letters, numbers, '.', '-', and '_'".to_string(),
        ));
    }

    Ok(())
}

/// Return the bundle directory for a caller-supplied bundle id under a configured root.
///
/// The id must be a single safe path segment; callers never provide arbitrary
/// filesystem paths.
pub fn bundle_path_for_id(root: &Path, bundle_id: &str) -> Result<PathBuf, EvidenceBundleError> {
    validate_bundle_id(bundle_id)?;
    Ok(root.join(bundle_id))
}

fn write_json_file<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), EvidenceBundleError> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| {
        EvidenceBundleError::Io(format!("could not serialize bundle artifact JSON: {error}"))
    })?;
    fs::write(path, bytes).map_err(|error| {
        EvidenceBundleError::Io(format!("could not write bundle JSON artifact: {error}"))
    })
}

fn bundle_manifest_artifact(
    bundle_dir: &Path,
    path: &str,
    role: &str,
) -> Result<EvidenceBundleManifestArtifact, EvidenceBundleError> {
    let bytes = fs::read(bundle_dir.join(path)).map_err(|error| {
        EvidenceBundleError::Io(format!(
            "could not read bundle artifact for manifest: {error}"
        ))
    })?;
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
    joined_components(message, "MSH.9").unwrap_or_else(|| "unknown".to_string())
}

fn joined_components(message: &Message, path: &str) -> Option<String> {
    let mut components = Vec::new();

    for index in 1.. {
        let component_path = format!("{}.{}", path, index);
        match hl7v2::get(message, &component_path) {
            Some(value) if !value.is_empty() => components.push(value.to_string()),
            Some(_) => {}
            None => break,
        }
    }

    if components.is_empty() {
        hl7v2::get(message, path).map(str::to_string)
    } else {
        Some(components.join("^"))
    }
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

fn field_to_text(field: &Field, delims: &hl7v2::Delims) -> String {
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

fn bundle_readme() -> &'static str {
    "# HL7v2 Evidence Bundle\n\n\
This directory contains a redacted, replayable evidence packet generated by `hl7v2-server`.\n\n\
## Contents\n\n\
- `message.redacted.hl7`: redacted HL7 message used for replay.\n\
- `validation-report.json`: validation report generated from the redacted message.\n\
- `field-paths.json`: field-path trace and redaction action metadata.\n\
- `profile.yaml`: profile used for replay validation.\n\
- `redaction-receipt.json`: receipt describing retained, hashed, dropped, or missing fields.\n\
- `environment.json`: tool version, bundle metadata, and input/profile/policy hashes.\n\
- `manifest.json`: bundle-relative artifact paths, roles, and SHA-256 hashes.\n\
- `replay.sh` and `replay.ps1`: shell helpers that replay the bundle.\n\
- `SAFE-SHARING.md`: operator checklist for reviewing the packet before attaching it to a ticket.\n\n\
## Replay\n\n\
Run `hl7v2 replay . --format json` from this directory, or run the generated script for your shell.\n\n\
## Safety Notes\n\n\
This bundle is intended for support and debugging after safe-analysis redaction. It should not contain raw message PHI in reports, receipts, traces, manifests, or replay output. The profile is user-authored and included as supplied; review it before sharing. Redaction receipts prove configured actions were applied, but they are not a general PHI detector. See `SAFE-SHARING.md` before sending this packet.\n"
}

fn safe_sharing_checklist() -> &'static str {
    "# Safe Sharing Checklist\n\n\
This checklist was generated by `hl7v2-server` for the surrounding HL7v2 evidence bundle.\n\n\
Before sending this bundle:\n\n\
- Run `hl7v2 replay . --format json` and confirm `reproduced` is `true`.\n\
- Review `redaction-receipt.json`; retained fields must have a support reason.\n\
- Review `profile.yaml`; it is included as supplied and can contain site-specific details.\n\
- Do not attach the original raw HL7, local config, API keys, server logs, or unreviewed policies.\n\
- Share the whole bundle directory so manifest hashes and replay scripts remain intact.\n\
- Treat redaction receipts as configured-policy proof, not universal PHI clearance.\n"
}
