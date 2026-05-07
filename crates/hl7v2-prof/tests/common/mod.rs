//! Common test helpers for hl7v2-prof integration tests.
//!
//! This module provides shared test fixtures and helper functions
//! to reduce duplication across test files.

#![allow(dead_code)]

use hl7v2_parser::parse;
use hl7v2_prof::{Profile, load_profile, validate};

// ============================================================================
// Test Message Fixtures
// ============================================================================

/// Build a valid ADT A01 message
pub fn valid_adt_a01() -> String {
    let mut s = String::new();
    s.push_str("MSH|^~\\&|SND|SF|RCV|RF|20250101000000||ADT^A01|MSG1|P|2.5.1\r");
    s.push_str("EVN|A01|20250101000000|\r");
    s.push_str("PID|1||123456^^^HOSP^MR||Doe^John||19800101|M||||||||||||||||\r");
    s.push_str("PV1|1|I|WARD1||||DOC123|||||||||||||||||||||||||20250101\r");
    s
}

/// Build an invalid ADT A01 message (missing required fields)
pub fn invalid_adt_a01() -> String {
    let mut s = String::new();
    s.push_str("MSH|^~\\&|SND|SF|RCV|RF|20250101000000||ADT^A01|MSG1|P|2.5.1\r");
    s.push_str("EVN|A01|20250101000000|\r");
    s.push_str("PID|1|||^^^HOSP^MR||Doe^John||19800101|X||||||||||||||||\r"); // Missing PID.3, invalid sex
    s.push_str("PV1|1|I|||||||||||||||||||||||||||||||||\r"); // Missing PV1.3
    s
}

/// Build a valid ORU R01 message
pub fn valid_oru_r01() -> String {
    let mut s = String::new();
    s.push_str("MSH|^~\\&|SND|SF|RCV|RF|20250101000000||ORU^R01|MSG1|P|2.5.1\r");
    s.push_str("PID|1||123456^^^HOSP^MR||Doe^John||19800101|M||||||||||||||||\r");
    s.push_str("PV1|1|O|||||||||||||||||||||||||||||||\r");
    s.push_str("ORC|RE|ORD123||20250101\r");
    s.push_str("OBR|1|ORD123||TEST^Test Panel^L|20250101000000\r");
    s.push_str("OBX|1|NM|TEST1^Test 1^L||100|mg/dL|10-200|N|||F\r");
    s
}

/// Build an invalid ORU R01 message (missing observation value)
pub fn invalid_oru_r01() -> String {
    let mut s = String::new();
    s.push_str("MSH|^~\\&|SND|SF|RCV|RF|20250101000000||ORU^R01|MSG1|P|2.5.1\r");
    s.push_str("PID|1||123456^^^HOSP^MR||Doe^John||19800101|M||||||||||||||||\r");
    s.push_str("PV1|1|O|||||||||||||||||||||||||||||||\r");
    s.push_str("ORC|RE|ORD123||20250101\r");
    s.push_str("OBR|1|ORD123||TEST^Test Panel^L|20250101000000\r");
    s.push_str("OBX|1|NM|TEST1^Test 1^L||||mg/dL|10-200|N|||F\r"); // Empty OBX.5
    s
}

// ============================================================================
// Validation Helpers
// ============================================================================

/// Check if issues contain a specific error code
pub fn has_error_code(issues: &[hl7v2_prof::Issue], code: &str) -> bool {
    issues.iter().any(|i| i.code == code)
}

/// Check if issues contain error for a specific path
pub fn has_error_for_path(issues: &[hl7v2_prof::Issue], path: &str) -> bool {
    issues.iter().any(|i| i.path.as_deref() == Some(path))
}

/// Check if issues contain any errors (convenience function)
pub fn has_any_error(issues: &[hl7v2_prof::Issue]) -> bool {
    !issues.is_empty()
}

// ============================================================================
// Profile Helpers
// ============================================================================

/// Load a profile from YAML string, panicking on error
pub fn load_profile_panicking(yaml: &str) -> Profile {
    load_profile(yaml).expect("Failed to load profile")
}

/// Validate a message against a profile, panicking on parse error
pub fn validate_message(msg_str: &str, profile: &Profile) -> Vec<hl7v2_prof::Issue> {
    let msg = parse(msg_str.as_bytes()).expect("Failed to parse message");
    validate(&msg, profile)
}

// ============================================================================
// Message Building Helpers
// ============================================================================

/// Build an ADT A01 message with custom control ID and patient ID
pub fn adt_a01_message(control_id: &str, patient_id: &str, sex: &str) -> String {
    format!(
        "MSH|^~\\&|SND|SF|RCV|RF|20250101000000||ADT^A01|{}|P|2.5.1\rPID|1||{}^^^HOSP^MR||Doe^John||19800101|{}||||||||||||||||\r",
        control_id, patient_id, sex
    )
}

/// Build an ACK message
pub fn ack_message(control_id: &str, ack_code: &str) -> String {
    format!(
        "MSH|^~\\&|RCV|RF|SND|SF|20250101000000||ACK|{}|P|2.5.1\rMSA|{}|MSG001|Message accepted\r",
        control_id, ack_code
    )
}

/// Build a simple PID-only message for basic validation
pub fn simple_pid_message(patient_id: &str, last_name: &str, first_name: &str) -> String {
    format!(
        "MSH|^~\\&|SND|SF|RCV|RF|20250101000000||ADT^A01|MSG1|P|2.5.1\rPID|1||{}^^^HOSP^MR||{}^{}||19800101|M||||||||||||||||\r",
        patient_id, last_name, first_name
    )
}
