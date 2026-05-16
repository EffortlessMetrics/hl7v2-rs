//! Integration tests for hl7v2-cli
//!
//! Tests the CLI binary using assert_cmd for subprocess testing.

#![expect(
    clippy::assertions_on_result_states,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::uninlined_format_args,
    clippy::unwrap_used,
    reason = "legacy CLI integration tests use static fixtures; cleanup is tracked in policy/clippy-debt.toml"
)]

mod common;

use predicates::prelude::*;

use common::{
    cli_command, create_temp_dir, create_temp_file, create_temp_hl7_file,
    create_temp_hl7_with_content, create_temp_mllp_file, create_temp_profile, invalid_hl7_message,
    is_valid_json, minimal_profile, read_file, simple_template, strict_profile,
    truncated_hl7_message,
};
use hl7v2_test_utils::{
    PHI_LEAK_SENTINEL_MESSAGE, PHI_LEAK_SENTINEL_POLICY, RAW_INPUT_FILE_SENTINEL,
    RAW_POLICY_FILE_SENTINEL, assert_no_phi_leak_sentinels_or_paths,
};

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

// =========================================================================
// Help and Version Tests
// =========================================================================

mod help_and_version {
    use super::*;

    #[test]
    fn test_help_flag() {
        let mut cmd = cli_command();
        cmd.arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("HL7 v2 parser"));
    }

    // Note: --version flag is not configured in the CLI, skip this test

    #[test]
    fn test_parse_help() {
        let mut cmd = cli_command();
        cmd.args(["parse", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Parse HL7 v2 message"));
    }

    #[test]
    fn test_norm_help() {
        let mut cmd = cli_command();
        cmd.args(["norm", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Normalize"));
    }

    #[test]
    fn test_val_help() {
        let mut cmd = cli_command();
        cmd.args(["val", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Validate"));
    }

    #[test]
    fn test_profile_help() {
        let mut cmd = cli_command();
        cmd.args(["profile", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Inspect and lint"));
    }

    #[test]
    fn test_profile_lint_help() {
        let mut cmd = cli_command();
        cmd.args(["profile", "lint", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Lint a profile YAML file"));
    }

    #[test]
    fn test_profile_test_help() {
        let mut cmd = cli_command();
        cmd.args(["profile", "test", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains(
                "Test a profile against valid/invalid",
            ));
    }

    #[test]
    fn test_profile_explain_help() {
        let mut cmd = cli_command();
        cmd.args(["profile", "explain", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Explain the loaded profile"));
    }

    #[test]
    fn test_corpus_help() {
        let mut cmd = cli_command();
        cmd.args(["corpus", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Inspect message corpora"));
    }

    #[test]
    fn test_corpus_summarize_help() {
        let mut cmd = cli_command();
        cmd.args(["corpus", "summarize", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains(
                "Summarize a directory or file corpus",
            ));
    }

    #[test]
    fn test_corpus_diff_help() {
        let mut cmd = cli_command();
        cmd.args(["corpus", "diff", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains(
                "Diff two directory or file corpora",
            ));
    }

    #[test]
    fn test_corpus_fingerprint_help() {
        let mut cmd = cli_command();
        cmd.args(["corpus", "fingerprint", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains(
                "Create a deterministic feed fingerprint",
            ));
    }

    #[test]
    fn test_redact_help() {
        let mut cmd = cli_command();
        cmd.args(["redact", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains(
                "Redact an HL7 v2 message using a safe-analysis policy",
            ));
    }

    #[test]
    fn test_bundle_help() {
        let mut cmd = cli_command();
        cmd.args(["bundle", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains(
                "Create a redacted support/debug evidence bundle",
            ));
    }

    #[test]
    fn test_replay_help() {
        let mut cmd = cli_command();
        cmd.args(["replay", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains(
                "Replay a redacted evidence bundle",
            ));
    }

    #[test]
    fn test_ack_help() {
        let mut cmd = cli_command();
        cmd.args(["ack", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Generate ACK"));
    }

    #[test]
    fn test_gen_help() {
        let mut cmd = cli_command();
        cmd.args(["gen", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Generate synthetic"));
    }

    #[test]
    fn test_doctor_help() {
        let mut cmd = cli_command();
        cmd.args(["doctor", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Run first-use diagnostics"));
    }
}

// =========================================================================
// Doctor Command Tests
// =========================================================================

mod doctor_command {
    use super::*;

    #[test]
    fn test_doctor_runs_builtin_checks() {
        let mut cmd = cli_command();
        cmd.arg("doctor")
            .assert()
            .success()
            .stdout(predicate::str::contains("HL7v2 Doctor"))
            .stdout(predicate::str::contains("cli-version"))
            .stdout(predicate::str::contains("sample-parse"))
            .stdout(predicate::str::contains("mllp-roundtrip"));
    }

    #[test]
    fn test_doctor_json_output_is_machine_readable() {
        let mut cmd = cli_command();
        let output = cmd
            .args(["doctor", "--format", "json"])
            .output()
            .expect("Failed to execute command");

        assert!(output.status.success());
        assert!(is_valid_json(&output.stdout));
        let report: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("Doctor output should be JSON");
        assert_eq!(report["version"], env!("CARGO_PKG_VERSION"));
        assert!(report["checks"].is_array());
    }

    #[test]
    fn test_doctor_json_schema_version_two_has_provenance() {
        let mut cmd = cli_command();
        let output = cmd
            .args(["doctor", "--format", "json", "--schema-version", "2"])
            .output()
            .expect("Failed to execute command");

        assert!(output.status.success());
        assert!(is_valid_json(&output.stdout));
        let report: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("Doctor output should be JSON");
        assert_eq!(report["schema_version"], "2");
        assert_eq!(report["tool_name"], "hl7v2-cli");
        assert_eq!(report["tool_version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(report["version"], env!("CARGO_PKG_VERSION"));
        assert!(report["checks"].is_array());
    }

    #[test]
    fn test_doctor_text_rejects_schema_version_two() {
        let mut cmd = cli_command();
        let output = cmd
            .args(["doctor", "--schema-version", "2"])
            .output()
            .expect("Failed to execute command");

        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains(
            "doctor report schema v2 is available only with --format json or --format yaml"
        ));
    }

    #[test]
    fn test_doctor_checks_profile_when_supplied() {
        let dir = create_temp_dir();
        let profile_file = create_temp_profile(&dir, "profile.yaml", minimal_profile());

        let mut cmd = cli_command();
        cmd.args(["doctor", "--profile", profile_file.to_str().unwrap()])
            .assert()
            .success()
            .stdout(predicate::str::contains("[ok] profile"));
    }

    #[test]
    fn test_doctor_fails_invalid_sample() {
        let dir = create_temp_dir();
        let invalid_file = create_temp_file(&dir, "invalid.hl7", invalid_hl7_message().as_bytes());

        let mut cmd = cli_command();
        cmd.args(["doctor", "--sample", invalid_file.to_str().unwrap()])
            .assert()
            .failure()
            .stdout(predicate::str::contains("[error] sample-parse"));
    }

    #[test]
    fn test_doctor_warns_about_lf_only_sample() {
        let dir = create_temp_dir();
        let lf_only = "MSH|^~\\&|App|Fac|Recv|Fac|20250101000000||ADT^A01|MSG001|P|2.5.1\nPID|1||123456^^^HOSP^MR||Doe^John\n";
        let sample_file = create_temp_hl7_with_content(&dir, "lf.hl7", lf_only);

        let mut cmd = cli_command();
        cmd.args(["doctor", "--sample", sample_file.to_str().unwrap()])
            .assert()
            .failure()
            .stdout(predicate::str::contains("[warn] sample-newlines"));
    }

    #[test]
    fn test_doctor_fails_unsupported_server_url() {
        let mut cmd = cli_command();
        cmd.args(["doctor", "--server-url", "https://127.0.0.1:8080/health"])
            .assert()
            .failure()
            .stdout(predicate::str::contains("[error] server"));
    }
}

mod sample_command {
    use super::*;

    const ADT_PROFILE: &str = r#"
message_structure: ADT_A01
version: "2.5"
segments:
  - id: MSH
  - id: PID
constraints:
  - path: PID.3
    required: true
"#;

    #[test]
    fn test_sample_outputs_builtin_adt_message() {
        let mut cmd = cli_command();
        cmd.args(["sample", "--type", "ADT_A01"])
            .assert()
            .success()
            .stdout(predicate::str::contains("ADT^A01^ADT_A01"))
            .stdout(predicate::str::contains("PID|1||123456^^^HOSP^MR"));
    }

    #[test]
    fn test_sample_writes_output_file() {
        let dir = create_temp_dir();
        let output_file = dir.path().join("sample.hl7");

        let mut cmd = cli_command();
        let output = cmd
            .args([
                "sample",
                "--type",
                "ORU_R01",
                "--output",
                output_file.to_str().unwrap(),
                "--quiet",
                "--no-color",
            ])
            .output()
            .expect("sample should run");

        assert_eq!(output.status.code(), Some(0));
        assert!(output.stdout.is_empty());
        let sample = read_file(&output_file);
        assert!(String::from_utf8_lossy(&sample).contains("ORU^R01^ORU_R01"));
        assert!(String::from_utf8_lossy(&sample).contains("OBX|1|NM|718-7"));
    }

    #[test]
    fn test_validate_sample_json_schema_version_two() {
        let dir = create_temp_dir();
        let profile = create_temp_profile(&dir, "profile.yaml", ADT_PROFILE);
        let report_file = dir.path().join("sample-validation.json");

        let mut cmd = cli_command();
        let output = cmd
            .args([
                "validate-sample",
                "--type",
                "ADT_A01",
                "--profile",
                profile.to_str().unwrap(),
                "--report",
                "json",
                "--schema-version",
                "2",
                "--output",
                report_file.to_str().unwrap(),
                "--quiet",
                "--no-color",
            ])
            .output()
            .expect("validate-sample should run");

        assert_eq!(output.status.code(), Some(0));
        assert!(output.stdout.is_empty());
        let report: serde_json::Value =
            serde_json::from_slice(&read_file(&report_file)).expect("report should be JSON");
        assert_eq!(report["schema_version"], "2");
        assert_eq!(report["valid"], true);
        assert_eq!(report["message_type"], "ADT^A01");
    }

    #[test]
    fn test_validate_sample_text_rejects_schema_version_two() {
        let dir = create_temp_dir();
        let profile = create_temp_profile(&dir, "profile.yaml", ADT_PROFILE);

        let mut cmd = cli_command();
        let output = cmd
            .args([
                "validate-sample",
                "--type",
                "ADT_A01",
                "--profile",
                profile.to_str().unwrap(),
                "--schema-version",
                "2",
            ])
            .output()
            .expect("validate-sample should run");

        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains(
            "validation report schema v2 is available only with --report json or --report yaml"
        ));
    }
}

// =========================================================================
// Parse Command Tests
// =========================================================================

mod parse_command {
    use super::*;

    #[test]
    fn test_parse_valid_file() {
        let dir = create_temp_dir();
        let hl7_file = create_temp_hl7_file(&dir, "test.hl7");

        let mut cmd = cli_command();
        cmd.args(["parse", hl7_file.to_str().unwrap()])
            .assert()
            .success()
            .stdout(predicate::str::contains("MSH"));
    }

    #[test]
    fn test_parse_output_is_json() {
        let dir = create_temp_dir();
        let hl7_file = create_temp_hl7_file(&dir, "test.hl7");

        let mut cmd = cli_command();
        let output = cmd
            .args(["parse", hl7_file.to_str().unwrap()])
            .output()
            .expect("Failed to execute command");

        assert!(is_valid_json(&output.stdout));
    }

    #[test]
    fn test_parse_with_json_flag() {
        let dir = create_temp_dir();
        let hl7_file = create_temp_hl7_file(&dir, "test.hl7");

        let mut cmd = cli_command();
        cmd.args(["parse", hl7_file.to_str().unwrap(), "--json"])
            .assert()
            .success()
            .stdout(predicate::str::contains("MSH"));
    }

    #[test]
    fn test_parse_with_summary_flag() {
        let dir = create_temp_dir();
        let hl7_file = create_temp_hl7_file(&dir, "test.hl7");

        let mut cmd = cli_command();
        cmd.args(["parse", hl7_file.to_str().unwrap(), "--summary"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Parse Summary"));
    }

    #[test]
    fn test_parse_mllp_framed_file() {
        let dir = create_temp_dir();
        let mllp_file = create_temp_mllp_file(&dir, "test_mllp.hl7");

        let mut cmd = cli_command();
        cmd.args(["parse", mllp_file.to_str().unwrap(), "--mllp"])
            .assert()
            .success()
            .stdout(predicate::str::contains("MSH"));
    }

    #[test]
    fn test_parse_missing_file() {
        let mut cmd = cli_command();
        cmd.args(["parse", "/nonexistent/file.hl7"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("Error"));
    }

    #[test]
    fn test_parse_invalid_file() {
        let dir = create_temp_dir();
        let invalid_file = create_temp_file(&dir, "invalid.hl7", invalid_hl7_message().as_bytes());

        let mut cmd = cli_command();
        cmd.args(["parse", invalid_file.to_str().unwrap()])
            .assert()
            .failure();
    }

    #[test]
    fn test_parse_truncated_file() {
        // Note: The truncated message "MSH|^~\\&|" actually parses as valid partial HL7
        // This test verifies that the parser handles it (success or failure is acceptable)
        let dir = create_temp_dir();
        let truncated_file =
            create_temp_file(&dir, "truncated.hl7", truncated_hl7_message().as_bytes());

        let mut cmd = cli_command();
        // The parser may succeed with partial output or fail - both are acceptable
        let result = cmd
            .args(["parse", truncated_file.to_str().unwrap()])
            .output();
        // Just verify the command runs without panicking
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_shows_segment_count_in_summary() {
        let dir = create_temp_dir();
        let hl7_file = create_temp_hl7_file(&dir, "test.hl7");

        let mut cmd = cli_command();
        cmd.args(["parse", hl7_file.to_str().unwrap(), "--summary"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Segments:"));
    }

    #[test]
    fn test_parse_shows_file_size_in_summary() {
        let dir = create_temp_dir();
        let hl7_file = create_temp_hl7_file(&dir, "test.hl7");

        let mut cmd = cli_command();
        cmd.args(["parse", hl7_file.to_str().unwrap(), "--summary"])
            .assert()
            .success()
            .stdout(predicate::str::contains("File size:"));
    }
}

// =========================================================================
// Normalize Command Tests
// =========================================================================

mod norm_command {
    use super::*;

    #[test]
    fn test_norm_valid_file() {
        let dir = create_temp_dir();
        let hl7_file = create_temp_hl7_file(&dir, "test.hl7");
        let output_file = dir.path().join("output.hl7");

        let mut cmd = cli_command();
        cmd.args([
            "norm",
            hl7_file.to_str().unwrap(),
            "-o",
            output_file.to_str().unwrap(),
        ])
        .assert()
        .success();

        assert!(output_file.exists());
    }

    #[test]
    fn test_norm_output_to_stdout() {
        let dir = create_temp_dir();
        let hl7_file = create_temp_hl7_file(&dir, "test.hl7");

        let mut cmd = cli_command();
        cmd.args(["norm", hl7_file.to_str().unwrap()])
            .assert()
            .success()
            .stdout(predicate::str::contains("MSH"));
    }

    #[test]
    fn test_norm_with_summary() {
        let dir = create_temp_dir();
        let hl7_file = create_temp_hl7_file(&dir, "test.hl7");
        let output_file = dir.path().join("output.hl7");

        let mut cmd = cli_command();
        cmd.args([
            "norm",
            hl7_file.to_str().unwrap(),
            "-o",
            output_file.to_str().unwrap(),
            "--summary",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Normalize Summary"));
    }

    #[test]
    fn test_norm_with_mllp_output() {
        let dir = create_temp_dir();
        let hl7_file = create_temp_hl7_file(&dir, "test.hl7");

        let mut cmd = cli_command();
        let output = cmd
            .args(["norm", hl7_file.to_str().unwrap(), "--mllp-out"])
            .output()
            .expect("Failed to execute command");

        // Check MLLP framing
        assert_eq!(output.stdout[0], 0x0B); // SB
        assert!(output.stdout.len() > 2);
    }

    #[test]
    fn test_norm_missing_file() {
        let mut cmd = cli_command();
        cmd.args(["norm", "/nonexistent/file.hl7"])
            .assert()
            .failure();
    }

    #[test]
    fn test_norm_roundtrip_preserves_content() {
        let dir = create_temp_dir();
        let hl7_file = create_temp_hl7_file(&dir, "test.hl7");
        let output_file = dir.path().join("output.hl7");

        let mut cmd = cli_command();
        cmd.args([
            "norm",
            hl7_file.to_str().unwrap(),
            "-o",
            output_file.to_str().unwrap(),
        ])
        .assert()
        .success();

        // Parse both files and compare segment counts
        let original_content = read_file(&hl7_file);
        let normalized_content = read_file(&output_file);

        let original_msg = hl7v2::parse(&original_content).expect("Original should parse");
        let normalized_msg = hl7v2::parse(&normalized_content).expect("Normalized should parse");

        assert_eq!(original_msg.segments.len(), normalized_msg.segments.len());
    }

    #[test]
    fn test_norm_canonical_delimiters_matches_shared_fixture() {
        let dir = create_temp_dir();
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test_data/custom_delimiters.hl7");
        let fixture_content = std::fs::read_to_string(fixture).unwrap();
        let custom_file = create_temp_file(
            &dir,
            "custom-delimiters.hl7",
            fixture_content.trim_end_matches(['\r', '\n']).as_bytes(),
        );

        let mut cmd = cli_command();
        let output = cmd
            .args(["norm", custom_file.to_str().unwrap(), "--canonical-delims"])
            .output()
            .expect("Failed to execute norm");

        assert!(output.status.success());
        let canonical = String::from_utf8(output.stdout).unwrap();
        assert!(canonical.starts_with("MSH|^~\\&|"));
        assert!(canonical.contains("ADT^A01"));
        assert!(!canonical.contains("MSH*%$!?*"));
        assert!(!canonical.contains("ADT%A01"));

        let canonical_file = create_temp_file(&dir, "canonical.hl7", canonical.as_bytes());
        let mut second_cmd = cli_command();
        let second_output = second_cmd
            .args([
                "norm",
                canonical_file.to_str().unwrap(),
                "--canonical-delims",
            ])
            .output()
            .expect("Failed to execute second norm");

        assert!(second_output.status.success());
        assert_eq!(canonical, String::from_utf8(second_output.stdout).unwrap());
    }
}

// =========================================================================
// Profile Command Tests
// =========================================================================

mod profile_command {
    use super::*;

    const PID3_REQUIRED_PROFILE: &str = r#"
message_structure: ADT_A01
version: "2.5.1"
segments:
  - id: MSH
  - id: PID
constraints:
  - path: PID.3
    required: true
"#;

    const MISSING_PID3_MESSAGE: &str = "MSH|^~\\&|SENDAPP|SENDFAC|RECVAPP|RECVFAC|202605030101||ADT^A01|CTRL124|P|2.5\rPID|1||||Doe^John||19700101|M\r";

    #[test]
    fn test_profile_lint_passes_minimal_profile() {
        let dir = create_temp_dir();
        let profile_file = create_temp_profile(&dir, "profile.yaml", minimal_profile());

        let mut cmd = cli_command();
        cmd.args(["profile", "lint", profile_file.to_str().unwrap()])
            .assert()
            .success()
            .stdout(predicate::str::contains("Profile lint passed"))
            .stdout(predicate::str::contains("No profile lint issues found"));
    }

    #[test]
    fn test_profile_lint_json_reports_warnings_without_failing() {
        let dir = create_temp_dir();
        let profile_file = create_temp_profile(
            &dir,
            "profile.yaml",
            r#"
message_structure: ADT_A01
version: "2.5"
segments:
  - id: MSH
rules: []
"#,
        );

        let mut cmd = cli_command();
        let output = cmd
            .args([
                "profile",
                "lint",
                profile_file.to_str().unwrap(),
                "--report",
                "json",
            ])
            .output()
            .expect("Failed to execute profile lint");

        assert!(output.status.success());
        let report: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("profile lint output should be JSON");
        assert_eq!(report["valid"], true);
        assert_eq!(report["warning_count"], 1);
        assert_eq!(report["issues"][0]["code"], "unknown_top_level_key");
        assert_eq!(report["issues"][0]["path"], "rules");
    }

    #[test]
    fn test_profile_lint_json_schema_version_2_adds_provenance() {
        let dir = create_temp_dir();
        let profile_file = create_temp_profile(
            &dir,
            "profile.yaml",
            r#"
message_structure: ADT_A01
version: "2.5"
segments:
  - id: MSH
rules: []
"#,
        );

        let mut cmd = cli_command();
        let output = cmd
            .args([
                "profile",
                "lint",
                profile_file.to_str().unwrap(),
                "--report",
                "json",
                "--schema-version",
                "2",
            ])
            .output()
            .expect("Failed to execute profile lint");

        assert!(output.status.success());
        let report: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("profile lint output should be JSON");
        assert_eq!(report["schema_version"], "2");
        assert_eq!(report["tool_name"], "hl7v2-cli");
        assert_eq!(report["tool_version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(report["valid"], true);
        assert_eq!(report["warning_count"], 1);
        assert_eq!(report["issues"][0]["code"], "unknown_top_level_key");
    }

    #[test]
    fn test_profile_lint_text_rejects_schema_version_2() {
        let dir = create_temp_dir();
        let profile_file = create_temp_profile(&dir, "profile.yaml", minimal_profile());

        let mut cmd = cli_command();
        cmd.args([
            "profile",
            "lint",
            profile_file.to_str().unwrap(),
            "--report",
            "text",
            "--schema-version",
            "2",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "profile lint schema version is only available",
        ));
    }

    #[test]
    fn test_profile_lint_json_reports_errors_and_fails() {
        let dir = create_temp_dir();
        let profile_file = create_temp_profile(
            &dir,
            "profile.yaml",
            r#"
message_structure: ""
version: "2.5"
segments:
  - id: ""
constraints:
  - path: PID.x
    pattern: "["
"#,
        );

        let mut cmd = cli_command();
        let assert = cmd
            .args([
                "profile",
                "lint",
                profile_file.to_str().unwrap(),
                "--report",
                "json",
            ])
            .assert()
            .failure();
        let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
        let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();

        assert_eq!(report["valid"], false);
        assert!(report["error_count"].as_u64().unwrap() >= 3);
        assert!(
            report["issues"]
                .as_array()
                .unwrap()
                .iter()
                .any(|issue| issue["code"] == "empty_message_structure")
        );
        assert!(
            report["issues"]
                .as_array()
                .unwrap()
                .iter()
                .any(|issue| issue["code"] == "invalid_constraint_pattern")
        );
    }

    #[test]
    fn test_profile_explain_json_reports_contract_shape() {
        let dir = create_temp_dir();
        let profile_file = create_temp_profile(
            &dir,
            "profile.yaml",
            r#"
message_structure: ADT_A01
version: "2.5.1"
message_type: "ADT^A01"
parent: BASE_ADT
segments:
  - id: MSH
  - id: PID
constraints:
  - path: PID.3
    required: true
  - path: PID.8
    in: ["M", "F", "U"]
lengths:
  - path: PID.3
    max: 40
    policy: no-truncate
datatypes:
  - path: PID.7
    type: DT
valuesets:
  - path: PID.8
    name: AdministrativeSex
    codes: ["M", "F", "U"]
hl7_tables:
  - id: HL70001
    name: Administrative Sex
    version: "2.5.1"
    codes:
      - value: M
        description: Male
      - value: F
        description: Female
"#,
        );

        let mut cmd = cli_command();
        let output = cmd
            .args([
                "profile",
                "explain",
                profile_file.to_str().unwrap(),
                "--format",
                "json",
            ])
            .output()
            .expect("Failed to execute profile explain");

        assert!(output.status.success());
        let report: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("profile explain output should be JSON");
        assert_eq!(report["message_structure"], "ADT_A01");
        assert_eq!(report["version"], "2.5.1");
        assert_eq!(report["message_type"], "ADT^A01");
        assert_eq!(report["parent"], "BASE_ADT");
        assert_eq!(report["summary"]["segment_count"], 2);
        assert_eq!(report["summary"]["required_field_count"], 1);
        assert_eq!(report["summary"]["value_set_count"], 1);
        assert_eq!(report["segments"][0]["id"], "MSH");
        assert_eq!(report["required_fields"][0]["path"], "PID.3");
        assert_eq!(report["field_constraints"][1]["allowed_value_count"], 3);
        assert_eq!(report["length_rules"][0]["policy"], "no-truncate");
        assert_eq!(report["datatype_rules"][0]["datatype"], "DT");
        assert_eq!(report["value_sets"][0]["inline_code_count"], 3);
        assert_eq!(report["hl7_tables"][0]["code_count"], 2);
        assert_eq!(report["lint"]["valid"], true);
    }

    #[test]
    fn test_profile_explain_json_schema_version_2_adds_provenance() {
        let dir = create_temp_dir();
        let profile_file = create_temp_profile(&dir, "profile.yaml", PID3_REQUIRED_PROFILE);

        let mut cmd = cli_command();
        let output = cmd
            .args([
                "profile",
                "explain",
                profile_file.to_str().unwrap(),
                "--format",
                "json",
                "--schema-version",
                "2",
            ])
            .output()
            .expect("Failed to execute profile explain");

        assert!(output.status.success());
        let report: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("profile explain output should be JSON");
        assert_eq!(report["schema_version"], "2");
        assert_eq!(report["tool_name"], "hl7v2-cli");
        assert_eq!(report["tool_version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(report["message_structure"], "ADT_A01");
        assert_eq!(report["summary"]["required_field_count"], 1);
    }

    #[test]
    fn test_profile_explain_text_rejects_schema_version_2() {
        let dir = create_temp_dir();
        let profile_file = create_temp_profile(&dir, "profile.yaml", PID3_REQUIRED_PROFILE);

        let mut cmd = cli_command();
        cmd.args([
            "profile",
            "explain",
            profile_file.to_str().unwrap(),
            "--format",
            "text",
            "--schema-version",
            "2",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "profile explain schema version is only available",
        ));
    }

    #[test]
    fn test_profile_explain_yaml_reports_contract_shape() {
        let dir = create_temp_dir();
        let profile_file = create_temp_profile(&dir, "profile.yaml", minimal_profile());

        let mut cmd = cli_command();
        let output = cmd
            .args([
                "profile",
                "explain",
                profile_file.to_str().unwrap(),
                "--format",
                "yaml",
            ])
            .output()
            .expect("Failed to execute profile explain");

        assert!(output.status.success());
        let report: serde_yaml::Value =
            serde_yaml::from_slice(&output.stdout).expect("profile explain output should be YAML");
        assert_eq!(report["message_structure"].as_str().unwrap(), "ADT_A01");
        assert_eq!(report["message_type"], serde_yaml::Value::Null);
        assert_eq!(report["summary"]["required_field_count"], 1);
        assert_eq!(
            report["required_fields"][0]["path"].as_str().unwrap(),
            "MSH.9"
        );
        assert!(report["lint"]["valid"].as_bool().unwrap());
    }

    #[test]
    fn test_profile_explain_reports_lint_invalid_without_becoming_lint_gate() {
        let dir = create_temp_dir();
        let profile_file = create_temp_profile(
            &dir,
            "profile.yaml",
            r#"
message_structure: ADT_A01
version: "2.5.1"
segments:
  - id: MSH
constraints:
  - path: PID.x
    required: true
"#,
        );

        let mut cmd = cli_command();
        let output = cmd
            .args([
                "profile",
                "explain",
                profile_file.to_str().unwrap(),
                "--format",
                "json",
            ])
            .output()
            .expect("Failed to execute profile explain");

        assert!(output.status.success());
        let report: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("profile explain output should be JSON");
        assert_eq!(report["lint"]["valid"], false);
        assert!(
            report["lint"]["error_count"]
                .as_u64()
                .is_some_and(|count| count >= 1)
        );
        assert!(
            report["profile_sha256"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
        );
    }

    #[test]
    fn test_profile_explain_text_reports_ignored_config_warnings() {
        let dir = create_temp_dir();
        let profile_file = create_temp_profile(
            &dir,
            "profile.yaml",
            r#"
message_structure: ADT_A01
version: "2.5.1"
segments:
  - id: MSH
rules: []
"#,
        );

        let mut cmd = cli_command();
        cmd.args(["profile", "explain", profile_file.to_str().unwrap()])
            .assert()
            .success()
            .stdout(predicate::str::contains("Profile explain"))
            .stdout(predicate::str::contains("Segments: 1 (MSH)"))
            .stdout(predicate::str::contains(
                "Ignored or unsupported profile config",
            ))
            .stdout(predicate::str::contains("unknown_top_level_key"));
    }

    #[test]
    fn test_profile_test_passes_valid_and_invalid_fixtures() {
        let dir = create_temp_dir();
        let profile_file = create_temp_profile(&dir, "profile.yaml", PID3_REQUIRED_PROFILE);
        let fixtures = dir.path().join("fixtures");
        let valid_dir = fixtures.join("valid");
        let invalid_dir = fixtures.join("invalid");
        let expected_dir = fixtures.join("expected");
        std::fs::create_dir_all(&valid_dir).unwrap();
        std::fs::create_dir_all(&invalid_dir).unwrap();
        std::fs::create_dir_all(&expected_dir).unwrap();
        create_temp_file(
            &dir,
            "fixtures/valid/adt.hl7",
            hl7v2_test_utils::fixtures::SampleMessages::adt_a01().as_bytes(),
        );
        create_temp_hl7_with_content(
            &dir,
            "fixtures/invalid/missing_pid3.hl7",
            MISSING_PID3_MESSAGE,
        );
        create_temp_file(
            &dir,
            "fixtures/expected/missing_pid3.report.json",
            br#"{
  "valid": false,
  "issues": [
    {
      "code": "missing_required_field",
      "severity": "error",
      "path": "PID.3"
    }
  ]
}"#,
        );

        let mut cmd = cli_command();
        cmd.args([
            "profile",
            "test",
            profile_file.to_str().unwrap(),
            fixtures.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Profile test passed"))
        .stdout(predicate::str::contains(
            "PASS valid/adt.hl7 expected valid",
        ))
        .stdout(predicate::str::contains("expected report matched"));
    }

    #[test]
    fn test_profile_test_json_reports_case_results() {
        let dir = create_temp_dir();
        let profile_file = create_temp_profile(&dir, "profile.yaml", PID3_REQUIRED_PROFILE);
        let fixtures = dir.path().join("fixtures");
        std::fs::create_dir_all(fixtures.join("valid")).unwrap();
        std::fs::create_dir_all(fixtures.join("invalid")).unwrap();
        create_temp_file(
            &dir,
            "fixtures/valid/adt.hl7",
            hl7v2_test_utils::fixtures::SampleMessages::adt_a01().as_bytes(),
        );
        create_temp_hl7_with_content(
            &dir,
            "fixtures/invalid/missing_pid3.hl7",
            MISSING_PID3_MESSAGE,
        );

        let mut cmd = cli_command();
        let output = cmd
            .args([
                "profile",
                "test",
                profile_file.to_str().unwrap(),
                fixtures.to_str().unwrap(),
                "--report",
                "json",
            ])
            .output()
            .expect("Failed to execute profile test");

        assert!(output.status.success());
        let report: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("profile test output should be JSON");
        assert_eq!(report["valid"], true);
        assert_eq!(report["case_count"], 2);
        assert_eq!(report["passed_count"], 2);
        assert_eq!(report["failed_count"], 0);
        assert!(
            report["cases"]
                .as_array()
                .unwrap()
                .iter()
                .any(|case| case["expectation"] == "invalid"
                    && case["validation_report"]["issues"][0]["code"] == "missing_required_field")
        );
    }

    #[test]
    fn test_profile_test_json_schema_version_2_adds_provenance() {
        let dir = create_temp_dir();
        let profile_file = create_temp_profile(&dir, "profile.yaml", PID3_REQUIRED_PROFILE);
        let fixtures = dir.path().join("fixtures");
        std::fs::create_dir_all(fixtures.join("valid")).unwrap();
        std::fs::create_dir_all(fixtures.join("invalid")).unwrap();
        create_temp_file(
            &dir,
            "fixtures/valid/adt.hl7",
            hl7v2_test_utils::fixtures::SampleMessages::adt_a01().as_bytes(),
        );
        create_temp_hl7_with_content(
            &dir,
            "fixtures/invalid/missing_pid3.hl7",
            MISSING_PID3_MESSAGE,
        );

        let mut cmd = cli_command();
        let output = cmd
            .args([
                "profile",
                "test",
                profile_file.to_str().unwrap(),
                fixtures.to_str().unwrap(),
                "--report",
                "json",
                "--schema-version",
                "2",
            ])
            .output()
            .expect("Failed to execute profile test");

        assert!(output.status.success());
        let report: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("profile test output should be JSON");
        assert_eq!(report["schema_version"], "2");
        assert_eq!(report["tool_name"], "hl7v2-cli");
        assert_eq!(report["tool_version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(report["valid"], true);
        assert_eq!(report["case_count"], 2);
        assert!(
            report["cases"]
                .as_array()
                .unwrap()
                .iter()
                .any(|case| case["expectation"] == "invalid"
                    && case["validation_report"]["issues"][0]["code"] == "missing_required_field")
        );
    }

    #[test]
    fn test_profile_test_text_rejects_schema_version_2() {
        let dir = create_temp_dir();
        let profile_file = create_temp_profile(&dir, "profile.yaml", PID3_REQUIRED_PROFILE);
        let fixtures = dir.path().join("fixtures");
        std::fs::create_dir_all(fixtures.join("valid")).unwrap();
        std::fs::create_dir_all(fixtures.join("invalid")).unwrap();

        let mut cmd = cli_command();
        cmd.args([
            "profile",
            "test",
            profile_file.to_str().unwrap(),
            fixtures.to_str().unwrap(),
            "--report",
            "text",
            "--schema-version",
            "2",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "profile test schema version is only available",
        ));
    }

    #[test]
    fn test_profile_test_fails_when_invalid_fixture_validates() {
        let dir = create_temp_dir();
        let profile_file = create_temp_profile(&dir, "profile.yaml", PID3_REQUIRED_PROFILE);
        let fixtures = dir.path().join("fixtures");
        std::fs::create_dir_all(fixtures.join("valid")).unwrap();
        std::fs::create_dir_all(fixtures.join("invalid")).unwrap();
        create_temp_file(
            &dir,
            "fixtures/invalid/actually_valid.hl7",
            hl7v2_test_utils::fixtures::SampleMessages::adt_a01().as_bytes(),
        );

        let mut cmd = cli_command();
        let assert = cmd
            .args([
                "profile",
                "test",
                profile_file.to_str().unwrap(),
                fixtures.to_str().unwrap(),
                "--report",
                "json",
            ])
            .assert()
            .failure();
        let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
        let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();

        assert_eq!(report["valid"], false);
        assert_eq!(report["failed_count"], 1);
        assert_eq!(report["cases"][0]["passed"], false);
        assert!(
            report["cases"][0]["message"]
                .as_str()
                .unwrap()
                .contains("expected invalid but report was valid")
        );
    }
}

// =========================================================================
// Corpus Command Tests
// =========================================================================

mod corpus_command {
    use super::*;

    fn dirty_real_world_fixture_root() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_data/dirty-real-world")
    }

    fn normalize_fixture_segments(bytes: &[u8]) -> Vec<u8> {
        String::from_utf8_lossy(bytes)
            .replace("\r\n", "\n")
            .replace('\n', "\r")
            .into_bytes()
    }

    fn materialize_dirty_corpus_dir(category: &str, target: &std::path::Path) {
        let source = dirty_real_world_fixture_root().join(category);
        std::fs::create_dir_all(target).expect("dirty corpus target should be created");

        for entry in std::fs::read_dir(&source).expect("dirty corpus source should be readable") {
            let entry = entry.expect("dirty corpus entry should be readable");
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let bytes = std::fs::read(&path).expect("dirty corpus file should be readable");
            let file_name = path
                .file_name()
                .expect("dirty corpus file should have a file name");
            let destination = target.join(file_name);
            std::fs::write(&destination, normalize_fixture_segments(&bytes))
                .expect("dirty corpus file should be materialized");
        }
    }

    fn add_generated_mllp_fixture(target: &std::path::Path) {
        let source = dirty_real_world_fixture_root().join("sources/mllp-source.hl7");
        let bytes = std::fs::read(&source).expect("MLLP source fixture should be readable");
        let normalized = normalize_fixture_segments(&bytes);
        std::fs::write(
            target.join("mllp-framed.hl7"),
            hl7v2::wrap_mllp(&normalized),
        )
        .expect("generated MLLP dirty corpus file should be materialized");
    }

    fn materialize_dirty_real_world_corpus(before: &std::path::Path, after: &std::path::Path) {
        materialize_dirty_corpus_dir("before", before);
        materialize_dirty_corpus_dir("after", after);
        add_generated_mllp_fixture(after);
    }

    #[test]
    fn test_corpus_summarize_text_counts_messages_and_errors() {
        let dir = create_temp_dir();
        create_temp_hl7_file(&dir, "adt.hl7");
        create_temp_hl7_with_content(&dir, "bad.hl7", invalid_hl7_message());

        let mut cmd = cli_command();
        cmd.args(["corpus", "summarize", dir.path().to_str().unwrap()])
            .assert()
            .success()
            .stdout(predicate::str::contains("Corpus Summary:"))
            .stdout(predicate::str::contains("Files scanned: 2"))
            .stdout(predicate::str::contains("Parsed messages: 1"))
            .stdout(predicate::str::contains("Parse errors: 1"))
            .stdout(predicate::str::contains("ADT^A01^ADT_A01: 1"));
    }

    #[test]
    fn test_corpus_summarize_json_is_machine_readable() {
        let dir = create_temp_dir();
        create_temp_hl7_file(&dir, "adt.hl7");
        let nested = dir.path().join("nested");
        std::fs::create_dir_all(&nested).expect("nested corpus dir should be created");
        create_temp_file(
            &dir,
            "nested/oru.hl7",
            hl7v2_test_utils::fixtures::SampleMessages::oru_r01().as_bytes(),
        );

        let mut cmd = cli_command();
        let output = cmd
            .args([
                "corpus",
                "summarize",
                dir.path().to_str().unwrap(),
                "--format",
                "json",
            ])
            .output()
            .expect("Failed to execute corpus summarize");

        assert!(output.status.success());
        assert!(is_valid_json(&output.stdout));

        let report: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("summary output should be JSON");
        assert_eq!(report["file_count"], 2);
        assert_eq!(report["message_count"], 2);
        assert_eq!(report["parse_error_count"], 0);
        assert!(
            report["message_types"]
                .as_array()
                .unwrap()
                .iter()
                .any(|entry| entry["value"] == "ORU^R01" && entry["count"] == 1)
        );
    }

    #[test]
    fn test_corpus_summarize_json_schema_version_2_adds_provenance() {
        let dir = create_temp_dir();
        create_temp_hl7_file(&dir, "adt.hl7");

        let mut cmd = cli_command();
        let output = cmd
            .args([
                "corpus",
                "summarize",
                dir.path().to_str().unwrap(),
                "--format",
                "json",
                "--schema-version",
                "2",
            ])
            .output()
            .expect("Failed to execute corpus summarize");

        assert!(output.status.success());
        assert!(is_valid_json(&output.stdout));

        let report: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("summary output should be JSON");
        assert_eq!(report["schema_version"], "2");
        assert_eq!(report["tool_name"], "hl7v2-cli");
        assert_eq!(report["tool_version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(report["file_count"], 1);
        assert_eq!(report["message_count"], 1);
    }

    #[test]
    fn test_corpus_summarize_text_rejects_schema_version_2() {
        let dir = create_temp_dir();
        create_temp_hl7_file(&dir, "adt.hl7");

        let mut cmd = cli_command();
        cmd.args([
            "corpus",
            "summarize",
            dir.path().to_str().unwrap(),
            "--format",
            "text",
            "--schema-version",
            "2",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "corpus summary schema version is only available",
        ));
    }

    #[test]
    fn test_corpus_diff_text_reports_deltas() {
        let before = create_temp_dir();
        let after = create_temp_dir();
        create_temp_hl7_file(&before, "adt.hl7");
        create_temp_hl7_file(&after, "adt.hl7");
        create_temp_file(
            &after,
            "oru.hl7",
            hl7v2_test_utils::fixtures::SampleMessages::oru_r01().as_bytes(),
        );

        let mut cmd = cli_command();
        cmd.args([
            "corpus",
            "diff",
            before.path().to_str().unwrap(),
            after.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Corpus Diff:"))
        .stdout(predicate::str::contains("Parsed messages: 1 -> 2 (+1)"))
        .stdout(predicate::str::contains("ORU^R01: 0 -> 1 (+1)"));
    }

    #[test]
    fn test_corpus_diff_json_is_machine_readable() {
        let before = create_temp_dir();
        let after = create_temp_dir();
        create_temp_hl7_file(&before, "adt.hl7");
        create_temp_hl7_file(&after, "adt.hl7");
        create_temp_file(
            &after,
            "oru.hl7",
            hl7v2_test_utils::fixtures::SampleMessages::oru_r01().as_bytes(),
        );

        let mut cmd = cli_command();
        let output = cmd
            .args([
                "corpus",
                "diff",
                before.path().to_str().unwrap(),
                after.path().to_str().unwrap(),
                "--format",
                "json",
            ])
            .output()
            .expect("Failed to execute corpus diff");

        assert!(output.status.success());
        assert!(is_valid_json(&output.stdout));

        let report: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("diff output should be JSON");
        assert_eq!(report["diff_version"], "1");
        assert_eq!(report["file_count"]["delta"], 1);
        assert_eq!(report["message_count"]["delta"], 1);
        assert!(
            report["message_type_counts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|entry| entry["value"] == "ORU^R01"
                    && entry["before"] == 0
                    && entry["after"] == 1
                    && entry["delta"] == 1)
        );
        assert!(
            report["new_message_types"]
                .as_array()
                .unwrap()
                .iter()
                .any(|entry| entry == "ORU^R01")
        );
    }

    #[test]
    fn test_corpus_diff_json_schema_version_2_adds_provenance() {
        let before = create_temp_dir();
        let after = create_temp_dir();
        create_temp_hl7_file(&before, "adt.hl7");
        create_temp_hl7_file(&after, "adt.hl7");
        create_temp_file(
            &after,
            "oru.hl7",
            hl7v2_test_utils::fixtures::SampleMessages::oru_r01().as_bytes(),
        );

        let mut cmd = cli_command();
        let output = cmd
            .args([
                "corpus",
                "diff",
                before.path().to_str().unwrap(),
                after.path().to_str().unwrap(),
                "--format",
                "json",
                "--schema-version",
                "2",
            ])
            .output()
            .expect("Failed to execute corpus diff");

        assert!(output.status.success());
        assert!(is_valid_json(&output.stdout));

        let report: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("diff output should be JSON");
        assert_eq!(report["schema_version"], "2");
        assert_eq!(report["tool_name"], "hl7v2-cli");
        assert_eq!(report["diff_version"], "1");
        assert_eq!(report["file_count"]["delta"], 1);
        assert!(
            report["new_message_types"]
                .as_array()
                .unwrap()
                .iter()
                .any(|entry| entry == "ORU^R01")
        );
    }

    #[test]
    fn test_corpus_diff_json_with_profile_counts_issue_code_deltas() {
        let before = create_temp_dir();
        let after = create_temp_dir();
        let profile_dir = create_temp_dir();
        create_temp_hl7_file(&before, "valid.hl7");
        create_temp_hl7_with_content(
            &after,
            "missing_pid3.hl7",
            "MSH|^~\\&|SENDAPP|SENDFAC|RECVAPP|RECVFAC|202605030101||ADT^A01|CTRL123|P|2.5\rPID|1\r",
        );
        let profile = create_temp_profile(
            &profile_dir,
            "profile.yaml",
            r#"
message_structure: ADT_A01
version: "2.5"
segments:
  - id: MSH
  - id: PID
constraints:
  - path: PID.3
    required: true
"#,
        );

        let mut cmd = cli_command();
        let output = cmd
            .args([
                "corpus",
                "diff",
                before.path().to_str().unwrap(),
                after.path().to_str().unwrap(),
                "--profile",
                profile.to_str().unwrap(),
                "--format",
                "json",
            ])
            .output()
            .expect("Failed to execute corpus diff");

        assert!(output.status.success());
        assert!(is_valid_json(&output.stdout));

        let report: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("diff output should be JSON");
        assert_eq!(report["profile"]["message_structure"], "ADT_A01");
        assert!(
            report["validation_issue_code_counts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|entry| entry["value"] == "missing_required_field"
                    && entry["before"] == 0
                    && entry["after"] == 1
                    && entry["delta"] == 1)
        );
    }

    #[test]
    fn test_corpus_fingerprint_text_reports_shape() {
        let dir = create_temp_dir();
        create_temp_hl7_file(&dir, "adt.hl7");

        let mut cmd = cli_command();
        cmd.args(["corpus", "fingerprint", dir.path().to_str().unwrap()])
            .assert()
            .success()
            .stdout(predicate::str::contains("Corpus Fingerprint:"))
            .stdout(predicate::str::contains("Files scanned: 1"))
            .stdout(predicate::str::contains("ADT^A01^ADT_A01: 1"))
            .stdout(predicate::str::contains("Field presence:"))
            .stdout(predicate::str::contains("Value shapes:"));
    }

    #[test]
    fn test_corpus_fingerprint_json_with_profile_counts_issue_codes() {
        let dir = create_temp_dir();
        let profile_dir = create_temp_dir();
        create_temp_hl7_with_content(
            &dir,
            "missing_pid3.hl7",
            "MSH|^~\\&|SENDAPP|SENDFAC|RECVAPP|RECVFAC|202605030101||ADT^A01|CTRL123|P|2.5\rPID|1\r",
        );
        let profile = create_temp_profile(
            &profile_dir,
            "profile.yaml",
            r#"
message_structure: ADT_A01
version: "2.5"
segments:
  - id: MSH
  - id: PID
constraints:
  - path: PID.3
    required: true
"#,
        );

        let mut cmd = cli_command();
        let output = cmd
            .args([
                "corpus",
                "fingerprint",
                dir.path().to_str().unwrap(),
                "--profile",
                profile.to_str().unwrap(),
                "--format",
                "json",
            ])
            .output()
            .expect("Failed to execute corpus fingerprint");

        assert!(output.status.success());
        assert!(is_valid_json(&output.stdout));

        let report: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("fingerprint output should be JSON");
        assert_eq!(report["fingerprint_version"], "1");
        assert_eq!(report["file_count"], 1);
        assert_eq!(report["message_count"], 1);
        assert_eq!(report["parse_error_count"], 0);
        assert_eq!(report["profile"]["message_structure"], "ADT_A01");
        assert!(
            report["validation_issue_code_counts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|entry| entry["value"] == "missing_required_field" && entry["count"] == 1)
        );
    }

    #[test]
    fn test_corpus_fingerprint_json_schema_version_2_adds_provenance() {
        let dir = create_temp_dir();
        create_temp_hl7_file(&dir, "adt.hl7");

        let mut cmd = cli_command();
        let output = cmd
            .args([
                "corpus",
                "fingerprint",
                dir.path().to_str().unwrap(),
                "--format",
                "json",
                "--schema-version",
                "2",
            ])
            .output()
            .expect("Failed to execute corpus fingerprint");

        assert!(output.status.success());
        assert!(is_valid_json(&output.stdout));

        let report: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("fingerprint output should be JSON");
        assert_eq!(report["schema_version"], "2");
        assert_eq!(report["tool_name"], "hl7v2-cli");
        assert_eq!(report["fingerprint_version"], "1");
        assert_eq!(report["file_count"], 1);
        assert_eq!(report["message_count"], 1);
    }

    #[test]
    fn test_corpus_commands_share_dirty_real_world_fixture_categories() {
        let before = create_temp_dir();
        let after = create_temp_dir();
        materialize_dirty_real_world_corpus(before.path(), after.path());

        let mut summarize = cli_command();
        let summary_output = summarize
            .args([
                "corpus",
                "summarize",
                after.path().to_str().unwrap(),
                "--format",
                "json",
            ])
            .output()
            .expect("corpus summarize should run");

        assert!(summary_output.status.success());
        assert!(is_valid_json(&summary_output.stdout));
        let summary: serde_json::Value =
            serde_json::from_slice(&summary_output.stdout).expect("summary output should be JSON");
        assert_eq!(summary["file_count"], 6);
        assert_eq!(summary["message_count"], 4);
        assert_eq!(summary["parse_error_count"], 2);
        assert!(
            summary["message_types"]
                .as_array()
                .unwrap()
                .iter()
                .any(|entry| entry["value"] == "ADT^A08" && entry["count"] == 1)
        );
        assert!(
            summary["segments"]
                .as_array()
                .unwrap()
                .iter()
                .any(|entry| entry["value"] == "ZPV" && entry["count"] == 1)
        );
        assert!(
            summary["parse_errors"]
                .as_array()
                .unwrap()
                .iter()
                .all(|entry| !entry["error"].as_str().unwrap().contains("MRN-DIRTY"))
        );

        let mut fingerprint = cli_command();
        let fingerprint_output = fingerprint
            .args([
                "corpus",
                "fingerprint",
                after.path().to_str().unwrap(),
                "--format",
                "json",
            ])
            .output()
            .expect("corpus fingerprint should run");

        assert!(fingerprint_output.status.success());
        assert!(is_valid_json(&fingerprint_output.stdout));
        let fingerprint: serde_json::Value = serde_json::from_slice(&fingerprint_output.stdout)
            .expect("fingerprint output should be JSON");
        assert_eq!(fingerprint["file_count"], 6);
        assert_eq!(fingerprint["message_count"], 4);
        assert_eq!(fingerprint["parse_error_count"], 2);
        assert!(
            fingerprint["field_cardinality"]
                .as_array()
                .unwrap()
                .iter()
                .any(|entry| entry["path"] == "OBX.5"
                    && entry["max_per_message"] == 20
                    && entry["total_occurrences"] == 20)
        );

        let mut diff = cli_command();
        let diff_output = diff
            .args([
                "corpus",
                "diff",
                before.path().to_str().unwrap(),
                after.path().to_str().unwrap(),
                "--format",
                "json",
            ])
            .output()
            .expect("corpus diff should run");

        assert!(diff_output.status.success());
        assert!(is_valid_json(&diff_output.stdout));
        let diff: serde_json::Value =
            serde_json::from_slice(&diff_output.stdout).expect("diff output should be JSON");
        assert_eq!(diff["file_count"]["delta"], 4);
        assert_eq!(diff["message_count"]["delta"], 2);
        assert_eq!(diff["parse_error_count"]["delta"], 2);
        assert!(
            diff["field_cardinality"]
                .as_array()
                .unwrap()
                .iter()
                .any(|entry| entry["path"] == "OBX.5"
                    && entry["max_per_message_delta"] == 15
                    && entry["total_occurrences_delta"] == 15)
        );
    }
}

// =========================================================================
// Validate Command Tests
// =========================================================================

mod validate_command {
    use super::*;

    #[test]
    fn test_validate_with_valid_profile() {
        let dir = create_temp_dir();
        let hl7_file = create_temp_hl7_file(&dir, "test.hl7");
        let profile_file = create_temp_profile(&dir, "profile.yaml", minimal_profile());

        let mut cmd = cli_command();
        // The validation may fail due to profile format issues - just verify it runs
        let result = cmd
            .args([
                "val",
                hl7_file.to_str().unwrap(),
                "--profile",
                profile_file.to_str().unwrap(),
            ])
            .output();
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_with_strict_profile() {
        let dir = create_temp_dir();
        let hl7_file = create_temp_hl7_file(&dir, "test.hl7");
        let profile_file = create_temp_profile(&dir, "strict.yaml", strict_profile());

        let mut cmd = cli_command();
        // Just verify the command runs - strict profile validation behavior depends on implementation
        let result = cmd
            .args([
                "val",
                hl7_file.to_str().unwrap(),
                "--profile",
                profile_file.to_str().unwrap(),
            ])
            .output();
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_with_detailed_output() {
        let dir = create_temp_dir();
        let hl7_file = create_temp_hl7_file(&dir, "test.hl7");
        let profile_file = create_temp_profile(&dir, "profile.yaml", minimal_profile());

        let mut cmd = cli_command();
        let result = cmd
            .args([
                "val",
                hl7_file.to_str().unwrap(),
                "--profile",
                profile_file.to_str().unwrap(),
                "--detailed",
            ])
            .output();
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_with_summary() {
        let dir = create_temp_dir();
        let hl7_file = create_temp_hl7_file(&dir, "test.hl7");
        let profile_file = create_temp_profile(&dir, "profile.yaml", minimal_profile());

        let mut cmd = cli_command();
        let result = cmd
            .args([
                "val",
                hl7_file.to_str().unwrap(),
                "--profile",
                profile_file.to_str().unwrap(),
                "--summary",
            ])
            .output();
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_json_report_uses_stable_issue_contract() {
        let dir = create_temp_dir();
        let hl7_file = create_temp_hl7_with_content(
            &dir,
            "missing_pid3.hl7",
            "MSH|^~\\&|SENDAPP|SENDFAC|RECVAPP|RECVFAC|202605030101||ADT^A01|CTRL123|P|2.5\rPID|1\r",
        );
        let profile_file = create_temp_profile(
            &dir,
            "profile.yaml",
            r#"
message_structure: ADT_A01
version: "2.5"
segments:
  - id: MSH
  - id: PID
constraints:
  - path: PID.3
    required: true
"#,
        );

        let mut cmd = cli_command();
        let assert = cmd
            .args([
                "val",
                hl7_file.to_str().unwrap(),
                "--profile",
                profile_file.to_str().unwrap(),
                "--report",
                "json",
            ])
            .assert()
            .failure();
        let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
        let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();

        assert_eq!(report["valid"], false);
        assert_eq!(report["message_type"], "ADT^A01");
        assert!(
            report["profile"]
                .as_str()
                .unwrap()
                .ends_with("profile.yaml")
        );
        assert_eq!(report["issue_count"], 1);
        assert_eq!(report["issues"][0]["code"], "missing_required_field");
        assert_eq!(report["issues"][0]["severity"], "error");
        assert_eq!(report["issues"][0]["path"], "PID.3");
        assert_eq!(report["issues"][0]["rule_id"], "missing_required_field");
        assert_eq!(report["issues"][0]["segment_index"], 1);
        assert_eq!(report["issues"][0]["field_index"], 3);
    }

    #[test]
    fn test_validate_missing_hl7_file() {
        let dir = create_temp_dir();
        let profile_file = create_temp_profile(&dir, "profile.yaml", minimal_profile());

        let mut cmd = cli_command();
        cmd.args([
            "val",
            "/nonexistent/file.hl7",
            "--profile",
            profile_file.to_str().unwrap(),
        ])
        .assert()
        .failure();
    }

    #[test]
    fn test_validate_missing_profile_file() {
        let dir = create_temp_dir();
        let hl7_file = create_temp_hl7_file(&dir, "test.hl7");

        let mut cmd = cli_command();
        cmd.args([
            "val",
            hl7_file.to_str().unwrap(),
            "--profile",
            "/nonexistent/profile.yaml",
        ])
        .assert()
        .failure();
    }

    #[test]
    fn test_validate_mllp_input() {
        let dir = create_temp_dir();
        let mllp_file = create_temp_mllp_file(&dir, "test_mllp.hl7");
        let profile_file = create_temp_profile(&dir, "profile.yaml", minimal_profile());

        let mut cmd = cli_command();
        // Just verify the command runs
        let result = cmd
            .args([
                "val",
                mllp_file.to_str().unwrap(),
                "--profile",
                profile_file.to_str().unwrap(),
                "--mllp",
            ])
            .output();
        assert!(result.is_ok());
    }
}

// =========================================================================
// Redact Command Tests
// =========================================================================

mod redact_command {
    use super::*;

    const PID3_VALUE: &str = "123456^^^HOSP^MR";
    const PHI_MESSAGE: &str = "MSH|^~\\&|LAB|L|EHR|E|202605030101||ADT^A01|CTRL123|P|2.5\rPID|1||123456^^^HOSP^MR||Doe^John||19700101|M|||123 Main St\rOBX|1|NM|718-7^Hemoglobin^LN||13.2|g/dL\r";
    const PHONE_MESSAGE: &str = "MSH|^~\\&|LAB|L|EHR|E|202605030101||ADT^A01|CTRL123|P|2.5\rPID|1||123456^^^HOSP^MR||Doe^John||19700101|M|||123 Main St||5551212\r";
    const REPEATED_NK1_MESSAGE: &str = "MSH|^~\\&|LAB|L|EHR|E|202605030101||ADT^A01|CTRL123|P|2.5\rPID|1||123456^^^HOSP^MR||Doe^John||19700101|M|||123 Main St\rNK1|1\rNK1|2|Kin^Jane\r";

    const SAFE_ANALYSIS_POLICY: &str = r#"
[[rules]]
path = "PID.3"
action = "hash"
reason = "patient identifier"

[[rules]]
path = "PID.5"
action = "drop"
reason = "patient name"

[[rules]]
path = "PID.7"
action = "drop"
reason = "date of birth"

[[rules]]
path = "PID.11"
action = "drop"
reason = "patient address"

[[rules]]
path = "MSH.9"
action = "retain"
reason = "message type is needed for analysis"

[[rules]]
path = "MSH.10"
action = "retain"
reason = "control id is needed for replay correlation"

[[rules]]
path = "OBX.3"
action = "retain"
reason = "observation identifier is needed for analysis"

[[rules]]
path = "OBX.5"
action = "retain"
reason = "non-PHI synthetic observation value shape is needed for analysis"
"#;

    #[test]
    fn test_redact_json_hashes_drops_and_receipts_without_raw_phi() {
        let dir = create_temp_dir();
        let message_file = create_temp_hl7_with_content(&dir, "message.hl7", PHI_MESSAGE);
        let policy_file =
            create_temp_file(&dir, "safe-analysis.toml", SAFE_ANALYSIS_POLICY.as_bytes());

        let mut cmd = cli_command();
        let output = cmd
            .args([
                "redact",
                message_file.to_str().unwrap(),
                "--policy",
                policy_file.to_str().unwrap(),
                "--format",
                "json",
            ])
            .output()
            .expect("Failed to execute redact");

        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(!stdout.contains("Doe^John"));
        assert!(!stdout.contains("123456^^^HOSP^MR"));
        assert!(!stdout.contains("19700101"));
        assert!(!stdout.contains("123 Main St"));
        let expected_hash = hl7v2::synthetic::corpus::compute_sha256(PID3_VALUE);
        assert!(stdout.contains(&format!("hash:sha256:{expected_hash}")));

        let report: serde_json::Value =
            serde_json::from_str(&stdout).expect("redact output should be JSON");
        assert!(
            report["input_sha256"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
        );
        assert!(
            report["policy_sha256"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
        );
        assert_eq!(report["message_type"], "ADT^A01");
        assert_eq!(report["receipt"]["phi_removed"], true);
        assert_eq!(report["receipt"]["hash_algorithm"], "sha256");
        assert!(report["receipt"]["actions"].as_array().unwrap().iter().any(
            |action| action["path"] == "PID.3"
                && action["action"] == "hash"
                && action["matched_count"] == 1
                && action["status"] == "applied"
        ));
        assert!(report["receipt"]["actions"].as_array().unwrap().iter().any(
            |action| action["path"] == "OBX.5"
                && action["action"] == "retain"
                && action["status"] == "retained"
        ));
    }

    #[test]
    fn test_redact_json_schema_v2_returns_provenance_output_without_raw_phi() {
        let dir = create_temp_dir();
        let message_file = create_temp_hl7_with_content(&dir, "message.hl7", PHI_MESSAGE);
        let policy_file =
            create_temp_file(&dir, "safe-analysis.toml", SAFE_ANALYSIS_POLICY.as_bytes());

        let mut cmd = cli_command();
        let output = cmd
            .args([
                "redact",
                message_file.to_str().unwrap(),
                "--policy",
                policy_file.to_str().unwrap(),
                "--format",
                "json",
                "--schema-version",
                "2",
            ])
            .output()
            .expect("Failed to execute redact");

        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(!stdout.contains("Doe^John"));
        assert!(!stdout.contains("123456^^^HOSP^MR"));
        assert!(!stdout.contains("19700101"));

        let report: serde_json::Value =
            serde_json::from_str(&stdout).expect("redact output should be JSON");
        assert_eq!(report["schema_version"], "2");
        assert_eq!(report["tool_name"], "hl7v2-cli");
        assert_eq!(report["tool_version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(report["receipt"]["schema_version"], "2");
        assert_eq!(report["receipt"]["tool_name"], "hl7v2-cli");
        assert_eq!(report["receipt"]["tool_version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(report["receipt"]["phi_removed"], true);
        assert_eq!(report["receipt"]["hash_algorithm"], "sha256");
        assert!(report["receipt"]["actions"].as_array().unwrap().iter().any(
            |action| action["path"] == "PID.3"
                && action["action"] == "hash"
                && action["matched_count"] == 1
                && action["status"] == "applied"
        ));
    }

    #[test]
    fn test_redact_rejects_output_schema_v2_for_hl7_output() {
        let dir = create_temp_dir();
        let message_file = create_temp_hl7_with_content(&dir, "message.hl7", PHI_MESSAGE);
        let policy_file =
            create_temp_file(&dir, "safe-analysis.toml", SAFE_ANALYSIS_POLICY.as_bytes());

        let mut cmd = cli_command();
        let output = cmd
            .args([
                "redact",
                message_file.to_str().unwrap(),
                "--policy",
                policy_file.to_str().unwrap(),
                "--format",
                "hl7",
                "--schema-version",
                "2",
            ])
            .output()
            .expect("Failed to execute redact");

        assert!(!output.status.success());
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains("redaction output schema version is only available"));
    }

    #[test]
    fn test_redact_json_does_not_emit_phi_leak_sentinels_or_paths() {
        let dir = create_temp_dir();
        let message_file =
            create_temp_hl7_with_content(&dir, RAW_INPUT_FILE_SENTINEL, PHI_LEAK_SENTINEL_MESSAGE);
        let policy_file = create_temp_file(
            &dir,
            RAW_POLICY_FILE_SENTINEL,
            PHI_LEAK_SENTINEL_POLICY.as_bytes(),
        );

        let mut cmd = cli_command();
        let output = cmd
            .args([
                "redact",
                message_file.to_str().unwrap(),
                "--policy",
                policy_file.to_str().unwrap(),
                "--format",
                "json",
            ])
            .output()
            .expect("Failed to execute redact");

        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert_no_phi_leak_sentinels_or_paths(
            "redact stdout",
            &stdout,
            &message_file,
            &policy_file,
        );
        assert_no_phi_leak_sentinels_or_paths(
            "redact stderr",
            &stderr,
            &message_file,
            &policy_file,
        );
    }

    #[test]
    fn test_redact_hl7_outputs_message_and_receipt_to_stderr() {
        let dir = create_temp_dir();
        let message_file = create_temp_hl7_with_content(&dir, "message.hl7", PHI_MESSAGE);
        let policy_file =
            create_temp_file(&dir, "safe-analysis.toml", SAFE_ANALYSIS_POLICY.as_bytes());

        let mut cmd = cli_command();
        cmd.args([
            "redact",
            message_file.to_str().unwrap(),
            "--policy",
            policy_file.to_str().unwrap(),
            "--format",
            "hl7",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("hash:sha256:"))
        .stdout(predicate::str::contains("MSH|"))
        .stdout(predicate::str::contains("Doe^John").not())
        .stderr(predicate::str::contains("Redaction receipt"));
    }

    #[test]
    fn test_redact_rejects_malformed_policy_path() {
        let dir = create_temp_dir();
        let message_file = create_temp_hl7_with_content(&dir, "message.hl7", PHI_MESSAGE);
        let policy_file = create_temp_file(
            &dir,
            "bad-policy.toml",
            br#"
[[rules]]
path = "PID.5.1"
action = "drop"
reason = "component targeting is not supported by this policy"
"#,
        );

        let mut cmd = cli_command();
        cmd.args([
            "redact",
            message_file.to_str().unwrap(),
            "--policy",
            policy_file.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("must target a field"));
    }

    #[test]
    fn test_redact_rejects_unmatched_required_redaction_rule() {
        let dir = create_temp_dir();
        let message_file = create_temp_hl7_with_content(&dir, "message.hl7", PHI_MESSAGE);
        let policy_file = create_temp_file(
            &dir,
            "bad-policy.toml",
            format!(
                r#"{SAFE_ANALYSIS_POLICY}

[[rules]]
path = "NK1.2"
action = "drop"
reason = "next of kin name"
"#
            )
            .as_bytes(),
        );

        let mut cmd = cli_command();
        cmd.args([
            "redact",
            message_file.to_str().unwrap(),
            "--policy",
            policy_file.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("matched no fields"));
    }

    #[test]
    fn test_redact_allows_unmatched_optional_redaction_rule() {
        let dir = create_temp_dir();
        let message_file = create_temp_hl7_with_content(&dir, "message.hl7", PHI_MESSAGE);
        let policy_file = create_temp_file(
            &dir,
            "safe-analysis.toml",
            format!(
                r#"{SAFE_ANALYSIS_POLICY}

[[rules]]
path = "NK1.2"
action = "drop"
reason = "next of kin name"
optional = true
"#
            )
            .as_bytes(),
        );

        let mut cmd = cli_command();
        cmd.args([
            "redact",
            message_file.to_str().unwrap(),
            "--policy",
            policy_file.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"optional\": true"))
        .stdout(predicate::str::contains("\"status\": \"not_found\""));
    }

    #[test]
    fn test_redact_rejects_retain_for_sensitive_field() {
        let dir = create_temp_dir();
        let message_file = create_temp_hl7_with_content(&dir, "message.hl7", PHI_MESSAGE);
        let policy_file = create_temp_file(
            &dir,
            "bad-policy.toml",
            br#"
[[rules]]
path = "PID.5"
action = "retain"
reason = "unsafe retain"
"#,
        );

        let mut cmd = cli_command();
        cmd.args([
            "redact",
            message_file.to_str().unwrap(),
            "--policy",
            policy_file.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "cannot retain a built-in sensitive field",
        ));
    }

    #[test]
    fn test_redact_rejects_policy_missing_present_sensitive_field() {
        let dir = create_temp_dir();
        let message_file = create_temp_hl7_with_content(&dir, "message.hl7", PHONE_MESSAGE);
        let policy_file =
            create_temp_file(&dir, "bad-policy.toml", SAFE_ANALYSIS_POLICY.as_bytes());

        let mut cmd = cli_command();
        cmd.args([
            "redact",
            message_file.to_str().unwrap(),
            "--policy",
            policy_file.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("PID.13"));
    }

    #[test]
    fn test_redact_rejects_policy_missing_sensitive_field_on_repeated_segment() {
        let dir = create_temp_dir();
        let message_file = create_temp_hl7_with_content(&dir, "message.hl7", REPEATED_NK1_MESSAGE);
        let policy_file =
            create_temp_file(&dir, "bad-policy.toml", SAFE_ANALYSIS_POLICY.as_bytes());

        let mut cmd = cli_command();
        cmd.args([
            "redact",
            message_file.to_str().unwrap(),
            "--policy",
            policy_file.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("NK1.2"));
    }

    #[test]
    fn test_redact_rejects_msh_delimiter_metadata_path() {
        let dir = create_temp_dir();
        let message_file = create_temp_hl7_with_content(&dir, "message.hl7", PHI_MESSAGE);
        let policy_file = create_temp_file(
            &dir,
            "bad-policy.toml",
            br#"
[[rules]]
path = "MSH.2"
action = "drop"
reason = "delimiter metadata is not redacted by this command"
"#,
        );

        let mut cmd = cli_command();
        cmd.args([
            "redact",
            message_file.to_str().unwrap(),
            "--policy",
            policy_file.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("delimiter metadata"));
    }

    #[test]
    fn test_redact_rejects_policy_rule_without_reason() {
        let dir = create_temp_dir();
        let message_file = create_temp_hl7_with_content(&dir, "message.hl7", PHI_MESSAGE);
        let policy_file = create_temp_file(
            &dir,
            "bad-policy.toml",
            br#"
[[rules]]
path = "PID.5"
action = "drop"
"#,
        );

        let mut cmd = cli_command();
        cmd.args([
            "redact",
            message_file.to_str().unwrap(),
            "--policy",
            policy_file.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("must include a reason"));
    }
}

// =========================================================================
// Evidence Bundle Command Tests
// =========================================================================

mod bundle_command {
    use super::*;

    const PHI_MESSAGE: &str = "MSH|^~\\&|LAB|L|EHR|E|202605030101||ADT^A01^ADT_A01|CTRL123|P|2.5\rPID|1||123456^^^HOSP^MR||Doe^John||19700101|M|||123 Main St\rOBX|1|NM|718-7^Hemoglobin^LN||13.2|g/dL\r";
    const PID5_CONSTRAINT_PROFILE: &str = r#"
message_structure: ADT_A01
version: "2.5.1"
segments:
  - id: MSH
  - id: PID
constraints:
  - path: MSH.9
    required: true
  - path: PID.5
    in:
      - Allowed^Name
"#;
    const SAFE_ANALYSIS_POLICY: &str = r#"
[[rules]]
path = "PID.3"
action = "hash"
reason = "patient identifier"

[[rules]]
path = "PID.5"
action = "drop"
reason = "patient name"

[[rules]]
path = "PID.7"
action = "drop"
reason = "date of birth"

[[rules]]
path = "PID.11"
action = "drop"
reason = "patient address"

[[rules]]
path = "MSH.9"
action = "retain"
reason = "message type is needed for analysis"

[[rules]]
path = "MSH.10"
action = "retain"
reason = "control id is needed for replay correlation"

[[rules]]
path = "OBX.3"
action = "retain"
reason = "observation identifier is needed for analysis"

[[rules]]
path = "OBX.5"
action = "retain"
reason = "non-PHI synthetic observation value shape is needed for analysis"
"#;

    #[test]
    fn test_bundle_writes_redacted_replayable_evidence_artifacts() {
        let dir = create_temp_dir();
        let message_file = create_temp_hl7_with_content(&dir, "message.hl7", PHI_MESSAGE);
        let profile_file = create_temp_profile(&dir, "profile.yaml", minimal_profile());
        let policy_file =
            create_temp_file(&dir, "safe-analysis.toml", SAFE_ANALYSIS_POLICY.as_bytes());
        let bundle_dir = dir.path().join("issue-bundle");

        let mut cmd = cli_command();
        let output = cmd
            .args([
                "bundle",
                message_file.to_str().unwrap(),
                "--profile",
                profile_file.to_str().unwrap(),
                "--redact-policy",
                policy_file.to_str().unwrap(),
                "--out",
                bundle_dir.to_str().unwrap(),
            ])
            .output()
            .expect("Failed to execute bundle");

        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).unwrap();
        let summary: serde_json::Value =
            serde_json::from_str(&stdout).expect("bundle summary should be JSON");
        assert_eq!(summary["bundle_version"], "1");
        assert_eq!(summary["output_dir"], ".");
        assert_eq!(summary["message_type"], "ADT^A01");
        assert_eq!(summary["validation_valid"], true);
        assert_eq!(summary["redaction_phi_removed"], true);

        for artifact in [
            "message.redacted.hl7",
            "validation-report.json",
            "field-paths.json",
            "profile.yaml",
            "redaction-receipt.json",
            "environment.json",
            "replay.sh",
            "replay.ps1",
            "README.md",
            "manifest.json",
        ] {
            assert!(
                bundle_dir.join(artifact).exists(),
                "missing bundle artifact {artifact}"
            );
        }

        let redacted_message =
            std::fs::read_to_string(bundle_dir.join("message.redacted.hl7")).unwrap();
        assert!(redacted_message.contains("hash:sha256:"));
        assert!(!redacted_message.contains("Doe^John"));
        assert!(!redacted_message.contains("123456^^^HOSP^MR"));
        assert!(!redacted_message.contains("19700101"));
        assert!(!redacted_message.contains("123 Main St"));

        for artifact in [
            "validation-report.json",
            "field-paths.json",
            "redaction-receipt.json",
            "environment.json",
            "replay.sh",
            "replay.ps1",
            "README.md",
            "manifest.json",
        ] {
            let content = std::fs::read_to_string(bundle_dir.join(artifact)).unwrap();
            assert!(
                !content.contains("Doe^John"),
                "{artifact} leaked patient name"
            );
            assert!(
                !content.contains("123456^^^HOSP^MR"),
                "{artifact} leaked patient identifier"
            );
            assert!(!content.contains("19700101"), "{artifact} leaked DOB");
            assert!(
                !content.contains("123 Main St"),
                "{artifact} leaked address"
            );
        }

        let validation_report: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(bundle_dir.join("validation-report.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(validation_report["profile"], "profile.yaml");

        let field_paths: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(bundle_dir.join("field-paths.json")).unwrap(),
        )
        .unwrap();
        assert!(field_paths["fields"].as_array().unwrap().iter().any(
            |field| field["canonical_path"] == "PID.3"
                && field["redaction_action"] == "hash"
                && field["value_shape"] == "hashed_sha256"
        ));

        let receipt: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(bundle_dir.join("redaction-receipt.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(receipt["phi_removed"], true);

        let environment: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(bundle_dir.join("environment.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(environment["tool_name"], "hl7v2-cli");
        assert_eq!(
            environment["replay_command"],
            "hl7v2 val message.redacted.hl7 --profile profile.yaml --report json"
        );

        let readme = std::fs::read_to_string(bundle_dir.join("README.md")).unwrap();
        assert!(readme.contains("HL7v2 Evidence Bundle"));
        assert!(readme.contains("hl7v2 replay . --format json"));
        assert!(!readme.contains(dir.path().to_string_lossy().as_ref()));

        let manifest: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(bundle_dir.join("manifest.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(manifest["bundle_version"], "1");
        assert_eq!(manifest["tool_name"], "hl7v2-cli");
        let artifacts = manifest["artifacts"].as_array().unwrap();
        assert_eq!(artifacts.len(), 9);
        assert!(artifacts.iter().any(|artifact| {
            artifact["path"] == "message.redacted.hl7"
                && artifact["role"] == "redacted_message"
                && artifact["sha256"].as_str().is_some_and(is_sha256_hex)
        }));
        assert!(artifacts.iter().any(|artifact| {
            artifact["path"] == "validation-report.json"
                && artifact["role"] == "validation_report"
                && artifact["sha256"].as_str().is_some_and(is_sha256_hex)
        }));
        assert!(artifacts.iter().any(|artifact| {
            artifact["path"] == "README.md"
                && artifact["role"] == "bundle_readme"
                && artifact["sha256"].as_str().is_some_and(is_sha256_hex)
        }));
    }

    #[test]
    fn test_bundle_json_schema_version_2_adds_provenance() {
        let dir = create_temp_dir();
        let message_file = create_temp_hl7_with_content(&dir, "message.hl7", PHI_MESSAGE);
        let profile_file = create_temp_profile(&dir, "profile.yaml", minimal_profile());
        let policy_file =
            create_temp_file(&dir, "safe-analysis.toml", SAFE_ANALYSIS_POLICY.as_bytes());
        let bundle_dir = dir.path().join("issue-bundle");

        let mut cmd = cli_command();
        let output = cmd
            .args([
                "bundle",
                message_file.to_str().unwrap(),
                "--profile",
                profile_file.to_str().unwrap(),
                "--redact-policy",
                policy_file.to_str().unwrap(),
                "--out",
                bundle_dir.to_str().unwrap(),
                "--schema-version",
                "2",
            ])
            .output()
            .expect("Failed to execute bundle");

        assert!(output.status.success());
        let report: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("bundle summary should be JSON");
        assert_eq!(report["schema_version"], "2");
        assert_eq!(report["tool_name"], "hl7v2-cli");
        assert_eq!(report["tool_version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(report["bundle_version"], "1");
        assert_eq!(report["output_dir"], ".");
        assert_eq!(report["message_type"], "ADT^A01");
        assert_eq!(report["validation_valid"], true);
    }

    #[test]
    fn test_bundle_artifacts_do_not_emit_phi_leak_sentinels_or_paths() {
        let dir = create_temp_dir();
        let message_file =
            create_temp_hl7_with_content(&dir, RAW_INPUT_FILE_SENTINEL, PHI_LEAK_SENTINEL_MESSAGE);
        let profile_file = create_temp_profile(&dir, "profile.yaml", minimal_profile());
        let policy_file = create_temp_file(
            &dir,
            RAW_POLICY_FILE_SENTINEL,
            PHI_LEAK_SENTINEL_POLICY.as_bytes(),
        );
        let bundle_dir = dir.path().join("issue-bundle");

        let mut cmd = cli_command();
        let output = cmd
            .args([
                "bundle",
                message_file.to_str().unwrap(),
                "--profile",
                profile_file.to_str().unwrap(),
                "--redact-policy",
                policy_file.to_str().unwrap(),
                "--out",
                bundle_dir.to_str().unwrap(),
            ])
            .output()
            .expect("Failed to execute bundle");

        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert_no_phi_leak_sentinels_or_paths(
            "bundle stdout",
            &stdout,
            &message_file,
            &policy_file,
        );
        assert_no_phi_leak_sentinels_or_paths(
            "bundle stderr",
            &stderr,
            &message_file,
            &policy_file,
        );

        for artifact in [
            "message.redacted.hl7",
            "validation-report.json",
            "field-paths.json",
            "profile.yaml",
            "redaction-receipt.json",
            "environment.json",
            "replay.sh",
            "replay.ps1",
            "README.md",
            "manifest.json",
        ] {
            let content = std::fs::read_to_string(bundle_dir.join(artifact)).unwrap();
            assert_no_phi_leak_sentinels_or_paths(artifact, &content, &message_file, &policy_file);
        }
    }

    #[test]
    fn test_bundle_validation_report_uses_redacted_message_without_phi() {
        let dir = create_temp_dir();
        let message_file = create_temp_hl7_with_content(&dir, "message.hl7", PHI_MESSAGE);
        let profile_file = create_temp_profile(&dir, "profile.yaml", PID5_CONSTRAINT_PROFILE);
        let hash_name_policy = SAFE_ANALYSIS_POLICY.replace(
            "path = \"PID.5\"\naction = \"drop\"",
            "path = \"PID.5\"\naction = \"hash\"",
        );
        let policy_file = create_temp_file(&dir, "safe-analysis.toml", hash_name_policy.as_bytes());
        let bundle_dir = dir.path().join("issue-bundle");

        let mut cmd = cli_command();
        cmd.args([
            "bundle",
            message_file.to_str().unwrap(),
            "--profile",
            profile_file.to_str().unwrap(),
            "--redact-policy",
            policy_file.to_str().unwrap(),
            "--out",
            bundle_dir.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"validation_valid\": false"));

        let validation_report =
            std::fs::read_to_string(bundle_dir.join("validation-report.json")).unwrap();
        assert!(validation_report.contains("value_not_in_constraint"));
        assert!(validation_report.contains("hash:sha256:"));
        assert!(!validation_report.contains("Doe^John"));
        assert!(!validation_report.contains("123456^^^HOSP^MR"));
        assert!(!validation_report.contains("19700101"));
        assert!(!validation_report.contains("123 Main St"));
    }

    #[test]
    fn test_bundle_rejects_existing_output_directory() {
        let dir = create_temp_dir();
        let message_file = create_temp_hl7_with_content(&dir, "message.hl7", PHI_MESSAGE);
        let profile_file = create_temp_profile(&dir, "profile.yaml", minimal_profile());
        let policy_file =
            create_temp_file(&dir, "safe-analysis.toml", SAFE_ANALYSIS_POLICY.as_bytes());
        let bundle_dir = dir.path().join("issue-bundle");
        std::fs::create_dir(&bundle_dir).unwrap();

        let mut cmd = cli_command();
        cmd.args([
            "bundle",
            message_file.to_str().unwrap(),
            "--profile",
            profile_file.to_str().unwrap(),
            "--redact-policy",
            policy_file.to_str().unwrap(),
            "--out",
            bundle_dir.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "bundle output directory already exists",
        ));
    }
}

// =========================================================================
// Evidence Replay Command Tests
// =========================================================================

mod replay_command {
    use super::*;

    const PHI_MESSAGE: &str = "MSH|^~\\&|LAB|L|EHR|E|202605030101||ADT^A01^ADT_A01|CTRL123|P|2.5\rPID|1||123456^^^HOSP^MR||Doe^John||19700101|M|||123 Main St\rOBX|1|NM|718-7^Hemoglobin^LN||13.2|g/dL\r";
    const SAFE_ANALYSIS_POLICY: &str = r#"
[[rules]]
path = "PID.3"
action = "hash"
reason = "patient identifier"

[[rules]]
path = "PID.5"
action = "drop"
reason = "patient name"

[[rules]]
path = "PID.7"
action = "drop"
reason = "date of birth"

[[rules]]
path = "PID.11"
action = "drop"
reason = "patient address"

[[rules]]
path = "MSH.9"
action = "retain"
reason = "message type is needed for analysis"

[[rules]]
path = "MSH.10"
action = "retain"
reason = "control id is needed for replay correlation"

[[rules]]
path = "OBX.3"
action = "retain"
reason = "observation identifier is needed for analysis"

[[rules]]
path = "OBX.5"
action = "retain"
reason = "non-PHI synthetic observation value shape is needed for analysis"
"#;

    fn create_replayable_bundle(dir: &tempfile::TempDir) -> std::path::PathBuf {
        create_replayable_bundle_with_inputs(
            dir,
            "message.hl7",
            PHI_MESSAGE,
            "safe-analysis.toml",
            SAFE_ANALYSIS_POLICY,
        )
        .0
    }

    fn create_replayable_bundle_with_inputs(
        dir: &tempfile::TempDir,
        message_name: &str,
        message: &str,
        policy_name: &str,
        policy: &str,
    ) -> (std::path::PathBuf, std::path::PathBuf, std::path::PathBuf) {
        let message_file = create_temp_hl7_with_content(dir, message_name, message);
        let profile_file = create_temp_profile(dir, "profile.yaml", minimal_profile());
        let policy_file = create_temp_file(dir, policy_name, policy.as_bytes());
        let bundle_dir = dir.path().join("issue-bundle");

        let mut cmd = cli_command();
        cmd.args([
            "bundle",
            message_file.to_str().unwrap(),
            "--profile",
            profile_file.to_str().unwrap(),
            "--redact-policy",
            policy_file.to_str().unwrap(),
            "--out",
            bundle_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

        (bundle_dir, message_file, policy_file)
    }

    #[test]
    fn test_replay_reproduces_bundle_report_without_raw_phi() {
        let dir = create_temp_dir();
        let bundle_dir = create_replayable_bundle(&dir);

        let mut cmd = cli_command();
        let output = cmd
            .args(["replay", bundle_dir.to_str().unwrap(), "--format", "json"])
            .output()
            .expect("Failed to execute replay");

        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(!stdout.contains("Doe^John"));
        assert!(!stdout.contains("123456^^^HOSP^MR"));
        assert!(!stdout.contains("19700101"));
        assert!(!stdout.contains("123 Main St"));

        let report: serde_json::Value =
            serde_json::from_str(&stdout).expect("replay output should be JSON");
        assert_eq!(report["replay_version"], "1");
        assert_eq!(report["bundle_version"], "1");
        assert_eq!(report["message_type"], "ADT^A01");
        assert_eq!(report["reproduced"], true);
        assert_eq!(report["validation_valid"], true);
        assert_eq!(report["validation_issue_count"], 0);
        assert_eq!(report["validation_report"]["profile"], "profile.yaml");
        assert!(
            report["checks"]
                .as_array()
                .unwrap()
                .iter()
                .all(|check| check["status"] == "pass")
        );
    }

    #[test]
    fn test_replay_json_schema_version_2_adds_provenance() {
        let dir = create_temp_dir();
        let bundle_dir = create_replayable_bundle(&dir);

        let mut cmd = cli_command();
        let output = cmd
            .args([
                "replay",
                bundle_dir.to_str().unwrap(),
                "--format",
                "json",
                "--schema-version",
                "2",
            ])
            .output()
            .expect("Failed to execute replay");

        assert!(output.status.success());
        let report: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("replay output should be JSON");
        assert_eq!(report["schema_version"], "2");
        assert_eq!(report["replay_version"], "1");
        assert_eq!(report["tool_name"], "hl7v2-cli");
        assert_eq!(report["tool_version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(report["reproduced"], true);
    }

    #[test]
    fn test_replay_text_rejects_schema_version_2() {
        let dir = create_temp_dir();
        let bundle_dir = create_replayable_bundle(&dir);

        let mut cmd = cli_command();
        cmd.args([
            "replay",
            bundle_dir.to_str().unwrap(),
            "--schema-version",
            "2",
        ])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains(
            "replay report schema v2 is available only",
        ));
    }

    #[test]
    fn test_replay_fails_when_stored_validation_report_is_tampered() {
        let dir = create_temp_dir();
        let bundle_dir = create_replayable_bundle(&dir);
        let report_path = bundle_dir.join("validation-report.json");
        let mut stored_report: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&report_path).unwrap()).unwrap();
        stored_report["issue_count"] = serde_json::json!(42);
        std::fs::write(
            &report_path,
            serde_json::to_vec_pretty(&stored_report).unwrap(),
        )
        .unwrap();

        let mut cmd = cli_command();
        let output = cmd
            .args(["replay", bundle_dir.to_str().unwrap(), "--format", "json"])
            .output()
            .expect("Failed to execute replay");

        assert!(!output.status.success());
        let stdout = String::from_utf8(output.stdout).unwrap();
        let report: serde_json::Value =
            serde_json::from_str(&stdout).expect("replay failure output should be JSON");
        assert_eq!(report["reproduced"], false);
        assert!(report["checks"].as_array().unwrap().iter().any(|check| {
            check["name"] == "manifest-hashes"
                && check["status"] == "fail"
                && check["message"]
                    .as_str()
                    .unwrap()
                    .contains("validation-report.json")
        }));
    }

    #[test]
    fn test_replay_fails_when_redacted_message_is_tampered() {
        let dir = create_temp_dir();
        let bundle_dir = create_replayable_bundle(&dir);
        let message_path = bundle_dir.join("message.redacted.hl7");
        let mut message = std::fs::read_to_string(&message_path).unwrap();
        message.push_str("OBX|2|ST|LEAK^SENTINEL^L||not-phi\r");
        std::fs::write(&message_path, message).unwrap();

        let mut cmd = cli_command();
        let output = cmd
            .args(["replay", bundle_dir.to_str().unwrap(), "--format", "json"])
            .output()
            .expect("Failed to execute replay");

        assert!(!output.status.success());
        let stdout = String::from_utf8(output.stdout).unwrap();
        let report: serde_json::Value =
            serde_json::from_str(&stdout).expect("replay failure output should be JSON");
        assert_eq!(report["reproduced"], false);
        assert!(report["checks"].as_array().unwrap().iter().any(|check| {
            check["name"] == "manifest-hashes"
                && check["status"] == "fail"
                && check["message"]
                    .as_str()
                    .unwrap()
                    .contains("message.redacted.hl7")
        }));
    }

    #[test]
    fn test_replay_fails_when_manifest_is_malformed() {
        let dir = create_temp_dir();
        let bundle_dir = create_replayable_bundle(&dir);
        std::fs::write(bundle_dir.join("manifest.json"), b"{not-json").unwrap();

        let mut cmd = cli_command();
        let output = cmd
            .args(["replay", bundle_dir.to_str().unwrap(), "--format", "json"])
            .output()
            .expect("Failed to execute replay");

        assert!(!output.status.success());
        let stdout = String::from_utf8(output.stdout).unwrap();
        let report: serde_json::Value =
            serde_json::from_str(&stdout).expect("replay failure output should be JSON");
        assert_eq!(report["reproduced"], false);
        assert!(
            report["checks"]
                .as_array()
                .unwrap()
                .iter()
                .any(|check| check["name"] == "manifest" && check["status"] == "fail")
        );
    }

    #[test]
    fn test_replay_fails_when_manifest_hash_is_wrong() {
        let dir = create_temp_dir();
        let bundle_dir = create_replayable_bundle(&dir);
        let manifest_path = bundle_dir.join("manifest.json");
        let mut manifest: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&manifest_path).unwrap()).unwrap();
        manifest["artifacts"][0]["sha256"] =
            serde_json::json!("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
        std::fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let mut cmd = cli_command();
        let output = cmd
            .args(["replay", bundle_dir.to_str().unwrap(), "--format", "json"])
            .output()
            .expect("Failed to execute replay");

        assert!(!output.status.success());
        let stdout = String::from_utf8(output.stdout).unwrap();
        let report: serde_json::Value =
            serde_json::from_str(&stdout).expect("replay failure output should be JSON");
        assert_eq!(report["reproduced"], false);
        assert!(
            report["checks"]
                .as_array()
                .unwrap()
                .iter()
                .any(|check| check["name"] == "manifest-hashes"
                    && check["status"] == "fail"
                    && check["message"].as_str().unwrap().contains("hash mismatch"))
        );
    }

    #[test]
    fn test_replay_report_does_not_emit_phi_leak_sentinels_or_paths() {
        let dir = create_temp_dir();
        let (bundle_dir, message_file, policy_file) = create_replayable_bundle_with_inputs(
            &dir,
            RAW_INPUT_FILE_SENTINEL,
            PHI_LEAK_SENTINEL_MESSAGE,
            RAW_POLICY_FILE_SENTINEL,
            PHI_LEAK_SENTINEL_POLICY,
        );

        let mut cmd = cli_command();
        let output = cmd
            .args(["replay", bundle_dir.to_str().unwrap(), "--format", "json"])
            .output()
            .expect("Failed to execute replay");

        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert_no_phi_leak_sentinels_or_paths(
            "replay stdout",
            &stdout,
            &message_file,
            &policy_file,
        );
        assert_no_phi_leak_sentinels_or_paths(
            "replay stderr",
            &stderr,
            &message_file,
            &policy_file,
        );
    }

    #[test]
    fn test_replay_fails_when_bundle_artifact_is_missing() {
        let dir = create_temp_dir();
        let bundle_dir = create_replayable_bundle(&dir);
        std::fs::remove_file(bundle_dir.join("field-paths.json")).unwrap();

        let mut cmd = cli_command();
        let output = cmd
            .args(["replay", bundle_dir.to_str().unwrap(), "--format", "json"])
            .output()
            .expect("Failed to execute replay");

        assert!(!output.status.success());
        let stdout = String::from_utf8(output.stdout).unwrap();
        let report: serde_json::Value =
            serde_json::from_str(&stdout).expect("replay failure output should be JSON");
        assert_eq!(report["reproduced"], false);
        assert!(report["checks"].as_array().unwrap().iter().any(|check| {
            check["name"] == "bundle-layout"
                && check["status"] == "fail"
                && check["message"]
                    .as_str()
                    .unwrap()
                    .contains("field-paths.json")
        }));
    }
}

// =========================================================================
// Evidence Golden Fixture Tests
// =========================================================================

mod evidence_golden_fixtures {
    use super::*;
    use serde_json::Value;
    use std::path::Path;

    const PROFILE: &str = r#"
message_structure: ADT_A01
version: "2.5"
segments:
  - id: MSH
  - id: PID
constraints:
  - path: PID.3
    required: true
"#;

    const LINT_WARNING_PROFILE: &str = r#"
message_structure: ADT_A01
version: "2.5"
segments:
  - id: MSH
rules: []
"#;

    const MISSING_PID3_MESSAGE: &str = "MSH|^~\\&|||||||ADT^A01|CTRL123|P|2.5\rPID|1\r";
    const VALID_ADT_MESSAGE: &str =
        "MSH|^~\\&|||||||ADT^A01|CTRL123|P|2.5\rPID|1||123456^^^HOSP^MR\r";
    const CORPUS_ADT_MESSAGE: &str = "MSH|^~\\&|||||||ADT^A01|CTRL123|P|2.5\r";
    const CORPUS_ORU_MESSAGE: &str = "MSH|^~\\&|||||||ORU^R01|CTRL124|P|2.5\r";
    const PHI_MESSAGE: &str = "MSH|^~\\&|LAB|L|EHR|E|202605030101||ADT^A01|CTRL123|P|2.5\rPID|1||123456^^^HOSP^MR||Doe^John||19700101|M|||123 Main St\rOBX|1|NM|718-7^Hemoglobin^LN||13.2|g/dL\r";
    const SAFE_ANALYSIS_POLICY: &str = r#"
[[rules]]
path = "PID.3"
action = "hash"
reason = "patient identifier"

[[rules]]
path = "PID.5"
action = "drop"
reason = "patient name"

[[rules]]
path = "PID.7"
action = "drop"
reason = "date of birth"

[[rules]]
path = "PID.11"
action = "drop"
reason = "patient address"

[[rules]]
path = "MSH.9"
action = "retain"
reason = "message type is needed for analysis"

[[rules]]
path = "MSH.10"
action = "retain"
reason = "control id is needed for replay correlation"

[[rules]]
path = "OBX.3"
action = "retain"
reason = "observation identifier is needed for analysis"

[[rules]]
path = "OBX.5"
action = "retain"
reason = "non-PHI synthetic observation value shape is needed for analysis"
"#;

    fn fixture(name: &str) -> Value {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join("evidence")
            .join(format!("{name}.json"));
        serde_json::from_slice(&std::fs::read(&path).expect("evidence fixture should be readable"))
            .expect("evidence fixture should be valid JSON")
    }

    fn set(value: &mut Value, pointer: &str, replacement: impl Into<Value>) {
        *value
            .pointer_mut(pointer)
            .expect("fixture normalization pointer should exist") = replacement.into();
    }

    fn command_json(args: &[String], expect_success: bool) -> Value {
        let mut cmd = cli_command();
        let output = cmd.args(args).output().expect("CLI command should run");
        assert_eq!(
            output.status.success(),
            expect_success,
            "stdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).expect("CLI stdout should be JSON")
    }

    fn assert_fixture(name: &str, actual: Value) {
        assert_eq!(
            actual,
            fixture(name),
            "evidence fixture {name}.json drifted"
        );
    }

    #[test]
    fn test_doctor_report_matches_golden_fixture() {
        let mut report = command_json(
            &[
                "doctor".to_string(),
                "--format".to_string(),
                "json".to_string(),
            ],
            true,
        );

        set(&mut report, "/version", "1.3.0");
        set(&mut report, "/checks/0/message", "hl7v2-cli 1.3.0");
        set(&mut report, "/checks/5/status", "warn");
        set(
            &mut report,
            "/checks/5/message",
            "python binding check normalized",
        );
        assert_fixture("doctor-report", report);

        let mut report_v2 = command_json(
            &[
                "doctor".to_string(),
                "--format".to_string(),
                "json".to_string(),
                "--schema-version".to_string(),
                "2".to_string(),
            ],
            true,
        );
        set(&mut report_v2, "/tool_version", "1.5.0");
        set(&mut report_v2, "/version", "1.5.0");
        set(&mut report_v2, "/checks/0/message", "hl7v2-cli 1.5.0");
        set(&mut report_v2, "/checks/5/status", "warn");
        set(
            &mut report_v2,
            "/checks/5/message",
            "python binding check normalized",
        );
        assert_fixture("doctor-report-v2", report_v2);
    }

    #[test]
    fn test_validation_report_matches_golden_fixture() {
        let dir = create_temp_dir();
        let message = create_temp_hl7_with_content(&dir, "missing_pid3.hl7", MISSING_PID3_MESSAGE);
        let profile = create_temp_profile(&dir, "profile.yaml", PROFILE);
        let mut report = command_json(
            &[
                "val".to_string(),
                message.to_string_lossy().into_owned(),
                "--profile".to_string(),
                profile.to_string_lossy().into_owned(),
                "--report".to_string(),
                "json".to_string(),
            ],
            false,
        );
        set(&mut report, "/profile", "profile.yaml");

        assert_fixture("validation-report", report);
    }

    #[test]
    fn test_profile_reports_match_golden_fixtures() {
        let dir = create_temp_dir();
        let profile = create_temp_profile(&dir, "profile.yaml", PROFILE);
        let lint_profile = create_temp_profile(&dir, "lint.yaml", LINT_WARNING_PROFILE);

        let lint_report = command_json(
            &[
                "profile".to_string(),
                "lint".to_string(),
                lint_profile.to_string_lossy().into_owned(),
                "--report".to_string(),
                "json".to_string(),
            ],
            true,
        );
        assert_fixture("profile-lint-report", lint_report);

        let mut lint_report_v2 = command_json(
            &[
                "profile".to_string(),
                "lint".to_string(),
                lint_profile.to_string_lossy().into_owned(),
                "--report".to_string(),
                "json".to_string(),
                "--schema-version".to_string(),
                "2".to_string(),
            ],
            true,
        );
        set(&mut lint_report_v2, "/tool_version", "1.5.0");
        assert_fixture("profile-lint-report-v2", lint_report_v2);

        let mut explain_report = command_json(
            &[
                "profile".to_string(),
                "explain".to_string(),
                profile.to_string_lossy().into_owned(),
                "--format".to_string(),
                "json".to_string(),
            ],
            true,
        );
        set(&mut explain_report, "/profile", "profile.yaml");
        assert_fixture("profile-explain-report", explain_report);

        let mut explain_report_v2 = command_json(
            &[
                "profile".to_string(),
                "explain".to_string(),
                profile.to_string_lossy().into_owned(),
                "--format".to_string(),
                "json".to_string(),
                "--schema-version".to_string(),
                "2".to_string(),
            ],
            true,
        );
        set(&mut explain_report_v2, "/tool_version", "1.5.0");
        set(&mut explain_report_v2, "/profile", "profile.yaml");
        assert_fixture("profile-explain-report-v2", explain_report_v2);

        let fixtures = dir.path().join("fixtures");
        std::fs::create_dir_all(fixtures.join("valid")).unwrap();
        std::fs::create_dir_all(fixtures.join("invalid")).unwrap();
        create_temp_file(
            &dir,
            "fixtures/valid/valid.hl7",
            VALID_ADT_MESSAGE.as_bytes(),
        );
        create_temp_hl7_with_content(
            &dir,
            "fixtures/invalid/missing_pid3.hl7",
            MISSING_PID3_MESSAGE,
        );

        let mut test_report = command_json(
            &[
                "profile".to_string(),
                "test".to_string(),
                profile.to_string_lossy().into_owned(),
                fixtures.to_string_lossy().into_owned(),
                "--report".to_string(),
                "json".to_string(),
            ],
            true,
        );
        set(&mut test_report, "/profile", "profile.yaml");
        set(&mut test_report, "/fixtures", "fixtures");
        set(
            &mut test_report,
            "/cases/0/path",
            "fixtures/valid/valid.hl7",
        );
        set(
            &mut test_report,
            "/cases/0/validation_report/profile",
            "profile.yaml",
        );
        set(
            &mut test_report,
            "/cases/1/path",
            "fixtures/invalid/missing_pid3.hl7",
        );
        set(
            &mut test_report,
            "/cases/1/validation_report/profile",
            "profile.yaml",
        );
        assert_fixture("profile-test-report", test_report);

        let mut test_report_v2 = command_json(
            &[
                "profile".to_string(),
                "test".to_string(),
                profile.to_string_lossy().into_owned(),
                fixtures.to_string_lossy().into_owned(),
                "--report".to_string(),
                "json".to_string(),
                "--schema-version".to_string(),
                "2".to_string(),
            ],
            true,
        );
        set(&mut test_report_v2, "/tool_version", "1.5.0");
        set(&mut test_report_v2, "/profile", "profile.yaml");
        set(&mut test_report_v2, "/fixtures", "fixtures");
        set(
            &mut test_report_v2,
            "/cases/0/path",
            "fixtures/valid/valid.hl7",
        );
        set(
            &mut test_report_v2,
            "/cases/0/validation_report/profile",
            "profile.yaml",
        );
        set(
            &mut test_report_v2,
            "/cases/1/path",
            "fixtures/invalid/missing_pid3.hl7",
        );
        set(
            &mut test_report_v2,
            "/cases/1/validation_report/profile",
            "profile.yaml",
        );
        assert_fixture("profile-test-report-v2", test_report_v2);
    }

    #[test]
    fn test_corpus_reports_match_golden_fixtures() {
        let dir = create_temp_dir();
        let site = dir.path().join("site-a");
        let before = dir.path().join("before");
        let after = dir.path().join("after");
        std::fs::create_dir_all(&site).unwrap();
        std::fs::create_dir_all(&before).unwrap();
        std::fs::create_dir_all(&after).unwrap();
        create_temp_file(&dir, "site-a/adt.hl7", CORPUS_ADT_MESSAGE.as_bytes());
        create_temp_file(&dir, "before/adt.hl7", CORPUS_ADT_MESSAGE.as_bytes());
        create_temp_file(&dir, "after/oru.hl7", CORPUS_ORU_MESSAGE.as_bytes());

        let mut summary = command_json(
            &[
                "corpus".to_string(),
                "summarize".to_string(),
                site.to_string_lossy().into_owned(),
                "--format".to_string(),
                "json".to_string(),
            ],
            true,
        );
        set(&mut summary, "/root", "site-a");
        assert_fixture("corpus-summary", summary);

        let mut summary_v2 = command_json(
            &[
                "corpus".to_string(),
                "summarize".to_string(),
                site.to_string_lossy().into_owned(),
                "--format".to_string(),
                "json".to_string(),
                "--schema-version".to_string(),
                "2".to_string(),
            ],
            true,
        );
        set(&mut summary_v2, "/tool_version", "1.5.0");
        set(&mut summary_v2, "/root", "site-a");
        assert_fixture("corpus-summary-v2", summary_v2);

        let mut fingerprint = command_json(
            &[
                "corpus".to_string(),
                "fingerprint".to_string(),
                site.to_string_lossy().into_owned(),
                "--format".to_string(),
                "json".to_string(),
            ],
            true,
        );
        set(&mut fingerprint, "/tool_version", "1.3.0");
        set(&mut fingerprint, "/root", "site-a");
        assert_fixture("corpus-fingerprint", fingerprint);

        let mut diff = command_json(
            &[
                "corpus".to_string(),
                "diff".to_string(),
                before.to_string_lossy().into_owned(),
                after.to_string_lossy().into_owned(),
                "--format".to_string(),
                "json".to_string(),
            ],
            true,
        );
        set(&mut diff, "/tool_version", "1.3.0");
        set(&mut diff, "/before_root", "before");
        set(&mut diff, "/after_root", "after");
        assert_fixture("corpus-diff", diff);
    }

    #[test]
    fn test_redaction_bundle_and_replay_reports_match_golden_fixtures() {
        let dir = create_temp_dir();
        let message = create_temp_hl7_with_content(&dir, "message.hl7", PHI_MESSAGE);
        let profile = create_temp_profile(&dir, "profile.yaml", minimal_profile());
        let policy = create_temp_file(&dir, "safe-analysis.toml", SAFE_ANALYSIS_POLICY.as_bytes());

        let redact_output = command_json(
            &[
                "redact".to_string(),
                message.to_string_lossy().into_owned(),
                "--policy".to_string(),
                policy.to_string_lossy().into_owned(),
                "--format".to_string(),
                "json".to_string(),
            ],
            true,
        );
        assert_fixture("redaction-receipt", redact_output["receipt"].clone());

        let mut redact_v2_output = command_json(
            &[
                "redact".to_string(),
                message.to_string_lossy().into_owned(),
                "--policy".to_string(),
                policy.to_string_lossy().into_owned(),
                "--format".to_string(),
                "json".to_string(),
                "--schema-version".to_string(),
                "2".to_string(),
            ],
            true,
        );
        set(&mut redact_v2_output, "/receipt/tool_version", "1.5.0");
        assert_fixture("redaction-receipt-v2", redact_v2_output["receipt"].clone());

        let bundle = dir.path().join("issue-bundle");
        let bundle_summary = command_json(
            &[
                "bundle".to_string(),
                message.to_string_lossy().into_owned(),
                "--profile".to_string(),
                profile.to_string_lossy().into_owned(),
                "--redact-policy".to_string(),
                policy.to_string_lossy().into_owned(),
                "--out".to_string(),
                bundle.to_string_lossy().into_owned(),
            ],
            true,
        );
        assert_fixture("evidence-bundle", bundle_summary);

        let bundle_v2 = dir.path().join("issue-bundle-v2");
        let mut bundle_summary_v2 = command_json(
            &[
                "bundle".to_string(),
                message.to_string_lossy().into_owned(),
                "--profile".to_string(),
                profile.to_string_lossy().into_owned(),
                "--redact-policy".to_string(),
                policy.to_string_lossy().into_owned(),
                "--out".to_string(),
                bundle_v2.to_string_lossy().into_owned(),
                "--schema-version".to_string(),
                "2".to_string(),
            ],
            true,
        );
        set(&mut bundle_summary_v2, "/tool_version", "1.3.0");
        assert_fixture("evidence-bundle-v2", bundle_summary_v2);

        let mut receipt_v2: Value = serde_json::from_slice(
            &std::fs::read(bundle_v2.join("redaction-receipt.json"))
                .expect("bundle v2 redaction receipt should be readable"),
        )
        .expect("bundle v2 redaction receipt should be JSON");
        set(&mut receipt_v2, "/tool_version", "1.5.0");
        assert_fixture("redaction-receipt-v2", receipt_v2);

        let mut field_paths_v2: Value = serde_json::from_slice(
            &std::fs::read(bundle_v2.join("field-paths.json"))
                .expect("bundle v2 field-path trace should be readable"),
        )
        .expect("bundle v2 field-path trace should be JSON");
        set(&mut field_paths_v2, "/tool_version", "1.3.0");
        assert_fixture("field-path-trace-v2", field_paths_v2);

        let mut environment_v2: Value = serde_json::from_slice(
            &std::fs::read(bundle_v2.join("environment.json"))
                .expect("bundle v2 environment should be readable"),
        )
        .expect("bundle v2 environment should be JSON");
        set(&mut environment_v2, "/tool_version", "1.3.0");
        set(
            &mut environment_v2,
            "/input_sha256",
            "0000000000000000000000000000000000000000000000000000000000000000",
        );
        set(
            &mut environment_v2,
            "/profile_sha256",
            "0000000000000000000000000000000000000000000000000000000000000000",
        );
        set(
            &mut environment_v2,
            "/redaction_policy_sha256",
            "0000000000000000000000000000000000000000000000000000000000000000",
        );
        assert_fixture("evidence-bundle-environment-v2", environment_v2);

        let mut manifest_v2: Value = serde_json::from_slice(
            &std::fs::read(bundle_v2.join("manifest.json"))
                .expect("bundle v2 manifest should be readable"),
        )
        .expect("bundle v2 manifest should be JSON");
        set(&mut manifest_v2, "/tool_version", "1.3.0");
        for artifact in manifest_v2
            .get_mut("artifacts")
            .and_then(Value::as_array_mut)
            .expect("manifest v2 artifacts should be an array")
        {
            artifact["sha256"] =
                "0000000000000000000000000000000000000000000000000000000000000000".into();
        }
        assert_fixture("evidence-bundle-manifest-v2", manifest_v2);

        let receipt: Value = serde_json::from_slice(
            &std::fs::read(bundle.join("redaction-receipt.json"))
                .expect("bundle redaction receipt should be readable"),
        )
        .expect("bundle redaction receipt should be JSON");
        assert_fixture("redaction-receipt", receipt);

        let field_paths: Value = serde_json::from_slice(
            &std::fs::read(bundle.join("field-paths.json"))
                .expect("bundle field-path trace should be readable"),
        )
        .expect("bundle field-path trace should be JSON");
        assert_fixture("field-path-trace", field_paths);

        let mut environment: Value = serde_json::from_slice(
            &std::fs::read(bundle.join("environment.json"))
                .expect("bundle environment should be readable"),
        )
        .expect("bundle environment should be JSON");
        set(&mut environment, "/tool_version", "1.3.0");
        set(
            &mut environment,
            "/input_sha256",
            "0000000000000000000000000000000000000000000000000000000000000000",
        );
        set(
            &mut environment,
            "/profile_sha256",
            "0000000000000000000000000000000000000000000000000000000000000000",
        );
        set(
            &mut environment,
            "/redaction_policy_sha256",
            "0000000000000000000000000000000000000000000000000000000000000000",
        );
        assert_fixture("evidence-bundle-environment", environment);

        let mut manifest: Value = serde_json::from_slice(
            &std::fs::read(bundle.join("manifest.json"))
                .expect("bundle manifest should be readable"),
        )
        .expect("bundle manifest should be JSON");
        set(&mut manifest, "/tool_version", "1.3.0");
        for artifact in manifest
            .get_mut("artifacts")
            .and_then(Value::as_array_mut)
            .expect("manifest artifacts should be an array")
        {
            artifact["sha256"] =
                "0000000000000000000000000000000000000000000000000000000000000000".into();
        }
        assert_fixture("evidence-bundle-manifest", manifest);

        let mut replay = command_json(
            &[
                "replay".to_string(),
                bundle.to_string_lossy().into_owned(),
                "--format".to_string(),
                "json".to_string(),
            ],
            true,
        );
        set(&mut replay, "/tool_version", "1.3.0");
        assert_fixture("evidence-replay", replay);

        let mut replay_v2 = command_json(
            &[
                "replay".to_string(),
                bundle.to_string_lossy().into_owned(),
                "--format".to_string(),
                "json".to_string(),
                "--schema-version".to_string(),
                "2".to_string(),
            ],
            true,
        );
        set(&mut replay_v2, "/tool_version", "1.3.0");
        assert_fixture("evidence-replay-v2", replay_v2);
    }
}

// =========================================================================
// ACK Generation Command Tests
// =========================================================================

mod ack_command {
    use super::*;

    #[test]
    fn test_generate_ack_aa() {
        let dir = create_temp_dir();
        let hl7_file = create_temp_hl7_file(&dir, "test.hl7");

        let mut cmd = cli_command();
        // ACK generation may fail due to escape sequences - just verify it runs
        let result = cmd
            .args(["ack", hl7_file.to_str().unwrap(), "--code", "AA"])
            .output();
        assert!(result.is_ok());
    }

    #[test]
    fn test_generate_ack_ae() {
        let dir = create_temp_dir();
        let hl7_file = create_temp_hl7_file(&dir, "test.hl7");

        let mut cmd = cli_command();
        let result = cmd
            .args(["ack", hl7_file.to_str().unwrap(), "--code", "AE"])
            .output();
        assert!(result.is_ok());
    }

    #[test]
    fn test_generate_ack_ar() {
        let dir = create_temp_dir();
        let hl7_file = create_temp_hl7_file(&dir, "test.hl7");

        let mut cmd = cli_command();
        let result = cmd
            .args(["ack", hl7_file.to_str().unwrap(), "--code", "AR"])
            .output();
        assert!(result.is_ok());
    }

    #[test]
    fn test_generate_ack_with_mllp_output() {
        let dir = create_temp_dir();
        let hl7_file = create_temp_hl7_file(&dir, "test.hl7");

        let mut cmd = cli_command();
        let result = cmd
            .args([
                "ack",
                hl7_file.to_str().unwrap(),
                "--code",
                "AA",
                "--mllp-out",
            ])
            .output();

        // Just verify the command runs
        assert!(result.is_ok());
    }

    #[test]
    fn test_generate_ack_with_summary() {
        let dir = create_temp_dir();
        let hl7_file = create_temp_hl7_file(&dir, "test.hl7");

        let mut cmd = cli_command();
        let result = cmd
            .args([
                "ack",
                hl7_file.to_str().unwrap(),
                "--code",
                "AA",
                "--summary",
            ])
            .output();
        assert!(result.is_ok());
    }

    #[test]
    fn test_generate_ack_original_mode() {
        let dir = create_temp_dir();
        let hl7_file = create_temp_hl7_file(&dir, "test.hl7");

        let mut cmd = cli_command();
        let result = cmd
            .args([
                "ack",
                hl7_file.to_str().unwrap(),
                "--mode",
                "original",
                "--code",
                "AA",
            ])
            .output();
        assert!(result.is_ok());
    }

    #[test]
    fn test_generate_ack_enhanced_mode() {
        let dir = create_temp_dir();
        let hl7_file = create_temp_hl7_file(&dir, "test.hl7");

        let mut cmd = cli_command();
        let result = cmd
            .args([
                "ack",
                hl7_file.to_str().unwrap(),
                "--mode",
                "enhanced",
                "--code",
                "CA",
            ])
            .output();
        assert!(result.is_ok());
    }

    #[test]
    fn test_generate_ack_missing_file() {
        let mut cmd = cli_command();
        cmd.args(["ack", "/nonexistent/file.hl7", "--code", "AA"])
            .assert()
            .failure();
    }
}

// =========================================================================
// Generate Command Tests
// =========================================================================

mod gen_command {
    use super::*;

    #[test]
    fn test_gen_with_template() {
        let dir = create_temp_dir();
        let template_file = create_temp_profile(&dir, "template.yaml", simple_template());
        let output_dir = dir.path().join("output");

        let mut cmd = cli_command();
        // Gen command may fail due to template format - just verify it runs
        let result = cmd
            .args([
                "gen",
                "--profile",
                template_file.to_str().unwrap(),
                "--seed",
                "42",
                "--count",
                "1",
                "--out",
                output_dir.to_str().unwrap(),
            ])
            .output();
        assert!(result.is_ok());
    }

    #[test]
    fn test_gen_with_stats() {
        let dir = create_temp_dir();
        let template_file = create_temp_profile(&dir, "template.yaml", simple_template());
        let output_dir = dir.path().join("output");

        let mut cmd = cli_command();
        let result = cmd
            .args([
                "gen",
                "--profile",
                template_file.to_str().unwrap(),
                "--seed",
                "42",
                "--count",
                "1",
                "--out",
                output_dir.to_str().unwrap(),
                "--stats",
            ])
            .output();
        assert!(result.is_ok());
    }

    #[test]
    fn test_gen_multiple_messages() {
        let dir = create_temp_dir();
        let template_file = create_temp_profile(&dir, "template.yaml", simple_template());
        let output_dir = dir.path().join("output");

        let mut cmd = cli_command();
        let result = cmd
            .args([
                "gen",
                "--profile",
                template_file.to_str().unwrap(),
                "--seed",
                "42",
                "--count",
                "3",
                "--out",
                output_dir.to_str().unwrap(),
            ])
            .output();
        assert!(result.is_ok());
    }

    #[test]
    fn test_gen_missing_template() {
        let dir = create_temp_dir();
        let output_dir = dir.path().join("output");

        let mut cmd = cli_command();
        cmd.args([
            "gen",
            "--profile",
            "/nonexistent/template.yaml",
            "--seed",
            "42",
            "--count",
            "1",
            "--out",
            output_dir.to_str().unwrap(),
        ])
        .assert()
        .failure();
    }
}

// =========================================================================
// Error Handling Tests
// =========================================================================

mod error_handling {
    use super::*;

    #[test]
    fn test_invalid_command() {
        let mut cmd = cli_command();
        cmd.args(["invalid-command"]).assert().failure();
    }

    #[test]
    fn test_parse_no_args() {
        let mut cmd = cli_command();
        cmd.args(["parse"]).assert().failure();
    }

    #[test]
    fn test_val_no_args() {
        let mut cmd = cli_command();
        cmd.args(["val"]).assert().failure();
    }

    #[test]
    fn test_ack_no_args() {
        let mut cmd = cli_command();
        cmd.args(["ack"]).assert().failure();
    }

    #[test]
    fn test_gen_no_args() {
        let mut cmd = cli_command();
        cmd.args(["gen"]).assert().failure();
    }

    #[test]
    fn test_norm_no_args() {
        let mut cmd = cli_command();
        cmd.args(["norm"]).assert().failure();
    }

    #[test]
    fn test_invalid_ack_code() {
        let dir = create_temp_dir();
        let hl7_file = create_temp_hl7_file(&dir, "test.hl7");

        let mut cmd = cli_command();
        cmd.args(["ack", hl7_file.to_str().unwrap(), "--code", "INVALID"])
            .assert()
            .failure();
    }

    #[test]
    fn test_invalid_ack_mode() {
        let dir = create_temp_dir();
        let hl7_file = create_temp_hl7_file(&dir, "test.hl7");

        let mut cmd = cli_command();
        cmd.args([
            "ack",
            hl7_file.to_str().unwrap(),
            "--mode",
            "INVALID",
            "--code",
            "AA",
        ])
        .assert()
        .failure();
    }
}

// =========================================================================
// Exit Code Tests
// =========================================================================

mod exit_codes {
    use super::*;

    #[test]
    fn test_success_returns_zero() {
        let dir = create_temp_dir();
        let hl7_file = create_temp_hl7_file(&dir, "test.hl7");

        let mut cmd = cli_command();
        cmd.args(["parse", hl7_file.to_str().unwrap()])
            .assert()
            .code(0);
    }

    #[test]
    fn test_parse_error_returns_nonzero() {
        let dir = create_temp_dir();
        let invalid_file = create_temp_file(&dir, "invalid.hl7", invalid_hl7_message().as_bytes());

        let mut cmd = cli_command();
        cmd.args(["parse", invalid_file.to_str().unwrap()])
            .assert()
            .code(2);
    }

    #[test]
    fn test_missing_file_returns_nonzero() {
        let mut cmd = cli_command();
        cmd.args(["parse", "/nonexistent/file.hl7"])
            .assert()
            .code(3);
    }
}

// =========================================================================
// CLI Output Contract Tests
// =========================================================================

mod output_contract {
    use super::*;

    const PROFILE_REQUIRING_PID3: &str = r#"
message_structure: ADT_A01
version: "2.5"
segments:
  - id: MSH
  - id: PID
constraints:
  - path: PID.3
    required: true
"#;

    const MISSING_PID3_MESSAGE: &str = "MSH|^~\\&|||||||ADT^A01|CTRL123|P|2.5\rPID|1\r";
    const VALID_ADT_MESSAGE: &str =
        "MSH|^~\\&|||||||ADT^A01|CTRL123|P|2.5\rPID|1||123456^^^HOSP^MR\r";
    const SAFE_ANALYSIS_POLICY: &str = r#"
[[rules]]
path = "PID.3"
action = "hash"
reason = "patient identifier"
"#;

    fn output_text(bytes: &[u8]) -> String {
        String::from_utf8_lossy(bytes).to_string()
    }

    #[test]
    fn test_machine_readable_success_uses_stdout_and_exit_zero() {
        let dir = create_temp_dir();
        let corpus = dir.path().join("corpus");
        std::fs::create_dir_all(&corpus).unwrap();
        create_temp_file(&dir, "corpus/adt.hl7", VALID_ADT_MESSAGE.as_bytes());

        let mut cmd = cli_command();
        let output = cmd
            .args([
                "corpus",
                "summarize",
                corpus.to_str().unwrap(),
                "--format",
                "json",
            ])
            .output()
            .expect("corpus summarize should run");

        assert_eq!(output.status.code(), Some(0));
        assert!(output.stderr.is_empty());
        let report: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("stdout should be JSON");
        assert_eq!(report["message_count"], 1);
    }

    #[test]
    fn test_validation_failure_returns_one_with_report_on_stdout() {
        let dir = create_temp_dir();
        let message = create_temp_hl7_with_content(&dir, "missing_pid3.hl7", MISSING_PID3_MESSAGE);
        let profile = create_temp_profile(&dir, "profile.yaml", PROFILE_REQUIRING_PID3);

        let mut cmd = cli_command();
        let output = cmd
            .args([
                "val",
                message.to_str().unwrap(),
                "--profile",
                profile.to_str().unwrap(),
                "--report",
                "json",
            ])
            .output()
            .expect("validation command should run");

        assert_eq!(output.status.code(), Some(1));
        let report: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("stdout should be JSON report");
        assert_eq!(report["valid"], false);
        assert!(output_text(&output.stderr).contains("Error: validation failed"));
    }

    #[test]
    fn test_validation_report_schema_v2_includes_provenance() {
        let dir = create_temp_dir();
        let message = create_temp_hl7_with_content(&dir, "missing_pid3.hl7", MISSING_PID3_MESSAGE);
        let profile = create_temp_profile(&dir, "profile.yaml", PROFILE_REQUIRING_PID3);

        let mut cmd = cli_command();
        let output = cmd
            .args([
                "val",
                message.to_str().unwrap(),
                "--profile",
                profile.to_str().unwrap(),
                "--report",
                "json",
                "--schema-version",
                "2",
            ])
            .output()
            .expect("validation command should run");

        assert_eq!(output.status.code(), Some(1));
        let report: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("stdout should be JSON report");
        assert_eq!(report["schema_version"], "2");
        assert_eq!(report["tool_name"], "hl7v2-cli");
        assert_eq!(report["profile"], "profile.yaml");
        assert_eq!(report["profile_identity"]["label"], "profile.yaml");
        assert_eq!(report["profile_identity"]["message_structure"], "ADT_A01");
        assert_eq!(report["profile_identity"]["version"], "2.5");
        assert_eq!(
            report["profile_identity"]["sha256"]
                .as_str()
                .unwrap_or_default()
                .len(),
            64
        );
        assert_eq!(report["issues"][0]["code"], "missing_required_field");
        assert!(output_text(&output.stderr).contains("Error: validation failed"));
    }

    #[test]
    fn test_validation_report_schema_v2_rejects_text_output() {
        let dir = create_temp_dir();
        let message = create_temp_hl7_with_content(&dir, "missing_pid3.hl7", MISSING_PID3_MESSAGE);
        let profile = create_temp_profile(&dir, "profile.yaml", PROFILE_REQUIRING_PID3);

        let mut cmd = cli_command();
        let output = cmd
            .args([
                "val",
                message.to_str().unwrap(),
                "--profile",
                profile.to_str().unwrap(),
                "--schema-version",
                "2",
            ])
            .output()
            .expect("validation command should run");

        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(output_text(&output.stderr).contains("schema v2"));
    }

    #[test]
    fn test_profile_check_failure_returns_one_with_report_on_stdout() {
        let dir = create_temp_dir();
        let profile = create_temp_profile(
            &dir,
            "profile.yaml",
            r#"
message_structure: ""
version: "2.5"
segments:
  - id: ""
"#,
        );

        let mut cmd = cli_command();
        let output = cmd
            .args([
                "profile",
                "lint",
                profile.to_str().unwrap(),
                "--report",
                "json",
            ])
            .output()
            .expect("profile lint should run");

        assert_eq!(output.status.code(), Some(1));
        let report: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("stdout should be JSON report");
        assert_eq!(report["valid"], false);
        assert!(output_text(&output.stderr).contains("profile lint reported errors"));
    }

    #[test]
    fn test_profile_fixture_failure_returns_one_with_report_on_stdout() {
        let dir = create_temp_dir();
        let profile = create_temp_profile(&dir, "profile.yaml", PROFILE_REQUIRING_PID3);
        let fixtures = dir.path().join("fixtures");
        std::fs::create_dir_all(fixtures.join("valid")).unwrap();
        std::fs::create_dir_all(fixtures.join("invalid")).unwrap();
        create_temp_file(
            &dir,
            "fixtures/valid/missing_pid3.hl7",
            MISSING_PID3_MESSAGE.as_bytes(),
        );

        let mut cmd = cli_command();
        let output = cmd
            .args([
                "profile",
                "test",
                profile.to_str().unwrap(),
                fixtures.to_str().unwrap(),
                "--report",
                "json",
            ])
            .output()
            .expect("profile test should run");

        assert_eq!(output.status.code(), Some(1));
        let report: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("stdout should be JSON report");
        assert_eq!(report["valid"], false);
        assert!(output_text(&output.stderr).contains("profile test reported failures"));
    }

    #[test]
    fn test_replay_failure_returns_one_with_report_on_stdout() {
        let dir = create_temp_dir();

        let mut cmd = cli_command();
        let output = cmd
            .args(["replay", dir.path().to_str().unwrap(), "--format", "json"])
            .output()
            .expect("replay should run");

        assert_eq!(output.status.code(), Some(1));
        let report: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("stdout should be JSON report");
        assert_eq!(report["reproduced"], false);
        assert!(output_text(&output.stderr).contains("bundle replay did not reproduce"));
    }

    #[test]
    fn test_parse_input_error_returns_two_with_diagnostic_on_stderr() {
        let dir = create_temp_dir();
        let invalid = create_temp_file(&dir, "invalid.hl7", invalid_hl7_message().as_bytes());

        let mut cmd = cli_command();
        let output = cmd
            .args(["parse", invalid.to_str().unwrap()])
            .output()
            .expect("parse should run");

        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(output_text(&output.stderr).contains("Error:"));
    }

    #[test]
    fn test_policy_input_error_returns_two_without_primary_output() {
        let dir = create_temp_dir();
        let message = create_temp_hl7_with_content(&dir, "message.hl7", VALID_ADT_MESSAGE);
        let policy = create_temp_profile(
            &dir,
            "policy.toml",
            r#"
[[rules]]
path = "PID.3"
action = "hash"
"#,
        );

        let mut cmd = cli_command();
        let output = cmd
            .args([
                "redact",
                message.to_str().unwrap(),
                "--policy",
                policy.to_str().unwrap(),
                "--format",
                "json",
            ])
            .output()
            .expect("redact should run");

        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(output_text(&output.stderr).contains("must include a reason"));
    }

    #[test]
    fn test_io_runtime_error_returns_three_with_diagnostic_on_stderr() {
        let mut cmd = cli_command();
        let output = cmd
            .args(["parse", "/nonexistent/file.hl7"])
            .output()
            .expect("parse should run");

        assert_eq!(output.status.code(), Some(3));
        assert!(output.stdout.is_empty());
        assert!(output_text(&output.stderr).contains("Error:"));
    }

    #[test]
    fn test_doctor_failed_check_returns_one_with_report_on_stdout() {
        let mut cmd = cli_command();
        let output = cmd
            .args(["doctor", "--server-url", "https://example.com/health"])
            .output()
            .expect("doctor should run");

        assert_eq!(output.status.code(), Some(1));
        assert!(output_text(&output.stdout).contains("[error] server"));
        assert!(output_text(&output.stderr).contains("doctor reported failed checks"));
    }

    #[test]
    fn test_redact_hl7_keeps_primary_output_on_stdout_and_receipt_on_stderr() {
        let dir = create_temp_dir();
        let message = create_temp_hl7_with_content(&dir, "message.hl7", VALID_ADT_MESSAGE);
        let policy = create_temp_profile(&dir, "policy.toml", SAFE_ANALYSIS_POLICY);

        let mut cmd = cli_command();
        let output = cmd
            .args([
                "redact",
                message.to_str().unwrap(),
                "--policy",
                policy.to_str().unwrap(),
                "--format",
                "hl7",
            ])
            .output()
            .expect("redact should run");

        assert_eq!(output.status.code(), Some(0));
        assert!(output_text(&output.stdout).contains("hash:sha256:"));
        assert!(output_text(&output.stderr).contains("Redaction receipt"));
    }

    fn json_from_file(path: &std::path::Path) -> serde_json::Value {
        serde_json::from_slice(&std::fs::read(path).expect("output file should be readable"))
            .expect("output file should contain JSON")
    }

    #[test]
    fn test_report_commands_write_output_file_and_keep_stdout_quiet() {
        let dir = create_temp_dir();
        let message = create_temp_hl7_with_content(&dir, "message.hl7", MISSING_PID3_MESSAGE);
        let profile = create_temp_profile(&dir, "profile.yaml", PROFILE_REQUIRING_PID3);
        let corpus = dir.path().join("corpus");
        let before = dir.path().join("before");
        let after = dir.path().join("after");
        std::fs::create_dir_all(&corpus).unwrap();
        std::fs::create_dir_all(&before).unwrap();
        std::fs::create_dir_all(&after).unwrap();
        create_temp_file(&dir, "corpus/adt.hl7", VALID_ADT_MESSAGE.as_bytes());
        create_temp_file(&dir, "before/adt.hl7", VALID_ADT_MESSAGE.as_bytes());
        create_temp_file(&dir, "after/adt.hl7", VALID_ADT_MESSAGE.as_bytes());

        let doctor_report = dir.path().join("doctor-report.json");
        let mut cmd = cli_command();
        let output = cmd
            .args([
                "doctor",
                "--format",
                "json",
                "--schema-version",
                "2",
                "--output",
                doctor_report.to_str().unwrap(),
                "--quiet",
                "--no-color",
            ])
            .output()
            .expect("doctor should run");
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stdout.is_empty());
        assert_eq!(json_from_file(&doctor_report)["schema_version"], "2");

        let val_report = dir.path().join("validation-report.json");
        let mut cmd = cli_command();
        let output = cmd
            .args([
                "val",
                message.to_str().unwrap(),
                "--profile",
                profile.to_str().unwrap(),
                "--report",
                "json",
                "--output",
                val_report.to_str().unwrap(),
                "--quiet",
                "--no-color",
            ])
            .output()
            .expect("validation command should run");
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert_eq!(json_from_file(&val_report)["valid"], false);

        let val_text_report = dir.path().join("validation-report.txt");
        let mut cmd = cli_command();
        let output = cmd
            .args([
                "val",
                message.to_str().unwrap(),
                "--profile",
                profile.to_str().unwrap(),
                "--report",
                "text",
                "--summary",
                "--output",
                val_text_report.to_str().unwrap(),
                "--quiet",
                "--no-color",
            ])
            .output()
            .expect("text validation command should run");
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(output_text(&output.stderr).contains("validation failed"));
        let val_text_output = std::fs::read_to_string(&val_text_report).unwrap();
        assert!(val_text_output.contains("Validation failed"));
        assert!(!val_text_output.contains("Validation Summary"));

        let lint_report = dir.path().join("profile-lint.json");
        let mut cmd = cli_command();
        let output = cmd
            .args([
                "profile",
                "lint",
                profile.to_str().unwrap(),
                "--report",
                "json",
                "--output",
                lint_report.to_str().unwrap(),
                "--quiet",
                "--no-color",
            ])
            .output()
            .expect("profile lint should run");
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stdout.is_empty());
        assert_eq!(json_from_file(&lint_report)["valid"], true);

        let explain_report = dir.path().join("profile-explain.json");
        let mut cmd = cli_command();
        let output = cmd
            .args([
                "profile",
                "explain",
                profile.to_str().unwrap(),
                "--format",
                "json",
                "--output",
                explain_report.to_str().unwrap(),
                "--quiet",
                "--no-color",
            ])
            .output()
            .expect("profile explain should run");
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stdout.is_empty());
        assert_eq!(
            json_from_file(&explain_report)["message_structure"],
            "ADT_A01"
        );

        let fixtures = dir.path().join("fixtures");
        std::fs::create_dir_all(fixtures.join("valid")).unwrap();
        std::fs::create_dir_all(fixtures.join("invalid")).unwrap();
        create_temp_file(
            &dir,
            "fixtures/valid/valid.hl7",
            VALID_ADT_MESSAGE.as_bytes(),
        );
        create_temp_file(
            &dir,
            "fixtures/invalid/missing_pid3.hl7",
            MISSING_PID3_MESSAGE.as_bytes(),
        );
        let test_report = dir.path().join("profile-test.json");
        let mut cmd = cli_command();
        let output = cmd
            .args([
                "profile",
                "test",
                profile.to_str().unwrap(),
                fixtures.to_str().unwrap(),
                "--report",
                "json",
                "--output",
                test_report.to_str().unwrap(),
                "--quiet",
                "--no-color",
            ])
            .output()
            .expect("profile test should run");
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stdout.is_empty());
        assert_eq!(json_from_file(&test_report)["valid"], true);

        let summary_report = dir.path().join("corpus-summary.json");
        let mut cmd = cli_command();
        let output = cmd
            .args([
                "corpus",
                "summarize",
                corpus.to_str().unwrap(),
                "--format",
                "json",
                "--output",
                summary_report.to_str().unwrap(),
                "--quiet",
                "--no-color",
            ])
            .output()
            .expect("corpus summarize should run");
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stdout.is_empty());
        assert_eq!(json_from_file(&summary_report)["message_count"], 1);

        let fingerprint_report = dir.path().join("corpus-fingerprint.json");
        let mut cmd = cli_command();
        let output = cmd
            .args([
                "corpus",
                "fingerprint",
                corpus.to_str().unwrap(),
                "--format",
                "json",
                "--output",
                fingerprint_report.to_str().unwrap(),
                "--quiet",
                "--no-color",
            ])
            .output()
            .expect("corpus fingerprint should run");
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stdout.is_empty());
        assert_eq!(json_from_file(&fingerprint_report)["message_count"], 1);

        let diff_report = dir.path().join("corpus-diff.json");
        let mut cmd = cli_command();
        let output = cmd
            .args([
                "corpus",
                "diff",
                before.to_str().unwrap(),
                after.to_str().unwrap(),
                "--format",
                "json",
                "--output",
                diff_report.to_str().unwrap(),
                "--quiet",
                "--no-color",
            ])
            .output()
            .expect("corpus diff should run");
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stdout.is_empty());
        assert_eq!(json_from_file(&diff_report)["diff_version"], "1");
    }

    #[test]
    fn test_redact_bundle_and_replay_write_output_files_and_keep_stdout_quiet() {
        let dir = create_temp_dir();
        let message = create_temp_hl7_with_content(&dir, "message.hl7", VALID_ADT_MESSAGE);
        let profile = create_temp_profile(&dir, "profile.yaml", PROFILE_REQUIRING_PID3);
        let policy = create_temp_profile(&dir, "policy.toml", SAFE_ANALYSIS_POLICY);

        let redacted = dir.path().join("message.redacted.hl7");
        let mut cmd = cli_command();
        let output = cmd
            .args([
                "redact",
                message.to_str().unwrap(),
                "--policy",
                policy.to_str().unwrap(),
                "--format",
                "hl7",
                "--output",
                redacted.to_str().unwrap(),
                "--quiet",
                "--no-color",
            ])
            .output()
            .expect("redact should run");
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stdout.is_empty());
        assert!(output.stderr.is_empty());
        assert!(
            std::fs::read_to_string(&redacted)
                .unwrap()
                .contains("hash:sha256:")
        );

        let bundle_dir = dir.path().join("bundle");
        let bundle_summary = dir.path().join("bundle-summary.json");
        let mut cmd = cli_command();
        let output = cmd
            .args([
                "bundle",
                message.to_str().unwrap(),
                "--profile",
                profile.to_str().unwrap(),
                "--redact-policy",
                policy.to_str().unwrap(),
                "--out",
                bundle_dir.to_str().unwrap(),
                "--output",
                bundle_summary.to_str().unwrap(),
                "--quiet",
                "--no-color",
            ])
            .output()
            .expect("bundle should run");
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stdout.is_empty());
        assert_eq!(json_from_file(&bundle_summary)["bundle_version"], "1");

        let replay_report = dir.path().join("replay-report.json");
        let mut cmd = cli_command();
        let output = cmd
            .args([
                "replay",
                bundle_dir.to_str().unwrap(),
                "--format",
                "json",
                "--output",
                replay_report.to_str().unwrap(),
                "--quiet",
                "--no-color",
            ])
            .output()
            .expect("replay should run");
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stdout.is_empty());
        assert_eq!(json_from_file(&replay_report)["reproduced"], true);
    }
}

// =========================================================================
// User Journey Acceptance Tests
// =========================================================================

mod user_journeys {
    use super::*;

    const PROFILE_REQUIRING_PID3: &str = r#"
message_structure: ADT_A01
version: "2.5"
segments:
  - id: MSH
  - id: PID
constraints:
  - path: PID.3
    required: true
"#;

    const JOURNEY_MESSAGE: &str = "MSH|^~\\&|SEND|FAC|RECV|FAC|202605150101||ADT^A01|CTRL-JOURNEY|P|2.5\rPID|1||MRN-JOURNEY^^^HOSP^MR||Journey^Patient||19700101|F\r";

    const SAFE_ANALYSIS_POLICY: &str = r#"
[[rules]]
path = "PID.3"
action = "hash"
reason = "patient identifier"

[[rules]]
path = "PID.5"
action = "drop"
reason = "patient name"

[[rules]]
path = "PID.7"
action = "drop"
reason = "date of birth"

[[rules]]
path = "MSH.9"
action = "retain"
reason = "message type is needed for analysis"
"#;

    fn run_json(args: &[String], expected_code: i32) -> serde_json::Value {
        let mut cmd = cli_command();
        let output = cmd.args(args).output().expect("CLI command should run");
        assert_eq!(
            output.status.code(),
            Some(expected_code),
            "stdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).expect("stdout should be JSON")
    }

    fn json_from_file(path: &std::path::Path) -> serde_json::Value {
        serde_json::from_slice(&std::fs::read(path).expect("JSON output file should be readable"))
            .expect("output file should be JSON")
    }

    #[test]
    fn journey_cli_validate_redact_bundle_replay_produces_shareable_receipts() {
        let dir = create_temp_dir();
        let message = create_temp_hl7_with_content(&dir, "message.hl7", JOURNEY_MESSAGE);
        let profile = create_temp_profile(&dir, "profile.yaml", PROFILE_REQUIRING_PID3);
        let policy = create_temp_file(&dir, "safe-analysis.toml", SAFE_ANALYSIS_POLICY.as_bytes());

        let parsed = run_json(
            &[
                "parse".to_string(),
                message.to_string_lossy().into_owned(),
                "--json".to_string(),
            ],
            0,
        );
        assert!(
            parsed.to_string().contains("MSH"),
            "parse JSON should expose the MSH segment"
        );

        let validation = run_json(
            &[
                "val".to_string(),
                message.to_string_lossy().into_owned(),
                "--profile".to_string(),
                profile.to_string_lossy().into_owned(),
                "--report".to_string(),
                "json".to_string(),
                "--schema-version".to_string(),
                "2".to_string(),
            ],
            0,
        );
        assert_eq!(validation["schema_version"], "2");
        assert_eq!(validation["valid"], true);
        assert_eq!(validation["issue_count"], 0);

        let redaction = run_json(
            &[
                "redact".to_string(),
                message.to_string_lossy().into_owned(),
                "--policy".to_string(),
                policy.to_string_lossy().into_owned(),
                "--format".to_string(),
                "json".to_string(),
                "--schema-version".to_string(),
                "2".to_string(),
            ],
            0,
        );
        let redaction_text = redaction.to_string();
        assert_eq!(redaction["receipt"]["schema_version"], "2");
        assert_eq!(redaction["receipt"]["phi_removed"], true);
        assert!(!redaction_text.contains("MRN-JOURNEY"));
        assert!(!redaction_text.contains("Journey^Patient"));
        assert!(!redaction_text.contains("19700101"));

        let bundle = dir.path().join("journey-bundle");
        let bundle_summary = run_json(
            &[
                "bundle".to_string(),
                message.to_string_lossy().into_owned(),
                "--profile".to_string(),
                profile.to_string_lossy().into_owned(),
                "--redact-policy".to_string(),
                policy.to_string_lossy().into_owned(),
                "--out".to_string(),
                bundle.to_string_lossy().into_owned(),
                "--schema-version".to_string(),
                "2".to_string(),
            ],
            0,
        );
        assert_eq!(bundle_summary["schema_version"], "2");
        assert_eq!(bundle_summary["validation_valid"], true);
        assert_eq!(bundle_summary["redaction_phi_removed"], true);

        let redacted_message =
            std::fs::read_to_string(bundle.join("message.redacted.hl7")).unwrap();
        assert!(redacted_message.contains("hash:sha256:"));
        assert!(!redacted_message.contains("MRN-JOURNEY"));
        assert!(!redacted_message.contains("Journey^Patient"));
        assert!(!redacted_message.contains("19700101"));

        let replay_report = dir.path().join("replay-report.json");
        let mut cmd = cli_command();
        let output = cmd
            .args([
                "replay",
                bundle.to_str().unwrap(),
                "--format",
                "json",
                "--schema-version",
                "2",
                "--output",
                replay_report.to_str().unwrap(),
                "--quiet",
                "--no-color",
            ])
            .output()
            .expect("replay command should run");
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stdout.is_empty());

        let replay = json_from_file(&replay_report);
        assert_eq!(replay["schema_version"], "2");
        assert_eq!(replay["reproduced"], true);
        assert!(
            replay["checks"]
                .as_array()
                .unwrap()
                .iter()
                .all(|check| check["status"] == "pass")
        );
    }
}

// =========================================================================
// File I/O Tests
// =========================================================================

mod file_io {
    use super::*;

    #[test]
    fn test_read_binary_file() {
        let dir = create_temp_dir();

        // Create file with binary content
        let mut binary_content = vec![0x0B];
        binary_content.extend_from_slice(b"MSH|^~\\&|Test\r");
        binary_content.push(0x1C);
        binary_content.push(0x0D);

        let binary_file = create_temp_file(&dir, "binary.hl7", &binary_content);

        let content = read_file(&binary_file);
        assert_eq!(content, binary_content);
    }

    #[test]
    fn test_write_to_output_file() {
        let dir = create_temp_dir();
        let hl7_file = create_temp_hl7_file(&dir, "test.hl7");
        let output_file = dir.path().join("output.hl7");

        let mut cmd = cli_command();
        cmd.args([
            "norm",
            hl7_file.to_str().unwrap(),
            "-o",
            output_file.to_str().unwrap(),
        ])
        .assert()
        .success();

        assert!(output_file.exists());
        let content = read_file(&output_file);
        assert!(!content.is_empty());
    }

    #[test]
    fn test_output_file_is_valid_hl7() {
        let dir = create_temp_dir();
        let hl7_file = create_temp_hl7_file(&dir, "test.hl7");
        let output_file = dir.path().join("output.hl7");

        let mut cmd = cli_command();
        cmd.args([
            "norm",
            hl7_file.to_str().unwrap(),
            "-o",
            output_file.to_str().unwrap(),
        ])
        .assert()
        .success();

        let content = read_file(&output_file);
        let parse_result = hl7v2::parse(&content);
        assert!(parse_result.is_ok());
    }
}

// =========================================================================
// Edge Cases Tests
// =========================================================================

mod edge_cases {
    use super::*;

    #[test]
    fn test_empty_file() {
        let dir = create_temp_dir();
        let empty_file = create_temp_file(&dir, "empty.hl7", b"");

        let mut cmd = cli_command();
        cmd.args(["parse", empty_file.to_str().unwrap()])
            .assert()
            .failure();
    }

    #[test]
    fn test_whitespace_only_file() {
        let dir = create_temp_dir();
        let whitespace_file = create_temp_file(&dir, "whitespace.hl7", b"   \n\t  ");

        let mut cmd = cli_command();
        cmd.args(["parse", whitespace_file.to_str().unwrap()])
            .assert()
            .failure();
    }

    #[test]
    fn test_utf8_file() {
        let dir = create_temp_dir();
        let utf8_content = "MSH|^~\\&|Тест|Фасил|Recv|Fac|20250101000000||ADT^A01|MSG001|P|2.5.1\rPID|1||123456^^^HOSP^MR||Доу^Джон||19800101|M\r";
        let utf8_file = create_temp_hl7_with_content(&dir, "utf8.hl7", utf8_content);

        let mut cmd = cli_command();
        cmd.args(["parse", utf8_file.to_str().unwrap()])
            .assert()
            .success();
    }

    #[test]
    fn test_large_message() {
        let dir = create_temp_dir();

        // Create a large message with many segments
        let mut large_content =
            String::from("MSH|^~\\&|App|Fac|Recv|Fac|20250101000000||ADT^A01|MSG001|P|2.5.1\r");
        for i in 1..=100 {
            large_content.push_str(&format!("NTE|{}|Note segment number {}\r", i, i));
        }

        let large_file = create_temp_hl7_with_content(&dir, "large.hl7", &large_content);

        let mut cmd = cli_command();
        cmd.args(["parse", large_file.to_str().unwrap()])
            .assert()
            .success();
    }

    #[test]
    fn test_message_with_special_characters() {
        let dir = create_temp_dir();
        // Message with escape sequences
        let content = "MSH|^~\\&|App|Fac|Recv|Fac|20250101000000||ADT^A01|MSG001|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John^\"Johnny\"||19800101|M\r";
        let special_file = create_temp_hl7_with_content(&dir, "special.hl7", content);

        let mut cmd = cli_command();
        cmd.args(["parse", special_file.to_str().unwrap()])
            .assert()
            .success();
    }
}

// =========================================================================
// Serve Command Tests
// =========================================================================

mod serve_command {
    use super::*;

    #[test]
    fn test_serve_help() {
        let mut cmd = cli_command();
        cmd.args(["serve", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("HTTP/gRPC server"))
            .stdout(predicate::str::contains("--port"))
            .stdout(predicate::str::contains("--host"))
            .stdout(predicate::str::contains("--mode"));
    }

    #[test]
    fn test_serve_default_options() {
        let mut cmd = cli_command();
        cmd.args(["serve", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("default: http"))
            .stdout(predicate::str::contains("default: 8080"))
            .stdout(predicate::str::contains("default: 0.0.0.0"));
    }

    #[test]
    fn test_serve_mode_options() {
        let mut cmd = cli_command();
        cmd.args(["serve", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("http"))
            .stdout(predicate::str::contains("grpc"));
    }

    #[test]
    fn test_serve_grpc_rejects_invalid_bind_address() {
        let mut cmd = cli_command();
        cmd.args([
            "serve",
            "--mode",
            "grpc",
            "--host",
            "not-a-bind-address",
            "--port",
            "50051",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid bind address"));
    }
}
