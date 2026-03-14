//! BDD tests for hl7v2-cli using Cucumber
//!
//! Run with: cargo test --test bdd_tests
//!
//! This test file implements the step definitions for the CLI feature file.

use cucumber::{World, given, then, when};
use hl7v2_test_utils::fixtures::SampleMessages;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Test world for CLI BDD tests
#[derive(Debug, World)]
#[world(init = Self::new)]
pub struct CliWorld {
    /// Temporary directory for test files
    temp_dir: Option<TempDir>,
    /// Input file path
    input_file: Option<PathBuf>,
    /// Output file path
    output_file: Option<PathBuf>,
    /// Profile file path
    profile_file: Option<PathBuf>,
    /// Template file path
    template_file: Option<PathBuf>,
    /// Output directory for generated files
    output_dir: Option<PathBuf>,
    /// Command result
    result: Option<Result<std::process::Output, std::io::Error>>,
    /// Current command arguments
    command_args: Vec<String>,
    /// File content for testing
    #[allow(dead_code)]
    file_content: Option<String>,
}

impl CliWorld {
    fn new() -> Self {
        Self {
            temp_dir: None,
            input_file: None,
            output_file: None,
            profile_file: None,
            template_file: None,
            output_dir: None,
            result: None,
            command_args: Vec::new(),
            file_content: None,
        }
    }

    /// Ensure temp directory exists
    fn ensure_temp_dir(&mut self) -> &TempDir {
        if self.temp_dir.is_none() {
            self.temp_dir = Some(TempDir::new().expect("Failed to create temp dir"));
        }
        self.temp_dir.as_ref().unwrap()
    }

    /// Run the CLI command with current arguments
    fn run_cli(&mut self) {
        // Binary is in target/debug/ not target/debug/deps/
        let binary_path = std::env::current_exe()
            .expect("Failed to get current exe path")
            .parent()
            .expect("Failed to get parent dir")
            .parent()
            .expect("Failed to get parent dir")
            .join("hl7v2-cli");

        let mut cmd = Command::new(&binary_path);
        for arg in &self.command_args.clone() {
            cmd.arg(arg);
        }

        self.result = Some(cmd.output());
    }

    /// Check if the command succeeded
    fn command_succeeded(&self) -> bool {
        if let Some(Ok(output)) = &self.result {
            output.status.success()
        } else {
            false
        }
    }

    /// Get stdout as string
    fn stdout(&self) -> String {
        if let Some(Ok(output)) = &self.result {
            String::from_utf8_lossy(&output.stdout).to_string()
        } else {
            String::new()
        }
    }

    /// Get stderr as string
    fn stderr(&self) -> String {
        if let Some(Ok(output)) = &self.result {
            String::from_utf8_lossy(&output.stderr).to_string()
        } else {
            String::new()
        }
    }

    /// Get exit code
    fn exit_code(&self) -> Option<i32> {
        if let Some(Ok(output)) = &self.result {
            output.status.code()
        } else {
            None
        }
    }
}

// ============================================================================
// Background Steps
// ============================================================================

#[given("the hl7v2-cli binary is available")]
fn given_cli_available(_world: &mut CliWorld) {
    // Just ensure we can find the binary
    // Binary is in target/debug/ not target/debug/deps/
    let binary_path = std::env::current_exe()
        .expect("Failed to get current exe path")
        .parent()
        .expect("Failed to get parent dir")
        .parent()
        .expect("Failed to get parent dir")
        .join("hl7v2-cli");

    assert!(
        binary_path.exists() || binary_path.with_extension("exe").exists(),
        "CLI binary should exist at {:?}",
        binary_path
    );
}

// ============================================================================
// File Setup Steps
// ============================================================================

#[given("a valid HL7 ADT^A01 message file")]
fn given_valid_adt_file(world: &mut CliWorld) {
    let dir = world.ensure_temp_dir();
    let path = dir.path().join("test.hl7");
    std::fs::write(&path, SampleMessages::adt_a01().as_bytes()).expect("Failed to write file");
    world.input_file = Some(path);
}

#[given("an MLLP-framed HL7 message file")]
fn given_mllp_file(world: &mut CliWorld) {
    let dir = world.ensure_temp_dir();
    let path = dir.path().join("test_mllp.hl7");

    // Create MLLP-framed content
    let content = SampleMessages::adt_a01();
    let mut mllp_bytes = vec![0x0B]; // SB
    mllp_bytes.extend_from_slice(content.as_bytes());
    mllp_bytes.push(0x1C); // EB
    mllp_bytes.push(0x0D); // CR

    std::fs::write(&path, &mllp_bytes).expect("Failed to write file");
    world.input_file = Some(path);
}

#[given("a non-existent file path")]
fn given_nonexistent_file(world: &mut CliWorld) {
    world.input_file = Some(PathBuf::from("/nonexistent/path/file.hl7"));
}

#[given("a file with invalid HL7 content")]
fn given_invalid_file(world: &mut CliWorld) {
    let dir = world.ensure_temp_dir();
    let path = dir.path().join("invalid.hl7");
    std::fs::write(&path, b"This is not valid HL7 content").expect("Failed to write file");
    world.input_file = Some(path);
}

#[given("an empty file")]
fn given_empty_file(world: &mut CliWorld) {
    let dir = world.ensure_temp_dir();
    let path = dir.path().join("empty.hl7");
    std::fs::write(&path, b"").expect("Failed to write file");
    world.input_file = Some(path);
}

#[given("a file with UTF-8 HL7 content")]
fn given_utf8_file(world: &mut CliWorld) {
    let dir = world.ensure_temp_dir();
    let path = dir.path().join("utf8.hl7");
    let content = "MSH|^~\\&|Тест|Фасил|Recv|Fac|20250101000000||ADT^A01|MSG001|P|2.5.1\rPID|1||123456^^^HOSP^MR||Доу^Джон||19800101|M\r";
    std::fs::write(&path, content.as_bytes()).expect("Failed to write file");
    world.input_file = Some(path);
}

#[given("a file with a large HL7 message")]
fn given_large_file(world: &mut CliWorld) {
    let dir = world.ensure_temp_dir();
    let path = dir.path().join("large.hl7");

    let mut content =
        String::from("MSH|^~\\&|App|Fac|Recv|Fac|20250101000000||ADT^A01|MSG001|P|2.5.1\r");
    for i in 1..=100 {
        content.push_str(&format!("NTE|{}|Note segment number {}\r", i, i));
    }

    std::fs::write(&path, content.as_bytes()).expect("Failed to write file");
    world.input_file = Some(path);
}

#[given(regex = r"^an output file path$")]
fn given_output_path(world: &mut CliWorld) {
    let dir = world.ensure_temp_dir();
    world.output_file = Some(dir.path().join("output.hl7"));
}

#[given("a minimal validation profile")]
fn given_minimal_profile(world: &mut CliWorld) {
    let dir = world.ensure_temp_dir();
    let path = dir.path().join("profile.yaml");
    // Profile format expected by hl7v2_prof::Profile struct:
    // - message_structure: message type
    // - version: HL7 version
    // - segments: list of segment specs with id
    let profile = r#"message_structure: ADT_A01
version: "2.5.1"
segments:
  - id: MSH
"#;
    std::fs::write(&path, profile.as_bytes()).expect("Failed to write file");
    world.profile_file = Some(path);
}

#[given("a strict validation profile requiring ZZ1 segment")]
fn given_strict_profile(world: &mut CliWorld) {
    let dir = world.ensure_temp_dir();
    let path = dir.path().join("strict_profile.yaml");
    let profile = r#"message_structure: ADT_A01
version: "2.5.1"
segments:
  - id: MSH
  - id: ZZ1
"#;
    std::fs::write(&path, profile.as_bytes()).expect("Failed to write file");
    world.profile_file = Some(path);
}

#[given("a valid template YAML file")]
fn given_template_file(world: &mut CliWorld) {
    let dir = world.ensure_temp_dir();
    let path = dir.path().join("template.yaml");
    // Template format expected by hl7v2_template::Template struct:
    // - name: template name
    // - delims: 4-character string for component, repetition, escape, subcomponent delimiters
    // - segments: list of segment template strings
    // - values: optional map of field paths to value sources
    let template = r#"name: "ADT_A01 Template"
delims: "^~\\&"
segments:
  - "MSH|^~\\&|TestApp|TestFac|RecvApp|RecvFac|20250101000000||ADT^A01|MSG001|P|2.5.1"
  - "PID|1||12345^^^HOSP^MR||Doe^John||19800101|M"
"#;
    std::fs::write(&path, template.as_bytes()).expect("Failed to write file");
    world.template_file = Some(path);
}

#[given("an output directory")]
fn given_output_directory(world: &mut CliWorld) {
    let dir = world.ensure_temp_dir();
    world.output_dir = Some(dir.path().join("generated"));
}

// ============================================================================
// Command Execution Steps
// ============================================================================

#[when(regex = r#"^I run "([^"]+)" with the file path$"#)]
fn when_run_with_file(world: &mut CliWorld, command: String) {
    let input = world.input_file.as_ref().expect("Input file should be set");
    world.command_args = vec![command, input.to_string_lossy().to_string()];
    world.run_cli();
}

#[when(regex = r#"^I run "([^"]+)" with the file path and "([^"]+)" flag$"#)]
fn when_run_with_flag(world: &mut CliWorld, command: String, flag: String) {
    let input = world.input_file.as_ref().expect("Input file should be set");
    world.command_args = vec![command, input.to_string_lossy().to_string(), flag];
    world.run_cli();
}

#[when(regex = r#"^I run "([^"]+)" with input and output paths$"#)]
fn when_run_with_io(world: &mut CliWorld, command: String) {
    let input = world.input_file.as_ref().expect("Input file should be set");
    let output = world
        .output_file
        .as_ref()
        .expect("Output file should be set");
    world.command_args = vec![
        command,
        input.to_string_lossy().to_string(),
        "-o".to_string(),
        output.to_string_lossy().to_string(),
    ];
    world.run_cli();
}

#[when(regex = r#"^I run "([^"]+)" with input, output paths and "([^"]+)" flag$"#)]
fn when_run_with_io_and_flag(world: &mut CliWorld, command: String, flag: String) {
    let input = world.input_file.as_ref().expect("Input file should be set");
    let output = world
        .output_file
        .as_ref()
        .expect("Output file should be set");
    world.command_args = vec![
        command,
        input.to_string_lossy().to_string(),
        "-o".to_string(),
        output.to_string_lossy().to_string(),
        flag,
    ];
    world.run_cli();
}

#[when(regex = r#"^I run "([^"]+)" with the file and profile paths$"#)]
fn when_run_validate(world: &mut CliWorld, command: String) {
    let input = world.input_file.as_ref().expect("Input file should be set");
    let profile = world
        .profile_file
        .as_ref()
        .expect("Profile file should be set");
    world.command_args = vec![
        command,
        input.to_string_lossy().to_string(),
        "--profile".to_string(),
        profile.to_string_lossy().to_string(),
    ];
    world.run_cli();
}

#[when(regex = r#"^I run "val" with "([^"]+)" flag$"#)]
fn when_run_val_with_flag(world: &mut CliWorld, flag: String) {
    let input = world.input_file.as_ref().expect("Input file should be set");
    let profile = world
        .profile_file
        .as_ref()
        .expect("Profile file should be set");
    world.command_args = vec![
        "val".to_string(),
        input.to_string_lossy().to_string(),
        "--profile".to_string(),
        profile.to_string_lossy().to_string(),
        flag,
    ];
    world.run_cli();
}

#[when(regex = r#"^I run "([^"]+)" with the file path and code "([^"]+)"$"#)]
fn when_run_ack(world: &mut CliWorld, command: String, code: String) {
    let input = world.input_file.as_ref().expect("Input file should be set");
    world.command_args = vec![
        command,
        input.to_string_lossy().to_string(),
        "--mode".to_string(),
        "original".to_string(),
        "--code".to_string(),
        code,
    ];
    world.run_cli();
}

#[when(regex = r#"^I run "([^"]+)" with "([^"]+)" flag and code "([^"]+)"$"#)]
fn when_run_ack_with_flag(world: &mut CliWorld, command: String, flag: String, code: String) {
    let input = world.input_file.as_ref().expect("Input file should be set");
    world.command_args = vec![
        command,
        input.to_string_lossy().to_string(),
        "--mode".to_string(),
        "original".to_string(),
        flag,
        "--code".to_string(),
        code,
    ];
    world.run_cli();
}

#[when(regex = r#"^I run "([^"]+)" with "--mode ([^"]+)" and code "([^"]+)"$"#)]
fn when_run_ack_with_mode(world: &mut CliWorld, command: String, mode: String, code: String) {
    let input = world.input_file.as_ref().expect("Input file should be set");
    world.command_args = vec![
        command,
        input.to_string_lossy().to_string(),
        "--mode".to_string(),
        mode,
        "--code".to_string(),
        code,
    ];
    world.run_cli();
}

#[when(regex = r#"^I run "([^"]+)" with invalid code "([^"]+)"$"#)]
fn when_run_ack_invalid_code(world: &mut CliWorld, command: String, code: String) {
    let input = world.input_file.as_ref().expect("Input file should be set");
    world.command_args = vec![
        command,
        input.to_string_lossy().to_string(),
        "--mode".to_string(),
        "original".to_string(),
        "--code".to_string(),
        code,
    ];
    world.run_cli();
}

#[when(regex = r#"^I run "([^"]+)" with the template and output paths$"#)]
fn when_run_gen(world: &mut CliWorld, command: String) {
    let template = world
        .template_file
        .as_ref()
        .expect("Template file should be set");
    let output = world.output_dir.as_ref().expect("Output dir should be set");
    world.command_args = vec![
        command,
        "--profile".to_string(),
        template.to_string_lossy().to_string(),
        "--seed".to_string(),
        "42".to_string(),
        "--count".to_string(),
        "1".to_string(),
        "--out".to_string(),
        output.to_string_lossy().to_string(),
    ];
    world.run_cli();
}

#[when(regex = r#"^I run "([^"]+)" with count (\d+)$"#)]
fn when_run_gen_count(world: &mut CliWorld, command: String, count: String) {
    let template = world
        .template_file
        .as_ref()
        .expect("Template file should be set");
    let output = world.output_dir.as_ref().expect("Output dir should be set");
    world.command_args = vec![
        command,
        "--profile".to_string(),
        template.to_string_lossy().to_string(),
        "--seed".to_string(),
        "42".to_string(),
        "--count".to_string(),
        count,
        "--out".to_string(),
        output.to_string_lossy().to_string(),
    ];
    world.run_cli();
}

#[when(regex = r#"^I run "gen" with "--stats" flag$"#)]
fn when_run_gen_stats(world: &mut CliWorld) {
    let template = world
        .template_file
        .as_ref()
        .expect("Template file should be set");
    let output = world.output_dir.as_ref().expect("Output dir should be set");
    world.command_args = vec![
        "gen".to_string(),
        "--profile".to_string(),
        template.to_string_lossy().to_string(),
        "--seed".to_string(),
        "42".to_string(),
        "--count".to_string(),
        "1".to_string(),
        "--out".to_string(),
        output.to_string_lossy().to_string(),
        "--stats".to_string(),
    ];
    world.run_cli();
}

#[when(regex = r#"^I run the command with "([^"]+)"$"#)]
fn when_run_with_global_flag(world: &mut CliWorld, flag: String) {
    world.command_args = vec![flag];
    world.run_cli();
}

#[when(regex = r#"^I run "([^"]+)" with "([^"]+)"$"#)]
fn when_run_subcommand_with_flag(world: &mut CliWorld, command: String, flag: String) {
    world.command_args = vec![command, flag];
    world.run_cli();
}

#[when("I run an invalid command")]
fn when_run_invalid(world: &mut CliWorld) {
    world.command_args = vec!["invalid-command".to_string()];
    world.run_cli();
}

#[when(regex = r#"^I run "([^"]+)" without arguments$"#)]
fn when_run_no_args(world: &mut CliWorld, command: String) {
    world.command_args = vec![command];
    world.run_cli();
}

// ============================================================================
// Assertion Steps
// ============================================================================

#[then("the command should succeed")]
fn then_succeed(world: &mut CliWorld) {
    assert!(
        world.command_succeeded(),
        "Command should succeed but failed with stderr: {}",
        world.stderr()
    );
}

#[then("the command should fail")]
fn then_fail(world: &mut CliWorld) {
    assert!(
        !world.command_succeeded(),
        "Command should fail but succeeded with stdout: {}",
        world.stdout()
    );
}

#[then("the command should fail with non-zero exit code")]
fn then_fail_nonzero(world: &mut CliWorld) {
    let code = world.exit_code();
    assert!(
        code.map(|c| c != 0).unwrap_or(true),
        "Command should have non-zero exit code but was {:?}",
        code
    );
}

#[then("the output should be valid JSON")]
fn then_valid_json(world: &mut CliWorld) {
    let stdout = world.stdout();
    let result: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
    assert!(
        result.is_ok(),
        "Output should be valid JSON but got: {}",
        stdout
    );
}

#[then("the output should be formatted JSON")]
fn then_formatted_json(world: &mut CliWorld) {
    let stdout = world.stdout();
    assert!(
        stdout.contains('\n'),
        "Formatted JSON should contain newlines"
    );
    let result: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
    assert!(result.is_ok(), "Output should be valid JSON");
}

#[then(regex = r#"^the output should contain segment "([^"]+)"$"#)]
fn then_contains_segment(world: &mut CliWorld, segment: String) {
    let stdout = world.stdout();
    assert!(
        stdout.contains(&segment),
        "Output should contain segment {} but got: {}",
        segment,
        stdout
    );
}

#[then(regex = r#"^the output should contain "([^"]+)"$"#)]
fn then_contains(world: &mut CliWorld, text: String) {
    let stdout = world.stdout();
    assert!(
        stdout.contains(&text),
        "Output should contain '{}' but got: {}",
        text,
        stdout
    );
}

#[then("the error output should contain \"Error\"")]
fn then_error_contains_error(world: &mut CliWorld) {
    let stderr = world.stderr();
    assert!(
        stderr.contains("Error"),
        "Stderr should contain 'Error' but got: {}",
        stderr
    );
}

#[then("the error should indicate file not found")]
fn then_error_file_not_found(world: &mut CliWorld) {
    let stderr = world.stderr();
    assert!(
        stderr.contains("Error") || stderr.contains("not found") || stderr.contains("No such"),
        "Error should indicate file not found: {}",
        stderr
    );
}

#[then("the error should indicate unknown command")]
fn then_error_unknown_command(world: &mut CliWorld) {
    let stderr = world.stderr();
    assert!(
        stderr.contains("error") || !stderr.is_empty(),
        "Error should indicate unknown command: {}",
        stderr
    );
}

#[then("the output should indicate validation issues")]
fn then_validation_issues(world: &mut CliWorld) {
    let output = world.stdout();
    let stderr = world.stderr();
    assert!(
        output.contains("issues")
            || output.contains("Error")
            || stderr.contains("issues")
            || stderr.contains("Error"),
        "Output should indicate validation issues"
    );
}

#[then("the output file should exist")]
fn then_output_exists(world: &mut CliWorld) {
    let output = world
        .output_file
        .as_ref()
        .expect("Output file should be set");
    assert!(output.exists(), "Output file should exist at {:?}", output);
}

#[then("the output file should be valid HL7")]
fn then_output_valid_hl7(world: &mut CliWorld) {
    let output = world
        .output_file
        .as_ref()
        .expect("Output file should be set");
    let content = std::fs::read(output).expect("Should read output file");
    let result = hl7v2_core::parse(&content);
    assert!(result.is_ok(), "Output file should be valid HL7");
}

#[then("the output should start with MLLP start block")]
fn then_mllp_start_block(world: &mut CliWorld) {
    if let Some(Ok(output)) = &world.result {
        assert!(!output.stdout.is_empty(), "Output should not be empty");
        assert_eq!(
            output.stdout[0], 0x0B,
            "Output should start with MLLP start block (0x0B)"
        );
    } else {
        panic!("No command result available");
    }
}

#[then("the output should be a valid HL7 ACK message")]
fn then_valid_ack(world: &mut CliWorld) {
    let stdout = world.stdout();
    let result = hl7v2_core::parse(stdout.as_bytes());
    assert!(result.is_ok(), "ACK output should be valid HL7");
    let msg = result.unwrap();
    assert!(
        msg.segments.iter().any(|s| &s.id == b"MSA"),
        "ACK should contain MSA segment"
    );
}

#[then("the output directory should contain generated messages")]
fn then_output_has_messages(world: &mut CliWorld) {
    let output_dir = world.output_dir.as_ref().expect("Output dir should be set");
    assert!(output_dir.exists(), "Output directory should exist");

    let count = std::fs::read_dir(output_dir)
        .map(|entries| entries.count())
        .unwrap_or(0);
    assert!(
        count > 0,
        "Output directory should contain at least one message"
    );
}

#[then(regex = r#"^the output directory should contain (\d+) messages$"#)]
fn then_output_has_n_messages(world: &mut CliWorld, count: usize) {
    let output_dir = world.output_dir.as_ref().expect("Output dir should be set");

    let actual_count = std::fs::read_dir(output_dir)
        .map(|entries| entries.count())
        .unwrap_or(0);
    assert!(
        actual_count >= count,
        "Output directory should contain at least {} messages but has {}",
        count,
        actual_count
    );
}

#[then("the output should contain generation statistics")]
fn then_gen_stats(world: &mut CliWorld) {
    let stdout = world.stdout();
    assert!(
        stdout.contains("Statistics") || stdout.contains("generated"),
        "Output should contain generation statistics: {}",
        stdout
    );
}

// ============================================================================
// Main function to run tests
// ============================================================================

fn main() {
    // Run the cucumber tests
    futures::executor::block_on(CliWorld::run("features/cli.feature"));
}
