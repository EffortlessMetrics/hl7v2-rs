//! Snapshot tests for hl7v2-validation crate using insta.
//!
//! These tests capture validation error messages and reports as snapshots
//! to detect unexpected changes in output format.

use hl7v2_parser::parse;
use hl7v2_test_utils::{builders::MessageBuilder, fixtures::SampleMessages};
use hl7v2_validation::{Issue, RuleCondition, Severity, check_rule_condition};
use insta::{assert_debug_snapshot, assert_json_snapshot, assert_yaml_snapshot};

// ============================================================================
// Test Helpers
// ============================================================================

/// Parse a message and return the result
fn parse_message(content: &str) -> hl7v2_core::Message {
    parse(content.as_bytes()).expect("Failed to parse message")
}

/// Create a validation report for a message
#[derive(Debug, serde::Serialize)]
struct ValidationReport {
    message_type: String,
    message_control_id: String,
    issues: Vec<Issue>,
    is_valid: bool,
}

impl ValidationReport {
    fn new(message_type: String, message_control_id: String) -> Self {
        Self {
            message_type,
            message_control_id,
            issues: Vec::new(),
            is_valid: true,
        }
    }

    fn add_issue(&mut self, issue: Issue) {
        if issue.severity == Severity::Error {
            self.is_valid = false;
        }
        self.issues.push(issue);
    }
}

// ============================================================================
// Validation Issue Snapshots
// ============================================================================

#[test]
fn snapshot_missing_required_field_error() {
    let issue = Issue::error(
        "MISSING_REQUIRED_FIELD",
        Some("PID.3".to_string()),
        "Patient Identifier (PID.3) is required for ADT^A01 messages".to_string(),
    );

    assert_yaml_snapshot!(issue, @"
    code: MISSING_REQUIRED_FIELD
    severity: Error
    path: PID.3
    detail: Patient Identifier (PID.3) is required for ADT^A01 messages
    ");
}

#[test]
fn snapshot_missing_optional_field_warning() {
    let issue = Issue::warning(
        "MISSING_OPTIONAL_FIELD",
        Some("PID.12".to_string()),
        "Country code (PID.12) is recommended for international patients".to_string(),
    );

    assert_yaml_snapshot!(issue, @"
    code: MISSING_OPTIONAL_FIELD
    severity: Warning
    path: PID.12
    detail: Country code (PID.12) is recommended for international patients
    ");
}

#[test]
fn snapshot_invalid_data_type_error() {
    let issue = Issue::error(
        "INVALID_DATA_TYPE",
        Some("PID.7".to_string()),
        "Birth date 'invalid' does not match expected format DT (YYYYMMDD)".to_string(),
    );

    assert_yaml_snapshot!(issue, @r#"
code: INVALID_DATA_TYPE
severity: Error
path: PID.7
detail: "Birth date 'invalid' does not match expected format DT (YYYYMMDD)"
"#);
}

#[test]
fn snapshot_field_length_exceeded_error() {
    let issue = Issue::error(
        "FIELD_LENGTH_EXCEEDED",
        Some("PID.3.1".to_string()),
        "Field value exceeds maximum length of 20 characters (actual: 50)".to_string(),
    );

    assert_yaml_snapshot!(issue, @r#"
code: FIELD_LENGTH_EXCEEDED
severity: Error
path: PID.3.1
detail: "Field value exceeds maximum length of 20 characters (actual: 50)"
"#);
}

#[test]
fn snapshot_invalid_code_value_error() {
    let issue = Issue::error(
        "INVALID_CODE_VALUE",
        Some("PID.8".to_string()),
        "Value 'X' is not valid for PID.8 (Sex). Valid values: M, F, O, U, A, N".to_string(),
    );

    assert_yaml_snapshot!(issue, @r#"
code: INVALID_CODE_VALUE
severity: Error
path: PID.8
detail: "Value 'X' is not valid for PID.8 (Sex). Valid values: M, F, O, U, A, N"
"#);
}

#[test]
fn snapshot_temporal_validation_error() {
    let issue = Issue::error(
        "TEMPORAL_VALIDATION_FAILED",
        Some("PID.7".to_string()),
        "Birth date (20990101) cannot be in the future".to_string(),
    );

    assert_yaml_snapshot!(issue, @"
    code: TEMPORAL_VALIDATION_FAILED
    severity: Error
    path: PID.7
    detail: Birth date (20990101) cannot be in the future
    ");
}

#[test]
fn snapshot_checksum_validation_error() {
    let issue = Issue::error(
        "CHECKSUM_VALIDATION_FAILED",
        Some("PID.3.1".to_string()),
        "Patient ID '12345678901' failed Luhn checksum validation".to_string(),
    );

    assert_yaml_snapshot!(issue, @r#"
code: CHECKSUM_VALIDATION_FAILED
severity: Error
path: PID.3.1
detail: "Patient ID '12345678901' failed Luhn checksum validation"
"#);
}

#[test]
fn snapshot_segment_order_error() {
    let issue = Issue::error(
        "INVALID_SEGMENT_ORDER",
        Some("PV1".to_string()),
        "Segment PV1 must appear after PID segment".to_string(),
    );

    assert_yaml_snapshot!(issue, @"
    code: INVALID_SEGMENT_ORDER
    severity: Error
    path: PV1
    detail: Segment PV1 must appear after PID segment
    ");
}

#[test]
fn snapshot_cardinality_error() {
    let issue = Issue::error(
        "CARDINALITY_VIOLATION",
        Some("PID".to_string()),
        "Message contains 2 PID segments, but maximum allowed is 1".to_string(),
    );

    assert_yaml_snapshot!(issue, @r#"
code: CARDINALITY_VIOLATION
severity: Error
path: PID
detail: "Message contains 2 PID segments, but maximum allowed is 1"
"#);
}

#[test]
fn snapshot_cross_field_validation_error() {
    let issue = Issue::error(
        "CROSS_FIELD_VALIDATION_FAILED",
        Some("PV1.3".to_string()),
        "When PV1.2 (Patient Class) is 'I' (Inpatient), PV1.3 (Assigned Patient Location) is required".to_string(),
    );

    assert_yaml_snapshot!(issue, @r#"
code: CROSS_FIELD_VALIDATION_FAILED
severity: Error
path: PV1.3
detail: "When PV1.2 (Patient Class) is 'I' (Inpatient), PV1.3 (Assigned Patient Location) is required"
"#);
}

// ============================================================================
// Validation Report Snapshots
// ============================================================================

#[test]
fn snapshot_valid_adt_a01_report() {
    let msg_content = SampleMessages::adt_a01();
    let msg = parse_message(msg_content);

    let mut report = ValidationReport::new("ADT^A01".to_string(), "ABC123".to_string());

    // Check required fields
    let required_fields = [
        ("MSH.7", "Message timestamp"),
        ("PID.3.1", "Patient ID"),
        ("PID.5.1", "Patient Name"),
    ];

    for (field, description) in required_fields {
        let condition = RuleCondition {
            field: field.to_string(),
            operator: "exists".to_string(),
            value: None,
            values: None,
        };

        if !check_rule_condition(&msg, &condition) {
            report.add_issue(Issue::error(
                "MISSING_REQUIRED_FIELD",
                Some(field.to_string()),
                format!("{} is required", description),
            ));
        }
    }

    assert_yaml_snapshot!(report, @r#"
message_type: ADT^A01
message_control_id: ABC123
issues: []
is_valid: true
"#);
}

#[test]
fn snapshot_invalid_message_report() {
    let msg_content = "MSH|^~\\&|App|Fac|Recv|Fac|20230101||ADT^A01|1|P|2.5\rPID|1||||||\r";
    let msg = parse_message(msg_content);

    let mut report = ValidationReport::new("ADT^A01".to_string(), "1".to_string());

    // Check required fields
    let required_fields = [("PID.3.1", "Patient ID"), ("PID.5.1", "Patient Name")];

    for (field, description) in required_fields {
        let condition = RuleCondition {
            field: field.to_string(),
            operator: "exists".to_string(),
            value: None,
            values: None,
        };

        if !check_rule_condition(&msg, &condition) {
            report.add_issue(Issue::error(
                "MISSING_REQUIRED_FIELD",
                Some(field.to_string()),
                format!("{} is required", description),
            ));
        }
    }

    assert_yaml_snapshot!(report, @r#"
message_type: ADT^A01
message_control_id: "1"
issues:
  - code: MISSING_REQUIRED_FIELD
    severity: Error
    path: PID.3.1
    detail: Patient ID is required
  - code: MISSING_REQUIRED_FIELD
    severity: Error
    path: PID.5.1
    detail: Patient Name is required
is_valid: false
"#);
}

#[test]
fn snapshot_message_with_warnings_report() {
    let msg_content = concat!(
        "MSH|^~\\&|App|Fac|Recv|Fac|20230101||ADT^A01|1|P|2.5\r",
        "PID|1||12345||Doe^John||19800101|M|||123 Main St^^Anytown^CA^12345\r"
    );
    let msg = parse_message(msg_content);

    let mut report = ValidationReport::new("ADT^A01".to_string(), "1".to_string());

    // Check optional fields (warnings)
    let optional_fields = [
        ("PID.11.6", "Country Code"),
        ("PID.13", "Phone Number - Home"),
    ];

    for (field, description) in optional_fields {
        let condition = RuleCondition {
            field: field.to_string(),
            operator: "exists".to_string(),
            value: None,
            values: None,
        };

        if !check_rule_condition(&msg, &condition) {
            report.add_issue(Issue::warning(
                "MISSING_OPTIONAL_FIELD",
                Some(field.to_string()),
                format!("{} is recommended", description),
            ));
        }
    }

    assert_yaml_snapshot!(report, @r#"
    message_type: ADT^A01
    message_control_id: "1"
    issues:
      - code: MISSING_OPTIONAL_FIELD
        severity: Warning
        path: PID.11.6
        detail: Country Code is recommended
      - code: MISSING_OPTIONAL_FIELD
        severity: Warning
        path: PID.13
        detail: Phone Number - Home is recommended
    is_valid: true
    "#);
}

// ============================================================================
// Multiple Issues Report Snapshots
// ============================================================================

#[test]
fn snapshot_multiple_issues_report() {
    let mut report = ValidationReport::new("ADT^A01".to_string(), "MSG002".to_string());

    // Add various issues
    report.add_issue(Issue::error(
        "MISSING_REQUIRED_FIELD",
        Some("PID.3".to_string()),
        "Patient Identifier is required".to_string(),
    ));

    report.add_issue(Issue::warning(
        "MISSING_OPTIONAL_FIELD",
        Some("PID.12".to_string()),
        "Country code is recommended".to_string(),
    ));

    report.add_issue(Issue::error(
        "INVALID_DATA_TYPE",
        Some("PID.7".to_string()),
        "Birth date format is invalid".to_string(),
    ));

    report.add_issue(Issue::warning(
        "FIELD_LENGTH_WARNING",
        Some("PID.5.1".to_string()),
        "Family name exceeds recommended length".to_string(),
    ));

    assert_yaml_snapshot!(report, @r#"
message_type: ADT^A01
message_control_id: MSG002
issues:
  - code: MISSING_REQUIRED_FIELD
    severity: Error
    path: PID.3
    detail: Patient Identifier is required
  - code: MISSING_OPTIONAL_FIELD
    severity: Warning
    path: PID.12
    detail: Country code is recommended
  - code: INVALID_DATA_TYPE
    severity: Error
    path: PID.7
    detail: Birth date format is invalid
  - code: FIELD_LENGTH_WARNING
    severity: Warning
    path: PID.5.1
    detail: Family name exceeds recommended length
is_valid: false
"#);
}

// ============================================================================
// JSON Format Snapshots
// ============================================================================

#[test]
fn snapshot_issue_json_format() {
    let issue = Issue::error(
        "MISSING_REQUIRED_FIELD",
        Some("PID.3".to_string()),
        "Patient Identifier is required".to_string(),
    );

    assert_json_snapshot!(issue, @r#"
    {
      "code": "MISSING_REQUIRED_FIELD",
      "severity": "Error",
      "path": "PID.3",
      "detail": "Patient Identifier is required"
    }
    "#);
}

#[test]
fn snapshot_report_json_format() {
    let mut report = ValidationReport::new("ORU^R01".to_string(), "LAB001".to_string());

    report.add_issue(Issue::error(
        "INVALID_DATA_TYPE",
        Some("OBX.5".to_string()),
        "Observation value must be numeric for NM type".to_string(),
    ));

    assert_json_snapshot!(report, @r#"
    {
      "message_type": "ORU^R01",
      "message_control_id": "LAB001",
      "issues": [
        {
          "code": "INVALID_DATA_TYPE",
          "severity": "Error",
          "path": "OBX.5",
          "detail": "Observation value must be numeric for NM type"
        }
      ],
      "is_valid": false
    }
    "#);
}

// ============================================================================
// Debug Format Snapshots
// ============================================================================

#[test]
fn snapshot_severity_debug() {
    assert_debug_snapshot!(Severity::Error, @"Error");
    assert_debug_snapshot!(Severity::Warning, @"Warning");
}

#[test]
fn snapshot_issue_debug() {
    let issue = Issue::error(
        "TEST_CODE",
        Some("TEST.FIELD".to_string()),
        "Test detail message".to_string(),
    );

    assert_debug_snapshot!(issue, @r#"
Issue {
    code: "TEST_CODE",
    severity: Error,
    path: Some(
        "TEST.FIELD",
    ),
    detail: "Test detail message",
}
"#);
}

// ============================================================================
// Edge Case Snapshots
// ============================================================================

#[test]
fn snapshot_issue_with_no_path() {
    let issue = Issue::error(
        "MESSAGE_LEVEL_ERROR",
        None,
        "Message structure is invalid".to_string(),
    );

    assert_yaml_snapshot!(issue, @"
    code: MESSAGE_LEVEL_ERROR
    severity: Error
    path: ~
    detail: Message structure is invalid
    ");
}

#[test]
fn snapshot_issue_with_special_characters() {
    let issue = Issue::error(
        "INVALID_CHARACTERS",
        Some("PID.5.1".to_string()),
        "Field contains invalid characters: \t, \n, \r".to_string(),
    );

    assert_yaml_snapshot!(issue, @r#"
code: INVALID_CHARACTERS
severity: Error
path: PID.5.1
detail: "Field contains invalid characters: \t, \n, \r"
"#);
}

#[test]
fn snapshot_issue_with_long_detail() {
    let long_detail = "This is a very long error message that explains in detail what went wrong during validation. \
        It includes multiple sentences and provides comprehensive information about the error condition, \
        including suggestions for how to fix the problem and references to relevant documentation.";

    let issue = Issue::error(
        "DETAILED_ERROR",
        Some("COMPLEX.FIELD.PATH".to_string()),
        long_detail.to_string(),
    );

    assert_yaml_snapshot!(issue, @r#"
code: DETAILED_ERROR
severity: Error
path: COMPLEX.FIELD.PATH
detail: "This is a very long error message that explains in detail what went wrong during validation. It includes multiple sentences and provides comprehensive information about the error condition, including suggestions for how to fix the problem and references to relevant documentation."
"#);
}

// ============================================================================
// Message Type Specific Snapshots
// ============================================================================

#[test]
fn snapshot_adt_a01_validation_report() {
    let msg_content = SampleMessages::adt_a01();
    let msg = parse_message(msg_content);

    let mut report = ValidationReport::new("ADT^A01".to_string(), "ABC123".to_string());

    // Validate ADT^A01 specific requirements
    let conditions = [
        ("MSH.7", "is_date", None, None),
        ("MSH.9.1", "eq", Some("ADT"), None),
        ("MSH.9.2", "eq", Some("A01"), None),
        ("PID.3.1", "exists", None, None),
        ("PID.5.1", "exists", None, None),
        (
            "PID.8",
            "in",
            None,
            Some(vec![
                "M".to_string(),
                "F".to_string(),
                "O".to_string(),
                "U".to_string(),
            ]),
        ),
    ];

    for (field, op, value, values) in conditions {
        let condition = RuleCondition {
            field: field.to_string(),
            operator: op.to_string(),
            value: value.map(|s| s.to_string()),
            values,
        };

        if !check_rule_condition(&msg, &condition) {
            report.add_issue(Issue::error(
                "VALIDATION_FAILED",
                Some(field.to_string()),
                format!("Field {} failed {} validation", field, op),
            ));
        }
    }

    assert_yaml_snapshot!(report, @r#"
message_type: ADT^A01
message_control_id: ABC123
issues: []
is_valid: true
"#);
}

#[test]
fn snapshot_oru_r01_validation_report() {
    let msg_content = SampleMessages::oru_r01();
    let msg = parse_message(msg_content);

    let mut report = ValidationReport::new("ORU^R01".to_string(), "MSG003".to_string());

    // Validate ORU^R01 specific requirements
    let conditions = [
        ("MSH.9.1", "eq", Some("ORU"), None),
        ("OBX.2", "exists", None, None),
    ];

    for (field, op, value, values) in conditions {
        let condition = RuleCondition {
            field: field.to_string(),
            operator: op.to_string(),
            value: value.map(|s| s.to_string()),
            values,
        };

        if !check_rule_condition(&msg, &condition) {
            report.add_issue(Issue::error(
                "VALIDATION_FAILED",
                Some(field.to_string()),
                format!("Field {} failed {} validation", field, op),
            ));
        }
    }

    assert_yaml_snapshot!(report, @r#"
message_type: ORU^R01
message_control_id: MSG003
issues: []
is_valid: true
"#);
}

// ============================================================================
// Batch Validation Snapshots
// ============================================================================

#[test]
fn snapshot_batch_validation_results() {
    #[derive(Debug, serde::Serialize)]
    struct BatchValidationResult {
        total_messages: usize,
        valid_messages: usize,
        invalid_messages: usize,
        total_errors: usize,
        total_warnings: usize,
        results: Vec<ValidationReport>,
    }

    let messages = [
        ("ADT^A01", SampleMessages::adt_a01()),
        ("ADT^A04", SampleMessages::adt_a04()),
        ("ORU^R01", SampleMessages::oru_r01()),
    ];

    let mut batch = BatchValidationResult {
        total_messages: messages.len(),
        valid_messages: 0,
        invalid_messages: 0,
        total_errors: 0,
        total_warnings: 0,
        results: Vec::new(),
    };

    for (msg_type, msg_content) in messages {
        let msg = parse_message(msg_content);
        let mut report = ValidationReport::new(msg_type.to_string(), "test".to_string());

        // Simple validation: check MSH.7 exists
        let condition = RuleCondition {
            field: "MSH.7".to_string(),
            operator: "exists".to_string(),
            value: None,
            values: None,
        };

        if !check_rule_condition(&msg, &condition) {
            report.add_issue(Issue::error(
                "MISSING_TIMESTAMP",
                Some("MSH.7".to_string()),
                "Message timestamp is required".to_string(),
            ));
        }

        if report.is_valid {
            batch.valid_messages += 1;
        } else {
            batch.invalid_messages += 1;
        }

        batch.total_errors += report
            .issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .count();
        batch.total_warnings += report
            .issues
            .iter()
            .filter(|i| i.severity == Severity::Warning)
            .count();

        batch.results.push(report);
    }

    assert_yaml_snapshot!(batch, @r#"
total_messages: 3
valid_messages: 3
invalid_messages: 0
total_errors: 0
total_warnings: 0
results:
  - message_type: ADT^A01
    message_control_id: test
    issues: []
    is_valid: true
  - message_type: ADT^A04
    message_control_id: test
    issues: []
    is_valid: true
  - message_type: ORU^R01
    message_control_id: test
    issues: []
    is_valid: true
"#);
}
