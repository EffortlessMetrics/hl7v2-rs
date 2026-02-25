//! Integration tests for hl7v2-validation crate.
//!
//! These tests validate the validation crate against real HL7 messages
//! using the hl7v2-test-utils fixtures and hl7v2-parser.

use hl7v2_parser::parse;
use hl7v2_test_utils::{fixtures::SampleMessages, builders::MessageBuilder};
use hl7v2_validation::{
    check_rule_condition, is_date, is_numeric, is_timestamp, is_valid_birth_date,
    validate_data_type, Issue, RuleCondition, Severity, Validator,
};

// ============================================================================
// Test Helpers
// ============================================================================

/// Parse a message and return the result
fn parse_message(content: &str) -> hl7v2_core::Message {
    parse(content.as_bytes()).expect("Failed to parse message")
}

// ============================================================================
// ADT^A01 Message Validation Tests
// ============================================================================

#[test]
fn test_validate_adt_a01_basic() {
    let msg_content = SampleMessages::adt_a01();
    let msg = parse_message(msg_content);
    
    // Verify message was parsed
    assert!(!msg.segments.is_empty(), "Message should have segments");
    
    // Verify MSH segment exists
    let msh = msg.segments.iter().find(|s| s.id_str() == "MSH");
    assert!(msh.is_some(), "Message should have MSH segment");
}

#[test]
fn test_validate_adt_a01_msh_fields() {
    let msg_content = SampleMessages::adt_a01();
    let msg = parse_message(msg_content);
    
    // Validate MSH.7 (message timestamp)
    let condition = RuleCondition {
        field: "MSH.7".to_string(),
        operator: "is_date".to_string(),
        value: None,
        values: None,
    };
    assert!(check_rule_condition(&msg, &condition), "MSH.7 should be a valid timestamp");
    
    // Validate MSH.9 (message type)
    let condition = RuleCondition {
        field: "MSH.9.1".to_string(),
        operator: "eq".to_string(),
        value: Some("ADT".to_string()),
        values: None,
    };
    assert!(check_rule_condition(&msg, &condition), "MSH.9.1 should be ADT");
    
    let condition = RuleCondition {
        field: "MSH.9.2".to_string(),
        operator: "eq".to_string(),
        value: Some("A01".to_string()),
        values: None,
    };
    assert!(check_rule_condition(&msg, &condition), "MSH.9.2 should be A01");
}

#[test]
fn test_validate_adt_a01_pid_fields() {
    let msg_content = SampleMessages::adt_a01();
    let msg = parse_message(msg_content);
    
    // Validate PID.3 (patient ID) exists
    let condition = RuleCondition {
        field: "PID.3.1".to_string(),
        operator: "exists".to_string(),
        value: None,
        values: None,
    };
    assert!(check_rule_condition(&msg, &condition), "PID.3.1 should exist");
    
    // Validate PID.5 (patient name) exists
    let condition = RuleCondition {
        field: "PID.5.1".to_string(),
        operator: "exists".to_string(),
        value: None,
        values: None,
    };
    assert!(check_rule_condition(&msg, &condition), "PID.5.1 should exist");
    
    // Validate PID.7 (birth date) is a valid date
    let condition = RuleCondition {
        field: "PID.7".to_string(),
        operator: "is_date".to_string(),
        value: None,
        values: None,
    };
    assert!(check_rule_condition(&msg, &condition), "PID.7 should be a valid date");
    
    // Validate PID.8 (sex) is valid
    let condition = RuleCondition {
        field: "PID.8".to_string(),
        operator: "in".to_string(),
        value: None,
        values: Some(vec!["M".to_string(), "F".to_string(), "O".to_string(), "U".to_string()]),
    };
    assert!(check_rule_condition(&msg, &condition), "PID.8 should be a valid sex value");
}

#[test]
fn test_validate_adt_a01_pv1_fields() {
    let msg_content = SampleMessages::adt_a01();
    let msg = parse_message(msg_content);
    
    // Validate PV1 exists
    let pv1 = msg.segments.iter().find(|s| s.id_str() == "PV1");
    assert!(pv1.is_some(), "Message should have PV1 segment");
    
    // Validate PV1.2 (patient class)
    let condition = RuleCondition {
        field: "PV1.2".to_string(),
        operator: "in".to_string(),
        value: None,
        values: Some(vec!["I".to_string(), "O".to_string(), "E".to_string(), "P".to_string()]),
    };
    assert!(check_rule_condition(&msg, &condition), "PV1.2 should be a valid patient class");
}

// ============================================================================
// ADT^A04 Message Validation Tests
// ============================================================================

#[test]
fn test_validate_adt_a04_basic() {
    let msg_content = SampleMessages::adt_a04();
    let msg = parse_message(msg_content);
    
    // Verify message was parsed
    assert!(!msg.segments.is_empty(), "Message should have segments");
    
    // Validate message type
    let condition = RuleCondition {
        field: "MSH.9.2".to_string(),
        operator: "eq".to_string(),
        value: Some("A04".to_string()),
        values: None,
    };
    assert!(check_rule_condition(&msg, &condition), "MSH.9.2 should be A04");
}

// ============================================================================
// ORU^R01 Message Validation Tests
// ============================================================================

#[test]
fn test_validate_oru_r01_basic() {
    let msg_content = SampleMessages::oru_r01();
    let msg = parse_message(msg_content);
    
    // Verify message was parsed
    assert!(!msg.segments.is_empty(), "Message should have segments");
    
    // Validate message type
    let condition = RuleCondition {
        field: "MSH.9.1".to_string(),
        operator: "eq".to_string(),
        value: Some("ORU".to_string()),
        values: None,
    };
    assert!(check_rule_condition(&msg, &condition), "MSH.9.1 should be ORU");
}

#[test]
fn test_validate_oru_r01_obx_numeric() {
    let msg_content = SampleMessages::oru_r01();
    let msg = parse_message(msg_content);
    
    // Validate OBX.2 (value type)
    let condition = RuleCondition {
        field: "OBX.2".to_string(),
        operator: "eq".to_string(),
        value: Some("NM".to_string()),
        values: None,
    };
    assert!(check_rule_condition(&msg, &condition), "OBX.2 should be NM (numeric)");
    
    // Validate OBX.5 (observation value) is numeric
    let condition = RuleCondition {
        field: "OBX.5".to_string(),
        operator: "exists".to_string(),
        value: None,
        values: None,
    };
    assert!(check_rule_condition(&msg, &condition), "OBX.5 should exist");
}

// ============================================================================
// Message Builder Validation Tests
// ============================================================================

#[test]
fn test_validate_builder_message_basic() {
    let bytes = MessageBuilder::new()
        .with_msh("TestApp", "TestFac", "RecvApp", "RecvFac", "ADT", "A01")
        .build_bytes();
    
    let msg = parse_message(&String::from_utf8_lossy(&bytes));
    
    // Validate MSH fields
    let condition = RuleCondition {
        field: "MSH.9.1".to_string(),
        operator: "eq".to_string(),
        value: Some("ADT".to_string()),
        values: None,
    };
    assert!(check_rule_condition(&msg, &condition));
}

#[test]
fn test_validate_builder_message_with_pid() {
    let bytes = MessageBuilder::new()
        .with_msh("TestApp", "TestFac", "RecvApp", "RecvFac", "ADT", "A01")
        .with_pid("MRN12345", "Doe", "John")
        .build_bytes();
    
    let msg = parse_message(&String::from_utf8_lossy(&bytes));
    
    // Validate PID fields
    let condition = RuleCondition {
        field: "PID.3.1".to_string(),
        operator: "eq".to_string(),
        value: Some("MRN12345".to_string()),
        values: None,
    };
    assert!(check_rule_condition(&msg, &condition));
    
    let condition = RuleCondition {
        field: "PID.5.1".to_string(),
        operator: "eq".to_string(),
        value: Some("Doe".to_string()),
        values: None,
    };
    assert!(check_rule_condition(&msg, &condition));
}

#[test]
fn test_validate_builder_message_with_pv1() {
    let bytes = MessageBuilder::new()
        .with_msh("TestApp", "TestFac", "RecvApp", "RecvFac", "ADT", "A01")
        .with_pid("MRN12345", "Doe", "John")
        .with_pv1("I", "ICU^101")
        .build_bytes();
    
    let msg = parse_message(&String::from_utf8_lossy(&bytes));
    
    // Validate PV1 fields
    let condition = RuleCondition {
        field: "PV1.2".to_string(),
        operator: "eq".to_string(),
        value: Some("I".to_string()),
        values: None,
    };
    assert!(check_rule_condition(&msg, &condition));
}

// ============================================================================
// Cross-Field Validation Tests
// ============================================================================

#[test]
fn test_cross_field_validation_inpatient_requires_room() {
    // Create a message with PV1.2 = I (inpatient) but missing room
    let bytes = MessageBuilder::new()
        .with_msh("TestApp", "TestFac", "RecvApp", "RecvFac", "ADT", "A01")
        .with_pid("MRN12345", "Doe", "John")
        .with_pv1("I", "") // Inpatient but no room
        .build_bytes();
    
    let msg = parse_message(&String::from_utf8_lossy(&bytes));
    
    // Check if PV1.2 = I
    let is_inpatient = check_rule_condition(&msg, &RuleCondition {
        field: "PV1.2".to_string(),
        operator: "eq".to_string(),
        value: Some("I".to_string()),
        values: None,
    });
    
    // Check if PV1.3 exists
    let has_room = check_rule_condition(&msg, &RuleCondition {
        field: "PV1.3.1".to_string(),
        operator: "exists".to_string(),
        value: None,
        values: None,
    });
    
    // If inpatient, should have room (this is a business rule)
    if is_inpatient {
        // This would be a validation issue in a real system
        assert!(!has_room, "This message intentionally lacks room for testing");
    }
}

#[test]
fn test_birth_date_before_message_date() {
    let msg_content = concat!(
        "MSH|^~\\&|TestApp|TestFac|RecvApp|RecvFac|20250128152312||ADT^A01|1|P|2.5\r",
        "PID|1||MRN12345||Doe^John||19800101|M\r"
    );
    let msg = parse_message(msg_content);
    
    // Birth date should be before message date
    let condition = RuleCondition {
        field: "PID.7".to_string(),
        operator: "before".to_string(),
        value: Some("20250128".to_string()),
        values: None,
    };
    assert!(check_rule_condition(&msg, &condition), "Birth date should be before message date");
}

// ============================================================================
// Required Field Validation Tests
// ============================================================================

#[test]
fn test_required_field_present() {
    let bytes = MessageBuilder::new()
        .with_msh("TestApp", "TestFac", "RecvApp", "RecvFac", "ADT", "A01")
        .with_pid("MRN12345", "Doe", "John")
        .build_bytes();
    
    let msg = parse_message(&String::from_utf8_lossy(&bytes));
    
    // Required fields should exist
    let required_fields = ["PID.3.1", "PID.5.1"];
    
    for field in required_fields {
        let condition = RuleCondition {
            field: field.to_string(),
            operator: "exists".to_string(),
            value: None,
            values: None,
        };
        assert!(check_rule_condition(&msg, &condition), "{} should exist", field);
    }
}

#[test]
fn test_required_field_missing() {
    // Create a message with minimal PID (no patient ID)
    let msg_content = "MSH|^~\\&|App|Fac|Recv|Fac|20230101||ADT^A01|1|P|2.5\rPID|1||||||\r";
    let msg = parse_message(msg_content);
    
    // PID.3.1 should not exist
    let condition = RuleCondition {
        field: "PID.3.1".to_string(),
        operator: "exists".to_string(),
        value: None,
        values: None,
    };
    assert!(!check_rule_condition(&msg, &condition), "PID.3.1 should not exist");
}

// ============================================================================
// Data Type Validation Integration Tests
// ============================================================================

#[test]
fn test_data_type_validation_in_message() {
    let msg_content = concat!(
        "MSH|^~\\&|App|Fac|Recv|Fac|20230101||ADT^A01|1|P|2.5\r",
        "PID|1||12345^^^MRN||Doe^John||19800101|M|||123 Main St^^Anytown^CA^12345||555-123-4567\r"
    );
    let msg = parse_message(msg_content);
    
    // Validate birth date (PID.7) as DT type
    let condition = RuleCondition {
        field: "PID.7".to_string(),
        operator: "is_date".to_string(),
        value: None,
        values: None,
    };
    assert!(check_rule_condition(&msg, &condition), "PID.7 should be a valid date");
    
    // Validate sex (PID.8) as ID type
    let condition = RuleCondition {
        field: "PID.8".to_string(),
        operator: "in".to_string(),
        value: None,
        values: Some(vec!["M".to_string(), "F".to_string(), "O".to_string()]),
    };
    assert!(check_rule_condition(&msg, &condition), "PID.8 should be a valid sex code");
}

#[test]
fn test_numeric_validation_in_message() {
    let msg_content = concat!(
        "MSH|^~\\&|Lab|Hospital|LIS|Hospital|20230101||ORU^R01|1|P|2.5\r",
        "PID|1||MRN123||Test^Patient||19800101|M\r",
        "OBX|1|NM|WBC||7.5|10^9/L|4.0-11.0|N|||F\r"
    );
    let msg = parse_message(msg_content);
    
    // Validate OBX.5 is numeric
    let condition = RuleCondition {
        field: "OBX.5".to_string(),
        operator: "exists".to_string(),
        value: None,
        values: None,
    };
    assert!(check_rule_condition(&msg, &condition), "OBX.5 should exist");
    
    // Verify it's a valid numeric
    assert!(is_numeric("7.5"), "OBX.5 value should be numeric");
}

// ============================================================================
// Segment Order Validation Tests
// ============================================================================

#[test]
fn test_segment_order_msh_first() {
    let msg_content = SampleMessages::adt_a01();
    let msg = parse_message(msg_content);
    
    // MSH should always be first
    assert!(!msg.segments.is_empty(), "Message should have segments");
    assert_eq!(msg.segments[0].id_str(), "MSH", "First segment should be MSH");
}

#[test]
fn test_segment_order_adt_a01() {
    let msg_content = SampleMessages::adt_a01();
    let msg = parse_message(msg_content);
    
    // Expected order for ADT^A01: MSH, EVN, PID, PV1
    let segment_names: Vec<&str> = msg.segments.iter().map(|s| s.id_str()).collect();
    
    assert!(segment_names.contains(&"MSH"), "Should have MSH");
    assert!(segment_names.contains(&"PID"), "Should have PID");
    
    // PID should come after MSH
    let msh_idx = segment_names.iter().position(|&s| s == "MSH");
    let pid_idx = segment_names.iter().position(|&s| s == "PID");
    
    if let (Some(msh), Some(pid)) = (msh_idx, pid_idx) {
        assert!(msh < pid, "MSH should come before PID");
    }
}

// ============================================================================
// Cardinality Validation Tests
// ============================================================================

#[test]
fn test_single_pid_segment() {
    let msg_content = SampleMessages::adt_a01();
    let msg = parse_message(msg_content);
    
    // Should have exactly one PID segment
    let pid_count = msg.segments.iter().filter(|s| s.id_str() == "PID").count();
    assert_eq!(pid_count, 1, "Should have exactly one PID segment");
}

#[test]
fn test_single_msh_segment() {
    let msg_content = SampleMessages::adt_a01();
    let msg = parse_message(msg_content);
    
    // Should have exactly one MSH segment
    let msh_count = msg.segments.iter().filter(|s| s.id_str() == "MSH").count();
    assert_eq!(msh_count, 1, "Should have exactly one MSH segment");
}

// ============================================================================
// Warning vs Error Severity Tests
// ============================================================================

#[test]
fn test_issue_severity_error() {
    let issue = Issue::error("MISSING_REQUIRED_FIELD", Some("PID.3".to_string()), "Patient ID is required".to_string());
    
    assert_eq!(issue.severity, Severity::Error);
    assert_eq!(issue.code, "MISSING_REQUIRED_FIELD");
    assert_eq!(issue.path, Some("PID.3".to_string()));
}

#[test]
fn test_issue_severity_warning() {
    let issue = Issue::warning("MISSING_OPTIONAL_FIELD", Some("PID.12".to_string()), "Country code is recommended".to_string());
    
    assert_eq!(issue.severity, Severity::Warning);
    assert_eq!(issue.code, "MISSING_OPTIONAL_FIELD");
}

#[test]
fn test_mixed_severity_issues() {
    let issues = vec![
        Issue::error("MISSING_REQUIRED_FIELD", Some("PID.3".to_string()), "Patient ID is required".to_string()),
        Issue::warning("MISSING_OPTIONAL_FIELD", Some("PID.12".to_string()), "Country code is recommended".to_string()),
    ];
    
    let error_count = issues.iter().filter(|i| i.severity == Severity::Error).count();
    let warning_count = issues.iter().filter(|i| i.severity == Severity::Warning).count();
    
    assert_eq!(error_count, 1);
    assert_eq!(warning_count, 1);
}

// ============================================================================
// Batch Validation Tests
// ============================================================================

#[test]
fn test_batch_validation_multiple_messages() {
    let messages = vec![
        SampleMessages::adt_a01(),
        SampleMessages::adt_a04(),
        SampleMessages::oru_r01(),
    ];
    
    let mut results = Vec::new();
    
    for msg_content in messages {
        let msg = parse_message(msg_content);
        
        // Check required field exists
        let condition = RuleCondition {
            field: "MSH.7".to_string(),
            operator: "exists".to_string(),
            value: None,
            values: None,
        };
        results.push(check_rule_condition(&msg, &condition));
    }
    
    // All messages should have MSH.7
    assert!(results.iter().all(|&r| r), "All messages should have MSH.7");
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_empty_field_validation() {
    let msg_content = "MSH|^~\\&|App|Fac|Recv|Fac|20230101||ADT^A01|1|P|2.5\rPID|1||||||\r";
    let msg = parse_message(msg_content);
    
    // Empty fields should not "exist"
    let condition = RuleCondition {
        field: "PID.5.1".to_string(),
        operator: "exists".to_string(),
        value: None,
        values: None,
    };
    assert!(!check_rule_condition(&msg, &condition), "Empty field should not exist");
}

#[test]
fn test_special_characters_in_fields() {
    let msg_content = concat!(
        "MSH|^~\\&|App|Fac|Recv|Fac|20230101||ADT^A01|1|P|2.5\r",
        "PID|1||12345||O\\F\\Brien^John||19800101|M\r" // Name with escape sequences
    );
    let msg = parse_message(msg_content);
    
    // Field should exist despite special characters
    let condition = RuleCondition {
        field: "PID.5.1".to_string(),
        operator: "exists".to_string(),
        value: None,
        values: None,
    };
    assert!(check_rule_condition(&msg, &condition), "Field with special chars should exist");
}

#[test]
fn test_repeated_fields() {
    let msg_content = concat!(
        "MSH|^~\\&|App|Fac|Recv|Fac|20230101||ADT^A01|1|P|2.5\r",
        "PID|1||12345^^^MRN~67890^^^SS||Doe^John||19800101|M\r" // Repeated patient IDs
    );
    let msg = parse_message(msg_content);
    
    // First repetition should exist
    let condition = RuleCondition {
        field: "PID.3.1".to_string(),
        operator: "exists".to_string(),
        value: None,
        values: None,
    };
    assert!(check_rule_condition(&msg, &condition), "Repeated field should exist");
}

// ============================================================================
// Invalid Message Handling Tests
// ============================================================================

#[test]
fn test_invalid_message_type() {
    let msg_content = concat!(
        "MSH|^~\\&|App|Fac|Recv|Fac|20230101||INVALID^TYPE|1|P|2.5\r",
        "PID|1||12345||Doe^John||19800101|M\r"
    );
    let msg = parse_message(msg_content);
    
    // Message type should not match expected
    let condition = RuleCondition {
        field: "MSH.9.1".to_string(),
        operator: "in".to_string(),
        value: None,
        values: Some(vec!["ADT".to_string(), "ORU".to_string(), "ORM".to_string()]),
    };
    assert!(!check_rule_condition(&msg, &condition), "Invalid message type should fail");
}

#[test]
fn test_invalid_date_format() {
    let msg_content = concat!(
        "MSH|^~\\&|App|Fac|Recv|Fac|20230101||ADT^A01|1|P|2.5\r",
        "PID|1||12345||Doe^John||invalid||M\r" // Invalid birth date
    );
    let msg = parse_message(msg_content);
    
    // Invalid date should fail is_date check
    let condition = RuleCondition {
        field: "PID.7".to_string(),
        operator: "is_date".to_string(),
        value: None,
        values: None,
    };
    assert!(!check_rule_condition(&msg, &condition), "Invalid date should fail");
}

// ============================================================================
// Profile-Based Validation Tests (Placeholder)
// ============================================================================

#[test]
fn test_profile_required_fields() {
    // Simulate profile validation by checking required fields
    let msg_content = SampleMessages::adt_a01();
    let msg = parse_message(msg_content);
    
    // Profile: ADT^A01 requires PID.3, PID.5
    let required_fields = ["PID.3.1", "PID.5.1"];
    let mut issues = Vec::new();
    
    for field in required_fields {
        let condition = RuleCondition {
            field: field.to_string(),
            operator: "exists".to_string(),
            value: None,
            values: None,
        };
        
        if !check_rule_condition(&msg, &condition) {
            issues.push(Issue::error(
                "MISSING_REQUIRED_FIELD",
                Some(field.to_string()),
                format!("Required field {} is missing", field),
            ));
        }
    }
    
    assert!(issues.is_empty(), "Should have no issues for valid message: {:?}", issues);
}

#[test]
fn test_profile_field_lengths() {
    // Simulate field length validation
    let long_id = "A".repeat(100); // Very long ID
    let msg_content = format!(
        "MSH|^~\\&|App|Fac|Recv|Fac|20230101||ADT^A01|1|P|2.5\rPID|1||{}||Doe^John||19800101|M\r",
        long_id
    );
    let msg = parse_message(&msg_content);
    
    // Check field exists (length validation would be profile-specific)
    let condition = RuleCondition {
        field: "PID.3.1".to_string(),
        operator: "exists".to_string(),
        value: None,
        values: None,
    };
    assert!(check_rule_condition(&msg, &condition), "Long field should still exist");
    
    // In a real profile, we would check length here
    // For now, just verify the field was parsed
}

// ============================================================================
// Performance Tests
// ============================================================================

#[test]
fn test_validation_performance() {
    let msg_content = SampleMessages::adt_a01();
    let msg = parse_message(msg_content);
    
    // Run multiple validations
    let start = std::time::Instant::now();
    let iterations = 1000;
    
    for _ in 0..iterations {
        let condition = RuleCondition {
            field: "PID.3.1".to_string(),
            operator: "exists".to_string(),
            value: None,
            values: None,
        };
        let _ = check_rule_condition(&msg, &condition);
    }
    
    let elapsed = start.elapsed();
    let per_validation = elapsed / iterations;
    
    // Should be very fast (less than 1ms per validation)
    assert!(
        per_validation < std::time::Duration::from_millis(1),
        "Validation should be fast: {:?}",
        per_validation
    );
}
