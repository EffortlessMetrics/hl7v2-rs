//! Integration tests for hl7v2-datatype crate
//!
//! Tests cover real-world HL7 data type validation scenarios

use hl7v2_datatype::*;

// ============================================================================
// HL7 Message Field Validation Scenarios
// ============================================================================

#[test]
fn test_msh_segment_field_types() {
    // MSH-1: Field Separator (ST)
    assert!(validate_datatype("|", "ST"));

    // MSH-2: Encoding Characters (ST)
    assert!(validate_datatype("^~\\&", "ST"));

    // MSH-7: Date/Time of Message (TS)
    assert!(validate_datatype("20250128152312", "TS"));

    // MSH-9: Message Type (MSG) - uses ID/IS components
    assert!(validate_datatype("ADT^A01", "ID"));

    // MSH-10: Message Control ID (ST)
    assert!(validate_datatype("MSG12345", "ST"));

    // MSH-11: Processing ID (PT)
    assert!(validate_datatype("P", "ID"));

    // MSH-12: Version ID (VID)
    assert!(validate_datatype("2.5.1", "ST"));
}

#[test]
fn test_pid_segment_field_types() {
    // PID-1: Set ID (SI)
    assert!(validate_datatype("1", "SI"));
    assert!(validate_datatype("123", "SI"));

    // PID-3: Patient Identifier List (CX)
    assert!(validate_datatype("12345^^^HOSP^MR", "CX"));

    // PID-5: Patient Name (PN)
    assert!(validate_datatype("Smith^John^A^^III", "PN"));

    // PID-7: Date/Time of Birth (TS)
    assert!(validate_datatype("19850615", "DT"));

    // PID-8: Administrative Sex (IS)
    assert!(validate_datatype("M", "IS"));
    assert!(validate_datatype("F", "IS"));

    // PID-11: Patient Address (AD)
    assert!(validate_datatype(
        "123 Main St^Apt 4B^Anytown^CA^12345",
        "AD"
    ));

    // PID-13: Phone Number - Home (XTN)
    assert!(validate_datatype("(555) 123-4567", "XTN"));
}

#[test]
fn test_obx_segment_field_types() {
    // OBX-1: Set ID (SI)
    assert!(validate_datatype("1", "SI"));

    // OBX-2: Value Type (ID)
    assert!(validate_datatype("NM", "ID"));
    assert!(validate_datatype("ST", "ID"));

    // OBX-3: Observation Identifier (CE)
    assert!(validate_datatype("12345^Test Name^LN", "IS"));

    // OBX-5: Observation Value (varies)
    assert!(validate_datatype("120", "NM")); // Numeric
    assert!(validate_datatype("Normal", "ST")); // String

    // OBX-6: Units (CE)
    assert!(validate_datatype("mg/dL", "IS"));

    // OBX-7: Reference Range (ST)
    assert!(validate_datatype("70-110", "ST"));
}

// ============================================================================
// Validator Builder Integration Tests
// ============================================================================

#[test]
fn test_gender_validator() {
    let validator = DataTypeValidator::new().with_allowed_values(vec![
        "M".to_string(),
        "F".to_string(),
        "O".to_string(),
        "U".to_string(),
        "A".to_string(),
        "N".to_string(),
    ]);

    assert!(validator.validate("M")); // Male
    assert!(validator.validate("F")); // Female
    assert!(validator.validate("O")); // Other
    assert!(validator.validate("U")); // Unknown
    assert!(validator.validate("A")); // Ambiguous
    assert!(validator.validate("N")); // Not applicable
    assert!(!validator.validate("X"));
    assert!(!validator.validate(""));
}

#[test]
fn test_processing_id_validator() {
    let validator = DataTypeValidator::new().with_allowed_values(vec![
        "D".to_string(), // Debugging
        "P".to_string(), // Production
        "T".to_string(), // Training
    ]);

    assert!(validator.validate("D"));
    assert!(validator.validate("P"));
    assert!(validator.validate("T"));
    assert!(!validator.validate("X"));
}

#[test]
fn test_ssn_validator() {
    let _validator = DataTypeValidator::new()
        .with_pattern(r"^\d{3}-\d{2}-\d{4}$")
        .with_checksum(ChecksumAlgorithm::Luhn);

    // Note: SSN doesn't use Luhn, this is just testing the combination
    // In practice, you'd use is_ssn() function
}

#[test]
fn test_patient_id_validator() {
    let validator = DataTypeValidator::new()
        .with_min_length(1)
        .with_max_length(20);

    assert!(validator.validate("MRN123456"));
    assert!(validator.validate("1"));
    assert!(!validator.validate("")); // Empty
    assert!(!validator.validate("123456789012345678901")); // Too long
}

#[test]
fn test_numeric_result_validator() {
    let validator = DataTypeValidator::new().with_pattern(r"^-?\d+\.?\d*$");

    assert!(validator.validate("120"));
    assert!(validator.validate("-5.5"));
    assert!(validator.validate("0.123"));
    assert!(!validator.validate("abc"));
    assert!(!validator.validate("12.34.56"));
}

// ============================================================================
// Real-World Validation Scenarios
// ============================================================================

#[test]
fn test_adt_a01_message_validation() {
    // Validate fields from a typical ADT^A01 message

    // Patient Name components
    assert!(is_person_name("Smith")); // Family name
    assert!(is_person_name("John")); // Given name
    assert!(is_person_name("A")); // Middle initial
    assert!(is_person_name("III")); // Suffix

    // Full name with HL7 delimiters
    assert!(is_person_name("Smith^John^A^^III"));

    // Date of Birth
    assert!(is_date("19850615"));

    // Phone numbers
    assert!(is_phone_number("(555) 123-4567"));
    assert!(is_phone_number("555-123-4567"));

    // SSN
    assert!(is_ssn("123-45-6789"));
}

#[test]
fn test_oru_r01_message_validation() {
    // Validate fields from a typical ORU^R01 (lab results) message

    // Observation Value - Numeric
    assert!(is_numeric("120"));
    assert!(is_numeric("98.6"));
    assert!(is_numeric("-5.0"));

    // Observation Value - String
    assert!(is_string("Normal"));
    assert!(is_string("Positive"));

    // Units
    assert!(is_string("mg/dL"));
    assert!(is_string("mmol/L"));

    // Reference Range
    assert!(is_string("70-110"));
    assert!(is_string("< 100"));

    // Abnormal Flags
    let abnormal_flags = DataTypeValidator::new().with_allowed_values(vec![
        "N".to_string(),  // Normal
        "H".to_string(),  // High
        "L".to_string(),  // Low
        "HH".to_string(), // Panic High
        "LL".to_string(), // Panic Low
    ]);

    assert!(abnormal_flags.validate("N"));
    assert!(abnormal_flags.validate("H"));
    assert!(abnormal_flags.validate("L"));
    assert!(!abnormal_flags.validate("X"));
}

#[test]
fn test_address_validation() {
    // Validate address components
    assert!(is_address("123 Main Street"));
    assert!(is_address("Apt 4B"));
    assert!(is_address("Anytown"));
    assert!(is_address("CA"));
    assert!(is_address("12345"));

    // Full address with HL7 delimiters
    assert!(is_address("123 Main St^Apt 4B^Anytown^CA^12345"));
}

#[test]
fn test_extended_id_validation() {
    // CX data type - Extended Composite ID
    assert!(is_extended_id("12345"));
    assert!(is_extended_id("MRN123456"));
    assert!(is_extended_id("12345^^^HOSP^MR"));
}

#[test]
fn test_hierarchic_designator_validation() {
    // HD data type - Hierarchic Designator
    assert!(is_hierarchic_designator("HOSPITAL"));
    assert!(is_hierarchic_designator("FACILITY.1"));
    assert!(is_hierarchic_designator("1.2.3.4.5"));
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_empty_values() {
    // Empty values are valid for ST, TX, FT
    assert!(is_string(""));
    assert!(is_text_data(""));
    assert!(is_formatted_text(""));

    // But not for required fields with min_length
    let validator = DataTypeValidator::new().with_min_length(1);
    assert!(!validator.validate(""));
}

#[test]
fn test_special_characters() {
    // HL7 escape sequences
    assert!(is_string("Text with \\E\\ escape"));
    assert!(is_string("Line1\\X0d\\Line2"));

    // Person names with special characters
    assert!(is_person_name("O'Brien"));
    assert!(is_person_name("Smith-Jones"));
    assert!(is_person_name("Dr. Smith"));
}

#[test]
fn test_unicode_handling() {
    // Unicode in string fields
    assert!(is_string("Patient name with é"));
    assert!(is_string("日本語"));

    // But not in person names (by our validation)
    // Note: This depends on the specific implementation requirements
}

// ============================================================================
// Date/Time Integration with hl7v2-datetime
// ============================================================================

#[test]
fn test_date_validation_integration() {
    // DT data type
    assert!(is_date("20250128"));
    assert!(!is_date("20251328")); // Invalid month
    assert!(!is_date("invalid"));
}

#[test]
fn test_time_validation_integration() {
    // TM data type
    assert!(is_time("152312"));
    assert!(is_time("1523"));
    assert!(!is_time("252300")); // Invalid hour
}

#[test]
fn test_timestamp_validation_integration() {
    // TS data type
    assert!(is_timestamp("20250128152312"));
    assert!(is_timestamp("20250128"));
    assert!(!is_timestamp("invalid"));
}

// ============================================================================
// Checksum Validation Scenarios
// ============================================================================

#[test]
fn test_credit_card_validation() {
    // Test credit card numbers (not real cards)
    assert!(validate_luhn_checksum("4111111111111111")); // Visa test
    assert!(validate_luhn_checksum("5500000000000004")); // Mastercard test
    assert!(validate_luhn_checksum("340000000000009")); // Amex test

    // Invalid numbers
    assert!(!validate_luhn_checksum("4111111111111112")); // One digit off
    assert!(!validate_luhn_checksum("1234567890123456")); // Random
}

#[test]
fn test_id_number_with_checksum() {
    // Example: ID numbers that use Luhn checksum
    let validator = DataTypeValidator::new()
        .with_min_length(10)
        .with_max_length(20)
        .with_checksum(ChecksumAlgorithm::Luhn);

    assert!(validator.validate("4111111111111111"));
    assert!(!validator.validate("4111111111111112"));
    assert!(!validator.validate("12345")); // Too short
}

// ============================================================================
// Format Matching Scenarios
// ============================================================================

#[test]
fn test_iso_date_format() {
    // ISO date format (YYYY-MM-DD) vs HL7 format (YYYYMMDD)
    assert!(matches_format("2025-01-28", "YYYY-MM-DD", "DT"));
    assert!(!matches_format("20250128", "YYYY-MM-DD", "DT"));

    // HL7 format should use is_date()
    assert!(is_date("20250128"));
}

#[test]
fn test_iso_time_format() {
    // ISO time format (HH:MM:SS) vs HL7 format (HHMMSS)
    assert!(matches_format("15:23:12", "HH:MM:SS", "TM"));
    assert!(!matches_format("152312", "HH:MM:SS", "TM"));

    // HL7 format should use is_time()
    assert!(is_time("152312"));
}

// ============================================================================
// Birth Date Validation
// ============================================================================

#[test]
fn test_valid_birth_dates() {
    // Valid birth dates (not in future)
    assert!(is_valid_birth_date("19850615"));
    assert!(is_valid_birth_date("20250101"));

    // Current date should be valid
    let today = chrono::Utc::now().format("%Y%m%d").to_string();
    assert!(is_valid_birth_date(&today));
}

#[test]
fn test_invalid_birth_dates() {
    // Future dates should be invalid
    let future = (chrono::Utc::now() + chrono::Duration::days(365))
        .format("%Y%m%d")
        .to_string();
    assert!(!is_valid_birth_date(&future));

    // Invalid format
    assert!(!is_valid_birth_date("invalid"));
}

// ============================================================================
// Age Range Validation
// ============================================================================

#[test]
fn test_valid_age_ranges() {
    // Birth date before reference date
    assert!(is_valid_age_range("19850615", "20250128"));
    assert!(is_valid_age_range("20250101", "20250128"));

    // Same date is valid (newborn)
    assert!(is_valid_age_range("20250128", "20250128"));
}

#[test]
fn test_invalid_age_ranges() {
    // Birth date after reference date
    assert!(!is_valid_age_range("20250128", "19850615"));

    // Invalid dates
    assert!(!is_valid_age_range("invalid", "20250128"));
    assert!(!is_valid_age_range("19850615", "invalid"));
}

// ============================================================================
// Range Validation
// ============================================================================

#[test]
fn test_numeric_range_validation() {
    // Within range
    assert!(is_within_range("50", "0", "100"));
    assert!(is_within_range("0", "0", "100")); // Min boundary
    assert!(is_within_range("100", "0", "100")); // Max boundary

    // Outside range
    assert!(!is_within_range("-1", "0", "100")); // Below min
    assert!(!is_within_range("101", "0", "100")); // Above max

    // Non-numeric
    assert!(!is_within_range("abc", "0", "100"));
}

// ============================================================================
// Combined Validation Scenarios
// ============================================================================

#[test]
fn test_patient_identifier_validation() {
    // Typical patient identifier validation
    let validator = DataTypeValidator::new()
        .with_min_length(1)
        .with_max_length(50)
        .with_pattern(r"^[A-Za-z0-9\-]+$");

    assert!(validator.validate("MRN123456"));
    assert!(validator.validate("123-456-789"));
    assert!(!validator.validate("")); // Too short
    assert!(!validator.validate("ID with spaces")); // Space not in pattern
}

#[test]
fn test_order_number_validation() {
    // Order number: alphanumeric, specific length
    let validator = DataTypeValidator::new()
        .with_min_length(3)
        .with_max_length(20)
        .with_pattern(r"^[A-Z]{2,3}\d+$");

    assert!(validator.validate("ORD12345"));
    assert!(validator.validate("AB1"));
    assert!(!validator.validate("ord12345")); // Lowercase
    assert!(!validator.validate("12345")); // No prefix
}
