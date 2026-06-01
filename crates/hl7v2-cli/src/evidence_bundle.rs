//! Redaction and evidence bundle command implementation.
//!
//! This module keeps safe-analysis policy handling, bundle creation, and replay
//! verification separate from generic CLI parsing and unrelated commands.

use super::{CliFailure, OutputOptions, RedactFormat, ReportFormat};
use hl7v2::synthetic::corpus::compute_sha256;
use hl7v2::{
    Atom, Comp, Field, Message, Rep, ValidationReport, load_profile_checked, parse, validate, write,
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(serde::Deserialize)]
struct SafeAnalysisPolicy {
    rules: Vec<SafeAnalysisPolicyRule>,
}

#[derive(serde::Deserialize)]
struct SafeAnalysisPolicyRule {
    path: String,
    action: RedactionAction,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    optional: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
enum RedactionAction {
    Hash,
    Drop,
    Retain,
}

#[derive(serde::Serialize)]
struct RedactionOutput {
    input_sha256: String,
    policy_sha256: String,
    message_type: String,
    redacted_hl7: String,
    receipt: RedactionReceipt,
}

#[derive(serde::Serialize)]
struct RedactionOutputWithReceiptV2<'a> {
    schema_version: &'static str,
    tool_name: &'static str,
    tool_version: &'static str,
    input_sha256: String,
    policy_sha256: String,
    message_type: String,
    redacted_hl7: String,
    receipt: RedactionReceiptV2<'a>,
}

#[derive(serde::Serialize)]
struct RedactionReceipt {
    phi_removed: bool,
    hash_algorithm: &'static str,
    actions: Vec<RedactionActionReceipt>,
}

#[derive(serde::Serialize)]
struct RedactionReceiptV2<'a> {
    schema_version: &'static str,
    tool_name: &'static str,
    tool_version: &'static str,
    phi_removed: bool,
    hash_algorithm: &'static str,
    actions: &'a [RedactionActionReceipt],
}

impl RedactionReceipt {
    fn to_v2(&self) -> RedactionReceiptV2<'_> {
        RedactionReceiptV2 {
            schema_version: "2",
            tool_name: "hl7v2-cli",
            tool_version: env!("CARGO_PKG_VERSION"),
            phi_removed: self.phi_removed,
            hash_algorithm: self.hash_algorithm,
            actions: &self.actions,
        }
    }
}

#[derive(serde::Serialize)]
struct RedactionActionReceipt {
    path: String,
    action: RedactionAction,
    reason: String,
    matched_count: usize,
    optional: bool,
    status: RedactionActionStatus,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum RedactionActionStatus {
    Applied,
    Retained,
    NotFound,
}

#[derive(serde::Serialize)]
struct EvidenceBundleSummary {
    bundle_version: &'static str,
    output_dir: String,
    message_type: String,
    validation_valid: bool,
    validation_issue_count: usize,
    redaction_phi_removed: bool,
    artifacts: Vec<String>,
}

#[derive(serde::Serialize)]
struct EvidenceBundleSummaryV2<'a> {
    schema_version: &'static str,
    tool_name: &'static str,
    tool_version: &'static str,
    #[serde(flatten)]
    summary: &'a EvidenceBundleSummary,
}

impl EvidenceBundleSummary {
    fn to_v2(&self) -> EvidenceBundleSummaryV2<'_> {
        EvidenceBundleSummaryV2 {
            schema_version: "2",
            tool_name: "hl7v2-cli",
            tool_version: env!("CARGO_PKG_VERSION"),
            summary: self,
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
struct EvidenceBundleManifest {
    bundle_version: String,
    tool_name: String,
    tool_version: String,
    artifacts: Vec<EvidenceBundleManifestArtifact>,
}

#[derive(serde::Serialize)]
struct EvidenceBundleManifestV2<'a> {
    schema_version: &'static str,
    #[serde(flatten)]
    manifest: &'a EvidenceBundleManifest,
}

impl EvidenceBundleManifest {
    fn to_v2(&self) -> EvidenceBundleManifestV2<'_> {
        EvidenceBundleManifestV2 {
            schema_version: "2",
            manifest: self,
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
struct EvidenceBundleManifestArtifact {
    path: String,
    role: String,
    sha256: String,
}

#[derive(serde::Serialize)]
struct EvidenceBundleEnvironment {
    bundle_version: &'static str,
    tool_name: &'static str,
    tool_version: &'static str,
    message_type: String,
    input_sha256: String,
    profile_sha256: String,
    redaction_policy_sha256: String,
    validation_valid: bool,
    validation_issue_count: usize,
    replay_command: &'static str,
}

#[derive(serde::Serialize)]
struct EvidenceBundleEnvironmentV2<'a> {
    schema_version: &'static str,
    #[serde(flatten)]
    environment: &'a EvidenceBundleEnvironment,
}

impl EvidenceBundleEnvironment {
    fn to_v2(&self) -> EvidenceBundleEnvironmentV2<'_> {
        EvidenceBundleEnvironmentV2 {
            schema_version: "2",
            environment: self,
        }
    }
}

#[derive(serde::Serialize)]
struct EvidenceReplayReport {
    replay_version: &'static str,
    bundle_version: Option<String>,
    tool_name: &'static str,
    tool_version: &'static str,
    message_type: Option<String>,
    reproduced: bool,
    validation_valid: Option<bool>,
    validation_issue_count: Option<usize>,
    checks: Vec<EvidenceReplayCheck>,
    #[serde(skip_serializing_if = "Option::is_none")]
    validation_report: Option<ValidationReport>,
}

#[derive(serde::Serialize)]
struct EvidenceReplayReportV2<'a> {
    schema_version: &'static str,
    #[serde(flatten)]
    report: &'a EvidenceReplayReport,
}

impl EvidenceReplayReport {
    fn to_v2(&self) -> EvidenceReplayReportV2<'_> {
        EvidenceReplayReportV2 {
            schema_version: "2",
            report: self,
        }
    }
}

#[derive(serde::Serialize)]
struct EvidenceReplayCheck {
    name: &'static str,
    status: EvidenceReplayCheckStatus,
    message: String,
}

#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum EvidenceReplayCheckStatus {
    Pass,
    Fail,
}

#[derive(serde::Serialize)]
struct FieldPathTraceReport {
    message_type: String,
    field_count: usize,
    fields: Vec<FieldPathTrace>,
}

#[derive(serde::Serialize)]
struct FieldPathTraceReportV2<'a> {
    schema_version: &'static str,
    tool_name: &'static str,
    tool_version: &'static str,
    #[serde(flatten)]
    trace: &'a FieldPathTraceReport,
}

impl FieldPathTraceReport {
    fn to_v2(&self) -> FieldPathTraceReportV2<'_> {
        FieldPathTraceReportV2 {
            schema_version: "2",
            tool_name: "hl7v2-cli",
            tool_version: env!("CARGO_PKG_VERSION"),
            trace: self,
        }
    }
}

#[derive(serde::Serialize)]
struct FieldPathTrace {
    path: String,
    canonical_path: String,
    segment_index: usize,
    field_index: usize,
    present: bool,
    value_shape: FieldValueShape,
    #[serde(skip_serializing_if = "Option::is_none")]
    redaction_action: Option<RedactionAction>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum FieldValueShape {
    Empty,
    Present,
    HashedSha256,
}

pub(super) fn redact_command(
    input: &Path,
    policy: &Path,
    format: &RedactFormat,
    schema_version: u8,
    output_options: &OutputOptions<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    if *format == RedactFormat::Hl7 && schema_version != 1 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "redaction output schema version is only available with --format json",
        )
        .into());
    }

    let contents = fs::read(input)?;
    let mut message = parse(&contents)?;
    let policy_text = fs::read_to_string(policy)?;
    let redaction_policy = policy::load_safe_analysis_policy(&policy_text)?;
    let receipt = policy::apply_safe_analysis_policy(&mut message, &redaction_policy)?;
    let redacted_hl7 = String::from_utf8(write(&message))?;

    match format {
        RedactFormat::Json => {
            let input_sha256 = compute_sha256(&String::from_utf8_lossy(&contents));
            let policy_sha256 = compute_sha256(&policy_text);
            let message_type = policy::message_field_text(&message, "MSH", 9)
                .unwrap_or_else(|| "unknown".to_string());
            match schema_version {
                1 => {
                    let output = RedactionOutput {
                        input_sha256,
                        policy_sha256,
                        message_type,
                        redacted_hl7,
                        receipt,
                    };
                    output_options.emit(&serde_json::to_string_pretty(&output)?)?;
                }
                2 => {
                    let output = RedactionOutputWithReceiptV2 {
                        schema_version: "2",
                        tool_name: "hl7v2-cli",
                        tool_version: env!("CARGO_PKG_VERSION"),
                        input_sha256,
                        policy_sha256,
                        message_type,
                        redacted_hl7,
                        receipt: receipt.to_v2(),
                    };
                    output_options.emit(&serde_json::to_string_pretty(&output)?)?;
                }
                _ => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "redaction output schema version must be 1 or 2",
                    )
                    .into());
                }
            }
        }
        RedactFormat::Hl7 => {
            output_options.diagnostic(format!(
                "Redaction receipt: {} action(s), PHI removed: {}",
                receipt.actions.len(),
                receipt.phi_removed
            ));
            output_options.emit_raw(&redacted_hl7)?;
        }
    }

    Ok(())
}

pub(super) fn bundle_command(
    input: &Path,
    profile: &Path,
    redact_policy: &Path,
    out: &Path,
    schema_version: u8,
    output_options: &OutputOptions<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    if out.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("bundle output directory already exists: {}", out.display()),
        )
        .into());
    }

    let contents = fs::read(input)?;
    let message = parse(&contents)?;
    let profile_yaml = fs::read_to_string(profile)?;
    let loaded_profile = load_profile_checked(&profile_yaml)?;
    let policy_text = fs::read_to_string(redact_policy)?;
    let redaction_policy = policy::load_safe_analysis_policy(&policy_text)?;

    let mut redacted_message = message.clone();
    let redaction_receipt =
        policy::apply_safe_analysis_policy(&mut redacted_message, &redaction_policy)?;
    let redacted_hl7 = String::from_utf8(write(&redacted_message))?;
    let field_trace = policy::build_field_path_trace(&redacted_message, &redaction_receipt);
    let validation_report = ValidationReport::from_issues(
        &redacted_message,
        Some("profile.yaml".to_string()),
        validate(&redacted_message, &loaded_profile),
    );
    let message_type = validation_report.message_type.clone();
    let environment = EvidenceBundleEnvironment {
        bundle_version: "1",
        tool_name: "hl7v2-cli",
        tool_version: env!("CARGO_PKG_VERSION"),
        message_type: message_type.clone(),
        input_sha256: compute_sha256(&String::from_utf8_lossy(&contents)),
        profile_sha256: compute_sha256(&profile_yaml),
        redaction_policy_sha256: compute_sha256(&policy_text),
        validation_valid: validation_report.valid,
        validation_issue_count: validation_report.issue_count,
        replay_command: "hl7v2 replay . --format json",
    };

    fs::create_dir(out)?;
    fs::write(out.join("message.redacted.hl7"), redacted_hl7)?;
    fs::write(out.join("profile.yaml"), profile_yaml)?;
    write_json_file(&out.join("validation-report.json"), &validation_report)?;
    if schema_version == 2 {
        write_json_file(
            &out.join("redaction-receipt.json"),
            &redaction_receipt.to_v2(),
        )?;
        write_json_file(&out.join("field-paths.json"), &field_trace.to_v2())?;
        write_json_file(&out.join("environment.json"), &environment.to_v2())?;
    } else {
        write_json_file(&out.join("redaction-receipt.json"), &redaction_receipt)?;
        write_json_file(&out.join("field-paths.json"), &field_trace)?;
        write_json_file(&out.join("environment.json"), &environment)?;
    }
    fs::write(out.join("replay.sh"), replay_shell_script())?;
    fs::write(out.join("replay.ps1"), replay_powershell_script())?;
    fs::write(out.join("README.md"), bundle_readme())?;
    fs::write(out.join("SAFE-SHARING.md"), safe_sharing_checklist())?;

    let artifact_specs = bundle_artifact_specs();
    let manifest = EvidenceBundleManifest {
        bundle_version: "1".to_string(),
        tool_name: "hl7v2-cli".to_string(),
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        artifacts: artifact_specs
            .iter()
            .map(|(path, role)| bundle_manifest_artifact(out, path, role))
            .collect::<Result<_, _>>()?,
    };
    if schema_version == 2 {
        write_json_file(&out.join("manifest.json"), &manifest.to_v2())?;
    } else {
        write_json_file(&out.join("manifest.json"), &manifest)?;
    }

    let mut artifacts = artifact_specs
        .iter()
        .map(|(path, _)| (*path).to_string())
        .collect::<Vec<_>>();
    artifacts.push("manifest.json".to_string());

    let summary = EvidenceBundleSummary {
        bundle_version: "1",
        output_dir: ".".to_string(),
        message_type,
        validation_valid: validation_report.valid,
        validation_issue_count: validation_report.issue_count,
        redaction_phi_removed: redaction_receipt.phi_removed,
        artifacts,
    };
    let output = match schema_version {
        1 => serde_json::to_string_pretty(&summary)?,
        2 => serde_json::to_string_pretty(&summary.to_v2())?,
        _ => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "bundle summary schema_version must be 1 or 2",
            )
            .into());
        }
    };
    output_options.emit(&output)?;

    Ok(())
}

pub(super) fn replay_command(
    bundle: &Path,
    format: &ReportFormat,
    schema_version: u8,
    output_options: &OutputOptions<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let report = build_replay_report(bundle);
    output_options.emit(&render_replay_report(&report, format, schema_version)?)?;

    if report.reproduced {
        Ok(())
    } else {
        Err(CliFailure::check_failed(
            "bundle replay did not reproduce stored evidence",
        ))
    }
}

fn build_replay_report(bundle: &Path) -> EvidenceReplayReport {
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
            replay_version: "1",
            bundle_version: manifest_bundle_version,
            tool_name: "hl7v2-cli",
            tool_version: env!("CARGO_PKG_VERSION"),
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
        replay_version: "1",
        bundle_version,
        tool_name: "hl7v2-cli",
        tool_version: env!("CARGO_PKG_VERSION"),
        message_type,
        reproduced,
        validation_valid,
        validation_issue_count,
        checks,
        validation_report: actual_report,
    }
}

fn replay_check(
    name: &'static str,
    status: EvidenceReplayCheckStatus,
    message: impl Into<String>,
) -> EvidenceReplayCheck {
    EvidenceReplayCheck {
        name,
        status,
        message: message.into(),
    }
}

fn required_bundle_artifact_specs() -> [(&'static str, &'static str); 9] {
    [
        ("message.redacted.hl7", "redacted_message"),
        ("validation-report.json", "validation_report"),
        ("field-paths.json", "field_path_trace"),
        ("profile.yaml", "profile"),
        ("redaction-receipt.json", "redaction_receipt"),
        ("environment.json", "environment"),
        ("replay.sh", "replay_shell_script"),
        ("replay.ps1", "replay_powershell_script"),
        ("README.md", "bundle_readme"),
    ]
}

fn bundle_artifact_specs() -> [(&'static str, &'static str); 10] {
    [
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
    ]
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
    let allowed = bundle_artifact_specs();
    let required = required_bundle_artifact_specs();
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
        if !allowed
            .iter()
            .any(|(path, role)| *path == artifact.path.as_str() && *role == artifact.role.as_str())
        {
            errors.push(format!(
                "{} has unexpected role {}",
                artifact.path, artifact.role
            ));
        }
    }

    for (expected_path, expected_role) in required {
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

fn render_replay_report(
    report: &EvidenceReplayReport,
    format: &ReportFormat,
    schema_version: u8,
) -> Result<String, Box<dyn std::error::Error>> {
    match format {
        ReportFormat::Json if schema_version == 2 => {
            Ok(serde_json::to_string_pretty(&report.to_v2())?)
        }
        ReportFormat::Yaml if schema_version == 2 => Ok(serde_yaml::to_string(&report.to_v2())?),
        ReportFormat::Text if schema_version == 2 => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "replay report schema v2 is available only with --format json or --format yaml",
        )
        .into()),
        ReportFormat::Json => Ok(serde_json::to_string_pretty(report)?),
        ReportFormat::Yaml => Ok(serde_yaml::to_string(report)?),
        ReportFormat::Text => {
            let mut output = String::new();
            output.push_str("Evidence Replay\n");
            output.push_str(&format!("  Reproduced: {}\n", report.reproduced));
            if let Some(message_type) = &report.message_type {
                output.push_str(&format!("  Message type: {message_type}\n"));
            }
            if let Some(valid) = report.validation_valid {
                output.push_str(&format!("  Validation valid: {valid}\n"));
            }
            if let Some(issue_count) = report.validation_issue_count {
                output.push_str(&format!("  Validation issues: {issue_count}\n"));
            }
            output.push_str("Checks:\n");
            for check in &report.checks {
                let status = match check.status {
                    EvidenceReplayCheckStatus::Pass => "PASS",
                    EvidenceReplayCheckStatus::Fail => "FAIL",
                };
                output.push_str(&format!("  {status} {} - {}\n", check.name, check.message));
            }
            Ok(output)
        }
    }
}

fn write_json_file<T: serde::Serialize>(
    path: &Path,
    value: &T,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

fn bundle_manifest_artifact(
    bundle_dir: &Path,
    path: &'static str,
    role: &'static str,
) -> Result<EvidenceBundleManifestArtifact, Box<dyn std::error::Error>> {
    let bytes = fs::read(bundle_dir.join(path))?;
    Ok(EvidenceBundleManifestArtifact {
        path: path.to_string(),
        role: role.to_string(),
        sha256: compute_sha256_bytes(&bytes),
    })
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
This directory contains a redacted, replayable evidence packet generated by `hl7v2-cli`.\n\n\
## Contents\n\n\
- `message.redacted.hl7`: redacted HL7 message used for replay.\n\
- `validation-report.json`: validation report generated from the redacted message.\n\
- `field-paths.json`: field-path trace and redaction action metadata.\n\
- `profile.yaml`: profile used for replay validation.\n\
- `redaction-receipt.json`: receipt describing retained, hashed, dropped, or missing fields.\n\
- `environment.json`: tool version, bundle metadata, and input/profile/policy hashes.\n\
- `manifest.json`: bundle-relative artifact paths, roles, and SHA-256 hashes.\n\
- `replay.sh` and `replay.ps1`: shell helpers that run replay and write `replay-report.json`.\n\
- `SAFE-SHARING.md`: operator checklist for reviewing the packet before attaching it to a ticket.\n\n\
## Replay\n\n\
Run `hl7v2 replay . --format json` from this directory, or run the generated script for your shell.\n\n\
## Safety Notes\n\n\
This bundle is intended for support and debugging after safe-analysis redaction. It should not contain raw message PHI in reports, receipts, traces, manifests, or replay output. The profile is user-authored and included as supplied; review it before sharing. Redaction receipts prove configured actions were applied, but they are not a general PHI detector. See `SAFE-SHARING.md` before sending this packet.\n"
}

fn safe_sharing_checklist() -> &'static str {
    "# Safe Sharing Checklist\n\n\
This checklist was generated by `hl7v2-cli` for the surrounding HL7v2 evidence bundle.\n\n\
Before sending this bundle:\n\n\
- Run `hl7v2 replay . --format json` and confirm `reproduced` is `true`.\n\
- Review `redaction-receipt.json`; retained fields must have a support reason.\n\
- Review `profile.yaml`; it is included as supplied and can contain site-specific details.\n\
- Do not attach the original raw HL7, local config, API keys, server logs, or unreviewed policies.\n\
- Share the whole bundle directory so manifest hashes and replay scripts remain intact.\n\
- Treat redaction receipts as configured-policy proof, not universal PHI clearance.\n"
}

mod policy {
    //! Safe-analysis policy loading, application, and path tracing.

    use super::*;

    pub(super) fn load_safe_analysis_policy(
        policy_text: &str,
    ) -> Result<SafeAnalysisPolicy, Box<dyn std::error::Error>> {
        let mut policy: SafeAnalysisPolicy = toml::from_str(policy_text)?;
        if policy.rules.is_empty() {
            return Err(
                std::io::Error::other("redaction policy must contain at least one rule").into(),
            );
        }

        let mut seen_paths = BTreeSet::new();
        for rule in &mut policy.rules {
            let parsed_path = parse_redaction_path(&rule.path).map_err(std::io::Error::other)?;
            rule.path = parsed_path.canonical_path.clone();
            if !seen_paths.insert(rule.path.clone()) {
                return Err(std::io::Error::other(format!(
                    "redaction policy contains duplicate rule for {}",
                    rule.path
                ))
                .into());
            }
            if rule.reason.as_deref().unwrap_or("").trim().is_empty() {
                return Err(std::io::Error::other(format!(
                    "redaction rule {} must include a reason",
                    rule.path
                ))
                .into());
            }
            if rule.action == RedactionAction::Retain
                && redaction_path_targets_builtin_sensitive_field(&parsed_path)
            {
                return Err(std::io::Error::other(format!(
                    "redaction rule {} cannot retain a built-in sensitive field",
                    rule.path
                ))
                .into());
            }
        }

        Ok(policy)
    }

    pub(super) fn apply_safe_analysis_policy(
        message: &mut Message,
        policy: &SafeAnalysisPolicy,
    ) -> Result<RedactionReceipt, Box<dyn std::error::Error>> {
        validate_safe_analysis_policy_covers_sensitive_fields(message, policy)?;

        let mut actions = Vec::new();
        let mut phi_removed = false;
        let mut errors = Vec::new();

        for rule in &policy.rules {
            let parsed_path = parse_redaction_path(&rule.path).map_err(std::io::Error::other)?;
            let mut matched_count = 0_usize;
            let mut segment_match_count = 0_usize;

            for segment in &mut message.segments {
                if segment.id_str() != parsed_path.segment_id {
                    continue;
                }
                segment_match_count = segment_match_count.saturating_add(1);
                if let Some(segment_repetition) = parsed_path.segment_repetition
                    && segment_match_count != segment_repetition
                {
                    continue;
                }

                let Some(field_index) =
                    modeled_field_index(&parsed_path.segment_id, parsed_path.field_index)
                else {
                    continue;
                };
                let Some(field) = segment.fields.get_mut(field_index) else {
                    continue;
                };

                if apply_redaction_target(field, &parsed_path, rule.action, &message.delims) {
                    matched_count = matched_count.saturating_add(1);
                    if rule.action != RedactionAction::Retain {
                        phi_removed = true;
                    }
                }
            }

            let status = match (matched_count, rule.action) {
                (0, _) => RedactionActionStatus::NotFound,
                (_, RedactionAction::Retain) => RedactionActionStatus::Retained,
                _ => RedactionActionStatus::Applied,
            };

            if matched_count == 0 && !rule.optional && rule.action != RedactionAction::Retain {
                errors.push(format!(
                    "redaction rule {} matched no fields; mark optional=true if absence is expected",
                    rule.path
                ));
            }

            actions.push(RedactionActionReceipt {
                path: rule.path.clone(),
                action: rule.action,
                reason: rule.reason.clone().unwrap_or_default(),
                matched_count,
                optional: rule.optional,
                status,
            });
        }

        if !errors.is_empty() {
            return Err(std::io::Error::other(errors.join("; ")).into());
        }

        Ok(RedactionReceipt {
            phi_removed,
            hash_algorithm: "sha256",
            actions,
        })
    }

    fn validate_safe_analysis_policy_covers_sensitive_fields(
        message: &Message,
        policy: &SafeAnalysisPolicy,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let protected_paths: Vec<_> = policy
            .rules
            .iter()
            .filter(|rule| rule.action != RedactionAction::Retain)
            .filter_map(|rule| parse_redaction_path(&rule.path).ok())
            .collect();
        let present_sensitive_paths = present_sensitive_paths(message);
        let missing_paths: BTreeSet<_> = present_sensitive_paths
            .iter()
            .filter(|path| {
                !protected_paths
                    .iter()
                    .any(|protected| protected_path_covers_sensitive_occurrence(protected, path))
            })
            .map(|path| path.base_path)
            .collect();

        if missing_paths.is_empty() {
            return Ok(());
        }

        Err(std::io::Error::other(format!(
            "redaction policy does not protect present sensitive field(s): {}",
            missing_paths.into_iter().collect::<Vec<_>>().join(", ")
        ))
        .into())
    }

    struct PresentSensitivePath {
        base_path: &'static str,
        segment_id: String,
        segment_repetition: usize,
        field_index: usize,
    }

    fn present_sensitive_paths(message: &Message) -> Vec<PresentSensitivePath> {
        let mut paths = Vec::new();
        for base_path in safe_analysis_sensitive_paths() {
            let Ok(parsed) = parse_redaction_path(base_path) else {
                continue;
            };
            let Some(modeled_field_index) =
                modeled_field_index(&parsed.segment_id, parsed.field_index)
            else {
                continue;
            };
            let mut segment_repetition = 0_usize;
            for segment in &message.segments {
                if segment.id_str() != parsed.segment_id {
                    continue;
                }
                segment_repetition = segment_repetition.saturating_add(1);
                let Some(field) = segment.fields.get(modeled_field_index) else {
                    continue;
                };
                if field_to_text(field, &message.delims).is_empty() {
                    continue;
                }
                paths.push(PresentSensitivePath {
                    base_path,
                    segment_id: parsed.segment_id.clone(),
                    segment_repetition,
                    field_index: parsed.field_index,
                });
            }
        }
        paths
    }

    fn safe_analysis_sensitive_paths() -> BTreeSet<&'static str> {
        [
            "PID.3", "PID.5", "PID.7", "PID.11", "PID.13", "PID.14", "PID.19", "NK1.2", "NK1.4",
            "NK1.5",
        ]
        .into_iter()
        .collect()
    }

    fn redaction_path_targets_builtin_sensitive_field(path: &ParsedRedactionPath) -> bool {
        if path.component.is_some() || path.subcomponent.is_some() {
            return false;
        }
        safe_analysis_sensitive_paths()
            .iter()
            .filter_map(|sensitive_path| parse_redaction_path(sensitive_path).ok())
            .any(|sensitive_path| {
                path.segment_id == sensitive_path.segment_id
                    && path.field_index == sensitive_path.field_index
            })
    }

    fn protected_path_covers_sensitive_occurrence(
        protected: &ParsedRedactionPath,
        sensitive: &PresentSensitivePath,
    ) -> bool {
        if protected.segment_id != sensitive.segment_id
            || protected.field_index != sensitive.field_index
        {
            return false;
        }
        if protected.component.is_some()
            || protected.subcomponent.is_some()
            || protected.field_repetition.is_some()
        {
            return false;
        }
        protected
            .segment_repetition
            .is_none_or(|repetition| repetition == sensitive.segment_repetition)
    }

    struct ParsedRedactionPath {
        segment_id: String,
        segment_repetition: Option<usize>,
        field_index: usize,
        field_repetition: Option<usize>,
        component: Option<usize>,
        subcomponent: Option<usize>,
        canonical_path: String,
    }

    fn parse_redaction_path(path: &str) -> Result<ParsedRedactionPath, String> {
        let located = hl7v2::parse_located_path(path).map_err(|error| {
            if !path.contains('.') && !path.contains('-') {
                format!("redaction path '{path}' must use SEG.field or SEG-FIELD syntax")
            } else {
                format!("redaction path '{path}' is invalid: {error}")
            }
        })?;

        if located.path.segment == "MSH" && located.path.field < 3 {
            return Err(format!(
                "redaction path '{path}' targets MSH.1/MSH.2, which are delimiter metadata and not redacted by this command"
            ));
        }

        let canonical_path = located.to_path_string();

        Ok(ParsedRedactionPath {
            segment_id: located.path.segment,
            segment_repetition: located.segment_repetition,
            field_index: located.path.field,
            field_repetition: located.path.repetition,
            component: located.path.component,
            subcomponent: located.path.subcomponent,
            canonical_path,
        })
    }

    pub(super) fn message_field_text(
        message: &Message,
        segment_id: &str,
        field_index: usize,
    ) -> Option<String> {
        let field_index = modeled_field_index(segment_id, field_index)?;
        let field = message
            .segments
            .iter()
            .find(|segment| segment.id_str() == segment_id)?
            .fields
            .get(field_index)?;
        Some(field_to_text(field, &message.delims))
    }

    fn apply_redaction_target(
        field: &mut Field,
        path: &ParsedRedactionPath,
        action: RedactionAction,
        delims: &hl7v2::Delims,
    ) -> bool {
        let Some(target) = select_target(field, path) else {
            return false;
        };

        match action {
            RedactionAction::Hash => target.hash(delims),
            RedactionAction::Drop => target.replace_with_text(String::new()),
            RedactionAction::Retain => {}
        }

        true
    }

    enum RedactionTarget<'a> {
        Field(&'a mut Field),
        Rep(&'a mut Rep),
        Comp(&'a mut Comp),
        Atom(&'a mut Atom),
    }

    impl RedactionTarget<'_> {
        fn hash(self, delims: &hl7v2::Delims) {
            let value = match &self {
                Self::Field(field) => field_to_text(field, delims),
                Self::Rep(rep) => rep_to_text(rep, delims),
                Self::Comp(comp) => comp_to_text(comp, delims),
                Self::Atom(atom) => atom_to_text(atom).to_string(),
            };
            self.replace_with_text(format!("hash:sha256:{}", compute_sha256(&value)));
        }

        fn replace_with_text(self, replacement: String) {
            match self {
                Self::Field(field) => {
                    *field = Field::from_text(replacement);
                }
                Self::Rep(rep) => {
                    *rep = Rep::from_text(replacement);
                }
                Self::Comp(comp) => {
                    *comp = Comp::from_text(replacement);
                }
                Self::Atom(atom) => {
                    *atom = Atom::Text(replacement);
                }
            }
        }
    }

    fn select_target<'a>(
        field: &'a mut Field,
        path: &ParsedRedactionPath,
    ) -> Option<RedactionTarget<'a>> {
        if path.field_repetition.is_none() && path.component.is_none() {
            return Some(RedactionTarget::Field(field));
        }

        let rep_index = path.field_repetition.unwrap_or(1).checked_sub(1)?;
        let rep = field.reps.get_mut(rep_index)?;
        let Some(component) = path.component else {
            return Some(RedactionTarget::Rep(rep));
        };

        let component_index = component.checked_sub(1)?;
        let comp = rep.comps.get_mut(component_index)?;
        let Some(subcomponent) = path.subcomponent else {
            return Some(RedactionTarget::Comp(comp));
        };

        let subcomponent_index = subcomponent.checked_sub(1)?;
        comp.subs
            .get_mut(subcomponent_index)
            .map(RedactionTarget::Atom)
    }

    pub(super) fn build_field_path_trace(
        message: &Message,
        receipt: &RedactionReceipt,
    ) -> FieldPathTraceReport {
        let redaction_actions: Vec<(&str, RedactionAction)> = receipt
            .actions
            .iter()
            .map(|action| (action.path.as_str(), action.action))
            .collect();
        let mut fields = Vec::new();
        let mut segment_occurrences = BTreeMap::<String, usize>::new();

        for (segment_position, segment) in message.segments.iter().enumerate() {
            let segment_index = segment_position.saturating_add(1);
            let segment_occurrence = {
                let count = segment_occurrences
                    .entry(segment.id_str().to_string())
                    .or_insert(0);
                *count = count.saturating_add(1);
                *count
            };
            for (modeled_index, field) in segment.fields.iter().enumerate() {
                let field_index = hl7_field_index(segment.id_str(), modeled_index);
                let canonical_path = format!("{}.{}", segment.id_str(), field_index);
                let occurrence_path = format!(
                    "{}[{}].{}",
                    segment.id_str(),
                    segment_occurrence,
                    field_index
                );
                let field_text = field_to_text(field, &message.delims);
                fields.push(FieldPathTrace {
                    path: occurrence_path.clone(),
                    canonical_path: canonical_path.clone(),
                    segment_index,
                    field_index,
                    present: !field_text.is_empty(),
                    value_shape: field_value_shape(&field_text),
                    redaction_action: redaction_action_for_field(
                        &redaction_actions,
                        &occurrence_path,
                        &canonical_path,
                    ),
                });
            }
        }

        FieldPathTraceReport {
            message_type: message_field_text(message, "MSH", 9).unwrap_or_else(|| "unknown".into()),
            field_count: fields.len(),
            fields,
        }
    }

    fn redaction_action_for_field(
        actions: &[(&str, RedactionAction)],
        occurrence_path: &str,
        canonical_path: &str,
    ) -> Option<RedactionAction> {
        actions.iter().find_map(|(action_path, action)| {
            (path_targets_field(action_path, occurrence_path)
                || path_targets_field(action_path, canonical_path))
            .then_some(*action)
        })
    }

    fn path_targets_field(action_path: &str, field_path: &str) -> bool {
        if action_path == field_path {
            return true;
        }

        action_path
            .strip_prefix(field_path)
            .is_some_and(|suffix| suffix.starts_with('.') || suffix.starts_with('['))
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

    fn modeled_field_index(segment_id: &str, field_index: usize) -> Option<usize> {
        if segment_id == "MSH" {
            field_index.checked_sub(2)
        } else {
            field_index.checked_sub(1)
        }
    }

    fn field_to_text(field: &Field, delims: &hl7v2::Delims) -> String {
        field
            .reps
            .iter()
            .map(|rep| rep_to_text(rep, delims))
            .collect::<Vec<_>>()
            .join(&delims.rep.to_string())
    }

    fn rep_to_text(rep: &Rep, delims: &hl7v2::Delims) -> String {
        rep.comps
            .iter()
            .map(|comp| comp_to_text(comp, delims))
            .collect::<Vec<_>>()
            .join(&delims.comp.to_string())
    }

    fn comp_to_text(comp: &Comp, delims: &hl7v2::Delims) -> String {
        comp.subs
            .iter()
            .map(atom_to_text)
            .collect::<Vec<_>>()
            .join(&delims.sub.to_string())
    }

    fn atom_to_text(atom: &Atom) -> &str {
        match atom {
            Atom::Text(text) => text.as_str(),
            Atom::Null => "\"\"",
        }
    }
}
