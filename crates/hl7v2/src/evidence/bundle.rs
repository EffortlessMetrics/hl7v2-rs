use super::hash::{compute_sha256, compute_sha256_bytes};
use super::models::{
    EvidenceBundleEnvironment, EvidenceBundleManifest, EvidenceBundleManifestArtifact,
    EvidenceBundleSummary, EvidenceError,
};
use super::trace::build_field_path_trace;
use super::{BUNDLE_ARTIFACT_SPECS, BUNDLE_VERSION, REPLAY_COMMAND};
use crate::conformance::profile::{load_profile_checked, validate};
use crate::conformance::validation::ValidationReport;
use crate::parser::parse;
use crate::redact::redact_hl7_safe_analysis;
use std::fs;
use std::path::Path;

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
    write_safe_analysis_bundle_with_schema_version(
        content,
        profile_yaml,
        policy_text,
        out,
        tool_name,
        1,
    )
}

/// Write a safe-analysis evidence bundle with an explicit internal artifact schema version.
///
/// `artifact_schema_version = 1` preserves the original bundle-internal
/// artifact shapes. `artifact_schema_version = 2` writes v2
/// `validation-report.json`, `manifest.json`, `environment.json`,
/// `field-paths.json`, and `redaction-receipt.json` artifacts with embedded
/// schema/tool provenance.
///
/// # Errors
///
/// Returns [`EvidenceError`] when `artifact_schema_version` is unsupported, or
/// when parsing, profile loading, redaction, validation setup, JSON
/// serialization, or filesystem writes fail. Existing output directories are
/// rejected.
pub fn write_safe_analysis_bundle_with_schema_version(
    content: impl AsRef<[u8]>,
    profile_yaml: &str,
    policy_text: &str,
    out: impl AsRef<Path>,
    tool_name: &str,
    artifact_schema_version: u8,
) -> Result<EvidenceBundleSummary, EvidenceError> {
    if !matches!(artifact_schema_version, 1 | 2) {
        return Err(EvidenceError::InvalidInput(
            "evidence bundle artifact schema version must be 1 or 2".to_string(),
        ));
    }

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
    if artifact_schema_version == 2 {
        write_json_file(
            &out.join("validation-report.json"),
            &validation_report.to_v2(
                tool_name.to_string(),
                env!("CARGO_PKG_VERSION").to_string(),
                None,
            ),
        )?;
        write_json_file(
            &out.join("redaction-receipt.json"),
            &redaction_output
                .receipt
                .to_v2(tool_name.to_string(), env!("CARGO_PKG_VERSION").to_string()),
        )?;
        write_json_file(
            &out.join("field-paths.json"),
            &field_trace.to_v2(tool_name.to_string(), env!("CARGO_PKG_VERSION").to_string()),
        )?;
        write_json_file(&out.join("environment.json"), &environment.to_v2())?;
    } else {
        write_json_file(
            &out.join("redaction-receipt.json"),
            &redaction_output.receipt,
        )?;
        write_json_file(&out.join("validation-report.json"), &validation_report)?;
        write_json_file(&out.join("field-paths.json"), &field_trace)?;
        write_json_file(&out.join("environment.json"), &environment)?;
    }
    fs::write(out.join("replay.sh"), replay_shell_script())
        .map_err(|error| EvidenceError::Io(error.to_string()))?;
    fs::write(out.join("replay.ps1"), replay_powershell_script())
        .map_err(|error| EvidenceError::Io(error.to_string()))?;
    fs::write(out.join("README.md"), bundle_readme(tool_name))
        .map_err(|error| EvidenceError::Io(error.to_string()))?;
    fs::write(
        out.join("SAFE-SHARING.md"),
        safe_sharing_checklist(tool_name),
    )
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
    if artifact_schema_version == 2 {
        write_json_file(&out.join("manifest.json"), &manifest.to_v2())?;
    } else {
        write_json_file(&out.join("manifest.json"), &manifest)?;
    }

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
- `replay.sh` and `replay.ps1`: shell helpers that replay the bundle.\n\
- `SAFE-SHARING.md`: operator checklist for reviewing the packet before attaching it to a ticket.\n\n\
## Replay\n\n\
Run `hl7v2 replay . --format json` from this directory, or run the generated script for your shell.\n\n\
## Safety Notes\n\n\
This bundle is intended for support and debugging after safe-analysis redaction. It should not contain raw message PHI in reports, receipts, traces, manifests, or replay output. The profile is user-authored and included as supplied; review it before sharing. Redaction receipts prove configured actions were applied, but they are not a general PHI detector. See `SAFE-SHARING.md` before sending this packet.\n"
    )
}

fn safe_sharing_checklist(tool_name: &str) -> String {
    format!(
        "# Safe Sharing Checklist\n\n\
This checklist was generated by `{tool_name}` for the surrounding HL7v2 evidence bundle.\n\n\
Before sending this bundle:\n\n\
- Run `hl7v2 replay . --format json` and confirm `reproduced` is `true`.\n\
- Review `redaction-receipt.json`; retained fields must have a support reason.\n\
- Review `profile.yaml`; it is included as supplied and can contain site-specific details.\n\
- Do not attach the original raw HL7, local config, API keys, server logs, or unreviewed policies.\n\
- Share the whole bundle directory so manifest hashes and replay scripts remain intact.\n\
- Treat redaction receipts as configured-policy proof, not universal PHI clearance.\n"
    )
}
