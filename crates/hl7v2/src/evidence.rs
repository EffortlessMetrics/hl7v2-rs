//! Evidence bundle and replay helpers.
//!
//! These helpers back evidence-grade workflows that combine redaction,
//! profile validation, artifact manifests, and replay verification.

mod bundle;
mod hash;
mod models;
mod replay;
mod trace;

pub use bundle::{write_safe_analysis_bundle, write_safe_analysis_bundle_with_schema_version};
pub use models::{
    EvidenceBundleEnvironment, EvidenceBundleEnvironmentV2, EvidenceBundleManifest,
    EvidenceBundleManifestArtifact, EvidenceBundleManifestV2, EvidenceBundleSummary,
    EvidenceBundleSummaryV2, EvidenceError, EvidenceReplayCheck, EvidenceReplayCheckStatus,
    EvidenceReplayReport, EvidenceReplayReportV2, FieldPathTrace, FieldPathTraceReport,
    FieldPathTraceReportV2, FieldValueShape,
};
pub use replay::replay_evidence_bundle;

#[cfg(test)]
pub(crate) use replay::{json_string, read_bundle_json_value};

pub(crate) const BUNDLE_VERSION: &str = "1";
pub(crate) const REPLAY_VERSION: &str = "1";
pub(crate) const REPLAY_COMMAND: &str = "hl7v2 replay . --format json";
pub(crate) const BUNDLE_REQUIRED_ARTIFACT_SPECS: [(&str, &str); 9] = [
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

pub(crate) const BUNDLE_ARTIFACT_SPECS: [(&str, &str); 10] = [
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

#[cfg(test)]
mod tests {
    use super::{
        EvidenceReplayCheckStatus, json_string, read_bundle_json_value, replay_evidence_bundle,
        write_safe_analysis_bundle, write_safe_analysis_bundle_with_schema_version,
    };

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
        ensure(
            summary
                .artifacts
                .iter()
                .any(|artifact| artifact == "SAFE-SHARING.md"),
            "expected safe-sharing checklist artifact",
        )?;
        let field_trace = read_bundle_json_value(&bundle_dir, "field-paths.json")?;
        ensure(
            json_string(&field_trace, "message_type").as_deref() == Some("ADT^A01"),
            "expected field trace message type to match validation report",
        )?;
        let summary_v2 = summary.to_v2("hl7v2-python", "1.3.0");
        ensure(
            summary_v2.schema_version == "2",
            "expected bundle summary v2 schema version",
        )?;
        ensure(
            summary_v2.tool_name == "hl7v2-python",
            "expected bundle summary producer",
        )?;
        ensure(
            summary_v2.summary.bundle_version == "1",
            "expected v2 summary to preserve bundle version",
        )?;

        let replay = replay_evidence_bundle(&bundle_dir, "hl7v2-python");
        ensure(replay.reproduced, "expected replay to reproduce")?;
        ensure(
            replay.tool_name == "hl7v2-python",
            "expected Python replay tool name",
        )?;
        let replay_v2 = replay.to_v2();
        ensure(
            replay_v2.schema_version == "2",
            "expected replay v2 schema version",
        )?;
        ensure(
            replay_v2.report.replay_version == "1",
            "expected replay v2 to preserve replay version",
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
            "SAFE-SHARING.md",
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
    fn bundle_schema_version_two_writes_v2_internal_artifacts()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let bundle_dir = temp.path().join("bundle-v2");
        let summary = write_safe_analysis_bundle_with_schema_version(
            raw_message(),
            profile_yaml(),
            policy_toml(),
            &bundle_dir,
            "hl7v2-python",
            2,
        )?;

        ensure(
            summary.bundle_version == "1",
            "expected bundle summary v1 fields",
        )?;
        for artifact in [
            "validation-report.json",
            "manifest.json",
            "field-paths.json",
            "redaction-receipt.json",
            "environment.json",
        ] {
            let json = read_bundle_json_value(&bundle_dir, artifact)?;
            ensure(
                json_string(&json, "schema_version").as_deref() == Some("2"),
                "expected v2 internal bundle artifact",
            )?;
            ensure(
                json_string(&json, "tool_name").as_deref() == Some("hl7v2-python"),
                "expected v2 artifact producer",
            )?;
        }

        let replay = replay_evidence_bundle(&bundle_dir, "hl7v2-python");
        ensure(replay.reproduced, "expected v2 bundle artifacts to replay")?;

        Ok(())
    }

    #[test]
    fn bundle_field_trace_marks_segment_specific_redaction()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let bundle_dir = temp.path().join("targeted-redaction-bundle");
        let message = "MSH|^~\\&|SEND|FAC|RECV|FAC|202605090101||ORU^R01|CTRL1|P|2.5\r\
OBX|1|ST|A||Alpha\r\
OBX|2|ST|B||Beta\r\
OBX|3|ST|C||Gamma";
        let profile = r#"
message_structure: "ORU_R01"
version: "2.5"
segments:
  - id: "MSH"
  - id: "OBX"
constraints:
  - path: "MSH.9"
    required: true
"#;
        let policy = r#"
[[rules]]
path = "OBX[2]-5"
action = "hash"
reason = "Targeted observation redaction"
"#;

        write_safe_analysis_bundle(message, profile, policy, &bundle_dir, "hl7v2-python")?;

        let receipt = read_bundle_json_value(&bundle_dir, "redaction-receipt.json")?;
        let actions = receipt
            .get("actions")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| std::io::Error::other("redaction receipt should contain actions"))?;
        ensure(
            actions.iter().any(|action| {
                action.get("path").and_then(serde_json::Value::as_str) == Some("OBX[2].5")
                    && action
                        .get("matched_count")
                        .and_then(serde_json::Value::as_u64)
                        == Some(1)
            }),
            "expected canonical segment-specific redaction receipt",
        )?;

        let trace = read_bundle_json_value(&bundle_dir, "field-paths.json")?;
        let fields = trace
            .get("fields")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| std::io::Error::other("field trace should contain fields"))?;
        let redacted_fields = fields
            .iter()
            .filter(|field| {
                field
                    .get("redaction_action")
                    .and_then(serde_json::Value::as_str)
                    == Some("hash")
            })
            .collect::<Vec<_>>();
        ensure(
            redacted_fields.len() == 1,
            "expected one field trace redaction marker",
        )?;
        let redacted_field = redacted_fields
            .first()
            .copied()
            .ok_or_else(|| std::io::Error::other("expected redacted field trace"))?;
        ensure(
            redacted_field
                .get("canonical_path")
                .and_then(serde_json::Value::as_str)
                == Some("OBX.5"),
            "expected OBX.5 trace path",
        )?;
        ensure(
            redacted_field
                .get("segment_index")
                .and_then(serde_json::Value::as_u64)
                == Some(3),
            "expected second OBX absolute segment index",
        )?;
        ensure(
            redacted_field
                .get("value_shape")
                .and_then(serde_json::Value::as_str)
                == Some("hashed_sha256"),
            "expected hashed trace value shape",
        )?;

        Ok(())
    }

    #[test]
    fn bundle_field_trace_marks_component_redaction_on_parent_field()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let bundle_dir = temp.path().join("component-redaction-bundle");
        let message = "MSH|^~\\&|SEND|FAC|RECV|FAC|202605090101||ORU^R01|CTRL1|P|2.5\r\
OBX|1|XPN|PATIENT_NAME||Doe^Jane^A";
        let profile = r#"
message_structure: "ORU_R01"
version: "2.5"
segments:
  - id: "MSH"
  - id: "OBX"
constraints:
  - path: "OBX-5.2"
    required: true
"#;
        let policy = r#"
[[rules]]
path = "OBX-5.1"
action = "drop"
reason = "Remove family name"
"#;

        write_safe_analysis_bundle(message, profile, policy, &bundle_dir, "hl7v2-python")?;

        let receipt = read_bundle_json_value(&bundle_dir, "redaction-receipt.json")?;
        let actions = receipt
            .get("actions")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| std::io::Error::other("redaction receipt should contain actions"))?;
        ensure(
            actions.iter().any(|action| {
                action.get("path").and_then(serde_json::Value::as_str) == Some("OBX.5.1")
                    && action
                        .get("matched_count")
                        .and_then(serde_json::Value::as_u64)
                        == Some(1)
            }),
            "expected canonical component redaction receipt",
        )?;

        let redacted_hl7 = std::fs::read_to_string(bundle_dir.join("message.redacted.hl7"))?;
        ensure(
            redacted_hl7.contains("OBX|1|XPN|PATIENT_NAME||^Jane^A"),
            "expected only OBX-5.1 to be cleared",
        )?;

        let trace = read_bundle_json_value(&bundle_dir, "field-paths.json")?;
        let fields = trace
            .get("fields")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| std::io::Error::other("field trace should contain fields"))?;
        let redacted_field = fields
            .iter()
            .find(|field| {
                field
                    .get("canonical_path")
                    .and_then(serde_json::Value::as_str)
                    == Some("OBX.5")
            })
            .ok_or_else(|| std::io::Error::other("expected OBX.5 field trace"))?;
        ensure(
            redacted_field
                .get("redaction_action")
                .and_then(serde_json::Value::as_str)
                == Some("drop"),
            "expected parent field trace to show component drop action",
        )?;
        ensure(
            redacted_field
                .get("value_shape")
                .and_then(serde_json::Value::as_str)
                == Some("present"),
            "expected sibling components to keep field present",
        )?;

        Ok(())
    }

    #[test]
    fn replay_accepts_legacy_bundle_without_safe_sharing_checklist()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let bundle_dir = temp.path().join("legacy-bundle");
        write_safe_analysis_bundle(
            raw_message(),
            profile_yaml(),
            policy_toml(),
            &bundle_dir,
            "hl7v2-python",
        )?;

        let manifest_path = bundle_dir.join("manifest.json");
        let mut manifest: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&manifest_path)?)?;
        let artifacts = manifest
            .get_mut("artifacts")
            .and_then(serde_json::Value::as_array_mut)
            .ok_or_else(|| std::io::Error::other("manifest artifacts should be an array"))?;
        artifacts.retain(|artifact| {
            artifact.get("path").and_then(serde_json::Value::as_str) != Some("SAFE-SHARING.md")
        });
        std::fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)?;
        std::fs::remove_file(bundle_dir.join("SAFE-SHARING.md"))?;

        let replay = replay_evidence_bundle(&bundle_dir, "hl7v2-python");
        ensure(
            replay.reproduced,
            "expected legacy bundle without SAFE-SHARING.md to replay",
        )?;
        ensure(
            replay.checks.iter().any(|check| {
                check.name == "manifest-artifacts"
                    && check.status == EvidenceReplayCheckStatus::Pass
            }),
            "expected legacy manifest catalog to pass",
        )?;

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
