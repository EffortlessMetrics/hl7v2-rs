//! Cross-surface regression coverage for field-repetition redaction.

use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const MESSAGE: &str = "MSH|^~\\&|SEND|FAC|RECV|FAC|202601010000||ORU^R01|CTRL|P|2.5\rOBX|1|TX|CODE||first^left~second^right";
const PROFILE: &str = r#"
message_structure: "ORU_R01"
version: "2.5.1"
segments:
  - id: "MSH"
  - id: "OBX"
"#;

fn cli_command() -> TestResult<Command> {
    Ok(Command::cargo_bin("hl7v2-cli")?)
}

fn require(condition: bool, message: &'static str) -> TestResult {
    if condition {
        Ok(())
    } else {
        Err(io::Error::other(message).into())
    }
}

fn write_input(dir: &TempDir, path: &str) -> TestResult<(PathBuf, PathBuf, PathBuf)> {
    let message_path = dir.path().join("message.hl7");
    let policy_path = dir.path().join("policy.toml");
    let profile_path = dir.path().join("profile.yaml");
    fs::write(&message_path, MESSAGE)?;
    fs::write(
        &policy_path,
        format!(
            r#"
[[rules]]
path = "{path}"
action = "drop"
reason = "remove observation component"
"#
        ),
    )?;
    fs::write(&profile_path, PROFILE)?;
    Ok((message_path, policy_path, profile_path))
}

fn run_redact(message: &Path, policy: &Path) -> TestResult<(String, u64)> {
    let output = cli_command()?
        .arg("redact")
        .arg(message)
        .arg("--policy")
        .arg(policy)
        .arg("--format")
        .arg("json")
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "redact command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
        .into());
    }
    let json: Value = serde_json::from_slice(&output.stdout)?;
    let redacted_hl7 = json
        .get("redacted_hl7")
        .and_then(Value::as_str)
        .ok_or_else(|| io::Error::other("redact output omitted redacted_hl7"))?
        .to_string();
    let matched_count = json
        .pointer("/receipt/actions/0/matched_count")
        .and_then(Value::as_u64)
        .ok_or_else(|| io::Error::other("redact output omitted matched_count"))?;
    Ok((redacted_hl7, matched_count))
}

fn run_bundle(
    message: &Path,
    policy: &Path,
    profile: &Path,
    out: &Path,
) -> TestResult<(String, u64)> {
    let output = cli_command()?
        .arg("support-bundle")
        .arg(message)
        .arg("--profile")
        .arg(profile)
        .arg("--redact-policy")
        .arg(policy)
        .arg("--out")
        .arg(out)
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "support-bundle command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
        .into());
    }
    let redacted_hl7 = fs::read_to_string(out.join("message.redacted.hl7"))?;
    let receipt: Value = serde_json::from_slice(&fs::read(out.join("redaction-receipt.json"))?)?;
    let matched_count = receipt
        .pointer("/actions/0/matched_count")
        .and_then(Value::as_u64)
        .ok_or_else(|| io::Error::other("bundle receipt omitted matched_count"))?;
    Ok((redacted_hl7, matched_count))
}

fn assert_all_repetitions(output: &str, matched_count: u64) -> TestResult {
    require(
        output.contains("OBX|1|TX|CODE||^left~^right"),
        "omitted repetition selector did not redact every repetition",
    )?;
    require(
        !output.contains("first") && !output.contains("second"),
        "omitted selector leaked a targeted component",
    )?;
    require(
        matched_count == 1,
        "matched_count must count the containing segment once",
    )
}

fn assert_explicit_repetition(output: &str, matched_count: u64) -> TestResult {
    require(
        output.contains("OBX|1|TX|CODE||first^left~^right"),
        "explicit selector did not remain narrow",
    )?;
    require(
        output.contains("first"),
        "explicit selector changed repetition one",
    )?;
    require(
        !output.contains("second"),
        "explicit selector leaked repetition two",
    )?;
    require(
        matched_count == 1,
        "matched_count must count the containing segment once",
    )
}

#[test]
fn redact_command_redacts_all_repetitions_when_selector_is_omitted() -> TestResult {
    let dir = TempDir::new()?;
    let (message, policy, _) = write_input(&dir, "OBX.5.1")?;
    let (output, matched_count) = run_redact(&message, &policy)?;
    assert_all_repetitions(&output, matched_count)
}

#[test]
fn redact_command_keeps_explicit_repetition_narrow() -> TestResult {
    let dir = TempDir::new()?;
    let (message, policy, _) = write_input(&dir, "OBX.5[2].1")?;
    let (output, matched_count) = run_redact(&message, &policy)?;
    assert_explicit_repetition(&output, matched_count)
}

#[test]
fn support_bundle_redacts_all_repetitions_when_selector_is_omitted() -> TestResult {
    let dir = TempDir::new()?;
    let (message, policy, profile) = write_input(&dir, "OBX.5.1")?;
    let out = dir.path().join("bundle");
    let (output, matched_count) = run_bundle(&message, &policy, &profile, &out)?;
    assert_all_repetitions(&output, matched_count)
}

#[test]
fn support_bundle_keeps_explicit_repetition_narrow() -> TestResult {
    let dir = TempDir::new()?;
    let (message, policy, profile) = write_input(&dir, "OBX.5[2].1")?;
    let out = dir.path().join("bundle");
    let (output, matched_count) = run_bundle(&message, &policy, &profile, &out)?;
    assert_explicit_repetition(&output, matched_count)
}
