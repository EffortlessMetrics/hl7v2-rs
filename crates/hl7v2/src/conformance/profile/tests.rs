#![expect(
    clippy::uninlined_format_args,
    clippy::unwrap_used,
    reason = "Pre-existing profile unit test debt moved from hl7v2-prof; cleanup is separate from this behavior-preserving module collapse."
)]

#[cfg(test)]
mod unit_tests {
    use super::super::{
        Profile, compare_timestamps_for_before, load_profile, parse_hl7_ts_with_precision, validate,
    };

    use crate::parse;

    // Helper: build a tiny valid ADT A01 (PID.3 and PID.8 filled)
    fn adt_a01_msg() -> String {
        let mut s = String::new();
        s.push_str("MSH|^~\\&|SND|SF|RCV|RF|20250101000000||ADT^A01|MSG1|P|2.5.1\r");
        s.push_str("PID|1||123456^^^HOSP^MR||Doe^John||19800101|M||||||||||||||||\r");
        s
    }

    #[test]
    fn test_load_simple_profile() {
        let y = r#"
message_structure: "simple"
version: "2.5.1"
segments:
  - id: "PID"
constraints:
  - path: "PID.3"
    required: true
  - path: "PID.8"
    required: true
"#;
        let p: Profile = load_profile(y).unwrap();
        let msg = parse(adt_a01_msg().as_bytes()).unwrap();
        let probs = validate(&msg, &p);
        assert!(probs.is_empty(), "unexpected problems: {probs:?}");
    }

    #[test]
    fn test_cross_field_equals() {
        let y = r#"
message_structure: "xfield"
version: "2.5.1"
segments:
  - id: "PID"
cross_field_rules:
  - id: "test-rule"
    description: "Sex must be M"
    conditions:
      - field: "PID.8"
        operator: "eq"
        value: "M"
    actions: []
"#;
        let p: Profile = load_profile(y).unwrap();
        let msg = parse(adt_a01_msg().as_bytes()).unwrap();
        let probs = validate(&msg, &p);
        assert!(probs.is_empty(), "unexpected problems: {probs:?}");
    }

    #[test]
    fn test_temporal_before_with_partial_precision() {
        // Test message with different timestamp precisions
        let mut msg = String::new();
        msg.push_str("MSH|^~\\&|SND|SF|RCV|RF|20250101000000||ADT^A01|MSG1|P|2.5.1\r");
        msg.push_str("PID|1||123456^^^HOSP^MR||Doe^John||19800101|M||||||||||||||||\r");
        msg.push_str("PV1|1|O|CLINIC|||||||20241201\r"); // Date only
        msg.push_str("ORC|RE|||20241201103000\r"); // Full datetime

        let y = r#"
message_structure: "temporal"
version: "2.5.1"
segments:
  - id: "PID"
  - id: "PV1"
  - id: "ORC"
cross_field_rules:
  - id: "date-before-datetime"
    description: "PV1 date should be before ORC datetime"
    conditions:
      - field: "PV1.10"
        operator: "before"
        value: "ORC.4"
    actions: []
"#;

        let p: Profile = load_profile(y).unwrap();
        let message = parse(msg.as_bytes()).unwrap();
        let probs = validate(&message, &p);
        // This should pass because 20241201 (interpreted as 2024-12-01 00:00:00)
        // is before 20241201103000 (2024-12-01 10:30:00)
        assert!(probs.is_empty(), "unexpected problems: {probs:?}");
    }

    #[test]
    fn test_temporal_before_with_same_date_partial_precision() {
        // Test with same date but different precision
        let mut msg = String::new();
        msg.push_str("MSH|^~\\&|SND|SF|RCV|RF|20250101000000||ADT^A01|MSG1|P|2.5.1\r");
        msg.push_str("PID|1||123456^^^HOSP^MR||Doe^John||19800101|M||||||||||||||||\r");
        msg.push_str("PV1|1|O|CLINIC|||||||20241201\r"); // Date only
        msg.push_str("ORC|RE|||20241201\r"); // Same date only

        let y = r#"
message_structure: "temporal"
version: "2.5.1"
segments:
  - id: "PID"
  - id: "PV1"
  - id: "ORC"
cross_field_rules:
  - id: "date-before-date"
    description: "PV1 date should be before ORC date"
    validation_mode: "assert"
    conditions:
      - field: "PV1.10"
        operator: "before"
        value: "ORC.4"
    actions: []
"#;

        let p: Profile = load_profile(y).unwrap();
        let message = parse(msg.as_bytes()).unwrap();
        let probs = validate(&message, &p);
        // This should fail because 20241201 is not before 20241201 (they're equal)
        assert!(!probs.is_empty(), "expected problems but got none");
    }

    #[test]
    fn test_temporal_rule_checks_later_repetitions() {
        let mut msg = String::new();
        msg.push_str("MSH|^~\\&|SND|SF|RCV|RF|20250101000000||ADT^A01|MSG1|P|2.5.1\r");
        msg.push_str("PID|1||123456^^^HOSP^MR||Doe^John||19800101|M||||||||||||||||\r");
        msg.push_str("PV1|1|O|CLINIC|||||||20240101~20250101\r");
        msg.push_str("ORC|RE|||20240201~20240101\r");

        let y = r#"
message_structure: "temporal_rule_repetitions"
version: "2.5.1"
segments:
  - id: "PID"
  - id: "PV1"
  - id: "ORC"
temporal_rules:
  - id: "date-before-date"
    description: "PV1 date should be before ORC date"
    before: "PV1.10"
    after: "ORC.4"
"#;

        let p: Profile = load_profile(y).unwrap();
        let message = parse(msg.as_bytes()).unwrap();
        let probs = validate(&message, &p);

        assert_eq!(
            probs.len(),
            1,
            "expected temporal rule issue for later repetition: {probs:?}"
        );
        assert_eq!(probs[0].code, "TEMPORAL_RULE_VIOLATION");
        assert_eq!(probs[0].path.as_deref(), Some("PV1.10"));
    }

    #[test]
    fn test_temporal_rule_reports_invalid_later_repetition() {
        let mut msg = String::new();
        msg.push_str("MSH|^~\\&|SND|SF|RCV|RF|20250101000000||ADT^A01|MSG1|P|2.5.1\r");
        msg.push_str("PID|1||123456^^^HOSP^MR||Doe^John||19800101|M||||||||||||||||\r");
        msg.push_str("PV1|1|O|CLINIC|||||||20240101~BAD\r");
        msg.push_str("ORC|RE|||20240201~20250101\r");

        let y = r#"
message_structure: "temporal_rule_invalid_repetitions"
version: "2.5.1"
segments:
  - id: "PID"
  - id: "PV1"
  - id: "ORC"
temporal_rules:
  - id: "date-before-date"
    description: "PV1 date should be before ORC date"
    before: "PV1.10"
    after: "ORC.4"
"#;

        let p: Profile = load_profile(y).unwrap();
        let message = parse(msg.as_bytes()).unwrap();
        let probs = validate(&message, &p);

        assert_eq!(
            probs.len(),
            1,
            "expected invalid datetime issue for later repetition: {probs:?}"
        );
        assert_eq!(probs[0].code, "INVALID_DATETIME");
        assert_eq!(probs[0].path.as_deref(), Some("PV1.10"));
    }

    #[test]
    fn debug_compare_same_dates() {
        let date_str = "20241201";
        let ts1 = parse_hl7_ts_with_precision(date_str).unwrap();
        let ts2 = parse_hl7_ts_with_precision(date_str).unwrap();

        println!("ts1: {:?}, ts2: {:?}", ts1, ts2);

        let result = compare_timestamps_for_before(&ts1, &ts2);
        println!("compare_timestamps_for_before result: {}", result);

        // This should be false because they're equal
        assert!(!result, "Expected false for equal dates, but got true");
    }

    #[test]
    fn test_table_precedence() {
        let y = r#"
message_structure: "table_precedence"
version: "2.5.1"
segments:
  - id: "PID"
valuesets:
  - path: "PID.8"
    name: "HL70001"
hl7_tables:
  - id: "HL70001"
    name: "Administrative Sex"
    version: "2.5.1"
    codes:
      - value: "M"
        description: "Male"
        status: "A"
      - value: "F"
        description: "Female"
        status: "A"
table_precedence:
  - "HL70001"
"#;

        let p: Profile = load_profile(y).unwrap();
        let msg = parse(adt_a01_msg().as_bytes()).unwrap();
        let probs = validate(&msg, &p);
        // This should pass because "M" is in the HL70001 table
        assert!(probs.is_empty(), "unexpected problems: {probs:?}");
    }

    #[test]
    fn test_literal_in_constraint_checks_later_repetitions() {
        let y = r#"
message_structure: "oru_repetitions"
version: "2.5.1"
segments:
  - id: "OBX"
constraints:
  - path: "OBX.8"
    in: ["N", "H"]
"#;
        let msg = parse(
            b"MSH|^~\\&|LAB|FAC|EHR|FAC|20250101000000||ORU^R01|MSG1|P|2.5.1\r\
OBX|1|NM|WBC^White Blood Count||7.2|10^9/L|4.0-11.0|N~BAD|||F\r",
        )
        .unwrap();
        let p: Profile = load_profile(y).unwrap();
        let probs = validate(&msg, &p);

        assert_eq!(probs.len(), 1, "expected later repetition issue: {probs:?}");
        assert_eq!(probs[0].code, "VALUE_NOT_IN_CONSTRAINT");
        assert_eq!(probs[0].path.as_deref(), Some("OBX.8"));
    }

    #[test]
    fn test_inline_valueset_checks_later_repetitions() {
        let y = r#"
message_structure: "oru_repetitions"
version: "2.5.1"
segments:
  - id: "OBX"
valuesets:
  - path: "OBX.8"
    name: "abnormal_flags"
    codes: ["N", "H"]
"#;
        let msg = parse(
            b"MSH|^~\\&|LAB|FAC|EHR|FAC|20250101000000||ORU^R01|MSG1|P|2.5.1\r\
OBX|1|NM|WBC^White Blood Count||7.2|10^9/L|4.0-11.0|N~BAD|||F\r",
        )
        .unwrap();
        let p: Profile = load_profile(y).unwrap();
        let probs = validate(&msg, &p);

        assert_eq!(probs.len(), 1, "expected later repetition issue: {probs:?}");
        assert_eq!(probs[0].code, "VALUE_NOT_IN_SET");
        assert_eq!(probs[0].path.as_deref(), Some("OBX.8"));
    }

    #[test]
    fn test_hl7_table_valueset_checks_later_repetitions() {
        let y = r#"
message_structure: "oru_repetitions"
version: "2.5.1"
segments:
  - id: "OBX"
valuesets:
  - path: "OBX.8"
    name: "HL70078"
hl7_tables:
  - id: "HL70078"
    name: "Abnormal Flags"
    version: "2.5.1"
    codes:
      - value: "N"
        description: "Normal"
        status: "A"
      - value: "H"
        description: "High"
        status: "A"
table_precedence:
  - "HL70078"
"#;
        let msg = parse(
            b"MSH|^~\\&|LAB|FAC|EHR|FAC|20250101000000||ORU^R01|MSG1|P|2.5.1\r\
OBX|1|NM|WBC^White Blood Count||7.2|10^9/L|4.0-11.0|N~BAD|||F\r",
        )
        .unwrap();
        let p: Profile = load_profile(y).unwrap();
        let probs = validate(&msg, &p);

        assert_eq!(probs.len(), 1, "expected later repetition issue: {probs:?}");
        assert_eq!(probs[0].code, "VALUE_NOT_IN_HL7_TABLE");
        assert_eq!(probs[0].path.as_deref(), Some("OBX.8"));
    }

    #[test]
    fn test_advanced_datatype_checks_later_repetitions() {
        let y = r#"
message_structure: "oru_repetitions"
version: "2.5.1"
segments:
  - id: "OBX"
advanced_datatypes:
  - path: "OBX.8"
    type: "ID"
    max_length: 1
"#;
        let msg = parse(
            b"MSH|^~\\&|LAB|FAC|EHR|FAC|20250101000000||ORU^R01|MSG1|P|2.5.1\r\
OBX|1|NM|WBC^White Blood Count||7.2|10^9/L|4.0-11.0|N~BAD|||F\r",
        )
        .unwrap();
        let p: Profile = load_profile(y).unwrap();
        let probs = validate(&msg, &p);

        assert_eq!(
            probs.len(),
            1,
            "expected advanced datatype issue for later repetition: {probs:?}"
        );
        assert_eq!(probs[0].code, "VALUE_TOO_LONG");
        assert_eq!(probs[0].path.as_deref(), Some("OBX.8"));
    }

    #[test]
    fn test_pattern_constraint_checks_later_repetitions() {
        let y = r#"
message_structure: "oru_repetitions"
version: "2.5.1"
segments:
  - id: "OBX"
constraints:
  - path: "OBX.8"
    pattern: "^[A-Z]$"
"#;
        let msg = parse(
            b"MSH|^~\\&|LAB|FAC|EHR|FAC|20250101000000||ORU^R01|MSG1|P|2.5.1\r\
OBX|1|NM|WBC^White Blood Count||7.2|10^9/L|4.0-11.0|N~BAD|||F\r",
        )
        .unwrap();
        let p: Profile = load_profile(y).unwrap();
        let probs = validate(&msg, &p);

        assert_eq!(
            probs.len(),
            1,
            "expected pattern issue for later repetition: {probs:?}"
        );
        assert_eq!(probs[0].code, "PATTERN_MISMATCH");
        assert_eq!(probs[0].path.as_deref(), Some("OBX.8"));
    }

    #[test]
    fn test_component_constraint_checks_later_repetitions() {
        let y = r#"
message_structure: "oru_repetitions"
version: "2.5.1"
segments:
  - id: "OBX"
constraints:
  - path: "OBX.3"
    components:
      max: 2
  - path: "OBX.6"
    components:
      min: 2
"#;
        let msg = parse(
            b"MSH|^~\\&|LAB|FAC|EHR|FAC|20250101000000||ORU^R01|MSG1|P|2.5.1\r\
OBX|1|NM|WBC^White Blood Count~WBC^White^Blood||7.2|10^9/L~BAD|4.0-11.0|N|||F\r",
        )
        .unwrap();
        let p: Profile = load_profile(y).unwrap();
        let probs = validate(&msg, &p);

        assert_eq!(
            probs.len(),
            2,
            "expected component count issues for later repetitions: {probs:?}"
        );
        assert_eq!(probs[0].code, "TOO_MANY_COMPONENTS");
        assert_eq!(probs[0].path.as_deref(), Some("OBX.3"));
        assert_eq!(probs[1].code, "TOO_FEW_COMPONENTS");
        assert_eq!(probs[1].path.as_deref(), Some("OBX.6"));
    }

    #[test]
    fn test_condition_eq_checks_later_repetitions() {
        let y = r#"
message_structure: "adt_identifier_condition"
version: "2.5.1"
segments:
  - id: "PID"
constraints:
  - path: "PID.5"
    required: true
    when:
      eq: ["PID.3.5", "MR"]
"#;
        let msg = parse(
            b"MSH|^~\\&|ADT|FAC|EHR|FAC|20250101000000||ADT^A01|MSG1|P|2.5.1\r\
PID|1||111^^^HOSP^SS~222^^^HOSP^MR|||||||||||||||||||||||\r",
        )
        .unwrap();
        let p: Profile = load_profile(y).unwrap();
        let probs = validate(&msg, &p);

        assert_eq!(
            probs.len(),
            1,
            "expected conditional issue for later repetition: {probs:?}"
        );
        assert_eq!(probs[0].code, "MISSING_REQUIRED_FIELD");
        assert_eq!(probs[0].path.as_deref(), Some("PID.5"));
    }

    #[test]
    fn test_condition_eq_keeps_unqualified_path_scalar() {
        let y = r#"
message_structure: "adt_identifier_condition"
version: "2.5.1"
segments:
  - id: "PID"
constraints:
  - path: "PID.5"
    required: true
    when:
      eq: ["PID.3", "MR"]
"#;
        let msg = parse(
            b"MSH|^~\\&|ADT|FAC|EHR|FAC|20250101000000||ADT^A01|MSG1|P|2.5.1\r\
PID|1||111^^^HOSP^MR|||||||||||||||||||||||\r",
        )
        .unwrap();
        let p: Profile = load_profile(y).unwrap();
        let probs = validate(&msg, &p);

        assert!(
            probs.is_empty(),
            "condition should not match later components for an unqualified path: {probs:?}"
        );
    }

    #[test]
    fn test_condition_eq_supports_msh_field_separator() {
        let y = r#"
message_structure: "adt_msh_condition"
version: "2.5.1"
segments:
  - id: "PID"
constraints:
  - path: "PID.5"
    required: true
    when:
      eq: ["MSH.1", "|"]
"#;
        let msg = parse(
            b"MSH|^~\\&|ADT|FAC|EHR|FAC|20250101000000||ADT^A01|MSG1|P|2.5.1\r\
PID|1||111^^^HOSP^MR|||||||||||||||||||||||\r",
        )
        .unwrap();
        let p: Profile = load_profile(y).unwrap();
        let probs = validate(&msg, &p);

        assert_eq!(
            probs.len(),
            1,
            "expected delimiter-gated required-field issue: {probs:?}"
        );
        assert_eq!(probs[0].code, "MISSING_REQUIRED_FIELD");
        assert_eq!(probs[0].path.as_deref(), Some("PID.5"));
    }

    #[test]
    fn test_condition_eq_checks_later_repetitions_for_field_one() {
        let y = r#"
message_structure: "oru_field_one_condition"
version: "2.5.1"
segments:
  - id: "OBX"
constraints:
  - path: "OBX.5"
    required: true
    when:
      eq: ["OBX.1", "2"]
"#;
        let msg = parse(
            b"MSH|^~\\&|LAB|FAC|EHR|FAC|20250101000000||ORU^R01|MSG1|P|2.5.1\r\
OBX|1~2|NM|WBC^White Blood Count||\r",
        )
        .unwrap();
        let p: Profile = load_profile(y).unwrap();
        let probs = validate(&msg, &p);

        assert_eq!(
            probs.len(),
            1,
            "expected field-one later repetition condition issue: {probs:?}"
        );
        assert_eq!(probs[0].code, "MISSING_REQUIRED_FIELD");
        assert_eq!(probs[0].path.as_deref(), Some("OBX.5"));
    }

    #[test]
    fn test_custom_rule_length_checks_later_repetitions() {
        let y = r#"
message_structure: "oru_custom_repetitions"
version: "2.5.1"
segments:
  - id: "OBX"
custom_rules:
  - id: "abnormal-flag-length"
    description: ""
    script: "field(OBX.8).length() > 1"
"#;
        let msg = parse(
            b"MSH|^~\\&|LAB|FAC|EHR|FAC|20250101000000||ORU^R01|MSG1|P|2.5.1\r\
OBX|1|NM|WBC^White Blood Count||7.2|10^9/L|4.0-11.0|OK~N|||F\r",
        )
        .unwrap();
        let p: Profile = load_profile(y).unwrap();
        let probs = validate(&msg, &p);

        assert_eq!(
            probs.len(),
            1,
            "expected custom rule issue for later repetition: {probs:?}"
        );
        assert_eq!(probs[0].code, "CUSTOM_RULE_VIOLATION");
        assert_eq!(probs[0].path.as_deref(), Some("OBX.8"));
    }

    #[test]
    fn test_custom_rule_in_checks_later_repetitions() {
        let y = r#"
message_structure: "oru_custom_in_repetitions"
version: "2.5.1"
segments:
  - id: "OBX"
custom_rules:
  - id: "abnormal-flag-values"
    description: ""
    script: "field(OBX.8) in ['N', 'H']"
"#;
        let msg = parse(
            b"MSH|^~\\&|LAB|FAC|EHR|FAC|20250101000000||ORU^R01|MSG1|P|2.5.1\r\
OBX|1|NM|WBC^White Blood Count||7.2|10^9/L|4.0-11.0|N~BAD|||F\r",
        )
        .unwrap();
        let p: Profile = load_profile(y).unwrap();
        let probs = validate(&msg, &p);

        assert_eq!(
            probs.len(),
            1,
            "expected custom rule issue for later repetition: {probs:?}"
        );
        assert_eq!(probs[0].code, "CUSTOM_RULE_VIOLATION");
        assert_eq!(probs[0].path.as_deref(), Some("OBX.8"));
    }

    #[test]
    fn test_custom_rule_unary_predicates_check_later_repetitions() {
        let cases = [
            ("field(OBX.8).matches_regex('^N$')", "N~BAD"),
            ("field(OBX.8).starts_with('N')", "N~BAD"),
            ("field(OBX.8).ends_with('N')", "N~BAD"),
            ("field(OBX.8).is_numeric()", "7~BAD"),
            ("field(OBX.8).is_phone_number()", "555-123-4567~BAD"),
            ("field(OBX.8).is_email()", "ops@example.org~BAD"),
            ("field(OBX.8).is_ssn()", "123-45-6789~BAD"),
            ("field(OBX.8).is_valid_birth_date()", "19800101~29990101"),
            ("field(OBX.8) between 1 and 5", "3~9"),
        ];

        for (script, value) in cases {
            let y = format!(
                r#"
message_structure: "oru_custom_predicate_repetitions"
version: "2.5.1"
segments:
  - id: "OBX"
custom_rules:
  - id: "predicate"
    description: ""
    script: "{script}"
"#
            );
            let message = format!(
                "MSH|^~\\&|LAB|FAC|EHR|FAC|20250101000000||ORU^R01|MSG1|P|2.5.1\r\
OBX|1|ST|FLAG^Flag||text|unit|range|{value}|||F\r"
            );
            let msg = parse(message.as_bytes()).unwrap();
            let p: Profile = load_profile(&y).unwrap();
            let probs = validate(&msg, &p);

            assert_eq!(
                probs.len(),
                1,
                "expected custom rule issue for script {script}: {probs:?}"
            );
            assert_eq!(probs[0].code, "CUSTOM_RULE_VIOLATION");
            assert_eq!(probs[0].path.as_deref(), Some("OBX.8"));
        }
    }

    #[test]
    fn test_custom_rule_field_equality_checks_later_repetitions() {
        let y = r#"
message_structure: "oru_custom_pair_repetitions"
version: "2.5.1"
segments:
  - id: "OBX"
custom_rules:
  - id: "flag-pairs-match"
    description: ""
    script: "field(OBX.8) == field(OBX.9)"
"#;
        let msg = parse(
            b"MSH|^~\\&|LAB|FAC|EHR|FAC|20250101000000||ORU^R01|MSG1|P|2.5.1\r\
OBX|1|ST|FLAG^Flag||text|unit|range|N~BAD|N~N||F\r",
        )
        .unwrap();
        let p: Profile = load_profile(y).unwrap();
        let probs = validate(&msg, &p);

        assert_eq!(
            probs.len(),
            1,
            "expected custom rule issue for later paired repetition: {probs:?}"
        );
        assert_eq!(probs[0].code, "CUSTOM_RULE_VIOLATION");
        assert_eq!(probs[0].path.as_deref(), Some("OBX.8"));
    }

    #[test]
    fn test_custom_rule_age_range_checks_later_repetitions() {
        let y = r#"
message_structure: "oru_custom_pair_repetitions"
version: "2.5.1"
segments:
  - id: "OBX"
custom_rules:
  - id: "age-range"
    description: ""
    script: "is_valid_age_range(field(OBX.7), field(OBX.8))"
"#;
        let msg = parse(
            b"MSH|^~\\&|LAB|FAC|EHR|FAC|20250101000000||ORU^R01|MSG1|P|2.5.1\r\
OBX|1|ST|DATES^Dates||text|unit|19800101~20250101|20200101~20240101||F\r",
        )
        .unwrap();
        let p: Profile = load_profile(y).unwrap();
        let probs = validate(&msg, &p);

        assert_eq!(
            probs.len(),
            1,
            "expected custom rule issue for later age-range repetition: {probs:?}"
        );
        assert_eq!(probs[0].code, "CUSTOM_RULE_VIOLATION");
        assert_eq!(probs[0].path.as_deref(), Some("OBX.7"));
    }

    #[test]
    fn test_custom_rule_length_keeps_unqualified_path_scalar() {
        let y = r#"
message_structure: "oru_custom_scalar"
version: "2.5.1"
segments:
  - id: "OBX"
custom_rules:
  - id: "identifier-length"
    description: ""
    script: "field(OBX.3).length() > 2"
"#;
        let msg = parse(
            b"MSH|^~\\&|LAB|FAC|EHR|FAC|20250101000000||ORU^R01|MSG1|P|2.5.1\r\
OBX|1|NM|WBC^X||7.2|10^9/L|4.0-11.0|N|||F\r",
        )
        .unwrap();
        let p: Profile = load_profile(y).unwrap();
        let probs = validate(&msg, &p);

        assert!(
            probs.is_empty(),
            "custom rule should not match later components for an unqualified path: {probs:?}"
        );
    }

    #[test]
    fn test_expression_guardrails() {
        let y = r#"
message_structure: "expression_guardrails"
version: "2.5.1"
segments:
  - id: "PID"
expression_guardrails:
  max_complexity: 10
  allowed_functions:
    - "length"
    - "matches_regex"
  prohibited_fields: []
  max_nesting_depth: 3
  allow_field_comparisons: true
custom_rules:
  - id: "simple_rule"
    description: "PID.5.1 should be at least 2 characters"
    script: "field(PID.5.1).length() > 1"
"#;

        let p: Profile = load_profile(y).unwrap();
        let msg = parse(adt_a01_msg().as_bytes()).unwrap();
        let probs = validate(&msg, &p);
        // This should pass because "Doe" has more than 1 character
        assert!(probs.is_empty(), "unexpected problems: {probs:?}");
    }
}

#[cfg(test)]
mod profile_load_error_tests {
    use super::super::{ProfileLoadError, load_profile_checked, load_profile_with_inheritance};

    #[test]
    fn test_load_profile_checked_valid() {
        let y = r#"
message_structure: "ADT_A01"
version: "2.5.1"
segments:
  - id: "MSH"
"#;
        let result = load_profile_checked(y);
        assert!(result.is_ok());
        let profile = result.unwrap();
        assert_eq!(profile.message_structure, "ADT_A01");
        assert_eq!(profile.version, "2.5.1");
    }

    #[test]
    fn test_load_profile_checked_invalid_yaml() {
        let y = "this is not: valid:: yaml:::";
        let result = load_profile_checked(y);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ProfileLoadError::YamlParse(_)));
    }

    #[test]
    fn test_profile_load_error_display() {
        // Test YamlParse variant
        let err = ProfileLoadError::YamlParse("unexpected token".to_string());
        assert_eq!(format!("{}", err), "YAML parse error: unexpected token");

        // Test MissingField variant
        let err = ProfileLoadError::MissingField {
            field: "message_structure".to_string(),
        };
        assert_eq!(
            format!("{}", err),
            "Missing required field: message_structure"
        );

        // Test InvalidValue variant
        let err = ProfileLoadError::InvalidValue {
            field: "version".to_string(),
            details: "must be a valid HL7 version".to_string(),
        };
        assert_eq!(
            format!("{}", err),
            "Invalid value for field 'version': must be a valid HL7 version"
        );

        // Test Io variant
        let err = ProfileLoadError::Io("file not found".to_string());
        assert_eq!(format!("{}", err), "IO error: file not found");

        // Test InheritanceCycle variant
        let err = ProfileLoadError::InheritanceCycle("A -> B -> A".to_string());
        assert_eq!(
            format!("{}", err),
            "Profile inheritance cycle detected: A -> B -> A"
        );

        // Test ParentNotFound variant
        let err = ProfileLoadError::ParentNotFound("base_profile".to_string());
        assert_eq!(format!("{}", err), "Parent profile not found: base_profile");
    }

    #[test]
    fn test_profile_load_error_from_yaml_error() {
        let yaml_err = serde_yaml::from_str::<serde_yaml::Value>("invalid: ::: yaml").unwrap_err();
        let load_err: ProfileLoadError = yaml_err.into();
        assert!(matches!(load_err, ProfileLoadError::YamlParse(_)));
    }

    #[test]
    fn test_profile_load_error_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let load_err: ProfileLoadError = io_err.into();
        assert!(matches!(load_err, ProfileLoadError::Io(_)));
    }

    #[test]
    fn test_load_profile_with_inheritance_rejects_parent_cycle() {
        let child = r#"
message_structure: "CHILD"
version: "2.5.1"
parent: "base"
segments:
  - id: "MSH"
"#;

        let result = load_profile_with_inheritance(child, |name| match name {
            "base" => load_profile_checked(
                r#"
message_structure: "BASE"
version: "2.5.1"
parent: "loop"
segments:
  - id: "MSH"
"#,
            ),
            "loop" => load_profile_checked(
                r#"
message_structure: "LOOP"
version: "2.5.1"
parent: "base"
segments:
  - id: "MSH"
"#,
            ),
            other => Err(ProfileLoadError::ParentNotFound(other.to_string())),
        });

        assert!(matches!(
            result,
            Err(ProfileLoadError::InheritanceCycle(ref cycle))
                if cycle == "base -> loop -> base"
        ));
    }
}

#[cfg(test)]
mod profile_lint_tests {
    use super::super::{ProfileLintSeverity, lint_profile_yaml};

    #[test]
    fn test_lint_profile_yaml_accepts_minimal_profile() {
        let y = r#"
message_structure: "ADT_A01"
version: "2.5.1"
segments:
  - id: "MSH"
constraints:
  - path: "MSH.9"
    required: true
"#;

        let report = lint_profile_yaml(y);

        assert!(report.valid, "unexpected lint report: {report:?}");
        assert_eq!(report.issue_count, 0);

        let report_v2 = report.to_v2("hl7v2", "1.3.0");
        assert_eq!(report_v2.schema_version, "2");
        assert_eq!(report_v2.tool_name, "hl7v2");
        assert_eq!(report_v2.tool_version, "1.3.0");
        assert_eq!(report_v2.report.issue_count, 0);
    }

    #[test]
    fn test_lint_profile_yaml_accepts_diagnostic_segment_repetition_paths() {
        let y = r#"
message_structure: "ORU_R01"
version: "2.5.1"
segments:
  - id: "MSH"
  - id: "OBX"
constraints:
  - path: "OBX[3]-5"
    required: true
valuesets:
  - path: "OBX[3]-2"
    name: "ValueType"
    codes: ["ST", "NM"]
datatypes:
  - path: "OBX[3]-14"
    type: "TS"
"#;

        let report = lint_profile_yaml(y);

        assert!(report.valid, "unexpected lint report: {report:?}");
        assert_eq!(report.issue_count, 0);
    }

    #[test]
    fn test_lint_profile_yaml_reports_structural_errors() {
        let y = r#"
message_structure: ""
version: "2.5.1"
segments:
  - id: ""
constraints:
  - path: "PID.x"
    pattern: "["
cross_field_rules:
  - id: "rule-1"
    validation_mode: "sometimes"
    description: "invalid mode"
    conditions:
      - field: "PID.3"
        operator: "matches_regex"
    actions:
      - field: "PID.5"
        action: "invent"
        valueset: "missing"
table_precedence:
  - "HL79999"
"#;

        let report = lint_profile_yaml(y);
        let codes: Vec<&str> = report
            .issues
            .iter()
            .map(|issue| issue.code.as_str())
            .collect();

        assert!(!report.valid);
        assert!(codes.contains(&"empty_message_structure"));
        assert!(codes.contains(&"empty_segment_id"));
        assert!(codes.contains(&"invalid_hl7_path"));
        assert!(codes.contains(&"invalid_constraint_pattern"));
        assert!(codes.contains(&"unknown_cross_field_validation_mode"));
        assert!(codes.contains(&"missing_rule_condition_regex"));
        assert!(codes.contains(&"unknown_rule_action"));
        assert!(codes.contains(&"unknown_action_valueset"));
        assert!(codes.contains(&"unknown_table_precedence_entry"));
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.severity == ProfileLintSeverity::Error)
        );
    }

    #[test]
    fn test_lint_profile_yaml_sanitizes_yaml_error_messages() {
        let y = "patient_name: Jane Secret\nmrn: MRN-SECRET-123\ninvalid: yaml: structure:";

        let report = lint_profile_yaml(y);

        assert!(!report.valid);
        assert_eq!(report.error_count, 1);
        assert_eq!(report.issues[0].code, "yaml_parse_error");
        assert!(
            report.issues[0]
                .message
                .contains("profile YAML could not be parsed")
        );
        assert!(!report.issues[0].message.contains("Jane Secret"));
        assert!(!report.issues[0].message.contains("MRN-SECRET-123"));
        assert!(!report.issues[0].message.contains(y));
    }

    #[test]
    fn test_lint_profile_yaml_warns_for_ignored_top_level_keys() {
        let y = r#"
message_structure: "ADT_A01"
version: "2.5.1"
segments:
  - id: "MSH"
rules: []
description: "profile metadata"
"#;

        let report = lint_profile_yaml(y);

        assert!(report.valid, "warnings should not fail profile lint");
        assert_eq!(report.warning_count, 1);
        assert!(
            report
                .issues
                .iter()
                .all(|issue| issue.severity == ProfileLintSeverity::Warning)
        );
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.path.as_deref() == Some("rules"))
        );
    }
}
