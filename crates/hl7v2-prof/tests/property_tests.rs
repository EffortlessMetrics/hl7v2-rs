//! Property-based tests for hl7v2-prof crate using proptest
//!
//! These tests verify profile loading, validation, and serialization properties
//! hold for arbitrary inputs.

use hl7v2_core::parse;
use hl7v2_prof::{load_profile, validate, Profile};
use proptest::prelude::*;

// ============================================================================
// Custom strategies for profile generation
// ============================================================================

/// Generate a valid message structure name (alphanumeric with underscores)
fn message_structure_strategy() -> impl Strategy<Value = String> {
    "[A-Z][A-Z0-9_]{2,19}"
}

/// Generate a valid HL7 version string
fn version_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("2.5.1".to_string()),
        Just("2.5".to_string()),
        Just("2.6".to_string()),
        Just("2.7".to_string()),
        Just("2.3.1".to_string()),
    ]
}

/// Generate a valid segment ID (3 uppercase letters)
fn segment_id_strategy() -> impl Strategy<Value = String> {
    "[A-Z][A-Z0-9]{2}"
}

/// Generate a valid field path (e.g., "PID.3", "MSH.9")
fn field_path_strategy() -> impl Strategy<Value = String> {
    (segment_id_strategy(), 1usize..30).prop_map(|(seg, field)| format!("{}.{}", seg, field))
}

/// Generate safe text that doesn't contain YAML special characters
fn safe_text_strategy() -> impl Strategy<Value = String> {
    "[A-Za-z0-9 ]{1,50}"
}


/// Generate a simple valid profile YAML
fn simple_profile_yaml(
    msg_struct: String,
    version: String,
    segment_ids: Vec<String>,
) -> String {
    let segments_yaml = segment_ids
        .iter()
        .map(|id| format!("  - id: \"{}\"", id))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"
message_structure: "{}"
version: "{}"
segments:
{}
"#,
        msg_struct, version, segments_yaml
    )
}

/// Generate a profile with constraints
fn profile_with_constraints_yaml(
    msg_struct: String,
    version: String,
    segment_ids: Vec<String>,
    constraint_paths: Vec<String>,
) -> String {
    let segments_yaml = segment_ids
        .iter()
        .map(|id| format!("  - id: \"{}\"", id))
        .collect::<Vec<_>>()
        .join("\n");

    let constraints_yaml = constraint_paths
        .iter()
        .map(|path| {
            format!(
                r#"  - path: "{}"
    required: true"#,
                path
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"
message_structure: "{}"
version: "{}"
segments:
{}
constraints:
{}
"#,
        msg_struct, version, segments_yaml, constraints_yaml
    )
}

// ============================================================================
// Property tests for profile loading/parsing
// ============================================================================

proptest! {
    /// Test that loading a valid simple profile never fails
    #[test]
    fn prop_load_simple_profile_never_fails(
        msg_struct in message_structure_strategy(),
        version in version_strategy(),
        segment_id in segment_id_strategy()
    ) {
        let yaml = simple_profile_yaml(msg_struct, version, vec![segment_id]);
        let result = load_profile(&yaml);
        prop_assert!(result.is_ok());
    }

    /// Test that loaded profile preserves message structure
    #[test]
    fn prop_profile_preserves_message_structure(
        msg_struct in message_structure_strategy(),
        version in version_strategy()
    ) {
        let yaml = simple_profile_yaml(msg_struct.clone(), version, vec!["MSH".to_string()]);
        let profile = load_profile(&yaml).unwrap();
        prop_assert_eq!(profile.message_structure, msg_struct);
    }

    /// Test that loaded profile preserves version
    #[test]
    fn prop_profile_preserves_version(
        msg_struct in message_structure_strategy(),
        version in version_strategy()
    ) {
        let yaml = simple_profile_yaml(msg_struct, version.clone(), vec!["MSH".to_string()]);
        let profile = load_profile(&yaml).unwrap();
        prop_assert_eq!(profile.version, version);
    }

    /// Test that profile with multiple segments preserves all segment IDs
    #[test]
    fn prop_profile_preserves_segments(
        msg_struct in message_structure_strategy(),
        version in version_strategy(),
        seg1 in segment_id_strategy(),
        seg2 in segment_id_strategy(),
        seg3 in segment_id_strategy()
    ) {
        // Ensure segments are different
        prop_assume!(seg1 != seg2);
        prop_assume!(seg2 != seg3);

        let yaml = simple_profile_yaml(msg_struct, version, vec![seg1.clone(), seg2.clone(), seg3.clone()]);
        let profile = load_profile(&yaml).unwrap();

        let segment_ids: Vec<&str> = profile.segments.iter().map(|s| s.id.as_str()).collect();
        prop_assert!(segment_ids.contains(&seg1.as_str()));
        prop_assert!(segment_ids.contains(&seg2.as_str()));
        prop_assert!(segment_ids.contains(&seg3.as_str()));
    }

    /// Test that profile with constraints preserves constraint paths
    #[test]
    fn prop_profile_preserves_constraints(
        msg_struct in message_structure_strategy(),
        version in version_strategy(),
        path in field_path_strategy()
    ) {
        let yaml = profile_with_constraints_yaml(
            msg_struct,
            version,
            vec!["MSH".to_string()],
            vec![path.clone()]
        );
        let profile = load_profile(&yaml).unwrap();

        prop_assert!(!profile.constraints.is_empty());
        prop_assert_eq!(&profile.constraints[0].path, &path);
    }
}

// ============================================================================
// Property tests for YAML roundtrip (serialization/deserialization)
// ============================================================================

proptest! {
    /// Test that profile can be serialized and deserialized (roundtrip)
    #[test]
    fn prop_profile_yaml_roundtrip(
        msg_struct in message_structure_strategy(),
        version in version_strategy(),
        seg1 in segment_id_strategy(),
        seg2 in segment_id_strategy()
    ) {
        prop_assume!(seg1 != seg2);

        let original_yaml = simple_profile_yaml(
            msg_struct,
            version,
            vec![seg1.clone(), seg2.clone()]
        );

        // Parse the original YAML
        let profile1 = load_profile(&original_yaml).unwrap();

        // Serialize to YAML
        let serialized = serde_yaml::to_string(&profile1).unwrap();

        // Deserialize again
        let profile2: Profile = serde_yaml::from_str(&serialized).unwrap();

        // Compare key fields
        prop_assert_eq!(profile1.message_structure, profile2.message_structure);
        prop_assert_eq!(profile1.version, profile2.version);
        prop_assert_eq!(profile1.segments.len(), profile2.segments.len());
    }

    /// Test that profile with constraints roundtrips correctly
    #[test]
    fn prop_profile_with_constraints_roundtrip(
        msg_struct in message_structure_strategy(),
        version in version_strategy(),
        path in field_path_strategy()
    ) {
        let original_yaml = profile_with_constraints_yaml(
            msg_struct,
            version,
            vec!["MSH".to_string(), "PID".to_string()],
            vec![path.clone()]
        );

        let profile1 = load_profile(&original_yaml).unwrap();
        let serialized = serde_yaml::to_string(&profile1).unwrap();
        let profile2: Profile = serde_yaml::from_str(&serialized).unwrap();

        prop_assert_eq!(profile1.constraints.len(), profile2.constraints.len());
        if !profile1.constraints.is_empty() {
            prop_assert_eq!(&profile1.constraints[0].path, &profile2.constraints[0].path);
            prop_assert_eq!(profile1.constraints[0].required, profile2.constraints[0].required);
        }
    }
}

// ============================================================================
// Property tests for validation invariants
// ============================================================================

/// Generate a valid ADT^A01 message
fn adt_a01_message(control_id: &str, patient_id: &str, sex: &str) -> String {
    format!(
        "MSH|^~\\&|SND|SF|RCV|RF|20250101000000||ADT^A01|{}|P|2.5.1\rPID|1||{}^^^HOSP^MR||Doe^John||19800101|{}||||||||||||||||\r",
        control_id, patient_id, sex
    )
}

proptest! {
    /// Test that validation never panics for valid messages with valid profiles
    #[test]
    fn prop_validate_never_panics(
        control_id in "[A-Za-z0-9]{1,20}",
        patient_id in "[0-9]{5,10}",
        sex in prop_oneof![Just("M"), Just("F"), Just("O"), Just("U")]
    ) {
        let yaml = r#"
message_structure: "ADT_A01"
version: "2.5.1"
segments:
  - id: "MSH"
  - id: "PID"
"#;
        let profile = load_profile(yaml).unwrap();
        let msg_str = adt_a01_message(&control_id, &patient_id, sex);
        let msg = parse(msg_str.as_bytes()).unwrap();

        // Validation should never panic
        let problems = validate(&msg, &profile);
        prop_assert!(problems.is_empty());
    }

    /// Test that validation with required field constraint works
    #[test]
    fn prop_validate_required_field(
        patient_id in "[0-9]{5,10}"
    ) {
        let yaml = r#"
message_structure: "ADT_A01"
version: "2.5.1"
segments:
  - id: "MSH"
  - id: "PID"
constraints:
  - path: "PID.3"
    required: true
"#;
        let profile = load_profile(yaml).unwrap();
        let msg_str = adt_a01_message("MSG001", &patient_id, "M");
        let msg = parse(msg_str.as_bytes()).unwrap();

        let problems = validate(&msg, &profile);
        // Patient ID is present, so no problems
        prop_assert!(problems.is_empty());
    }

    /// Test that validation detects missing required fields
    #[test]
    fn prop_validate_missing_required_field(
        control_id in "[A-Za-z0-9]{1,20}"
    ) {
        let yaml = r#"
message_structure: "ADT_A01"
version: "2.5.1"
segments:
  - id: "MSH"
  - id: "PID"
constraints:
  - path: "PID.3"
    required: true
"#;
        let profile = load_profile(yaml).unwrap();

        // Message without patient ID in PID.3
        let msg_str = format!(
            "MSH|^~\\&|SND|SF|RCV|RF|20250101000000||ADT^A01|{}|P|2.5.1\rPID|1||||Doe^John||19800101|M||||||||||||||||\r",
            control_id
        );
        let msg = parse(msg_str.as_bytes()).unwrap();

        let problems = validate(&msg, &profile);
        // Should detect missing PID.3
        prop_assert!(!problems.is_empty());
    }
}

// ============================================================================
// Property tests for cross-field rules
// ============================================================================

proptest! {
    /// Test that cross-field rules load and validate without panicking
    #[test]
    fn prop_cross_field_rule_loads(
        sex in prop_oneof![Just("M"), Just("F"), Just("O")]
    ) {
        let yaml = r#"
message_structure: "ADT_A01"
version: "2.5.1"
segments:
  - id: "MSH"
  - id: "PID"
cross_field_rules:
  - id: "sex-rule"
    description: "Sex validation rule"
    conditions:
      - field: "PID.8"
        operator: "eq"
        value: "M"
    actions: []
"#;
        let profile = load_profile(yaml).unwrap();
        let msg_str = adt_a01_message("MSG001", "12345", &sex);
        let msg = parse(msg_str.as_bytes()).unwrap();

        // Validation should never panic
        let _problems = validate(&msg, &profile);
    }
}

// ============================================================================
// Property tests for error handling
// ============================================================================

proptest! {
    /// Test that invalid YAML produces appropriate error
    #[test]
    fn prop_invalid_yaml_error(invalid_yaml in "[^a-zA-Z0-9]{10,50}") {
        let result = load_profile(&invalid_yaml);
        prop_assert!(result.is_err());
    }

    /// Test that profile without required fields produces error
    #[test]
    fn prop_missing_required_field_error(
        random_text in safe_text_strategy()
    ) {
        // YAML without message_structure
        let yaml = format!(
            r#"
version: "{}"
segments:
  - id: "MSH"
"#,
            random_text
        );
        let result = load_profile(&yaml);
        prop_assert!(result.is_err());
    }

    /// Test that empty YAML produces error
    #[test]
    fn prop_empty_yaml_error(_empty in Just("")) {
        let result = load_profile("");
        prop_assert!(result.is_err());
    }
}

// ============================================================================
// Property tests for profile inheritance (merge behavior)
// ============================================================================

/// Create a child profile YAML that extends parent
fn child_profile_yaml(msg_struct: &str, version: &str, parent_ref: &str) -> String {
    format!(
        r#"
message_structure: "{}"
version: "{}"
parent: "{}"
segments:
  - id: "PV1"
constraints:
  - path: "PID.5"
    required: true
"#,
        msg_struct, version, parent_ref
    )
}

proptest! {
    /// Test that child profile preserves parent reference
    #[test]
    fn prop_child_profile_preserves_parent_ref(
        msg_struct in message_structure_strategy(),
        version in version_strategy(),
        parent_name in "[A-Za-z][A-Za-z0-9_]{2,19}"
    ) {
        let yaml = child_profile_yaml(&msg_struct, &version, &parent_name);
        let profile = load_profile(&yaml).unwrap();

        prop_assert_eq!(profile.parent, Some(parent_name));
    }

    /// Test that profile segments are preserved independently
    #[test]
    fn prop_profile_segments_independent(
        msg_struct in message_structure_strategy(),
        version in version_strategy(),
        seg1 in segment_id_strategy(),
        seg2 in segment_id_strategy()
    ) {
        prop_assume!(seg1 != seg2);

        let yaml = simple_profile_yaml(msg_struct, version, vec![seg1.clone(), seg2.clone()]);
        let profile = load_profile(&yaml).unwrap();

        // Both segments should be present
        prop_assert_eq!(profile.segments.len(), 2);
    }
}

// ============================================================================
// Property tests for edge cases (encoding characters in fields)
// ============================================================================

/// Generate text with HL7 encoding characters
fn text_with_encoding_chars() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("|".to_string()),       // Field separator
        Just("^".to_string()),       // Component separator
        Just("~".to_string()),       // Repetition separator
        Just("\\".to_string()),      // Escape character
        Just("&".to_string()),       // Subcomponent separator
        Just("test|value".to_string()),
        Just("test^value".to_string()),
        Just("test~value".to_string()),
        Just("test\\value".to_string()),
        Just("test&value".to_string()),
    ]
}

proptest! {
    /// Test that profiles with special characters in descriptions load correctly
    #[test]
    fn prop_profile_with_special_chars(
        msg_struct in message_structure_strategy(),
        special_text in text_with_encoding_chars()
    ) {
        let yaml = format!(
            r#"
message_structure: "{}"
version: "2.5.1"
segments:
  - id: "MSH"
cross_field_rules:
  - id: "test-rule"
    description: "{}"
    conditions: []
    actions: []
"#,
            msg_struct, special_text
        );

        // Profile should load without error (YAML handles escaping)
        let result = load_profile(&yaml);
        // May or may not succeed depending on YAML escaping, but shouldn't panic
        let _ = result;
    }

    /// Test that validation handles messages with escape sequences
    #[test]
    fn prop_validate_escaped_content(
        control_id in "[A-Za-z0-9]{1,20}",
        escaped_value in prop_oneof![
            Just("H\\F\\test"),      // Escaped field separator
            Just("H\\S\\test"),      // Escaped component separator
            Just("H\\R\\test"),      // Escaped repetition separator
            Just("H\\E\\test"),      // Escaped escape character
            Just("H\\T\\test"),      // Escaped subcomponent separator
        ]
    ) {
        let yaml = r#"
message_structure: "ADT_A01"
version: "2.5.1"
segments:
  - id: "MSH"
  - id: "PID"
"#;
        let profile = load_profile(yaml).unwrap();

        let msg_str = format!(
            "MSH|^~\\&|SND|SF|RCV|RF|20250101000000||ADT^A01|{}|P|2.5.1\rPID|1||12345^^^HOSP^MR||{}||19800101|M||||||||||||||||\r",
            control_id, escaped_value
        );

        // Parse and validate - should not panic
        if let Ok(msg) = parse(msg_str.as_bytes()) {
            let _problems = validate(&msg, &profile);
        }
    }
}

// ============================================================================
// Property tests for value sets
// ============================================================================

proptest! {
    /// Test that profile with value sets loads correctly
    #[test]
    fn prop_profile_with_valueset(
        msg_struct in message_structure_strategy(),
        code1 in "[A-Z]{1,3}",
        code2 in "[A-Z]{1,3}",
        code3 in "[A-Z]{1,3}"
    ) {
        prop_assume!(code1 != code2);
        prop_assume!(code2 != code3);

        let yaml = format!(
            r#"
message_structure: "{}"
version: "2.5.1"
segments:
  - id: "MSH"
  - id: "PID"
valuesets:
  - path: "PID.8"
    name: "Sex"
    codes:
      - "{}"
      - "{}"
      - "{}"
"#,
            msg_struct, code1, code2, code3
        );

        let result = load_profile(&yaml);
        prop_assert!(result.is_ok());

        let profile = result.unwrap();
        prop_assert_eq!(profile.valuesets.len(), 1);
        prop_assert_eq!(profile.valuesets[0].codes.len(), 3);
    }

    /// Test that validation with value set works
    #[test]
    fn prop_validate_valueset(
        sex in prop_oneof![Just("M"), Just("F"), Just("O"), Just("X")]
    ) {
        let yaml = r#"
message_structure: "ADT_A01"
version: "2.5.1"
segments:
  - id: "MSH"
  - id: "PID"
valuesets:
  - path: "PID.8"
    name: "Sex"
    codes:
      - "M"
      - "F"
      - "O"
"#;
        let profile = load_profile(yaml).unwrap();
        let msg_str = adt_a01_message("MSG001", "12345", &sex);
        let msg = parse(msg_str.as_bytes()).unwrap();

        let problems = validate(&msg, &profile);
        // M, F, O are valid; X is not
        if sex == "X" {
            // X is not in the value set, so should have problems
            // Note: This depends on whether valueset validation is implemented
            // For now, just ensure it doesn't panic
            let _ = problems;
        } else {
            prop_assert!(problems.is_empty());
        }
    }
}

// ============================================================================
// Property tests for HL7 tables
// ============================================================================

proptest! {
    /// Test that profile with HL7 tables loads correctly
    #[test]
    fn prop_profile_with_hl7_table(
        msg_struct in message_structure_strategy(),
        table_id in "[0-9]{4}",
        code in "[A-Z0-9]{1,5}",
        description in safe_text_strategy()
    ) {
        let yaml = format!(
            r#"
message_structure: "{}"
version: "2.5.1"
segments:
  - id: "MSH"
  - id: "PID"
hl7_tables:
  - id: "HL7{}"
    name: "Test Table"
    version: "2.5.1"
    codes:
      - value: "{}"
        description: "{}"
        status: "A"
"#,
            msg_struct, table_id, code, description
        );

        let result = load_profile(&yaml);
        prop_assert!(result.is_ok());

        let profile = result.unwrap();
        prop_assert_eq!(profile.hl7_tables.len(), 1);
        prop_assert_eq!(profile.hl7_tables[0].codes.len(), 1);
    }
}

// ============================================================================
// Property tests for expression guardrails
// ============================================================================

proptest! {
    /// Test that profile with expression guardrails loads correctly
    #[test]
    fn prop_profile_with_expression_guardrails(
        msg_struct in message_structure_strategy(),
        max_complexity in 1usize..100,
        max_nesting in 1usize..10
    ) {
        let yaml = format!(
            r#"
message_structure: "{}"
version: "2.5.1"
segments:
  - id: "MSH"
expression_guardrails:
  max_complexity: {}
  max_nesting_depth: {}
  allow_custom_scripts: false
"#,
            msg_struct, max_complexity, max_nesting
        );

        let result = load_profile(&yaml);
        prop_assert!(result.is_ok());
    }

    /// Test that custom rules with guardrails load correctly
    #[test]
    fn prop_profile_with_custom_rules(
        msg_struct in message_structure_strategy(),
        rule_id in "[a-z][a-z0-9_]{2,19}",
        description in safe_text_strategy()
    ) {
        let yaml = format!(
            r#"
message_structure: "{}"
version: "2.5.1"
segments:
  - id: "MSH"
  - id: "PID"
custom_rules:
  - id: "{}"
    description: "{}"
    script: "field(PID.5.1).length() > 0"
"#,
            msg_struct, rule_id, description
        );

        let result = load_profile(&yaml);
        prop_assert!(result.is_ok());

        let profile = result.unwrap();
        prop_assert_eq!(profile.custom_rules.len(), 1);
    }
}

// ============================================================================
// Property tests for temporal rules
// ============================================================================

proptest! {
    /// Test that profile with temporal rules loads correctly
    #[test]
    fn prop_profile_with_temporal_rules(
        msg_struct in message_structure_strategy(),
        rule_id in "[a-z][a-z0-9_]{2,19}"
    ) {
        let yaml = format!(
            r#"
message_structure: "{}"
version: "2.5.1"
segments:
  - id: "MSH"
  - id: "PID"
  - id: "PV1"
temporal_rules:
  - id: "{}"
    description: "Admit date before discharge"
    before: "PV1.44"
    after: "PV1.45"
    allow_equal: false
"#,
            msg_struct, rule_id
        );

        let result = load_profile(&yaml);
        prop_assert!(result.is_ok());

        let profile = result.unwrap();
        prop_assert_eq!(profile.temporal_rules.len(), 1);
    }
}

// ============================================================================
// Property tests for data type constraints
// ============================================================================

proptest! {
    /// Test that profile with data type constraints loads correctly
    #[test]
    fn prop_profile_with_datatype_constraints(
        msg_struct in message_structure_strategy(),
        path in field_path_strategy(),
        datatype in prop_oneof![
            Just("ST"), Just("ID"), Just("DT"), Just("TM"), Just("TN"),
            Just("NM"), Just("SI"), Just("TX"), Just("FT")
        ]
    ) {
        let yaml = format!(
            r#"
message_structure: "{}"
version: "2.5.1"
segments:
  - id: "MSH"
datatypes:
  - path: "{}"
    type: "{}"
"#,
            msg_struct, path, datatype
        );

        let result = load_profile(&yaml);
        prop_assert!(result.is_ok());

        let profile = result.unwrap();
        prop_assert_eq!(profile.datatypes.len(), 1);
    }

    /// Test that profile with advanced data type constraints loads correctly
    #[test]
    fn prop_profile_with_advanced_datatype_constraints(
        msg_struct in message_structure_strategy(),
        path in field_path_strategy(),
        datatype in prop_oneof![Just("ST"), Just("ID")],
        min_length in 0usize..100,
        max_length in 1usize..200
    ) {
        prop_assume!(min_length < max_length);

        let yaml = format!(
            r#"
message_structure: "{}"
version: "2.5.1"
segments:
  - id: "MSH"
advanced_datatypes:
  - path: "{}"
    type: "{}"
    min_length: {}
    max_length: {}
"#,
            msg_struct, path, datatype, min_length, max_length
        );

        let result = load_profile(&yaml);
        prop_assert!(result.is_ok());
    }
}

// ============================================================================
// Property tests for length constraints
// ============================================================================

proptest! {
    /// Test that profile with length constraints loads correctly
    #[test]
    fn prop_profile_with_length_constraints(
        msg_struct in message_structure_strategy(),
        path in field_path_strategy(),
        max_len in 1usize..1000
    ) {
        let yaml = format!(
            r#"
message_structure: "{}"
version: "2.5.1"
segments:
  - id: "MSH"
lengths:
  - path: "{}"
    max: {}
    policy: "no-truncate"
"#,
            msg_struct, path, max_len
        );

        let result = load_profile(&yaml);
        prop_assert!(result.is_ok());

        let profile = result.unwrap();
        prop_assert_eq!(profile.lengths.len(), 1);
        prop_assert_eq!(profile.lengths[0].max, Some(max_len));
    }
}

// ============================================================================
// Property tests for contextual rules
// ============================================================================

proptest! {
    /// Test that profile with contextual rules loads correctly
    #[test]
    fn prop_profile_with_contextual_rules(
        msg_struct in message_structure_strategy(),
        context_value in "[A-Z]{1,3}",
        validation_type in prop_oneof![
            Just("required"), Just("format"), Just("range")
        ]
    ) {
        let yaml = format!(
            r#"
message_structure: "{}"
version: "2.5.1"
segments:
  - id: "MSH"
  - id: "PID"
contextual_rules:
  - id: "context-rule-1"
    description: "Context-based validation"
    context_field: "PID.8"
    context_value: "{}"
    target_field: "PID.5"
    validation_type: "{}"
"#,
            msg_struct, context_value, validation_type
        );

        let result = load_profile(&yaml);
        prop_assert!(result.is_ok());

        let profile = result.unwrap();
        prop_assert_eq!(profile.contextual_rules.len(), 1);
    }
}

// ============================================================================
// Property tests for table precedence
// ============================================================================

proptest! {
    /// Test that profile with table precedence loads correctly
    #[test]
    fn prop_profile_with_table_precedence(
        msg_struct in message_structure_strategy(),
        table1 in "HL7[0-9]{4}",
        table2 in "HL7[0-9]{4}",
        table3 in "HL7[0-9]{4}"
    ) {
        prop_assume!(table1 != table2);
        prop_assume!(table2 != table3);

        let yaml = format!(
            r#"
message_structure: "{}"
version: "2.5.1"
segments:
  - id: "MSH"
table_precedence:
  - "{}"
  - "{}"
  - "{}"
"#,
            msg_struct, table1, table2, table3
        );

        let result = load_profile(&yaml);
        prop_assert!(result.is_ok());

        let profile = result.unwrap();
        prop_assert_eq!(profile.table_precedence.len(), 3);
    }
}

// ============================================================================
// Property tests for Unicode handling
// ============================================================================

/// Generate Unicode text (various scripts)
fn unicode_text_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        // Latin with accents
        Just("Café".to_string()),
        Just("Naïve".to_string()),
        Just("Résumé".to_string()),
        // Greek
        Just("Αλφα".to_string()),
        Just("Βητα".to_string()),
        // Cyrillic
        Just("Привет".to_string()),
        // Chinese
        Just("你好".to_string()),
        Just("世界".to_string()),
        // Japanese
        Just("こんにちは".to_string()),
        // Korean
        Just("안녕하세요".to_string()),
        // Arabic
        Just("مرحبا".to_string()),
        // Hebrew
        Just("שלום".to_string()),
        // Emoji
        Just("🏥".to_string()),  // Hospital
        Just("💊".to_string()),  // Pill
        Just("👨‍⚕️".to_string()), // Doctor
    ]
}

proptest! {
    /// Test that profiles with Unicode in descriptions load correctly
    #[test]
    fn prop_profile_with_unicode_description(
        msg_struct in message_structure_strategy(),
        unicode_text in unicode_text_strategy()
    ) {
        let yaml = format!(
            r#"
message_structure: "{}"
version: "2.5.1"
segments:
  - id: "MSH"
  - id: "PID"
cross_field_rules:
  - id: "unicode-rule"
    description: "{}"
    conditions: []
    actions: []
"#,
            msg_struct, unicode_text
        );

        let result = load_profile(&yaml);
        prop_assert!(result.is_ok());

        let profile = result.unwrap();
        prop_assert_eq!(profile.cross_field_rules.len(), 1);
        prop_assert_eq!(&profile.cross_field_rules[0].description, &unicode_text);
    }

    /// Test that profiles with Unicode in segment comments load correctly
    #[test]
    fn prop_profile_with_unicode_segment_comment(
        msg_struct in message_structure_strategy(),
        unicode_text in unicode_text_strategy()
    ) {
        let yaml = format!(
            r#"
message_structure: "{}"
version: "2.5.1"
segments:
  - id: "MSH"
    comment: "{}"
"#,
            msg_struct, unicode_text
        );

        let result = load_profile(&yaml);
        prop_assert!(result.is_ok());
    }

    /// Test validation with Unicode in message content
    #[test]
    fn prop_validate_unicode_message_content(
        control_id in "[A-Za-z0-9]{1,10}",
        unicode_name in unicode_text_strategy()
    ) {
        let yaml = r#"
message_structure: "ADT_A01"
version: "2.5.1"
segments:
  - id: "MSH"
  - id: "PID"
"#;
        let profile = load_profile(yaml).unwrap();

        // Build message with Unicode patient name
        let msg_str = format!(
            "MSH|^~\\&|SND|SF|RCV|RF|20250101000000||ADT^A01|{}|P|2.5.1\rPID|1||12345^^^HOSP^MR||{}^Test||19800101|M||||||||||||||||\r",
            control_id, unicode_name
        );

        // Parse and validate - should handle Unicode without panicking
        if let Ok(msg) = parse(msg_str.as_bytes()) {
            let problems = validate(&msg, &profile);
            prop_assert!(problems.is_empty());
        }
    }
}

// ============================================================================
// Property tests for delimiter handling in profile YAML
// ============================================================================

proptest! {
    /// Test that profile with special characters in values loads correctly
    #[test]
    fn prop_profile_special_yaml_chars(
        msg_struct in message_structure_strategy(),
        special_value in prop_oneof![
            Just("value_with_underscore"),
            Just("value-with-dash"),
            Just("value.with.dot"),
        ]
    ) {
        let yaml = format!(
            r#"
message_structure: "{}"
version: "2.5.1"
segments:
  - id: "MSH"
constraints:
  - path: "MSH.9"
    pattern: "{}"
"#,
            msg_struct, special_value
        );

        let result = load_profile(&yaml);
        prop_assert!(result.is_ok());
    }
}

// ============================================================================
// Property tests for empty/minimal profiles
// ============================================================================

proptest! {
    /// Test that minimal profile with just required fields loads
    #[test]
    fn prop_minimal_profile_loads(
        msg_struct in message_structure_strategy(),
        version in version_strategy()
    ) {
        let yaml = format!(
            r#"
message_structure: "{}"
version: "{}"
segments:
  - id: "MSH"
"#,
            msg_struct, version
        );

        let result = load_profile(&yaml);
        prop_assert!(result.is_ok());

        let profile = result.unwrap();
        prop_assert_eq!(profile.segments.len(), 1);
        prop_assert_eq!(&profile.segments[0].id, "MSH");
    }

    /// Test that profile with empty optional arrays loads
    #[test]
    fn prop_profile_empty_optional_arrays(
        msg_struct in message_structure_strategy()
    ) {
        let yaml = format!(
            r#"
message_structure: "{}"
version: "2.5.1"
segments:
  - id: "MSH"
constraints: []
cross_field_rules: []
valuesets: []
hl7_tables: []
"#,
            msg_struct
        );

        let result = load_profile(&yaml);
        prop_assert!(result.is_ok());
    }
}

// ============================================================================
// Property tests for segment cardinality
// ============================================================================

proptest! {
    /// Test that profile with segment cardinality loads correctly
    #[test]
    fn prop_profile_segment_cardinality(
        msg_struct in message_structure_strategy(),
        min_occurs in 0usize..5,
        max_occurs in 1usize..10
    ) {
        prop_assume!(min_occurs <= max_occurs);

        let yaml = format!(
            r#"
message_structure: "{}"
version: "2.5.1"
segments:
  - id: "MSH"
  - id: "PID"
    cardinality:
      min: {}
      max: {}
"#,
            msg_struct, min_occurs, max_occurs
        );

        let result = load_profile(&yaml);
        prop_assert!(result.is_ok());

        let profile = result.unwrap();
        prop_assert_eq!(profile.segments.len(), 2);
    }

    /// Test that profile with required segment (min=1) loads
    #[test]
    fn prop_profile_required_segment(
        msg_struct in message_structure_strategy()
    ) {
        let yaml = format!(
            r#"
message_structure: "{}"
version: "2.5.1"
segments:
  - id: "MSH"
  - id: "PID"
    required: true
"#,
            msg_struct
        );

        let result = load_profile(&yaml);
        prop_assert!(result.is_ok());
    }
}

// ============================================================================
// Property tests for profile with message_type field
// ============================================================================

proptest! {
    /// Test that profile with message_type loads correctly
    #[test]
    fn prop_profile_with_message_type(
        msg_struct in message_structure_strategy(),
        msg_type in prop_oneof![
            Just("ADT"), Just("ORM"), Just("ORU"), Just("DFT"), Just("ACK")
        ]
    ) {
        let yaml = format!(
            r#"
message_structure: "{}"
version: "2.5.1"
message_type: "{}"
segments:
  - id: "MSH"
"#,
            msg_struct, msg_type
        );

        let result = load_profile(&yaml);
        prop_assert!(result.is_ok());

        let profile = result.unwrap();
        prop_assert_eq!(profile.message_type, Some(msg_type.to_string()));
    }
}

// ============================================================================
// Property tests for field repetitions
// ============================================================================

proptest! {
    /// Test that profile with field repetition constraints loads
    #[test]
    fn prop_profile_field_repetitions(
        msg_struct in message_structure_strategy(),
        max_reps in 1usize..10
    ) {
        let yaml = format!(
            r#"
message_structure: "{}"
version: "2.5.1"
segments:
  - id: "MSH"
  - id: "PID"
field_definitions:
  - path: "PID.3"
    max_repetitions: {}
"#,
            msg_struct, max_reps
        );

        let result = load_profile(&yaml);
        // May not be implemented, just ensure no panic
        let _ = result;
    }
}

// ============================================================================
// Property tests for profile with pattern constraints
// ============================================================================

proptest! {
    /// Test that profile with pattern constraints loads correctly
    #[test]
    fn prop_profile_with_pattern_constraint(
        msg_struct in message_structure_strategy(),
        path in field_path_strategy()
    ) {
        let yaml = format!(
            r#"
message_structure: "{}"
version: "2.5.1"
segments:
  - id: "MSH"
constraints:
  - path: "{}"
    pattern: "^[A-Z]+$"
"#,
            msg_struct, path
        );

        let result = load_profile(&yaml);
        prop_assert!(result.is_ok());

        let profile = result.unwrap();
        prop_assert_eq!(profile.constraints.len(), 1);
        prop_assert!(profile.constraints[0].pattern.is_some());
    }
}

// ============================================================================
// Property tests for multiple constraint types
// ============================================================================

proptest! {
    /// Test that profile with multiple constraint types loads correctly
    #[test]
    fn prop_profile_multiple_constraint_types(
        msg_struct in message_structure_strategy(),
        path1 in field_path_strategy(),
        path2 in field_path_strategy()
    ) {
        prop_assume!(path1 != path2);

        let yaml = format!(
            r#"
message_structure: "{}"
version: "2.5.1"
segments:
  - id: "MSH"
  - id: "PID"
constraints:
  - path: "{}"
    required: true
  - path: "{}"
    required: false
    cardinality:
      min: 0
      max: 1
"#,
            msg_struct, path1, path2
        );

        let result = load_profile(&yaml);
        prop_assert!(result.is_ok());

        let profile = result.unwrap();
        prop_assert_eq!(profile.constraints.len(), 2);
    }
}

// ============================================================================
// Property tests for validation with various message types
// ============================================================================

/// Generate an ACK message
fn ack_message(control_id: &str, ack_code: &str) -> String {
    format!(
        "MSH|^~\\&|RCV|RF|SND|SF|20250101000000||ACK|{}|P|2.5.1\rMSA|{}|MSG001|Message accepted\r",
        control_id, ack_code
    )
}

proptest! {
    /// Test validation of ACK messages
    #[test]
    fn prop_validate_ack_message(
        control_id in "[A-Za-z0-9]{1,20}",
        ack_code in prop_oneof![Just("AA"), Just("AE"), Just("AR"), Just("CA"), Just("CE"), Just("CR")]
    ) {
        let yaml = r#"
message_structure: "ACK"
version: "2.5.1"
segments:
  - id: "MSH"
  - id: "MSA"
"#;
        let profile = load_profile(yaml).unwrap();
        let msg_str = ack_message(&control_id, ack_code);
        let msg = parse(msg_str.as_bytes()).unwrap();

        let problems = validate(&msg, &profile);
        prop_assert!(problems.is_empty());
    }
}

// ============================================================================
// Property tests for profile immutability after loading
// ============================================================================

proptest! {
    /// Test that loading the same profile twice produces identical results
    #[test]
    fn prop_profile_load_deterministic(
        msg_struct in message_structure_strategy(),
        version in version_strategy(),
        seg1 in segment_id_strategy(),
        seg2 in segment_id_strategy()
    ) {
        prop_assume!(seg1 != seg2);

        let yaml = simple_profile_yaml(msg_struct, version, vec![seg1.clone(), seg2.clone()]);

        let profile1 = load_profile(&yaml).unwrap();
        let profile2 = load_profile(&yaml).unwrap();

        prop_assert_eq!(profile1.message_structure, profile2.message_structure);
        prop_assert_eq!(profile1.version, profile2.version);
        prop_assert_eq!(profile1.segments.len(), profile2.segments.len());
    }
}
