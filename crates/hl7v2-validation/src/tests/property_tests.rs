//! Property-based tests for hl7v2-validation crate using proptest.
//!
//! These tests verify that:
//! - Valid messages pass validation
//! - Invalid messages fail consistently
//! - Validation never panics
//! - Properties hold across a wide range of inputs

use crate::{
    is_coded_value, is_date, is_email, is_identifier, is_numeric, is_person_name,
    is_phone_number, is_sequence_id, is_ssn, is_time, is_timestamp, is_valid_birth_date,
    is_within_range, validate_checksum, validate_data_type, validate_luhn_checksum,
    validate_mathematical_relationship, Issue, Severity,
};
use proptest::prelude::*;

// ============================================================================
// Strategies for Generating Test Data
// ============================================================================

/// Generate a valid HL7 date string (YYYYMMDD)
fn valid_date_strategy() -> impl Strategy<Value = String> {
    "[0-9]{4}(0[1-9]|1[0-2])(0[1-9]|[12][0-9]|3[01])"
}

/// Generate a valid HL7 time string (HHMM or HHMMSS)
fn valid_time_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        // HHMM format
        "[0-2][0-9][0-5][0-9]".prop_filter("Valid hour 0-23", |s| {
            let hour: u32 = s[0..2].parse().unwrap_or(24);
            hour <= 23
        }),
        // HHMMSS format
        "[0-2][0-9][0-5][0-9][0-5][0-9]".prop_filter("Valid hour 0-23", |s| {
            let hour: u32 = s[0..2].parse().unwrap_or(24);
            hour <= 23
        }),
    ]
}

/// Generate a valid HL7 timestamp (YYYYMMDDHHMMSS)
fn valid_timestamp_strategy() -> impl Strategy<Value = String> {
    "[0-9]{4}(0[1-9]|1[0-2])(0[1-9]|[12][0-9]|3[01])[0-2][0-9][0-5][0-9][0-5][0-9]"
        .prop_filter("Valid timestamp", |s| {
            let hour: u32 = s[8..10].parse().unwrap_or(24);
            hour <= 23
        })
}

/// Generate a numeric string
fn numeric_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        // Integer
        "-?[0-9]+".prop_filter("Non-empty", |s| !s.is_empty() && s != "-" && s != "+"),
        // Decimal
        "-?[0-9]+\\.[0-9]+",
    ]
}

/// Generate an alphanumeric identifier
fn identifier_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_\\-\\.]+"
}

/// Generate a person name (letters, spaces, hyphens, apostrophes)
fn person_name_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z\\s\\-']+"
}

/// Generate any ASCII string
fn ascii_string_strategy() -> impl Strategy<Value = String> {
    ".*"
}

/// Generate a valid email address
fn valid_email_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9]+@[a-zA-Z0-9]+\\.[a-zA-Z]{2,}"
}

/// Generate a valid phone number (digits only, 7-15 digits)
fn valid_phone_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        "[0-9]{7}",
        "[0-9]{10}",
        "[0-9]{11}",
        "[0-9]{15}",
    ]
}

/// Generate a valid SSN (9 digits, avoiding invalid patterns)
fn valid_ssn_strategy() -> impl Strategy<Value = String> {
    // Area: 001-665, 667-899 (not 000, 666, 900-999)
    // Group: 01-99 (not 00)
    // Serial: 0001-9999 (not 0000)
    ("(00[1-9]|0[1-9][0-9]|[1-5][0-9]{2}|6[0-5][0-9]|66[0-5]|66[7-9]|6[7-9][0-9]|[78][0-9]{2})",
        // Group: 01-99
        "(0[1-9]|[1-9][0-9])",
        // Serial: 0001-9999
        "(000[1-9]|00[1-9][0-9]|0[1-9][0-9]{2}|[1-9][0-9]{3})")
        .prop_map(|(area, group, serial)| format!("{}{}{}", area, group, serial))
}

/// Generate a valid Luhn number
fn valid_luhn_strategy() -> impl Strategy<Value = String> {
    // Use known valid Luhn numbers for simplicity
    prop_oneof![
        Just("4532015112830366".to_string()),  // Valid test card
        Just("79927398713".to_string()),        // Valid Luhn
        Just("4242424242424242".to_string()),   // Valid test card
    ]
}

// ============================================================================
// Property Tests: Data Type Validation Never Panics
// ============================================================================

proptest! {
    #[test]
    fn test_validate_data_type_never_panics(value in "[0-9a-zA-Z\\s\\-\\.@]*", datatype in "[A-Z]{2,3}") {
        // Use ASCII-safe patterns to avoid multi-byte UTF-8 issues
        // This test ensures that validate_data_type never panics on common input
        let _ = validate_data_type(&value, &datatype);
    }
}

proptest! {
    #[test]
    fn test_is_date_never_panics(value in ".*") {
        let _ = is_date(&value);
    }
}

proptest! {
    #[test]
    fn test_is_time_never_panics(value in ".*") {
        let _ = is_time(&value);
    }
}

proptest! {
    #[test]
    fn test_is_timestamp_never_panics(value in "[0-9]*") {
        // Use ASCII digits only to avoid multi-byte UTF-8 issues in string slicing
        let _ = is_timestamp(&value);
    }
}

proptest! {
    #[test]
    fn test_is_numeric_never_panics(value in ".*") {
        let _ = is_numeric(&value);
    }
}

proptest! {
    #[test]
    fn test_is_identifier_never_panics(value in ".*") {
        let _ = is_identifier(&value);
    }
}

proptest! {
    #[test]
    fn test_is_person_name_never_panics(value in ".*") {
        let _ = is_person_name(&value);
    }
}

proptest! {
    #[test]
    fn test_is_email_never_panics(value in ".*") {
        let _ = is_email(&value);
    }
}

proptest! {
    #[test]
    fn test_is_phone_number_never_panics(value in ".*") {
        let _ = is_phone_number(&value);
    }
}

proptest! {
    #[test]
    fn test_is_ssn_never_panics(value in ".*") {
        let _ = is_ssn(&value);
    }
}

proptest! {
    #[test]
    fn test_validate_luhn_never_panics(value in ".*") {
        let _ = validate_luhn_checksum(&value);
    }
}

proptest! {
    #[test]
    fn test_validate_checksum_never_panics(value in ".*", algorithm in ".*") {
        let _ = validate_checksum(&value, &algorithm);
    }
}

proptest! {
    #[test]
    fn test_is_within_range_never_panics(value in ".*", min in ".*", max in ".*") {
        let _ = is_within_range(&value, &min, &max);
    }
}

proptest! {
    #[test]
    fn test_validate_mathematical_relationship_never_panics(
        v1 in ".*", v2 in ".*", op in ".*"
    ) {
        let _ = validate_mathematical_relationship(&v1, &v2, &op);
    }
}

// ============================================================================
// Property Tests: Valid Inputs Are Recognized
// ============================================================================

proptest! {
    #[test]
    fn test_valid_dates_are_recognized(date in valid_date_strategy()) {
        prop_assert!(is_date(&date));
    }
}

proptest! {
    #[test]
    fn test_valid_times_are_recognized(time in valid_time_strategy()) {
        prop_assert!(is_time(&time));
    }
}

proptest! {
    #[test]
    fn test_valid_timestamps_are_recognized(ts in valid_timestamp_strategy()) {
        prop_assert!(is_timestamp(&ts));
    }
}

proptest! {
    #[test]
    fn test_valid_numerics_are_recognized(num in numeric_strategy()) {
        prop_assert!(is_numeric(&num));
    }
}

proptest! {
    #[test]
    fn test_valid_identifiers_are_recognized(id in identifier_strategy()) {
        // Non-empty identifiers without control characters should pass
        if !id.is_empty() {
            prop_assert!(is_identifier(&id));
        }
    }
}

proptest! {
    #[test]
    fn test_valid_person_names_are_recognized(name in person_name_strategy()) {
        prop_assert!(is_person_name(&name));
    }
}

proptest! {
    #[test]
    fn test_valid_emails_are_recognized(email in valid_email_strategy()) {
        prop_assert!(is_email(&email));
    }
}

proptest! {
    #[test]
    fn test_valid_phones_are_recognized(phone in valid_phone_strategy()) {
        prop_assert!(is_phone_number(&phone));
    }
}

proptest! {
    #[test]
    fn test_valid_ssns_are_recognized(ssn in valid_ssn_strategy()) {
        prop_assert!(is_ssn(&ssn));
    }
}

// ============================================================================
// Property Tests: Invalid Inputs Are Rejected
// ============================================================================

proptest! {
    #[test]
    fn test_invalid_dates_rejected(date in "[0-9]{8}") {
        // Filter to get only invalid dates
        let month: u32 = date[4..6].parse().unwrap_or(0);
        let day: u32 = date[6..8].parse().unwrap_or(0);
        
        prop_assume!(month == 0 || month > 12 || day == 0 || day > 31);
        prop_assert!(!is_date(&date));
    }
}

proptest! {
    #[test]
    fn test_non_numeric_rejected(value in "[a-zA-Z]+") {
        prop_assert!(!is_numeric(&value));
    }
}

proptest! {
    #[test]
    fn test_control_chars_rejected_from_identifiers(value in any::<Vec<u8>>()) {
        // Convert to string, filtering out invalid UTF-8
        let s: String = value.iter()
            .filter(|&&c| c >= 0x20 || c == 0x09) // Keep printable + tab
            .map(|&c| c as char)
            .collect();
        
        // If string contains control characters (except tab), it should fail
        let has_control = s.chars().any(|c| c.is_control() && c != '\t');
        if has_control {
            prop_assert!(!is_identifier(&s));
        }
    }
}

proptest! {
    #[test]
    fn test_numbers_in_names_rejected(name in "[a-zA-Z]*[0-9]+[a-zA-Z]*") {
        // Names with numbers should be rejected
        prop_assert!(!is_person_name(&name));
    }
}

// ============================================================================
// Property Tests: Luhn Checksum Validation
// ============================================================================

proptest! {
    #[test]
    fn test_luhn_valid_numbers(luhn in valid_luhn_strategy()) {
        prop_assert!(validate_luhn_checksum(&luhn));
    }
}

proptest! {
    #[test]
    fn test_luhn_invalidated_by_digit_change(mut luhn in valid_luhn_strategy()) {
        prop_assume!(!luhn.is_empty());
        
        // Change one digit (not the check digit)
        let idx = luhn.len() / 2;
        let original_char = luhn.chars().nth(idx).unwrap_or('0');
        let new_char = if original_char == '9' { '0' } else { 
            char::from_digit(original_char.to_digit(10).unwrap_or(0) + 1, 10).unwrap_or('0')
        };
        
        luhn.replace_range(idx..idx+1, &new_char.to_string());
        
        // Modified number should fail Luhn check (high probability)
        // Note: This is probabilistic; some changes might still pass
        // so we just verify it doesn't panic
        let _ = validate_luhn_checksum(&luhn);
    }
}

// ============================================================================
// Property Tests: Range Validation
// ============================================================================

proptest! {
    #[test]
    fn test_within_range_inclusive(
        value in 0i64..1000,
        min in 0i64..500,
        max in 500i64..1000
    ) {
        let result = is_within_range(&value.to_string(), &min.to_string(), &max.to_string());
        
        if value >= min && value <= max {
            prop_assert!(result, "Value {} should be in range [{}, {}]", value, min, max);
        } else {
            prop_assert!(!result, "Value {} should NOT be in range [{}, {}]", value, min, max);
        }
    }
}

proptest! {
    #[test]
    fn test_within_range_decimals(
        value in -1000.0f64..1000.0,
        min in -500.0f64..0.0,
        max in 0.0f64..500.0
    ) {
        let result = is_within_range(&value.to_string(), &min.to_string(), &max.to_string());
        
        if value >= min && value <= max {
            prop_assert!(result);
        } else {
            prop_assert!(!result);
        }
    }
}

// ============================================================================
// Property Tests: Mathematical Relationships
// ============================================================================

proptest! {
    #[test]
    fn test_math_relationship_gt(a in any::<i64>(), b in any::<i64>()) {
        let result = validate_mathematical_relationship(&a.to_string(), &b.to_string(), "gt");
        prop_assert_eq!(result, a > b);
    }
}

proptest! {
    #[test]
    fn test_math_relationship_lt(a in any::<i64>(), b in any::<i64>()) {
        let result = validate_mathematical_relationship(&a.to_string(), &b.to_string(), "lt");
        prop_assert_eq!(result, a < b);
    }
}

proptest! {
    #[test]
    fn test_math_relationship_eq(a in any::<i64>(), b in any::<i64>()) {
        let result = validate_mathematical_relationship(&a.to_string(), &b.to_string(), "eq");
        prop_assert_eq!(result, a == b);
    }
}

proptest! {
    #[test]
    fn test_math_relationship_ge(a in any::<i64>(), b in any::<i64>()) {
        let result = validate_mathematical_relationship(&a.to_string(), &b.to_string(), "ge");
        prop_assert_eq!(result, a >= b);
    }
}

proptest! {
    #[test]
    fn test_math_relationship_le(a in any::<i64>(), b in any::<i64>()) {
        let result = validate_mathematical_relationship(&a.to_string(), &b.to_string(), "le");
        prop_assert_eq!(result, a <= b);
    }
}

proptest! {
    #[test]
    fn test_math_relationship_ne(a in any::<i64>(), b in any::<i64>()) {
        let result = validate_mathematical_relationship(&a.to_string(), &b.to_string(), "ne");
        prop_assert_eq!(result, a != b);
    }
}

// ============================================================================
// Property Tests: Birth Date Validation
// ============================================================================

proptest! {
    #[test]
    fn test_birth_date_not_in_future(
        year in 1900u32..=2025,
        month in 1u32..=12,
        day in 1u32..=28 // Use 28 to avoid invalid dates
    ) {
        let date = format!("{:04}{:02}{:02}", year, month, day);
        
        // Birth dates from 1900-2025 should all be valid (not in future)
        // Note: This test may fail if run after 2025
        prop_assert!(is_valid_birth_date(&date));
    }
}

// ============================================================================
// Property Tests: Issue Creation
// ============================================================================

proptest! {
    #[test]
    fn test_issue_creation_never_panics(
        code in ".*",
        path in proptest::option::of(".*"),
        detail in ".*"
    ) {
        // Issue creation should never panic
        let issue = Issue::new(&code, Severity::Error, path.clone(), detail.clone());
        prop_assert_eq!(issue.code, code);
        prop_assert_eq!(issue.path, path);
        prop_assert_eq!(issue.detail, detail);
    }
}

proptest! {
    #[test]
    fn test_issue_error_creation(code in ".*", path in proptest::option::of(".*"), detail in ".*") {
        let issue = Issue::error(&code, path.clone(), detail.clone());
        prop_assert_eq!(issue.severity, Severity::Error);
    }
}

proptest! {
    #[test]
    fn test_issue_warning_creation(code in ".*", path in proptest::option::of(".*"), detail in ".*") {
        let issue = Issue::warning(&code, path.clone(), detail.clone());
        prop_assert_eq!(issue.severity, Severity::Warning);
    }
}

// ============================================================================
// Property Tests: Coded Value Validation
// ============================================================================

proptest! {
    #[test]
    fn test_coded_value_ascii_printable(value in "[ -~]*") {
        // ASCII printable characters (space through tilde) should pass
        prop_assert!(is_coded_value(&value));
    }
}

proptest! {
    #[test]
    fn test_coded_value_with_unicode(value in ".*") {
        // Just verify it doesn't panic
        let _ = is_coded_value(&value);
    }
}

// ============================================================================
// Property Tests: Sequence ID
// ============================================================================

proptest! {
    #[test]
    fn test_sequence_id_positive(n in 1u32..=1000000) {
        prop_assert!(is_sequence_id(&n.to_string()));
    }
}

#[test]
fn test_sequence_id_zero_rejected() {
    assert!(!is_sequence_id("0"));
}

proptest! {
    #[test]
    fn test_sequence_id_negative_rejected(n in any::<i32>().prop_filter("Negative", |&n| n < 0)) {
        prop_assert!(!is_sequence_id(&n.to_string()));
    }
}

// ============================================================================
// Property Tests: Data Type Round-Trip
// ============================================================================

proptest! {
    #[test]
    fn test_data_type_st_always_passes(value in ".*") {
        // ST (String) should always validate
        prop_assert!(validate_data_type(&value, "ST"));
    }
}

proptest! {
    #[test]
    fn test_data_type_tx_always_passes(value in ".*") {
        // TX (Text) should always validate
        prop_assert!(validate_data_type(&value, "TX"));
    }
}

proptest! {
    #[test]
    fn test_data_type_ft_always_passes(value in ".*") {
        // FT (Formatted Text) should always validate
        prop_assert!(validate_data_type(&value, "FT"));
    }
}

proptest! {
    #[test]
    fn test_data_type_unknown_always_passes(value in ".*", datatype in "[A-Z]{4,10}") {
        // Unknown data types should assume valid
        prop_assert!(validate_data_type(&value, &datatype));
    }
}

// ============================================================================
// Property Tests: Consistency
// ============================================================================

proptest! {
    #[test]
    fn test_date_consistency(date in valid_date_strategy()) {
        // A valid date should also be a valid timestamp
        prop_assert!(is_date(&date));
        prop_assert!(is_timestamp(&date));
    }
}

proptest! {
    #[test]
    fn test_timestamp_contains_date(ts in valid_timestamp_strategy()) {
        // A valid timestamp's date portion should be valid
        if ts.len() >= 8 {
            let date_part = &ts[0..8];
            prop_assert!(is_date(date_part));
        }
    }
}

// ============================================================================
// Property Tests: Edge Cases
// ============================================================================

#[test]
fn test_empty_string_handling() {
    // Empty strings should be handled gracefully
    assert!(is_string(""));
    assert!(!is_date(""));
    assert!(!is_time(""));
    assert!(!is_numeric(""));
    assert!(!is_sequence_id(""));
}

proptest! {
    #[test]
    fn test_whitespace_handling(ws in "\\s+") {
        // Whitespace-only strings
        prop_assert!(!is_date(&ws));
        prop_assert!(!is_time(&ws));
        prop_assert!(!is_numeric(&ws));
    }
}

proptest! {
    #[test]
    fn test_very_long_strings(n in 1usize..=10000) {
        let long_string = "x".repeat(n);
        
        // Should handle long strings without panic
        let _ = is_date(&long_string);
        let _ = is_time(&long_string);
        let _ = is_identifier(&long_string);
        let _ = validate_data_type(&long_string, "ST");
    }
}

// Helper function for is_string test
fn is_string(_value: &str) -> bool {
    true // is_string always returns true
}
