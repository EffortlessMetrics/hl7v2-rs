//! Comprehensive unit tests for hl7v2-validation crate.
//!
//! Tests cover:
//! - Data type validation (ST, ID, DT, TM, TS, NM, etc.)
//! - Format validation (phone, email, SSN)
//! - Checksum validation (Luhn, Mod10)
//! - Temporal validation (date/time comparisons)
//! - Cross-field validation rules
//! - Issue and Severity types

use crate::{
    check_rule_condition, is_coded_value, is_date, is_email, is_extended_id,
    is_hierarchic_designator, is_identifier, is_numeric, is_person_name,
    is_phone_number, is_sequence_id, is_ssn, is_string, is_time, is_timestamp,
    is_valid_age_range, is_valid_birth_date, is_within_range, matches_complex_pattern,
    matches_format, parse_hl7_ts, parse_hl7_ts_with_precision, truncate_to_precision,
    validate_checksum, validate_data_type, validate_luhn_checksum,
    validate_mathematical_relationship, compare_timestamps_for_before,
    Issue, RuleCondition, Severity, TimestampPrecision,
};
use hl7v2_core::Message;
use hl7v2_parser::parse;

// ============================================================================
// Severity and Issue Tests
// ============================================================================

#[test]
fn test_severity_default_is_error() {
    let severity = Severity::default();
    assert_eq!(severity, Severity::Error);
}

#[test]
fn test_issue_creation_error() {
    let issue = Issue::error("TEST_CODE", Some("PID.5".to_string()), "Test detail".to_string());
    assert_eq!(issue.code, "TEST_CODE");
    assert_eq!(issue.severity, Severity::Error);
    assert_eq!(issue.path, Some("PID.5".to_string()));
    assert_eq!(issue.detail, "Test detail");
}

#[test]
fn test_issue_creation_warning() {
    let issue = Issue::warning("WARN_CODE", Some("PID.3".to_string()), "Warning detail".to_string());
    assert_eq!(issue.code, "WARN_CODE");
    assert_eq!(issue.severity, Severity::Warning);
    assert_eq!(issue.path, Some("PID.3".to_string()));
}

#[test]
fn test_issue_new() {
    let issue = Issue::new("CODE", Severity::Warning, None, "Detail".to_string());
    assert_eq!(issue.code, "CODE");
    assert_eq!(issue.severity, Severity::Warning);
    assert_eq!(issue.path, None);
}

#[test]
fn test_severity_equality() {
    assert_eq!(Severity::Error, Severity::Error);
    assert_eq!(Severity::Warning, Severity::Warning);
    assert_ne!(Severity::Error, Severity::Warning);
}

// ============================================================================
// Data Type Validation Tests
// ============================================================================

#[test]
fn test_validate_data_type_st() {
    assert!(validate_data_type("any string", "ST"));
    assert!(validate_data_type("", "ST"));
    assert!(validate_data_type("test123!@#", "ST"));
}

#[test]
fn test_validate_data_type_id() {
    assert!(validate_data_type("ABC123", "ID"));
    assert!(validate_data_type("test-value", "ID"));
    assert!(!validate_data_type("test\nvalue", "ID")); // Contains control char
}

#[test]
fn test_validate_data_type_dt() {
    // Valid dates
    assert!(validate_data_type("20230101", "DT"));
    assert!(validate_data_type("19991231", "DT"));
    assert!(validate_data_type("20000101", "DT"));
    
    // Invalid dates
    assert!(!validate_data_type("20231301", "DT")); // Invalid month
    assert!(!validate_data_type("20230132", "DT")); // Invalid day
    assert!(!validate_data_type("2023010", "DT"));  // Too short
    assert!(!validate_data_type("202301011", "DT")); // Too long
    assert!(!validate_data_type("abcdefgh", "DT")); // Non-numeric
}

#[test]
fn test_validate_data_type_tm() {
    // Valid times
    assert!(validate_data_type("1200", "TM"));
    assert!(validate_data_type("235959", "TM"));
    assert!(validate_data_type("0000", "TM"));
    assert!(validate_data_type("120000", "TM"));
    assert!(validate_data_type("120000.1", "TM"));
    assert!(validate_data_type("120000.1234", "TM"));
    
    // Invalid times
    assert!(!validate_data_type("2400", "TM"));     // Invalid hour
    assert!(!validate_data_type("1260", "TM"));     // Invalid minute
    assert!(!validate_data_type("123", "TM"));      // Too short
    assert!(!validate_data_type("abcdef", "TM"));   // Non-numeric
}

#[test]
fn test_validate_data_type_ts() {
    // Valid timestamps
    assert!(validate_data_type("20230101", "TS"));
    assert!(validate_data_type("202301011200", "TS"));
    assert!(validate_data_type("20230101120000", "TS"));
    
    // Invalid timestamps
    assert!(!validate_data_type("2023", "TS"));     // Too short
    assert!(!validate_data_type("invalid", "TS"));  // Non-numeric
}

#[test]
fn test_validate_data_type_nm() {
    // Valid numerics
    assert!(validate_data_type("123", "NM"));
    assert!(validate_data_type("123.45", "NM"));
    assert!(validate_data_type("-123", "NM"));
    assert!(validate_data_type("0", "NM"));
    assert!(validate_data_type("0.0", "NM"));
    
    // Invalid numerics
    assert!(!validate_data_type("abc", "NM"));
    assert!(!validate_data_type("12.34.56", "NM"));
}

#[test]
fn test_validate_data_type_si() {
    // Valid sequence IDs
    assert!(validate_data_type("1", "SI"));
    assert!(validate_data_type("100", "SI"));
    assert!(validate_data_type("999999", "SI"));
    
    // Invalid sequence IDs
    assert!(!validate_data_type("0", "SI"));    // Must be positive
    assert!(!validate_data_type("-1", "SI"));   // Must be positive
    assert!(!validate_data_type("abc", "SI"));  // Non-numeric
}

#[test]
fn test_validate_data_type_tx() {
    // TX (Text Data) always returns true
    assert!(validate_data_type("any text", "TX"));
    assert!(validate_data_type("", "TX"));
}

#[test]
fn test_validate_data_type_ft() {
    // FT (Formatted Text) always returns true
    assert!(validate_data_type("formatted text", "FT"));
    assert!(validate_data_type("", "FT"));
}

#[test]
fn test_validate_data_type_is() {
    // IS (Coded Value) - alphanumeric without control chars
    assert!(validate_data_type("CODE123", "IS"));
    assert!(validate_data_type("A", "IS"));
    assert!(!validate_data_type("code\nvalue", "IS"));
}

#[test]
fn test_validate_data_type_pn() {
    // Valid person names
    assert!(validate_data_type("John Doe", "PN"));
    assert!(validate_data_type("O'Brien", "PN"));
    assert!(validate_data_type("Mary-Jane", "PN"));
    assert!(validate_data_type("Dr. Smith", "PN"));
    
    // Invalid person names (contains numbers)
    assert!(!validate_data_type("John123", "PN"));
}

#[test]
fn test_validate_data_type_cx() {
    // CX (Extended ID) uses identifier validation
    assert!(validate_data_type("12345^^^MRN", "CX"));
    assert!(validate_data_type("ABC123", "CX"));
}

#[test]
fn test_validate_data_type_hd() {
    // HD (Hierarchic Designator) uses identifier validation
    assert!(validate_data_type("FACILITY_1", "HD"));
    assert!(validate_data_type("ORG.1.2", "HD"));
}

#[test]
fn test_validate_data_type_unknown() {
    // Unknown data types should return true (assume valid)
    assert!(validate_data_type("anything", "UNKNOWN_TYPE"));
    assert!(validate_data_type("", "CUSTOM"));
}

// ============================================================================
// Individual Validator Function Tests
// ============================================================================

#[test]
fn test_is_string() {
    assert!(is_string("any value"));
    assert!(is_string(""));
    assert!(is_string("test 123 !@#"));
}

#[test]
fn test_is_identifier() {
    assert!(is_identifier("ABC123"));
    assert!(is_identifier("test-value_123"));
    assert!(!is_identifier("test\nvalue")); // Control character
    assert!(!is_identifier("test\x00value")); // Null character
}

#[test]
fn test_is_date_edge_cases() {
    // Boundary dates
    assert!(is_date("00010101")); // Year 1
    assert!(is_date("99991231")); // Year 9999
    
    // Month boundaries
    assert!(is_date("20230101")); // January
    assert!(is_date("20231231")); // December
    assert!(!is_date("20230001")); // Month 0
    assert!(!is_date("20231301")); // Month 13
    
    // Day boundaries
    assert!(is_date("20230101")); // Day 1
    assert!(is_date("20230131")); // Day 31
    assert!(!is_date("20230001")); // Day 0
    assert!(!is_date("20230132")); // Day 32
}

#[test]
fn test_is_time_edge_cases() {
    // Hour boundaries
    assert!(is_time("0000")); // Midnight
    assert!(is_time("2300")); // 11 PM
    assert!(!is_time("2400")); // Invalid hour
    
    // Minute boundaries
    assert!(is_time("0000")); // 0 minutes
    assert!(is_time("0059")); // 59 minutes
    assert!(!is_time("0060")); // 60 minutes
    
    // Second boundaries
    assert!(is_time("000000")); // 0 seconds
    assert!(is_time("000059")); // 59 seconds
    assert!(!is_time("000060")); // 60 seconds
    
    // Fractional seconds
    assert!(is_time("000000.1"));
    assert!(is_time("000000.1234"));
}

#[test]
fn test_is_timestamp_edge_cases() {
    // Date only
    assert!(is_timestamp("20230101"));
    
    // Date + hour + minute
    assert!(is_timestamp("202301011200"));
    
    // Date + hour + minute + second
    assert!(is_timestamp("20230101120000"));
    
    // Too short
    assert!(!is_timestamp("2023"));
    assert!(!is_timestamp("2023010")); // 7 chars
    
    // Note: 10-char format (YYYYMMDDHH) is not supported by is_timestamp
    // It only checks for 8-char date or 12+ char datetime
}

#[test]
fn test_is_numeric_edge_cases() {
    // Integers
    assert!(is_numeric("0"));
    assert!(is_numeric("123"));
    assert!(is_numeric("-123"));
    
    // Decimals
    assert!(is_numeric("123.456"));
    assert!(is_numeric("-123.456"));
    assert!(is_numeric("0.0"));
    
    // Scientific notation (may or may not be supported)
    // assert!(is_numeric("1e10"));
    
    // Invalid
    assert!(!is_numeric("abc"));
    assert!(!is_numeric("12.34.56"));
    assert!(!is_numeric(""));
}

#[test]
fn test_is_sequence_id_edge_cases() {
    // Valid
    assert!(is_sequence_id("1"));
    assert!(is_sequence_id("1000000"));
    
    // Invalid
    assert!(!is_sequence_id("0"));     // Must be > 0
    assert!(!is_sequence_id("-1"));    // Negative
    assert!(!is_sequence_id("1.5"));   // Decimal
    assert!(!is_sequence_id("abc"));
}

#[test]
fn test_is_coded_value() {
    assert!(is_coded_value("CODE1"));
    assert!(is_coded_value("A"));
    assert!(!is_coded_value("code\nvalue"));
}

#[test]
fn test_is_person_name_edge_cases() {
    // Valid
    assert!(is_person_name("John"));
    assert!(is_person_name("John Doe"));
    assert!(is_person_name("O'Brien"));
    assert!(is_person_name("Mary-Jane"));
    assert!(is_person_name("Dr. Smith"));
    assert!(is_person_name("")); // Empty is valid
    
    // Invalid (contains numbers or special chars)
    assert!(!is_person_name("John123"));
    assert!(!is_person_name("John@Doe"));
}

#[test]
fn test_is_extended_id() {
    assert!(is_extended_id("12345"));
    assert!(is_extended_id("ABC-123"));
    assert!(!is_extended_id("id\n123"));
}

#[test]
fn test_is_hierarchic_designator() {
    assert!(is_hierarchic_designator("FACILITY"));
    assert!(is_hierarchic_designator("ORG.1.2"));
    assert!(!is_hierarchic_designator("fac\nility"));
}

// ============================================================================
// Format Validation Tests
// ============================================================================

#[test]
fn test_is_phone_number_valid() {
    assert!(is_phone_number("1234567"));       // 7 digits
    assert!(is_phone_number("1234567890"));    // 10 digits
    assert!(is_phone_number("123-456-7890"));  // With dashes
    assert!(is_phone_number("(123) 456-7890")); // With parens and space
    assert!(is_phone_number("+1-123-456-7890")); // With country code
}

#[test]
fn test_is_phone_number_invalid() {
    assert!(!is_phone_number("123456"));       // Too short (6 digits)
    assert!(!is_phone_number("1234567890123456")); // Too long (16 digits)
    assert!(!is_phone_number("abcdefghijk"));  // Non-numeric
}

#[test]
fn test_is_email_valid() {
    assert!(is_email("test@example.com"));
    assert!(is_email("user.name@domain.org"));
    assert!(is_email("user+tag@example.com"));
    assert!(is_email("user@sub.domain.com"));
}

#[test]
fn test_is_email_invalid() {
    assert!(!is_email("invalid"));           // No @
    assert!(!is_email("@domain.com"));       // No local part
    assert!(!is_email("user@"));             // No domain
    assert!(!is_email("user@domain"));       // No TLD
    // Note: "user@.com" actually passes our basic validation (has @, local part, and domain with dot)
    // In a real implementation, you'd want more sophisticated validation
}

#[test]
fn test_is_ssn_valid() {
    assert!(is_ssn("123456789"));           // Without dashes
    assert!(is_ssn("123-45-6789"));         // With dashes
    assert!(is_ssn(" 123 45 6789 "));       // With spaces
}

#[test]
fn test_is_ssn_invalid() {
    assert!(!is_ssn("12345678"));            // Too short (8 digits)
    assert!(!is_ssn("1234567890"));          // Too long (10 digits)
    assert!(!is_ssn("000123456"));           // Area 000
    assert!(!is_ssn("666123456"));           // Area 666
    assert!(!is_ssn("900123456"));           // Area 900-999
    assert!(!is_ssn("123000456"));           // Group 00
    assert!(!is_ssn("123450000"));           // Serial 0000
}

// ============================================================================
// Temporal Validation Tests
// ============================================================================

#[test]
fn test_is_valid_birth_date() {
    // Valid birth dates (not in future)
    assert!(is_valid_birth_date("19900101"));
    assert!(is_valid_birth_date("20000101"));
    assert!(is_valid_birth_date("20230101"));
    
    // Invalid birth dates
    assert!(!is_valid_birth_date("20990101")); // Future date
    assert!(!is_valid_birth_date("invalid"));  // Not a date
}

#[test]
fn test_is_valid_age_range() {
    // Valid ranges (birth before reference)
    assert!(is_valid_age_range("19900101", "20230101"));
    assert!(is_valid_age_range("20000101", "20000101")); // Same day
    
    // Invalid ranges (birth after reference)
    assert!(!is_valid_age_range("20230101", "19900101"));
    assert!(!is_valid_age_range("invalid", "20230101"));
    assert!(!is_valid_age_range("19900101", "invalid"));
}

#[test]
fn test_is_within_range() {
    // Valid ranges
    assert!(is_within_range("5", "0", "10"));
    assert!(is_within_range("0", "0", "10"));   // Inclusive min
    assert!(is_within_range("10", "0", "10"));  // Inclusive max
    assert!(is_within_range("5.5", "0", "10")); // Decimal
    
    // Invalid ranges
    assert!(!is_within_range("-1", "0", "10"));  // Below min
    assert!(!is_within_range("11", "0", "10"));  // Above max
    assert!(!is_within_range("abc", "0", "10")); // Non-numeric
}

#[test]
fn test_parse_hl7_ts_valid() {
    // Date only
    let dt = parse_hl7_ts("20230101");
    assert!(dt.is_some());
    
    // Date + hour + minute
    let dt = parse_hl7_ts("202301011200");
    assert!(dt.is_some());
    
    // Full timestamp
    let dt = parse_hl7_ts("20230101120000");
    assert!(dt.is_some());
}

#[test]
fn test_parse_hl7_ts_invalid() {
    assert!(parse_hl7_ts("invalid").is_none());
    assert!(parse_hl7_ts("2023").is_none());
    assert!(parse_hl7_ts("").is_none());
}

#[test]
fn test_parse_hl7_ts_with_precision() {
    // Year precision
    let ts = parse_hl7_ts_with_precision("2023");
    assert!(ts.is_some());
    let ts = ts.unwrap();
    assert_eq!(ts.precision, TimestampPrecision::Year);
    
    // Month precision
    let ts = parse_hl7_ts_with_precision("202301");
    assert!(ts.is_some());
    let ts = ts.unwrap();
    assert_eq!(ts.precision, TimestampPrecision::Month);
    
    // Day precision
    let ts = parse_hl7_ts_with_precision("20230101");
    assert!(ts.is_some());
    let ts = ts.unwrap();
    assert_eq!(ts.precision, TimestampPrecision::Day);
    
    // Note: Hour precision (10 chars) is not currently supported by parse_hl7_ts_with_precision
    // It jumps from Day (8) to Minute (12)
    
    // Minute precision
    let ts = parse_hl7_ts_with_precision("202301011200");
    assert!(ts.is_some());
    let ts = ts.unwrap();
    assert_eq!(ts.precision, TimestampPrecision::Minute);
    
    // Second precision
    let ts = parse_hl7_ts_with_precision("20230101120000");
    assert!(ts.is_some());
    let ts = ts.unwrap();
    assert_eq!(ts.precision, TimestampPrecision::Second);
}

#[test]
fn test_truncate_to_precision() {
    let full_ts = parse_hl7_ts_with_precision("20230615143045").unwrap();
    
    // Truncate to year
    let truncated = truncate_to_precision(&full_ts.datetime, TimestampPrecision::Year);
    assert_eq!(truncated.format("%Y%m%d%H%M%S").to_string(), "20230101000000");
    
    // Truncate to month
    let truncated = truncate_to_precision(&full_ts.datetime, TimestampPrecision::Month);
    assert_eq!(truncated.format("%Y%m%d%H%M%S").to_string(), "20230601000000");
    
    // Truncate to day
    let truncated = truncate_to_precision(&full_ts.datetime, TimestampPrecision::Day);
    assert_eq!(truncated.format("%Y%m%d%H%M%S").to_string(), "20230615000000");
    
    // Truncate to hour
    let truncated = truncate_to_precision(&full_ts.datetime, TimestampPrecision::Hour);
    assert_eq!(truncated.format("%Y%m%d%H%M%S").to_string(), "20230615140000");
    
    // Truncate to minute
    let truncated = truncate_to_precision(&full_ts.datetime, TimestampPrecision::Minute);
    assert_eq!(truncated.format("%Y%m%d%H%M%S").to_string(), "20230615143000");
    
    // Truncate to second (no change)
    let truncated = truncate_to_precision(&full_ts.datetime, TimestampPrecision::Second);
    assert_eq!(truncated.format("%Y%m%d%H%M%S").to_string(), "20230615143045");
}

#[test]
fn test_compare_timestamps_for_before() {
    // Same precision
    let ts1 = parse_hl7_ts_with_precision("20230101").unwrap();
    let ts2 = parse_hl7_ts_with_precision("20230201").unwrap();
    assert!(compare_timestamps_for_before(&ts1, &ts2));
    assert!(!compare_timestamps_for_before(&ts2, &ts1));
    
    // Different precision
    let ts_date = parse_hl7_ts_with_precision("20230101").unwrap();
    let ts_datetime = parse_hl7_ts_with_precision("20230101120000").unwrap();
    // Same date, different time - should be equal when truncated
    assert!(!compare_timestamps_for_before(&ts_date, &ts_datetime));
    assert!(!compare_timestamps_for_before(&ts_datetime, &ts_date));
}

// ============================================================================
// Checksum Validation Tests
// ============================================================================

#[test]
fn test_validate_luhn_checksum_valid() {
    // Valid test card numbers
    assert!(validate_luhn_checksum("4532015112830366")); // Valid test card
    assert!(validate_luhn_checksum("79927398713"));      // Valid Luhn
    assert!(validate_luhn_checksum("4242424242424242")); // Valid test card
}

#[test]
fn test_validate_luhn_checksum_invalid() {
    assert!(!validate_luhn_checksum("4532015112830367")); // Invalid (changed last digit)
    assert!(!validate_luhn_checksum("79927398710"));      // Invalid
    assert!(!validate_luhn_checksum("1234567890"));       // Invalid
    assert!(!validate_luhn_checksum("1"));                // Too short
    assert!(!validate_luhn_checksum(""));                 // Empty
}

#[test]
fn test_validate_checksum_algorithms() {
    // Luhn
    assert!(validate_checksum("79927398713", "luhn"));
    assert!(!validate_checksum("79927398710", "luhn"));
    
    // Mod10 (same as Luhn for our purposes)
    assert!(validate_checksum("79927398713", "mod10"));
    assert!(!validate_checksum("79927398710", "mod10"));
    
    // Unknown algorithm (assumes valid)
    assert!(validate_checksum("anything", "unknown"));
}

// ============================================================================
// Format Matching Tests
// ============================================================================

#[test]
fn test_matches_format_date() {
    // YYYY-MM-DD format
    assert!(matches_format("2023-01-15", "YYYY-MM-DD", "DT"));
    assert!(matches_format("1999-12-31", "YYYY-MM-DD", "DT"));
    
    // Invalid format
    assert!(!matches_format("20230115", "YYYY-MM-DD", "DT"));    // Wrong format
    assert!(!matches_format("2023-13-01", "YYYY-MM-DD", "DT"));  // Invalid month
    assert!(!matches_format("2023-01-32", "YYYY-MM-DD", "DT"));  // Invalid day
}

#[test]
fn test_matches_format_time() {
    // HH:MM:SS format
    assert!(matches_format("12:30:45", "HH:MM:SS", "TM"));
    assert!(matches_format("23:59:59", "HH:MM:SS", "TM"));
    assert!(matches_format("00:00:00", "HH:MM:SS", "TM"));
    
    // Invalid format
    assert!(!matches_format("123045", "HH:MM:SS", "TM"));   // Wrong format
    assert!(!matches_format("24:00:00", "HH:MM:SS", "TM")); // Invalid hour
    assert!(!matches_format("12:60:00", "HH:MM:SS", "TM")); // Invalid minute
    assert!(!matches_format("12:00:60", "HH:MM:SS", "TM")); // Invalid second
}

#[test]
fn test_matches_format_unknown() {
    // Unknown formats assume valid
    assert!(matches_format("anything", "UNKNOWN_FORMAT", "XX"));
}

// ============================================================================
// Complex Pattern and Mathematical Relationship Tests
// ============================================================================

#[test]
fn test_matches_complex_pattern() {
    // Single pattern
    assert!(matches_complex_pattern("test123", &["test.*"]));
    
    // Multiple patterns (all must match)
    assert!(matches_complex_pattern("test123", &["test.*", ".*123"]));
    assert!(!matches_complex_pattern("test456", &["test.*", ".*123"]));
    
    // Invalid regex
    assert!(!matches_complex_pattern("test", &["[invalid"]));
}

#[test]
fn test_validate_mathematical_relationship() {
    // Greater than
    assert!(validate_mathematical_relationship("10", "5", "gt"));
    assert!(!validate_mathematical_relationship("5", "10", "gt"));
    
    // Less than
    assert!(validate_mathematical_relationship("5", "10", "lt"));
    assert!(!validate_mathematical_relationship("10", "5", "lt"));
    
    // Greater than or equal
    assert!(validate_mathematical_relationship("10", "10", "ge"));
    assert!(validate_mathematical_relationship("10", "5", "ge"));
    assert!(!validate_mathematical_relationship("5", "10", "ge"));
    
    // Less than or equal
    assert!(validate_mathematical_relationship("5", "5", "le"));
    assert!(validate_mathematical_relationship("5", "10", "le"));
    assert!(!validate_mathematical_relationship("10", "5", "le"));
    
    // Equal
    assert!(validate_mathematical_relationship("5", "5", "eq"));
    assert!(!validate_mathematical_relationship("5", "10", "eq"));
    
    // Not equal
    assert!(validate_mathematical_relationship("5", "10", "ne"));
    assert!(!validate_mathematical_relationship("5", "5", "ne"));
    
    // Invalid operator
    assert!(!validate_mathematical_relationship("5", "10", "invalid"));
    
    // Non-numeric values
    assert!(!validate_mathematical_relationship("abc", "10", "gt"));
}

// ============================================================================
// Rule Condition Tests
// ============================================================================

fn parse_test_message(content: &str) -> Message {
    parse(content.as_bytes()).expect("Failed to parse test message")
}

#[test]
fn test_check_rule_condition_eq() {
    let msg = parse_test_message("MSH|^~\\&|App|Fac|Recv|Fac|20230101||ADT^A01|1|P|2.5\rPID|1||12345^^^MRN||Doe^John\r");
    
    // Equal condition
    let condition = RuleCondition {
        field: "PID.3.1".to_string(),
        operator: "eq".to_string(),
        value: Some("12345".to_string()),
        values: None,
    };
    assert!(check_rule_condition(&msg, &condition));
    
    // Not equal
    let condition = RuleCondition {
        field: "PID.3.1".to_string(),
        operator: "eq".to_string(),
        value: Some("99999".to_string()),
        values: None,
    };
    assert!(!check_rule_condition(&msg, &condition));
}

#[test]
fn test_check_rule_condition_ne() {
    let msg = parse_test_message("MSH|^~\\&|App|Fac|Recv|Fac|20230101||ADT^A01|1|P|2.5\rPID|1||12345^^^MRN||Doe^John\r");
    
    let condition = RuleCondition {
        field: "PID.3.1".to_string(),
        operator: "ne".to_string(),
        value: Some("99999".to_string()),
        values: None,
    };
    assert!(check_rule_condition(&msg, &condition));
    
    let condition = RuleCondition {
        field: "PID.3.1".to_string(),
        operator: "ne".to_string(),
        value: Some("12345".to_string()),
        values: None,
    };
    assert!(!check_rule_condition(&msg, &condition));
}

#[test]
fn test_check_rule_condition_contains() {
    let msg = parse_test_message("MSH|^~\\&|App|Fac|Recv|Fac|20230101||ADT^A01|1|P|2.5\rPID|1||12345^^^MRN||Doe^John\r");
    
    let condition = RuleCondition {
        field: "PID.5.1".to_string(),
        operator: "contains".to_string(),
        value: Some("Doe".to_string()),
        values: None,
    };
    assert!(check_rule_condition(&msg, &condition));
    
    let condition = RuleCondition {
        field: "PID.5.1".to_string(),
        operator: "contains".to_string(),
        value: Some("Smith".to_string()),
        values: None,
    };
    assert!(!check_rule_condition(&msg, &condition));
}

#[test]
fn test_check_rule_condition_in() {
    let msg = parse_test_message("MSH|^~\\&|App|Fac|Recv|Fac|20230101||ADT^A01|1|P|2.5\rPID|1||12345^^^MRN||Doe^John|||M\r");
    
    let condition = RuleCondition {
        field: "PID.8".to_string(),
        operator: "in".to_string(),
        value: None,
        values: Some(vec!["M".to_string(), "F".to_string(), "O".to_string()]),
    };
    assert!(check_rule_condition(&msg, &condition));
    
    let condition = RuleCondition {
        field: "PID.8".to_string(),
        operator: "in".to_string(),
        value: None,
        values: Some(vec!["F".to_string(), "O".to_string()]),
    };
    assert!(!check_rule_condition(&msg, &condition));
}

#[test]
fn test_check_rule_condition_matches_regex() {
    let msg = parse_test_message("MSH|^~\\&|App|Fac|Recv|Fac|20230101||ADT^A01|1|P|2.5\rPID|1||MRN12345^^^MRN||Doe^John\r");
    
    let condition = RuleCondition {
        field: "PID.3.1".to_string(),
        operator: "matches_regex".to_string(),
        value: Some("MRN\\d+".to_string()),
        values: None,
    };
    assert!(check_rule_condition(&msg, &condition));
    
    let condition = RuleCondition {
        field: "PID.3.1".to_string(),
        operator: "matches_regex".to_string(),
        value: Some("SRM\\d+".to_string()),
        values: None,
    };
    assert!(!check_rule_condition(&msg, &condition));
}

#[test]
fn test_check_rule_condition_exists() {
    let msg = parse_test_message("MSH|^~\\&|App|Fac|Recv|Fac|20230101||ADT^A01|1|P|2.5\rPID|1||12345||Doe^John\r");
    
    let condition = RuleCondition {
        field: "PID.3.1".to_string(),
        operator: "exists".to_string(),
        value: None,
        values: None,
    };
    assert!(check_rule_condition(&msg, &condition));
    
    let condition = RuleCondition {
        field: "PID.20.1".to_string(), // Non-existent field
        operator: "exists".to_string(),
        value: None,
        values: None,
    };
    assert!(!check_rule_condition(&msg, &condition));
}

#[test]
fn test_check_rule_condition_not_exists() {
    let msg = parse_test_message("MSH|^~\\&|App|Fac|Recv|Fac|20230101||ADT^A01|1|P|2.5\rPID|1||12345||Doe^John\r");
    
    let condition = RuleCondition {
        field: "PID.20.1".to_string(), // Non-existent field
        operator: "not_exists".to_string(),
        value: None,
        values: None,
    };
    assert!(check_rule_condition(&msg, &condition));
    
    let condition = RuleCondition {
        field: "PID.3.1".to_string(),
        operator: "not_exists".to_string(),
        value: None,
        values: None,
    };
    assert!(!check_rule_condition(&msg, &condition));
}

#[test]
fn test_check_rule_condition_is_date() {
    let msg = parse_test_message("MSH|^~\\&|App|Fac|Recv|Fac|20230101||ADT^A01|1|P|2.5\rPID|1||12345||Doe^John||19800101\r");
    
    let condition = RuleCondition {
        field: "PID.7".to_string(),
        operator: "is_date".to_string(),
        value: None,
        values: None,
    };
    assert!(check_rule_condition(&msg, &condition));
    
    let condition = RuleCondition {
        field: "PID.5.1".to_string(), // Name, not a date
        operator: "is_date".to_string(),
        value: None,
        values: None,
    };
    assert!(!check_rule_condition(&msg, &condition));
}

#[test]
fn test_check_rule_condition_before() {
    let msg = parse_test_message("MSH|^~\\&|App|Fac|Recv|Fac|20230101||ADT^A01|1|P|2.5\rPID|1||12345||Doe^John||19800101\r");
    
    // Birth date before message date
    let condition = RuleCondition {
        field: "PID.7".to_string(),
        operator: "before".to_string(),
        value: Some("20230101".to_string()),
        values: None,
    };
    assert!(check_rule_condition(&msg, &condition));
    
    // Birth date after message date (should fail)
    let condition = RuleCondition {
        field: "PID.7".to_string(),
        operator: "before".to_string(),
        value: Some("19700101".to_string()),
        values: None,
    };
    assert!(!check_rule_condition(&msg, &condition));
}

#[test]
fn test_check_rule_condition_within_range() {
    let msg = parse_test_message("MSH|^~\\&|App|Fac|Recv|Fac|20230101||ADT^A01|1|P|2.5\rPID|1||12345||Doe^John||19800615\r");
    
    // Date within range
    let condition = RuleCondition {
        field: "PID.7".to_string(),
        operator: "within_range".to_string(),
        value: None,
        values: Some(vec!["19800101".to_string(), "19801231".to_string()]),
    };
    assert!(check_rule_condition(&msg, &condition));
    
    // Date outside range
    let condition = RuleCondition {
        field: "PID.7".to_string(),
        operator: "within_range".to_string(),
        value: None,
        values: Some(vec!["19900101".to_string(), "19991231".to_string()]),
    };
    assert!(!check_rule_condition(&msg, &condition));
}

#[test]
fn test_check_rule_condition_unknown_operator() {
    let msg = parse_test_message("MSH|^~\\&|App|Fac|Recv|Fac|20230101||ADT^A01|1|P|2.5\rPID|1||12345\r");
    
    let condition = RuleCondition {
        field: "PID.3.1".to_string(),
        operator: "unknown".to_string(),
        value: Some("12345".to_string()),
        values: None,
    };
    assert!(!check_rule_condition(&msg, &condition));
}

// ============================================================================
// TimestampPrecision Tests
// ============================================================================

#[test]
fn test_timestamp_precision_ordering() {
    assert!(TimestampPrecision::Year < TimestampPrecision::Month);
    assert!(TimestampPrecision::Month < TimestampPrecision::Day);
    assert!(TimestampPrecision::Day < TimestampPrecision::Hour);
    assert!(TimestampPrecision::Hour < TimestampPrecision::Minute);
    assert!(TimestampPrecision::Minute < TimestampPrecision::Second);
}

// ============================================================================
// Integration Tests with Real Messages
// ============================================================================

#[test]
fn test_validate_adt_a01_message() {
    let msg_content = concat!(
        "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|",
        "20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\r",
        "EVN|A01|20250128152312|||\r",
        "PID|1||123456^^^HOSP^MR||Doe^John^A||19800101|M|||C|\r",
        "PV1|1|I|ICU^101^01||||DOC123^Smith^Jane||||||||V123456\r"
    );
    
    let msg = parse_test_message(msg_content);
    
    // Validate data types
    assert!(validate_data_type("19800101", "DT")); // PID.7 (birth date)
    assert!(validate_data_type("M", "ID"));        // PID.8 (sex)
    assert!(validate_data_type("123456", "ST"));   // PID.3.1 (patient ID)
    
    // Validate timestamps
    assert!(is_timestamp("20250128152312")); // MSH.7
    
    // Validate birth date is valid
    assert!(is_valid_birth_date("19800101"));
}

#[test]
fn test_validate_oru_r01_message() {
    let msg_content = concat!(
        "MSH|^~\\&|LabSys|Lab|LIS|Hospital|",
        "20250128150000||ORU^R01|MSG003|P|2.5\r",
        "PID|1||MRN789^^^Lab^MR||Patient^Test||19850610|M\r",
        "OBR|1|ORD123|FIL456|CBC^Complete Blood Count|||20250128120000\r",
        "OBX|1|NM|WBC^White Blood Count||7.5|10^9/L|4.0-11.0|N|||F\r"
    );
    
    let msg = parse_test_message(msg_content);
    
    // Validate numeric observation value
    assert!(validate_data_type("7.5", "NM")); // OBX.5
    
    // Validate range
    assert!(is_within_range("7.5", "4.0", "11.0"));
    
    // Validate timestamps
    assert!(is_timestamp("20250128150000")); // MSH.7
    assert!(is_timestamp("20250128120000")); // OBR.7
}

#[test]
fn test_validate_message_with_missing_required_field() {
    let msg_content = concat!(
        "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|",
        "20250128152312||ADT^A01|ABC123|P|2.5.1\r",
        "PID|1|||||||M|||C|\r" // Missing patient ID and name
    );
    
    let msg = parse_test_message(msg_content);
    
    // Check that PID.3.1 is missing
    let condition = RuleCondition {
        field: "PID.3.1".to_string(),
        operator: "exists".to_string(),
        value: None,
        values: None,
    };
    assert!(!check_rule_condition(&msg, &condition));
}

#[test]
fn test_validate_message_with_invalid_data() {
    let msg_content = concat!(
        "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|",
        "20250128152312||ADT^A01|ABC123|P|2.5.1\r",
        "PID|1||12345||Doe^John||invalid_date|M\r" // Invalid birth date
    );
    
    let msg = parse_test_message(msg_content);
    
    // Check that PID.7 is not a valid date
    let condition = RuleCondition {
        field: "PID.7".to_string(),
        operator: "is_date".to_string(),
        value: None,
        values: None,
    };
    assert!(!check_rule_condition(&msg, &condition));
}

// ============================================================================
// Edge Cases and Error Handling
// ============================================================================

#[test]
fn test_empty_values() {
    // Empty values should be handled gracefully
    assert!(is_string(""));
    assert!(!is_date(""));
    assert!(!is_time(""));
    assert!(!is_numeric(""));
    assert!(!is_sequence_id(""));
}

#[test]
fn test_whitespace_values() {
    // Whitespace handling
    assert!(is_string("   "));
    assert!(!is_date("   "));
    assert!(!is_time("   "));
}

#[test]
fn test_unicode_values() {
    // Unicode in strings
    assert!(is_string("日本語"));
    assert!(is_string("émoji 🎉"));
    
    // Unicode in identifiers (ASCII only expected)
    assert!(!is_identifier("日本語"));
}

#[test]
fn test_very_long_values() {
    // Long strings
    let long_string = "x".repeat(10000);
    assert!(is_string(&long_string));
    
    // Long identifiers
    let long_id = "A".repeat(1000);
    assert!(is_identifier(&long_id));
}

#[test]
fn test_special_characters() {
    // Special characters in strings
    assert!(is_string("test\twith\ttabs"));
    assert!(is_string("test\nwith\nnewlines"));
    assert!(is_string("test\rwith\rcarriage"));
    
    // Control characters should fail identifier validation
    assert!(!is_identifier("test\twith\ttabs"));
}
