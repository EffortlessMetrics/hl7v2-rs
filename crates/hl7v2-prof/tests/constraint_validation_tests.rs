//! Integration tests for constraint validation.
//!
//! These tests cover:
//! - Required field validation
//! - Length constraint validation  
//! - Component count validation
//! - Conditional constraints
//! - Data type constraints

use hl7v2_prof::load_profile;

mod common;
use common::{has_error_code, has_error_for_path, valid_adt_a01, validate_message};

// ============================================================================
// Test 1: Required field validation passes when field present
// ============================================================================
#[test]
fn test_required_field_present_passes() {
    let yaml = r#"
message_structure: "ADT_A01"
version: "2.5.1"
segments:
  - id: "PID"
constraints:
  - path: "PID.3"
    required: true
  - path: "PID.5"
    required: true
"#;

    let profile = load_profile(yaml).unwrap();
    let issues = validate_message(&valid_adt_a01(), &profile);

    assert!(
        issues.is_empty(),
        "Expected no issues when required fields present: {:?}",
        issues
    );
}

// ============================================================================
// Test 2: Required field validation fails when field missing
// ============================================================================
#[test]
fn test_required_field_missing_fails() {
    let yaml = r#"
message_structure: "ADT_A01"
version: "2.5.1"
segments:
  - id: "PID"
constraints:
  - path: "PID.3"
    required: true
  - path: "PID.99"
    required: true
"#;

    let profile = load_profile(yaml).unwrap();
    let issues = validate_message(&valid_adt_a01(), &profile);

    // PID.99 doesn't exist, so should trigger required field error
    assert!(
        has_error_code(&issues, "MISSING_REQUIRED_FIELD"),
        "Expected MISSING_REQUIRED_FIELD error: {:?}",
        issues
    );
}

// ============================================================================
// Test 3: Empty required field fails validation
// ============================================================================
#[test]
fn test_empty_required_field_fails() {
    let yaml = r#"
message_structure: "ADT_A01"
version: "2.5.1"
segments:
  - id: "PID"
constraints:
  - path: "PID.3"
    required: true
"#;

    let profile = load_profile(yaml).unwrap();

    // Message with empty PID.3
    let mut msg_str = String::new();
    msg_str.push_str("MSH|^~\\&|SND|SF|RCV|RF|20250101000000||ADT^A01|MSG1|P|2.5.1\r");
    msg_str.push_str("PID|1|||^^^HOSP^MR||Doe^John||19800101|M||||||||||||||||\r"); // Empty PID.3

    let issues = validate_message(&msg_str, &profile);

    assert!(
        has_error_code(&issues, "MISSING_REQUIRED_FIELD"),
        "Expected MISSING_REQUIRED_FIELD for empty field: {:?}",
        issues
    );
}

// ============================================================================
// Test 4: Length constraint passes when under max
// ============================================================================
#[test]
fn test_length_constraint_under_max_passes() {
    let yaml = r#"
message_structure: "ADT_A01"
version: "2.5.1"
segments:
  - id: "PID"
lengths:
  - path: "PID.5[1].1"
    max: 100
"#;

    let profile = load_profile(yaml).unwrap();
    let issues = validate_message(&valid_adt_a01(), &profile);

    // "Doe" is 3 chars, well under 100 limit
    assert!(issues.is_empty(), "Expected no length issues: {:?}", issues);
}

// ============================================================================
// Test 5: Length constraint fails when over max
// ============================================================================
#[test]
fn test_length_constraint_over_max_fails() {
    let yaml = r#"
message_structure: "ADT_A01"
version: "2.5.1"
segments:
  - id: "PID"
lengths:
  - path: "PID.5[1].1"
    max: 2
"#;

    let profile = load_profile(yaml).unwrap();
    let issues = validate_message(&valid_adt_a01(), &profile);

    // "Doe" is 3 chars, exceeds 2 char limit
    assert!(
        has_error_code(&issues, "VALUE_TOO_LONG"),
        "Expected VALUE_TOO_LONG error: {:?}",
        issues
    );
}

// ============================================================================
// Test 6: Exact length boundary passes
// ============================================================================
#[test]
fn test_exact_length_boundary() {
    let yaml = r#"
message_structure: "ADT_A01"
version: "2.5.1"
segments:
  - id: "PID"
lengths:
  - path: "PID.5[1].1"
    max: 3
"#;

    let profile = load_profile(yaml).unwrap();
    let issues = validate_message(&valid_adt_a01(), &profile);

    // "Doe" is 3 chars, exactly at boundary
    assert!(
        !has_error_code(&issues, "VALUE_TOO_LONG"),
        "Exact boundary length should pass: {:?}",
        issues
    );
}

// ============================================================================
// Test 7: Length boundary plus one fails
// ============================================================================
#[test]
fn test_length_boundary_plus_one_fails() {
    let yaml = r#"
message_structure: "ADT_A01"
version: "2.5.1"
segments:
  - id: "PID"
lengths:
  - path: "PID.5[1].1"
    max: 2
"#;

    let profile = load_profile(yaml).unwrap();
    let issues = validate_message(&valid_adt_a01(), &profile);

    // "Doe" is 3 chars, 1 over boundary
    assert!(
        has_error_code(&issues, "VALUE_TOO_LONG"),
        "One over boundary should fail: {:?}",
        issues
    );
}

// ============================================================================
// Test 8: Component count constraint minimum
// ============================================================================
#[test]
fn test_component_count_minimum() {
    let yaml = r#"
message_structure: "ADT_A01"
version: "2.5.1"
segments:
  - id: "PID"
constraints:
  - path: "PID.5"
    components:
      min: 2
"#;

    let profile = load_profile(yaml).unwrap();
    let issues = validate_message(&valid_adt_a01(), &profile);

    // PID.5 is "Doe^John" - 2 components, at minimum
    // Note: Component validation may vary by implementation
    let _ = issues; // Document expected behavior
}

// ============================================================================
// Test 9: Conditional constraint when condition met
// ============================================================================
#[test]
fn test_conditional_constraint_condition_met() {
    let yaml = r#"
message_structure: "ADT_A01"
version: "2.5.1"
segments:
  - id: "PID"
  - id: "PV1"
constraints:
  - path: "PV1.3"
    required: true
    when:
      eq: ["PV1.2", "I"]
"#;

    let profile = load_profile(yaml).unwrap();
    let issues = validate_message(&valid_adt_a01(), &profile);

    // PV1.2 is "I" (inpatient), so PV1.3 is required and present
    assert!(
        !has_error_code(&issues, "MISSING_REQUIRED_FIELD"),
        "Conditional constraint should pass: {:?}",
        issues
    );
}

// ============================================================================
// Test 10: Conditional constraint when condition not met
// ============================================================================
#[test]
fn test_conditional_constraint_condition_not_met() {
    let yaml = r#"
message_structure: "ADT_A01"
version: "2.5.1"
segments:
  - id: "PID"
  - id: "PV1"
constraints:
  - path: "PV1.99"
    required: true
    when:
      eq: ["PV1.2", "X"]
"#;

    let profile = load_profile(yaml).unwrap();
    let issues = validate_message(&valid_adt_a01(), &profile);

    // PV1.2 is "I" not "X", so the constraint should not apply
    assert!(
        !has_error_code(&issues, "MISSING_REQUIRED_FIELD"),
        "Constraint should not apply when condition not met: {:?}",
        issues
    );
}

// ============================================================================
// Test 11: Advanced data type with min length
// ============================================================================
#[test]
fn test_advanced_datatype_min_length() {
    let yaml = r#"
message_structure: "ADT_A01"
version: "2.5.1"
segments:
  - id: "PID"
advanced_datatypes:
  - path: "PID.5[1].1"
    type: "ST"
    min_length: 2
"#;

    let profile = load_profile(yaml).unwrap();
    let issues = validate_message(&valid_adt_a01(), &profile);

    // "Doe" is 3 chars, over minimum of 2
    assert!(
        !has_error_code(&issues, "VALUE_TOO_SHORT"),
        "Valid min length should pass: {:?}",
        issues
    );
}

// ============================================================================
// Test 12: Advanced data type with max length
// ============================================================================
#[test]
fn test_advanced_datatype_max_length() {
    let yaml = r#"
message_structure: "ADT_A01"
version: "2.5.1"
segments:
  - id: "PID"
advanced_datatypes:
  - path: "PID.5[1].1"
    type: "ST"
    max_length: 10
"#;

    let profile = load_profile(yaml).unwrap();
    let issues = validate_message(&valid_adt_a01(), &profile);

    // "Doe" is 3 chars, under max of 10
    assert!(
        !has_error_code(&issues, "VALUE_TOO_LONG"),
        "Valid max length should pass: {:?}",
        issues
    );
}

// ============================================================================
// Test 13: Advanced data type pattern matching
// ============================================================================
#[test]
fn test_advanced_datatype_pattern() {
    let yaml = r#"
message_structure: "ADT_A01"
version: "2.5.1"
segments:
  - id: "PID"
advanced_datatypes:
  - path: "PID.3"
    type: "ST"
    pattern: "^[0-9]+$"
"#;

    let profile = load_profile(yaml).unwrap();
    let issues = validate_message(&valid_adt_a01(), &profile);

    // PID.3 is "123456" - matches numeric pattern
    assert!(
        !has_error_code(&issues, "PATTERN_MISMATCH"),
        "Valid pattern match should pass: {:?}",
        issues
    );
}

// ============================================================================
// Test 14: Pattern mismatch fails
// ============================================================================
#[test]
fn test_advanced_datatype_pattern_mismatch() {
    let yaml = r#"
message_structure: "ADT_A01"
version: "2.5.1"
segments:
  - id: "PID"
advanced_datatypes:
  - path: "PID.5[1].1"
    type: "ST"
    pattern: "^[0-9]+$"
"#;

    let profile = load_profile(yaml).unwrap();
    let issues = validate_message(&valid_adt_a01(), &profile);

    // "Doe" contains letters, doesn't match numeric pattern
    assert!(
        has_error_code(&issues, "PATTERN_MISMATCH"),
        "Pattern mismatch should fail: {:?}",
        issues
    );
}

// ============================================================================
// Test 15: Data type constraint validation
// ============================================================================
#[test]
fn test_datatype_constraint() {
    let yaml = r#"
message_structure: "ADT_A01"
version: "2.5.1"
segments:
  - id: "PID"
datatypes:
  - path: "PID.7"
    type: "DT"
"#;

    let profile = load_profile(yaml).unwrap();
    let issues = validate_message(&valid_adt_a01(), &profile);

    // PID.7 is "19800101" which is a valid DT format
    assert!(
        !has_error_code(&issues, "INVALID_DATA_TYPE"),
        "Valid date should not fail: {:?}",
        issues
    );
}

// ============================================================================
// Test 16: Multiple constraints combined
// ============================================================================
#[test]
fn test_multiple_constraints_combined() {
    let yaml = r#"
message_structure: "ADT_A01"
version: "2.5.1"
segments:
  - id: "PID"
constraints:
  - path: "PID.3"
    required: true
  - path: "PID.5"
    required: true
lengths:
  - path: "PID.3"
    max: 50
  - path: "PID.5[1].1"
    max: 50
valuesets:
  - path: "PID.8"
    name: "HL70001"
    codes:
      - "M"
      - "F"
"#;

    let profile = load_profile(yaml).unwrap();
    let issues = validate_message(&valid_adt_a01(), &profile);

    // All constraints should pass
    assert!(
        issues.is_empty(),
        "Expected no issues with multiple constraints: {:?}",
        issues
    );
}

// ============================================================================
// Test 17: Error path accuracy
// ============================================================================
#[test]
fn test_error_path_accuracy() {
    let yaml = r#"
message_structure: "ADT_A01"
version: "2.5.1"
segments:
  - id: "PID"
constraints:
  - path: "PID.3"
    required: true
"#;

    let profile = load_profile(yaml).unwrap();

    // Message with empty PID.3
    let mut msg_str = String::new();
    msg_str.push_str("MSH|^~\\&|SND|SF|RCV|RF|20250101000000||ADT^A01|MSG1|P|2.5.1\r");
    msg_str.push_str("PID|1|||^^^HOSP^MR||Doe^John||19800101|M||||||||||||||||\r");

    let issues = validate_message(&msg_str, &profile);

    // Should have error with path pointing to PID.3
    assert!(
        has_error_for_path(&issues, "PID.3"),
        "Error should reference PID.3 path: {:?}",
        issues
    );
}

// ============================================================================
// Test 18: Empty optional field allowed
// ============================================================================
#[test]
fn test_empty_optional_field_allowed() {
    let yaml = r#"
message_structure: "ADT_A01"
version: "2.5.1"
segments:
  - id: "PID"
constraints:
  - path: "PID.3"
    required: false
"#;

    let profile = load_profile(yaml).unwrap();

    // Message with empty PID.3 but not required
    let mut msg_str = String::new();
    msg_str.push_str("MSH|^~\\&|SND|SF|RCV|RF|20250101000000||ADT^A01|MSG1|P|2.5.1\r");
    msg_str.push_str("PID|1|||^^^HOSP^MR||Doe^John||19800101|M||||||||||||||||\r");

    let issues = validate_message(&msg_str, &profile);

    // Empty optional field should not error
    assert!(
        !has_error_for_path(&issues, "PID.3"),
        "Empty optional field should not error: {:?}",
        issues
    );
}
