#![cfg(all(feature = "profile", feature = "redact"))]

use hl7v2::evidence::{
    EvidenceReplayCheckStatus, replay_evidence_bundle,
    write_safe_analysis_bundle_with_schema_version,
};
use hl7v2::redact::redact_hl7_safe_analysis;
use hl7v2::{ValidationReport, get, load_profile_checked, parse, validate};
use std::error::Error;
use std::fs;

fn raw_message() -> &'static str {
    "MSH|^~\\&|SEND|FAC|RECV|FAC|202605150930||ADT^A01|CTRL123|P|2.5\r\
PID|1||123456^^^HOSP^MR||Doe^John||19700101|M|||123 Main^^Boston^MA||555-1212\r"
}

fn profile_yaml() -> &'static str {
    r#"
message_structure: "ADT_A01"
version: "2.5"
message_type: "ADT^A01"
segments:
  - id: "MSH"
  - id: "PID"
constraints:
  - path: "MSH.9"
    required: true
  - path: "PID.3"
    required: true
"#
}

fn redaction_policy() -> &'static str {
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

[[rules]]
path = "PID.11"
action = "drop"
reason = "Address"

[[rules]]
path = "PID.13"
action = "drop"
reason = "Phone"
"#
}

fn require(condition: bool, message: &'static str) -> Result<(), Box<dyn Error>> {
    if condition {
        Ok(())
    } else {
        Err(std::io::Error::other(message).into())
    }
}

#[test]
fn journey_rust_validate_redact_bundle_replay_produces_shareable_receipts()
-> Result<(), Box<dyn Error>> {
    let message = parse(raw_message().as_bytes())?;
    require(
        get(&message, "MSH.9.1") == Some("ADT"),
        "expected message code",
    )?;
    require(
        get(&message, "MSH.9.2") == Some("A01"),
        "expected trigger event",
    )?;
    require(
        get(&message, "PID.5.1") == Some("Doe"),
        "expected parsed patient family name",
    )?;

    let profile = load_profile_checked(profile_yaml())?;
    let validation_report = ValidationReport::from_issues(
        &message,
        Some("profile.yaml".into()),
        validate(&message, &profile),
    );
    require(validation_report.valid, "expected message to validate")?;
    require(
        validation_report
            .to_v2("hl7v2", env!("CARGO_PKG_VERSION"), None)
            .schema_version
            == "2",
        "expected validation report v2 provenance",
    )?;

    let redacted = redact_hl7_safe_analysis(raw_message(), redaction_policy())?;
    require(redacted.receipt.phi_removed, "expected PHI removal receipt")?;
    require(
        redacted.redacted_hl7.contains("hash:sha256:"),
        "expected deterministic hash marker",
    )?;
    for sentinel in ["Doe^John", "123456", "19700101", "123 Main", "555-1212"] {
        require(
            !redacted.redacted_hl7.contains(sentinel),
            "redacted HL7 leaked raw PHI",
        )?;
    }

    let temp = tempfile::tempdir()?;
    let bundle_dir = temp.path().join("shareable-bundle");
    let summary = write_safe_analysis_bundle_with_schema_version(
        raw_message(),
        profile_yaml(),
        redaction_policy(),
        &bundle_dir,
        "hl7v2",
        2,
    )?;
    require(summary.validation_valid, "expected valid evidence bundle")?;
    require(
        summary.redaction_phi_removed,
        "expected bundle redaction receipt",
    )?;
    require(
        summary
            .artifacts
            .iter()
            .any(|artifact| artifact == "manifest.json"),
        "expected bundle manifest",
    )?;

    let replay = replay_evidence_bundle(&bundle_dir, "hl7v2");
    require(replay.reproduced, "expected bundle replay to reproduce")?;
    require(
        replay
            .checks
            .iter()
            .all(|check| check.status == EvidenceReplayCheckStatus::Pass),
        "expected replay checks to pass",
    )?;

    let mut shareable_artifacts = String::new();
    for artifact in [
        "message.redacted.hl7",
        "validation-report.json",
        "field-paths.json",
        "redaction-receipt.json",
        "environment.json",
        "manifest.json",
    ] {
        shareable_artifacts.push_str(&fs::read_to_string(bundle_dir.join(artifact))?);
    }
    shareable_artifacts.push_str(&serde_json::to_string(&replay)?);
    for sentinel in ["Doe^John", "123456", "19700101", "123 Main", "555-1212"] {
        require(
            !shareable_artifacts.contains(sentinel),
            "shareable evidence artifact leaked raw PHI",
        )?;
    }

    Ok(())
}
