use super::hash::compute_sha256_bytes;
use super::models::{
    EvidenceBundleManifest, EvidenceReplayCheck, EvidenceReplayCheckStatus, EvidenceReplayReport,
};
use super::{BUNDLE_ARTIFACT_SPECS, REPLAY_VERSION};
use crate::conformance::profile::{load_profile_checked, validate};
use crate::conformance::validation::ValidationReport;
use crate::parser::parse;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

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

pub(crate) fn read_bundle_json_value(
    bundle: &Path,
    artifact: &str,
) -> Result<serde_json::Value, String> {
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

pub(crate) fn json_string(value: &serde_json::Value, key: &str) -> Option<String> {
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
