//! Property-based tests for hl7v2-datatype crate
//!
//! Uses proptest to verify properties hold across a wide range of inputs

use hl7v2_datatype::*;
use proptest::prelude::*;

// ============================================================================
// String Data Type Property Tests
// ============================================================================

proptest! {
    #[test]
    fn test_string_always_valid(s in ".*") {
        // ST data type should accept any string
        prop_assert!(is_string(&s));
        prop_assert!(validate_datatype(&s, "ST"));
    }

    #[test]
    fn test_text_data_always_valid(s in ".*") {
        // TX data type should accept any string
        prop_assert!(is_text_data(&s));
        prop_assert!(validate_datatype(&s, "TX"));
    }

    #[test]
    fn test_formatted_text_always_valid(s in ".*") {
        // FT data type should accept any string
        prop_assert!(is_formatted_text(&s));
        prop_assert!(validate_datatype(&s, "FT"));
    }
}

// ============================================================================
// Numeric Data Type Property Tests
// ============================================================================

proptest! {
    #[test]
    fn test_numeric_valid_integers(n in any::<i64>()) {
        let s = n.to_string();
        prop_assert!(is_numeric(&s));
        prop_assert!(validate_datatype(&s, "NM"));
    }

    #[test]
    fn test_numeric_valid_floats(n in any::<f64>()) {
        // Skip NaN and Infinity as they don't parse correctly
        if n.is_finite() {
            let s = n.to_string();
            prop_assert!(is_numeric(&s));
            prop_assert!(validate_datatype(&s, "NM"));
        }
    }

    #[test]
    fn test_numeric_invalid_non_digits(s in "[a-zA-Z]+") {
        // Pure alphabetic strings should not be valid numbers
        if !s.is_empty() {
            prop_assert!(!is_numeric(&s));
            prop_assert!(!validate_datatype(&s, "NM"));
        }
    }
}

// ============================================================================
// Sequence ID Property Tests
// ============================================================================

proptest! {
    #[test]
    fn test_sequence_id_valid(n in 1u32..=1000000) {
        let s = n.to_string();
        prop_assert!(is_sequence_id(&s));
        prop_assert!(validate_datatype(&s, "SI"));
    }
}

#[test]
fn test_sequence_id_zero_invalid() {
    assert!(!is_sequence_id("0"));
    assert!(!validate_datatype("0", "SI"));
}

proptest! {
    #[test]
    fn test_sequence_id_negative_invalid(n in -1000000i32..=-1) {
        let s = n.to_string();
        prop_assert!(!is_sequence_id(&s));
        prop_assert!(!validate_datatype(&s, "SI"));
    }
}

// ============================================================================
// Identifier Property Tests
// ============================================================================

proptest! {
    #[test]
    fn test_identifier_printable_ascii(s in "[ -~]+") {
        // Printable ASCII should be valid identifiers
        prop_assert!(is_identifier(&s));
        prop_assert!(validate_datatype(&s, "ID"));
    }

    #[test]
    fn test_identifier_with_control_chars_invalid(s in ".*") {
        // If string contains control characters, it should be invalid
        let has_control = s.chars().any(|c| c.is_control());
        if has_control {
            prop_assert!(!is_identifier(&s));
        }
    }
}

// ============================================================================
// Person Name Property Tests
// ============================================================================

proptest! {
    #[test]
    fn test_person_name_alphabetic(name in "[a-zA-Z]+") {
        // Pure alphabetic names should be valid
        prop_assert!(is_person_name(&name));
    }

    #[test]
    fn test_person_name_with_allowed_special(name in "[a-zA-Z'\\-\\^\\. ]+") {
        // Names with allowed special characters should be valid
        prop_assert!(is_person_name(&name));
    }

    #[test]
    fn test_person_name_with_digits_invalid(name in "[a-zA-Z0-9]+") {
        // Names containing digits should be invalid
        let has_digits = name.chars().any(|c| c.is_ascii_digit());
        if has_digits {
            prop_assert!(!is_person_name(&name));
        }
    }
}

// ============================================================================
// Phone Number Property Tests
// ============================================================================

proptest! {
    #[test]
    fn test_phone_number_valid_digits(digits in "[0-9]{7,15}") {
        // 7-15 digits should be valid phone number
        prop_assert!(is_phone_number(&digits));
    }

    #[test]
    fn test_phone_number_with_formatting(phone in "\\([0-9]{3}\\) [0-9]{3}-[0-9]{4}") {
        // Formatted phone numbers should be valid
        prop_assert!(is_phone_number(&phone));
    }

    #[test]
    fn test_phone_number_too_short(digits in "[0-9]{1,6}") {
        // Less than 7 digits should be invalid
        prop_assert!(!is_phone_number(&digits));
    }

    #[test]
    fn test_phone_number_too_long(digits in "[0-9]{16,20}") {
        // More than 15 digits should be invalid
        prop_assert!(!is_phone_number(&digits));
    }
}

// ============================================================================
// Email Property Tests
// ============================================================================

proptest! {
    #[test]
    fn test_email_valid_format(local in "[a-zA-Z0-9\\.]+", domain in "[a-zA-Z0-9]+", tld in "[a-zA-Z]{2,}") {
        let email = format!("{}@{}.{}", local, domain, tld);
        prop_assert!(is_email(&email));
    }

    #[test]
    fn test_email_no_at_sign_invalid(s in "[^@]+") {
        // Without @ sign, should be invalid
        prop_assert!(!is_email(&s));
    }

    #[test]
    fn test_email_no_dot_invalid(local in "[a-zA-Z0-9]+", domain in "[a-zA-Z0-9]+") {
        // Without dot in domain, should be invalid
        let email = format!("{}@{}", local, domain);
        prop_assert!(!is_email(&email));
    }
}

// ============================================================================
// SSN Property Tests
// ============================================================================

proptest! {
    #[test]
    fn test_ssn_valid_format(area in 1u32..=665, group in 1u32..=99, serial in 1u32..=9999) {
        let ssn = format!("{:03}{:02}{:04}", area, group, serial);
        prop_assert!(is_ssn(&ssn));
    }

    #[test]
    fn test_ssn_formatted(area in 1u32..=665, group in 1u32..=99, serial in 1u32..=9999) {
        let ssn = format!("{:03}-{:02}-{:04}", area, group, serial);
        prop_assert!(is_ssn(&ssn));
    }

    #[test]
    fn test_ssn_invalid_area_000(group in 0u32..=99, serial in 0u32..=9999) {
        let ssn = format!("000{:02}{:04}", group, serial);
        prop_assert!(!is_ssn(&ssn));
    }

    #[test]
    fn test_ssn_invalid_group_000(area in 1u32..=665, serial in 0u32..=9999) {
        let ssn = format!("{:03}00{:04}", area, serial);
        prop_assert!(!is_ssn(&ssn));
    }

    #[test]
    fn test_ssn_invalid_serial_0000(area in 1u32..=665, group in 1u32..=99) {
        let ssn = format!("{:03}{:02}0000", area, group);
        prop_assert!(!is_ssn(&ssn));
    }
}

// ============================================================================
// Validator Builder Property Tests
// ============================================================================

proptest! {
    #[test]
    fn test_validator_min_length(min in 0usize..=100, s in ".*") {
        let validator = DataTypeValidator::new().with_min_length(min);
        let valid = s.len() >= min;
        prop_assert_eq!(validator.validate(&s), valid);
    }

    #[test]
    fn test_validator_max_length(max in 0usize..=100, s in ".*") {
        let validator = DataTypeValidator::new().with_max_length(max);
        let valid = s.len() <= max;
        prop_assert_eq!(validator.validate(&s), valid);
    }

    #[test]
    fn test_validator_min_max_length(min in 0usize..=50, max in 50usize..=100, s in ".*") {
        let validator = DataTypeValidator::new()
            .with_min_length(min)
            .with_max_length(max);
        let valid = s.len() >= min && s.len() <= max;
        prop_assert_eq!(validator.validate(&s), valid);
    }
}

// ============================================================================
// Luhn Checksum Property Tests
// ============================================================================

#[test]
fn test_luhn_valid_test_numbers() {
    // Known valid test numbers
    let valid_numbers = vec![
        "4532015112830366",
        "6011111111111117",
        "378282246310005",
        "4111111111111111",
    ];
    for num in &valid_numbers {
        assert!(validate_luhn_checksum(num));
    }
}

proptest! {
    #[test]
    fn test_luhn_single_digit_invalid(n in 0u32..=9) {
        let s = n.to_string();
        prop_assert!(!validate_luhn_checksum(&s));
    }
}

#[test]
fn test_luhn_empty_invalid() {
    assert!(!validate_luhn_checksum(""));
}

// ============================================================================
// Date Validation Property Tests
// ============================================================================

proptest! {
    #[test]
    fn test_date_valid_format(year in 1i32..=9999, month in 1u32..=12, day in 1u32..=28) {
        let date = format!("{:04}{:02}{:02}", year, month, day);
        prop_assert!(is_date(&date));
        prop_assert!(validate_datatype(&date, "DT"));
    }

    #[test]
    fn test_date_invalid_month(month in 13u32..=99) {
        let date = format!("2025{:02}01", month);
        prop_assert!(!is_date(&date));
        prop_assert!(!validate_datatype(&date, "DT"));
    }

    #[test]
    fn test_date_non_digits_invalid(s in "[a-zA-Z]+") {
        if s.len() == 8 {
            prop_assert!(!is_date(&s));
        }
    }
}

// ============================================================================
// Time Validation Property Tests
// ============================================================================

proptest! {
    #[test]
    fn test_time_valid_hhmmss(hour in 0u32..=23, minute in 0u32..=59, second in 0u32..=59) {
        let time = format!("{:02}{:02}{:02}", hour, minute, second);
        prop_assert!(is_time(&time));
        prop_assert!(validate_datatype(&time, "TM"));
    }

    #[test]
    fn test_time_valid_hhmm(hour in 0u32..=23, minute in 0u32..=59) {
        let time = format!("{:02}{:02}", hour, minute);
        prop_assert!(is_time(&time));
        prop_assert!(validate_datatype(&time, "TM"));
    }

    #[test]
    fn test_time_invalid_hour(hour in 24u32..=99) {
        let time = format!("{:02}0000", hour);
        prop_assert!(!is_time(&time));
        prop_assert!(!validate_datatype(&time, "TM"));
    }

    #[test]
    fn test_time_invalid_minute(minute in 60u32..=99) {
        let time = format!("00{:02}00", minute);
        prop_assert!(!is_time(&time));
        prop_assert!(!validate_datatype(&time, "TM"));
    }
}

// ============================================================================
// Timestamp Validation Property Tests
// ============================================================================

proptest! {
    #[test]
    fn test_timestamp_valid(
        year in 1i32..=9999,
        month in 1u32..=12,
        day in 1u32..=28,
        hour in 0u32..=23,
        minute in 0u32..=59,
        second in 0u32..=59
    ) {
        let ts = format!("{:04}{:02}{:02}{:02}{:02}{:02}", year, month, day, hour, minute, second);
        prop_assert!(is_timestamp(&ts));
        prop_assert!(validate_datatype(&ts, "TS"));
    }

    #[test]
    fn test_timestamp_date_only(year in 1i32..=9999, month in 1u32..=12, day in 1u32..=28) {
        let ts = format!("{:04}{:02}{:02}", year, month, day);
        prop_assert!(is_timestamp(&ts));
        prop_assert!(validate_datatype(&ts, "TS"));
    }
}

// ============================================================================
// Range Validation Property Tests
// ============================================================================

proptest! {
    #[test]
    fn test_within_range_boundary(val in 0i32..=100) {
        let val_str = val.to_string();
        prop_assert!(is_within_range(&val_str, "0", "100"));
    }

    #[test]
    fn test_within_range_outside(val in 101i32..=200) {
        let val_str = val.to_string();
        prop_assert!(!is_within_range(&val_str, "0", "100"));
    }
}
