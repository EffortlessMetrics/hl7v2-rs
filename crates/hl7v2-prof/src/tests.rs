#[cfg(test)]
mod tests {
    use crate::{load_profile, validate, Profile, parse_hl7_ts_with_precision, compare_timestamps_for_before, ExpressionGuardrails};
    use hl7v2_core::parse;
    use chrono::NaiveDate;

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

    #[test]
    fn test_issue_display() {
        use crate::{Issue, Severity};

        let issue = Issue {
            code: "ERR_001",
            severity: Severity::Error,
            path: Some("PID.5.1".to_string()),
            detail: "Value too long".to_string(),
        };

        let display = format!("{}", issue);
        assert_eq!(display, "[ERROR] ERR_001: Value too long (at PID.5.1)");

        let warning = Issue {
            code: "WARN_001",
            severity: Severity::Warning,
            path: None,
            detail: "Something suspicious".to_string(),
        };

        let display_warn = format!("{}", warning);
        assert_eq!(display_warn, "[WARNING] WARN_001: Something suspicious (at unknown)");
    }
}
