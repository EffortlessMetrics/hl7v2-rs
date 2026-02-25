//! Common test utilities for hl7v2-cli integration tests.
//!
//! This module provides helper functions and utilities for testing the CLI
//! binary using assert_cmd and tempfile.

use assert_cmd::Command;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use hl7v2_test_utils::fixtures::SampleMessages;

/// Helper to create a new CLI command
pub fn cli_command() -> Command {
    Command::cargo_bin("hl7v2-cli").expect("Failed to find hl7v2-cli binary")
}

/// Helper to create a temp directory for test files
pub fn create_temp_dir() -> TempDir {
    TempDir::new().expect("Failed to create temp directory")
}

/// Helper to create a temp file with content
pub fn create_temp_file(dir: &TempDir, filename: &str, content: &[u8]) -> PathBuf {
    let path = dir.path().join(filename);
    fs::write(&path, content).expect("Failed to write temp file");
    path
}

/// Helper to create a temp HL7 file with a valid ADT^A01 message
pub fn create_temp_hl7_file(dir: &TempDir, filename: &str) -> PathBuf {
    create_temp_file(dir, filename, SampleMessages::adt_a01().as_bytes())
}

/// Helper to create a temp HL7 file with custom content
pub fn create_temp_hl7_with_content(dir: &TempDir, filename: &str, content: &str) -> PathBuf {
    create_temp_file(dir, filename, content.as_bytes())
}

/// Helper to create a temp profile YAML file
pub fn create_temp_profile(dir: &TempDir, filename: &str, content: &str) -> PathBuf {
    create_temp_file(dir, filename, content.as_bytes())
}

/// Helper to create a minimal valid profile
pub fn minimal_profile() -> &'static str {
    r#"
message_structure: ADT_A01
version: "2.5.1"
segments:
  - id: MSH
  - id: PID
constraints:
  - path: MSH.9
    required: true
"#
}

/// Helper to create a strict profile that requires specific segments
pub fn strict_profile() -> &'static str {
    r#"
message_structure: ADT_A01
version: "2.5.1"
segments:
  - id: MSH
  - id: EVN
  - id: PID
  - id: PV1
  - id: ZZ1
constraints:
  - path: MSH.9
    required: true
"#
}

/// Helper to create a template YAML for message generation
pub fn simple_template() -> &'static str {
    r#"
message:
  type: ADT
  trigger: A01
segments:
  - MSH
  - PID
"#
}

/// Helper to create an invalid HL7 message
pub fn invalid_hl7_message() -> &'static str {
    "This is not a valid HL7 message"
}

/// Helper to create a truncated HL7 message
pub fn truncated_hl7_message() -> &'static str {
    "MSH|^~\\&|"
}

/// Helper to create an MLLP-framed message
pub fn create_mllp_message(content: &[u8]) -> Vec<u8> {
    let mut mllp_bytes = vec![0x0B]; // SB - Start Block
    mllp_bytes.extend_from_slice(content);
    mllp_bytes.push(0x1C); // EB - End Block
    mllp_bytes.push(0x0D); // CR - Carriage Return
    mllp_bytes
}

/// Helper to create a temp MLLP-framed file
pub fn create_temp_mllp_file(dir: &TempDir, filename: &str) -> PathBuf {
    let mllp_content = create_mllp_message(SampleMessages::adt_a01().as_bytes());
    create_temp_file(dir, filename, &mllp_content)
}

/// Helper to read file contents
pub fn read_file(path: &PathBuf) -> Vec<u8> {
    fs::read(path).expect("Failed to read file")
}

/// Helper to check if output contains valid JSON
pub fn is_valid_json(output: &[u8]) -> bool {
    let output_str = String::from_utf8_lossy(output);
    serde_json::from_str::<serde_json::Value>(&output_str).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_temp_hl7_file() {
        let dir = create_temp_dir();
        let path = create_temp_hl7_file(&dir, "test.hl7");
        assert!(path.exists());
        
        let content = read_file(&path);
        assert!(content.starts_with(b"MSH|"));
    }

    #[test]
    fn test_create_mllp_message() {
        let content = b"MSH|^~\\&|Test\r";
        let mllp = create_mllp_message(content);
        
        assert_eq!(mllp[0], 0x0B);
        assert_eq!(mllp[mllp.len() - 2], 0x1C);
        assert_eq!(mllp[mllp.len() - 1], 0x0D);
    }

    #[test]
    fn test_is_valid_json() {
        let valid = br#"{"key": "value"}"#;
        let invalid = b"not json";
        
        assert!(is_valid_json(valid));
        assert!(!is_valid_json(invalid));
    }

    #[test]
    fn test_minimal_profile_is_valid_yaml() {
        let result: Result<serde_yaml::Value, _> = serde_yaml::from_str(minimal_profile());
        assert!(result.is_ok());
    }

    #[test]
    fn test_strict_profile_is_valid_yaml() {
        let result: Result<serde_yaml::Value, _> = serde_yaml::from_str(strict_profile());
        assert!(result.is_ok());
    }
}
