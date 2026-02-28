//! CLI integration tests for hl7v2-cli.
//!
//! These tests validate the command-line interface by:
//! - Running the CLI binary as a subprocess
//! - Testing parse, validate, and generate commands
//! - Verifying output formats and error handling

use assert_cmd::Command;
use predicates::prelude::*;
use std::io::Write;
use tempfile::TempDir;

use super::common::init_tracing;

// =========================================================================
// Helper Functions
// =========================================================================

/// Create a CLI command
fn cli() -> Command {
    Command::cargo_bin("hl7v2-cli").expect("Failed to find hl7v2-cli binary")
}

/// Create a temporary HL7 file
fn create_hl7_file(dir: &TempDir, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    let mut file = std::fs::File::create(&path).expect("Failed to create file");
    file.write_all(content.as_bytes())
        .expect("Failed to write file");
    path
}

/// Create a temporary profile YAML file
fn create_profile_file(dir: &TempDir, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    let mut file = std::fs::File::create(&path).expect("Failed to create profile file");
    file.write_all(content.as_bytes())
        .expect("Failed to write profile file");
    path
}

/// Sample ADT^A01 message
fn sample_adt_a01() -> String {
    concat!(
        "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|",
        "20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\r",
        "EVN|A01|20250128152312|||\r",
        "PID|1||123456^^^HOSP^MR||Doe^John^A||19800101|M|||C|\r",
        "PV1|1|I|ICU^101^01||||DOC123^Smith^Jane||||||||V123456\r"
    )
    .to_string()
}

/// Sample ADT^A04 message
fn sample_adt_a04() -> String {
    concat!(
        "MSH|^~\\&|RegSys|Hospital|ADT|Hospital|",
        "20250128140000||ADT^A04|MSG002|P|2.5\r",
        "PID|1||MRN456^^^Hospital^MR||Smith^Jane^M||19900215|F\r"
    )
    .to_string()
}

/// Sample ORU^R01 message
fn sample_oru_r01() -> String {
    concat!(
        "MSH|^~\\&|LabSys|Lab|LIS|Hospital|",
        "20250128150000||ORU^R01|MSG003|P|2.5\r",
        "PID|1||MRN789^^^Lab^MR||Patient^Test||19850610|M\r",
        "OBR|1|ORD123|FIL456|CBC^Complete Blood Count|||20250128120000\r",
        "OBX|1|NM|WBC^White Blood Count||7.5|10^9/L|4.0-11.0|N|||F\r"
    )
    .to_string()
}

/// Basic validation profile
fn basic_profile() -> String {
    r#"
name: Basic_Profile
version: "1.0"
message_types:
  - type: ADT
    trigger: A01
    required_segments:
      - MSH
      - PID
      - PV1
  - type: ADT
    trigger: A04
    required_segments:
      - MSH
      - PID
  - type: ORU
    trigger: R01
    required_segments:
      - MSH
      - PID
      - OBR
"#
    .to_string()
}

// =========================================================================
// Parse Command Tests
// =========================================================================

mod parse_command {
    use super::*;

    #[test]
    fn test_parse_valid_adt_a01() {
        init_tracing();

        let dir = TempDir::new().expect("Failed to create temp dir");
        let hl7_file = create_hl7_file(&dir, "adt_a01.hl7", &sample_adt_a01());

        let mut cmd = cli();
        cmd.arg("parse")
            .arg(&hl7_file)
            .assert()
            .success()
            .stdout(predicate::str::contains("MSH"));
    }

    #[test]
    fn test_parse_valid_adt_a04() {
        init_tracing();

        let dir = TempDir::new().expect("Failed to create temp dir");
        let hl7_file = create_hl7_file(&dir, "adt_a04.hl7", &sample_adt_a04());

        let mut cmd = cli();
        cmd.arg("parse").arg(&hl7_file).assert().success();
    }

    #[test]
    fn test_parse_valid_oru_r01() {
        init_tracing();

        let dir = TempDir::new().expect("Failed to create temp dir");
        let hl7_file = create_hl7_file(&dir, "oru_r01.hl7", &sample_oru_r01());

        let mut cmd = cli();
        cmd.arg("parse")
            .arg(&hl7_file)
            .assert()
            .success()
            .stdout(predicate::str::contains("OBX"));
    }

    #[test]
    fn test_parse_json_output() {
        init_tracing();

        let dir = TempDir::new().expect("Failed to create temp dir");
        let hl7_file = create_hl7_file(&dir, "message.hl7", &sample_adt_a01());

        let mut cmd = cli();
        cmd.arg("parse")
            .arg(&hl7_file)
            .arg("--json")
            .assert()
            .success()
            .stdout(predicate::str::contains("{"));
    }

    #[test]
    fn test_parse_summary_flag() {
        init_tracing();

        let dir = TempDir::new().expect("Failed to create temp dir");
        let hl7_file = create_hl7_file(&dir, "message.hl7", &sample_adt_a01());

        let mut cmd = cli();
        cmd.arg("parse")
            .arg(&hl7_file)
            .arg("--summary")
            .assert()
            .success();
    }

    #[test]
    fn test_parse_nonexistent_file() {
        init_tracing();

        let mut cmd = cli();
        cmd.arg("parse")
            .arg("/nonexistent/path/file.hl7")
            .assert()
            .failure();
    }

    #[test]
    fn test_parse_invalid_message() {
        init_tracing();

        let dir = TempDir::new().expect("Failed to create temp dir");
        let hl7_file = create_hl7_file(&dir, "invalid.hl7", "This is not a valid HL7 message");

        let mut cmd = cli();
        cmd.arg("parse").arg(&hl7_file).assert().failure();
    }
}

// =========================================================================
// Normalize Command Tests
// =========================================================================

mod normalize_command {
    use super::*;

    #[test]
    fn test_norm_basic() {
        init_tracing();

        let dir = TempDir::new().expect("Failed to create temp dir");
        let hl7_file = create_hl7_file(&dir, "message.hl7", &sample_adt_a01());

        let mut cmd = cli();
        cmd.arg("norm").arg(&hl7_file).assert().success();
    }

    #[test]
    fn test_norm_with_canonical_delims() {
        init_tracing();

        let dir = TempDir::new().expect("Failed to create temp dir");
        let hl7_file = create_hl7_file(&dir, "message.hl7", &sample_adt_a01());

        let mut cmd = cli();
        cmd.arg("norm")
            .arg(&hl7_file)
            .arg("--canonical-delims")
            .assert()
            .success();
    }

    #[test]
    fn test_norm_with_output_file() {
        init_tracing();

        let dir = TempDir::new().expect("Failed to create temp dir");
        let input_file = create_hl7_file(&dir, "input.hl7", &sample_adt_a01());
        let output_file = dir.path().join("output.hl7");

        let mut cmd = cli();
        cmd.arg("norm")
            .arg(&input_file)
            .arg("--output")
            .arg(&output_file)
            .assert()
            .success();

        // Verify output file was created
        assert!(output_file.exists(), "Output file should be created");
    }

    #[test]
    fn test_norm_with_summary() {
        init_tracing();

        let dir = TempDir::new().expect("Failed to create temp dir");
        let hl7_file = create_hl7_file(&dir, "message.hl7", &sample_adt_a01());

        let mut cmd = cli();
        cmd.arg("norm")
            .arg(&hl7_file)
            .arg("--summary")
            .assert()
            .success();
    }
}

// =========================================================================
// Validate Command Tests
// =========================================================================

mod validate_command {
    use super::*;

    #[test]
    #[ignore = "Profile format needs to be aligned with CLI implementation"]
    fn test_val_valid_adt_a01() {
        init_tracing();

        let dir = TempDir::new().expect("Failed to create temp dir");
        let hl7_file = create_hl7_file(&dir, "adt_a01.hl7", &sample_adt_a01());
        let profile_file = create_profile_file(&dir, "profile.yaml", &basic_profile());

        let mut cmd = cli();
        cmd.arg("val")
            .arg(&hl7_file)
            .arg("--profile")
            .arg(&profile_file)
            .assert()
            .success();
    }

    #[test]
    #[ignore = "Profile format needs to be aligned with CLI implementation"]
    fn test_val_valid_oru_r01() {
        init_tracing();

        let dir = TempDir::new().expect("Failed to create temp dir");
        let hl7_file = create_hl7_file(&dir, "oru_r01.hl7", &sample_oru_r01());
        let profile_file = create_profile_file(&dir, "profile.yaml", &basic_profile());

        let mut cmd = cli();
        cmd.arg("val")
            .arg(&hl7_file)
            .arg("--profile")
            .arg(&profile_file)
            .assert()
            .success();
    }

    #[test]
    #[ignore = "Profile format needs to be aligned with CLI implementation"]
    fn test_val_with_detailed_output() {
        init_tracing();

        let dir = TempDir::new().expect("Failed to create temp dir");
        let hl7_file = create_hl7_file(&dir, "message.hl7", &sample_adt_a01());
        let profile_file = create_profile_file(&dir, "profile.yaml", &basic_profile());

        let mut cmd = cli();
        cmd.arg("val")
            .arg(&hl7_file)
            .arg("--profile")
            .arg(&profile_file)
            .arg("--detailed")
            .assert()
            .success();
    }

    #[test]
    fn test_val_missing_profile() {
        init_tracing();

        let dir = TempDir::new().expect("Failed to create temp dir");
        let hl7_file = create_hl7_file(&dir, "message.hl7", &sample_adt_a01());

        let mut cmd = cli();
        cmd.arg("val").arg(&hl7_file).assert().failure(); // Profile is required
    }

    #[test]
    fn test_val_invalid_profile_path() {
        init_tracing();

        let dir = TempDir::new().expect("Failed to create temp dir");
        let hl7_file = create_hl7_file(&dir, "message.hl7", &sample_adt_a01());

        let mut cmd = cli();
        cmd.arg("val")
            .arg(&hl7_file)
            .arg("--profile")
            .arg("/nonexistent/profile.yaml")
            .assert()
            .failure();
    }
}

// =========================================================================
// ACK Generation Command Tests
// =========================================================================

mod ack_command {
    use super::*;

    #[test]
    #[ignore = "Escape sequence handling in test messages needs alignment"]
    fn test_ack_accept() {
        init_tracing();

        let dir = TempDir::new().expect("Failed to create temp dir");
        let hl7_file = create_hl7_file(&dir, "message.hl7", &sample_adt_a01());

        let mut cmd = cli();
        cmd.arg("ack")
            .arg(&hl7_file)
            .arg("--mode")
            .arg("original")
            .arg("--code")
            .arg("AA")
            .assert()
            .success()
            .stdout(predicate::str::contains("MSA|AA"));
    }

    #[test]
    #[ignore = "Escape sequence handling in test messages needs alignment"]
    fn test_ack_error() {
        init_tracing();

        let dir = TempDir::new().expect("Failed to create temp dir");
        let hl7_file = create_hl7_file(&dir, "message.hl7", &sample_adt_a01());

        let mut cmd = cli();
        cmd.arg("ack")
            .arg(&hl7_file)
            .arg("--mode")
            .arg("original")
            .arg("--code")
            .arg("AE")
            .assert()
            .success()
            .stdout(predicate::str::contains("MSA|AE"));
    }

    #[test]
    #[ignore = "Escape sequence handling in test messages needs alignment"]
    fn test_ack_reject() {
        init_tracing();

        let dir = TempDir::new().expect("Failed to create temp dir");
        let hl7_file = create_hl7_file(&dir, "message.hl7", &sample_adt_a01());

        let mut cmd = cli();
        cmd.arg("ack")
            .arg(&hl7_file)
            .arg("--mode")
            .arg("original")
            .arg("--code")
            .arg("AR")
            .assert()
            .success()
            .stdout(predicate::str::contains("MSA|AR"));
    }

    #[test]
    #[ignore = "Escape sequence handling in test messages needs alignment"]
    fn test_ack_with_summary() {
        init_tracing();

        let dir = TempDir::new().expect("Failed to create temp dir");
        let hl7_file = create_hl7_file(&dir, "message.hl7", &sample_adt_a01());

        let mut cmd = cli();
        cmd.arg("ack")
            .arg(&hl7_file)
            .arg("--mode")
            .arg("original")
            .arg("--code")
            .arg("AA")
            .arg("--summary")
            .assert()
            .success();
    }
}

// =========================================================================
// Generate Command Tests
// =========================================================================

mod generate_command {
    use super::*;

    #[test]
    #[ignore = "Profile format needs to be aligned with CLI implementation"]
    fn test_gen_basic() {
        init_tracing();

        let dir = TempDir::new().expect("Failed to create temp dir");
        let profile_file = create_profile_file(&dir, "profile.yaml", &basic_profile());
        let output_dir = dir.path().join("output");
        std::fs::create_dir(&output_dir).expect("Failed to create output dir");

        let mut cmd = cli();
        cmd.arg("gen")
            .arg("--profile")
            .arg(&profile_file)
            .arg("--seed")
            .arg("42")
            .arg("--count")
            .arg("5")
            .arg("--out")
            .arg(&output_dir)
            .assert()
            .success();
    }

    #[test]
    #[ignore = "Profile format needs to be aligned with CLI implementation"]
    fn test_gen_with_stats() {
        init_tracing();

        let dir = TempDir::new().expect("Failed to create temp dir");
        let profile_file = create_profile_file(&dir, "profile.yaml", &basic_profile());
        let output_dir = dir.path().join("output");
        std::fs::create_dir(&output_dir).expect("Failed to create output dir");

        let mut cmd = cli();
        cmd.arg("gen")
            .arg("--profile")
            .arg(&profile_file)
            .arg("--seed")
            .arg("12345")
            .arg("--count")
            .arg("10")
            .arg("--out")
            .arg(&output_dir)
            .arg("--stats")
            .assert()
            .success();
    }

    #[test]
    fn test_gen_missing_profile() {
        init_tracing();

        let dir = TempDir::new().expect("Failed to create temp dir");
        let output_dir = dir.path().join("output");
        std::fs::create_dir(&output_dir).expect("Failed to create output dir");

        let mut cmd = cli();
        cmd.arg("gen")
            .arg("--seed")
            .arg("42")
            .arg("--count")
            .arg("5")
            .arg("--out")
            .arg(&output_dir)
            .assert()
            .failure(); // Profile is required
    }
}

// =========================================================================
// Help and Version Tests
// =========================================================================

mod help_tests {
    use super::*;

    #[test]
    fn test_help() {
        let mut cmd = cli();
        cmd.arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("HL7 v2 parser"));
    }

    #[test]
    fn test_parse_help() {
        let mut cmd = cli();
        cmd.args(["parse", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Parse HL7"));
    }

    #[test]
    fn test_norm_help() {
        let mut cmd = cli();
        cmd.args(["norm", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Normalize"));
    }

    #[test]
    fn test_val_help() {
        let mut cmd = cli();
        cmd.args(["val", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Validate"));
    }

    #[test]
    fn test_ack_help() {
        let mut cmd = cli();
        cmd.args(["ack", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Generate ACK"));
    }

    #[test]
    fn test_gen_help() {
        let mut cmd = cli();
        cmd.args(["gen", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Generate synthetic"));
    }
}

// =========================================================================
// Edge Cases and Error Handling Tests
// =========================================================================

mod edge_cases {
    use super::*;

    #[test]
    fn test_parse_empty_file() {
        init_tracing();

        let dir = TempDir::new().expect("Failed to create temp dir");
        let hl7_file = create_hl7_file(&dir, "empty.hl7", "");

        let mut cmd = cli();
        cmd.arg("parse").arg(&hl7_file).assert().failure();
    }

    #[test]
    fn test_parse_mllp_framed_input() {
        init_tracing();

        let dir = TempDir::new().expect("Failed to create temp dir");

        // Create MLLP-framed message
        let message = sample_adt_a01();
        let mut framed = Vec::new();
        framed.push(0x0B); // SB
        framed.extend_from_slice(message.as_bytes());
        framed.push(0x1C); // EB
        framed.push(0x0D); // CR

        let hl7_file = create_hl7_file(&dir, "mllp.hl7", &String::from_utf8_lossy(&framed));

        let mut cmd = cli();
        cmd.arg("parse")
            .arg(&hl7_file)
            .arg("--mllp")
            .assert()
            .success();
    }

    #[test]
    fn test_norm_mllp_output() {
        init_tracing();

        let dir = TempDir::new().expect("Failed to create temp dir");
        let hl7_file = create_hl7_file(&dir, "message.hl7", &sample_adt_a01());

        let mut cmd = cli();
        cmd.arg("norm")
            .arg(&hl7_file)
            .arg("--mllp-out")
            .assert()
            .success();
    }

    #[test]
    #[ignore = "Profile format needs to be aligned with CLI implementation"]
    fn test_val_with_mllp_input() {
        init_tracing();

        let dir = TempDir::new().expect("Failed to create temp dir");

        // Create MLLP-framed message
        let message = sample_adt_a01();
        let mut framed = Vec::new();
        framed.push(0x0B);
        framed.extend_from_slice(message.as_bytes());
        framed.push(0x1C);
        framed.push(0x0D);

        let hl7_file = create_hl7_file(&dir, "mllp.hl7", &String::from_utf8_lossy(&framed));
        let profile_file = create_profile_file(&dir, "profile.yaml", &basic_profile());

        let mut cmd = cli();
        cmd.arg("val")
            .arg(&hl7_file)
            .arg("--profile")
            .arg(&profile_file)
            .arg("--mllp")
            .assert()
            .success();
    }

    #[test]
    #[ignore = "Escape sequence handling in test messages needs alignment"]
    fn test_ack_mllp_output() {
        init_tracing();

        let dir = TempDir::new().expect("Failed to create temp dir");
        let hl7_file = create_hl7_file(&dir, "message.hl7", &sample_adt_a01());

        let mut cmd = cli();
        cmd.arg("ack")
            .arg(&hl7_file)
            .arg("--mode")
            .arg("original")
            .arg("--code")
            .arg("AA")
            .arg("--mllp-out")
            .assert()
            .success();
    }
}

// =========================================================================
// Integration Tests - Full Workflows
// =========================================================================

mod workflows {
    use super::*;

    #[test]
    #[ignore = "Profile format and escape sequence handling needs alignment"]
    fn test_full_workflow_parse_validate_ack() {
        init_tracing();

        let dir = TempDir::new().expect("Failed to create temp dir");
        let hl7_file = create_hl7_file(&dir, "message.hl7", &sample_adt_a01());
        let profile_file = create_profile_file(&dir, "profile.yaml", &basic_profile());

        // Step 1: Parse
        let mut cmd = cli();
        let parse_result = cmd
            .arg("parse")
            .arg(&hl7_file)
            .arg("--json")
            .assert()
            .success();

        // Verify JSON output
        let output = String::from_utf8_lossy(&parse_result.get_output().stdout);
        assert!(output.contains("{"));

        // Step 2: Validate
        let mut cmd = cli();
        cmd.arg("val")
            .arg(&hl7_file)
            .arg("--profile")
            .arg(&profile_file)
            .assert()
            .success();

        // Step 3: Generate ACK
        let mut cmd = cli();
        let ack_result = cmd
            .arg("ack")
            .arg(&hl7_file)
            .arg("--mode")
            .arg("original")
            .arg("--code")
            .arg("AA")
            .assert()
            .success();

        // Verify ACK output
        let ack_output = String::from_utf8_lossy(&ack_result.get_output().stdout);
        assert!(ack_output.contains("MSA|AA"));
    }

    #[test]
    #[ignore = "Profile format needs to be aligned with CLI implementation"]
    fn test_workflow_normalize_and_validate() {
        init_tracing();

        let dir = TempDir::new().expect("Failed to create temp dir");

        // Message with non-standard delimiters
        let custom_message = concat!(
            "MSH#$*@!SendingApp#SendingFac#ReceivingApp#ReceivingFac#",
            "20250128152312##ADT$A01#ABC123#P#2.5.1\r",
            "PID#1##123456^^^HOSP^MR##Doe$John$A##19800101#M###C#\r"
        );

        let input_file = create_hl7_file(&dir, "custom.hl7", custom_message);
        let normalized_file = dir.path().join("normalized.hl7");
        let profile_file = create_profile_file(&dir, "profile.yaml", &basic_profile());

        // Normalize with canonical delimiters
        let mut cmd = cli();
        cmd.arg("norm")
            .arg(&input_file)
            .arg("--canonical-delims")
            .arg("--output")
            .arg(&normalized_file)
            .assert()
            .success();

        // Validate the normalized message
        let mut cmd = cli();
        cmd.arg("val")
            .arg(&normalized_file)
            .arg("--profile")
            .arg(&profile_file)
            .assert()
            .success();
    }
}
