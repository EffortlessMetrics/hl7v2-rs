//! Unit tests for hl7v2-datatype crate
//!
//! Tests cover:
//! - Data type definitions
//! - Data type validation
//! - Type conversions
//! - Validator builder

use super::*;
use chrono::Datelike;

// ============================================================================
// DataType Enum Tests
// ============================================================================

#[cfg(test)]
mod datatype_enum_tests {
    use super::*;

    #[test]
    fn test_datatype_parse_valid() {
        assert_eq!(DataType::parse("ST"), Some(DataType::ST));
        assert_eq!(DataType::parse("ID"), Some(DataType::ID));
        assert_eq!(DataType::parse("IS"), Some(DataType::IS));
        assert_eq!(DataType::parse("DT"), Some(DataType::DT));
        assert_eq!(DataType::parse("TM"), Some(DataType::TM));
        assert_eq!(DataType::parse("TS"), Some(DataType::TS));
        assert_eq!(DataType::parse("NM"), Some(DataType::NM));
        assert_eq!(DataType::parse("SI"), Some(DataType::SI));
        assert_eq!(DataType::parse("TX"), Some(DataType::TX));
        assert_eq!(DataType::parse("FT"), Some(DataType::FT));
        assert_eq!(DataType::parse("PN"), Some(DataType::PN));
        assert_eq!(DataType::parse("CX"), Some(DataType::CX));
        assert_eq!(DataType::parse("HD"), Some(DataType::HD));
        assert_eq!(DataType::parse("AD"), Some(DataType::AD));
        assert_eq!(DataType::parse("XTN"), Some(DataType::XTN));
    }

    #[test]
    fn test_datatype_parse_invalid() {
        assert_eq!(DataType::parse("INVALID"), None);
        assert_eq!(DataType::parse(""), None);
        assert_eq!(DataType::parse("st"), None); // Case sensitive
        assert_eq!(DataType::parse("XX"), None);
    }

    #[test]
    fn test_datatype_equality() {
        assert_eq!(DataType::ST, DataType::ST);
        assert_ne!(DataType::ST, DataType::ID);
    }

    #[test]
    fn test_datatype_clone() {
        let dt = DataType::PN;
        let cloned = dt;
        assert_eq!(dt, cloned);
    }
}

// ============================================================================
// DataTypeValidator Tests
// ============================================================================

#[cfg(test)]
mod validator_tests {
    use super::*;

    #[test]
    fn test_validator_new() {
        let validator = DataTypeValidator::new();
        assert!(validator.min_length.is_none());
        assert!(validator.max_length.is_none());
        assert!(validator.pattern.is_none());
        assert!(validator.allowed_values.is_none());
        assert!(validator.checksum.is_none());
    }

    #[test]
    fn test_validator_default() {
        let validator = DataTypeValidator::default();
        // Default validator should pass any value
        assert!(validator.validate("any value"));
    }

    #[test]
    fn test_validator_min_length() {
        let validator = DataTypeValidator::new().with_min_length(5);

        assert!(!validator.validate("abc")); // Too short
        assert!(validator.validate("abcde")); // Exactly min
        assert!(validator.validate("abcdef")); // Longer than min
    }

    #[test]
    fn test_validator_max_length() {
        let validator = DataTypeValidator::new().with_max_length(10);

        assert!(validator.validate("abc")); // Under max
        assert!(validator.validate("abcdeabcde")); // Exactly max
        assert!(!validator.validate("abcdeabcdef")); // Over max
    }

    #[test]
    fn test_validator_min_max_length() {
        let validator = DataTypeValidator::new()
            .with_min_length(3)
            .with_max_length(10);

        assert!(!validator.validate("ab")); // Too short
        assert!(validator.validate("abc")); // Min boundary
        assert!(validator.validate("abcde")); // In range
        assert!(validator.validate("abcdeabcde")); // Max boundary
        assert!(!validator.validate("abcdeabcdef")); // Too long
    }

    #[test]
    fn test_validator_pattern() {
        let validator = DataTypeValidator::new().with_pattern(r"^\d{3}$");

        assert!(validator.validate("123"));
        assert!(validator.validate("456"));
        assert!(!validator.validate("12"));
        assert!(!validator.validate("1234"));
        assert!(!validator.validate("abc"));
    }

    #[test]
    fn test_validator_allowed_values() {
        let validator = DataTypeValidator::new().with_allowed_values(vec![
            "M".to_string(),
            "F".to_string(),
            "U".to_string(),
        ]);

        assert!(validator.validate("M"));
        assert!(validator.validate("F"));
        assert!(validator.validate("U"));
        assert!(!validator.validate("X"));
        assert!(!validator.validate("m")); // Case sensitive
    }

    #[test]
    fn test_validator_checksum_luhn() {
        let validator = DataTypeValidator::new().with_checksum(ChecksumAlgorithm::Luhn);

        // Valid Luhn numbers
        assert!(validator.validate("4532015112830366")); // Test Visa
        assert!(validator.validate("6011111111111117")); // Test Discover

        // Invalid Luhn numbers
        assert!(!validator.validate("4532015112830367"));
        assert!(!validator.validate("1234567890123456"));
    }

    #[test]
    fn test_validator_checksum_mod10() {
        let validator = DataTypeValidator::new().with_checksum(ChecksumAlgorithm::Mod10);

        // Mod10 uses same algorithm as Luhn
        assert!(validator.validate("4532015112830366"));
        assert!(!validator.validate("4532015112830367"));
    }

    #[test]
    fn test_validator_combined_constraints() {
        let validator = DataTypeValidator::new()
            .with_min_length(16)
            .with_max_length(16)
            .with_checksum(ChecksumAlgorithm::Luhn);

        assert!(validator.validate("4532015112830366"));
        assert!(!validator.validate("453201511283036")); // Wrong length
        assert!(!validator.validate("4532015112830367")); // Bad checksum
    }

    #[test]
    fn test_validator_validate_detailed_success() {
        let validator = DataTypeValidator::new()
            .with_min_length(1)
            .with_max_length(10);

        let result = validator.validate_detailed("test");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validator_validate_detailed_too_short() {
        let validator = DataTypeValidator::new().with_min_length(5);

        let result = validator.validate_detailed("abc");
        assert!(result.is_err());
        match result.unwrap_err() {
            DataTypeError::TooShort { length, min } => {
                assert_eq!(length, 3);
                assert_eq!(min, 5);
            }
            _ => panic!("Expected TooShort error"),
        }
    }

    #[test]
    fn test_validator_validate_detailed_too_long() {
        let validator = DataTypeValidator::new().with_max_length(5);

        let result = validator.validate_detailed("abcdefgh");
        assert!(result.is_err());
        match result.unwrap_err() {
            DataTypeError::TooLong { length, max } => {
                assert_eq!(length, 8);
                assert_eq!(max, 5);
            }
            _ => panic!("Expected TooLong error"),
        }
    }

    #[test]
    fn test_validator_validate_detailed_pattern_mismatch() {
        let validator = DataTypeValidator::new().with_pattern(r"^\d+$");

        let result = validator.validate_detailed("abc123");
        assert!(result.is_err());
        match result.unwrap_err() {
            DataTypeError::PatternMismatch { value, pattern } => {
                assert_eq!(value, "abc123");
                assert_eq!(pattern, r"^\d+$");
            }
            _ => panic!("Expected PatternMismatch error"),
        }
    }

    #[test]
    fn test_validator_validate_detailed_not_in_allowed_set() {
        let validator =
            DataTypeValidator::new().with_allowed_values(vec!["A".to_string(), "B".to_string()]);

        let result = validator.validate_detailed("C");
        assert!(result.is_err());
        match result.unwrap_err() {
            DataTypeError::NotInAllowedSet { value } => {
                assert_eq!(value, "C");
            }
            _ => panic!("Expected NotInAllowedSet error"),
        }
    }

    #[test]
    fn test_validator_validate_detailed_checksum_failed() {
        let validator = DataTypeValidator::new().with_checksum(ChecksumAlgorithm::Luhn);

        let result = validator.validate_detailed("1234567890123456");
        assert!(result.is_err());
        match result.unwrap_err() {
            DataTypeError::ChecksumFailed => (),
            _ => panic!("Expected ChecksumFailed error"),
        }
    }
}

// ============================================================================
// validate_datatype Function Tests
// ============================================================================

#[cfg(test)]
mod validate_datatype_tests {
    use super::*;

    #[test]
    fn test_validate_string() {
        assert!(validate_datatype("any string", "ST"));
        assert!(validate_datatype("", "ST"));
        assert!(validate_datatype("test123!@#", "ST"));
    }

    #[test]
    fn test_validate_identifier() {
        assert!(validate_datatype("ABC123", "ID"));
        assert!(validate_datatype("test-value", "ID"));
        assert!(!validate_datatype("test\nvalue", "ID")); // Control char
    }

    #[test]
    fn test_validate_coded_value() {
        assert!(validate_datatype("CODE1", "IS"));
        assert!(validate_datatype("123", "IS"));
    }

    #[test]
    fn test_validate_date() {
        assert!(validate_datatype("20250128", "DT"));
        assert!(!validate_datatype("20251328", "DT")); // Invalid month
        assert!(!validate_datatype("invalid", "DT"));
    }

    #[test]
    fn test_validate_time() {
        assert!(validate_datatype("152312", "TM"));
        assert!(validate_datatype("1523", "TM"));
        assert!(!validate_datatype("252300", "TM")); // Invalid hour
    }

    #[test]
    fn test_validate_timestamp() {
        assert!(validate_datatype("20250128152312", "TS"));
        assert!(validate_datatype("20250128", "TS"));
        assert!(!validate_datatype("invalid", "TS"));
    }

    #[test]
    fn test_validate_numeric() {
        assert!(validate_datatype("123", "NM"));
        assert!(validate_datatype("123.45", "NM"));
        assert!(validate_datatype("-123", "NM"));
        assert!(validate_datatype("0", "NM"));
        assert!(validate_datatype("-123.456", "NM"));
        assert!(!validate_datatype("abc", "NM"));
        assert!(!validate_datatype("12.34.56", "NM"));
    }

    #[test]
    fn test_validate_sequence_id() {
        assert!(validate_datatype("1", "SI"));
        assert!(validate_datatype("123", "SI"));
        assert!(!validate_datatype("0", "SI")); // Must be > 0
        assert!(!validate_datatype("-1", "SI")); // Must be positive
        assert!(!validate_datatype("abc", "SI"));
    }

    #[test]
    fn test_validate_text_data() {
        assert!(validate_datatype("any text", "TX"));
        assert!(validate_datatype("", "TX"));
    }

    #[test]
    fn test_validate_formatted_text() {
        assert!(validate_datatype("formatted text", "FT"));
        assert!(validate_datatype("", "FT"));
    }

    #[test]
    fn test_validate_person_name() {
        assert!(validate_datatype("Smith^John", "PN"));
        assert!(validate_datatype("O'Brien^Mary", "PN"));
        assert!(validate_datatype("Doe-Jane", "PN"));
        assert!(validate_datatype("Dr. Smith", "PN"));
        assert!(!validate_datatype("Smith123", "PN")); // Contains digits
    }

    #[test]
    fn test_validate_extended_id() {
        assert!(validate_datatype("12345", "CX"));
        assert!(validate_datatype("ABC-123", "CX"));
    }

    #[test]
    fn test_validate_hierarchic_designator() {
        assert!(validate_datatype("HOSPITAL.1", "HD"));
        assert!(validate_datatype("FACILITY", "HD"));
    }

    #[test]
    fn test_validate_address() {
        assert!(validate_datatype("123 Main St", "AD"));
        assert!(validate_datatype("Apt 4B, 456 Oak Ave", "AD"));
        assert!(!validate_datatype("Line\nBreak", "AD")); // Control char
    }

    #[test]
    fn test_validate_phone_number() {
        assert!(validate_datatype("1234567", "XTN"));
        assert!(validate_datatype("1234567890", "XTN"));
        assert!(validate_datatype("(555) 123-4567", "XTN"));
        assert!(validate_datatype("555-123-4567", "XTN"));
        assert!(!validate_datatype("123", "XTN")); // Too short
        assert!(!validate_datatype("1234567890123456", "XTN")); // Too long
    }

    #[test]
    fn test_validate_unknown_datatype() {
        // Unknown data types should return true (permissive)
        assert!(validate_datatype("anything", "UNKNOWN"));
        assert!(validate_datatype("", "XX"));
    }
}

// ============================================================================
// Specific Validation Function Tests
// ============================================================================

#[cfg(test)]
mod specific_validation_tests {
    use super::*;

    #[test]
    fn test_is_string() {
        assert!(is_string("anything"));
        assert!(is_string(""));
    }

    #[test]
    fn test_is_identifier_valid() {
        assert!(is_identifier("ABC123"));
        assert!(is_identifier("test-value_123"));
        assert!(is_identifier("A"));
    }

    #[test]
    fn test_is_identifier_invalid() {
        assert!(!is_identifier("test\nvalue")); // Control char
        assert!(!is_identifier("test\u{0000}")); // Null char
    }

    #[test]
    fn test_is_numeric_valid() {
        assert!(is_numeric("123"));
        assert!(is_numeric("123.456"));
        assert!(is_numeric("-123"));
        assert!(is_numeric("-123.456"));
        assert!(is_numeric("0"));
        assert!(is_numeric("0.0"));
    }

    #[test]
    fn test_is_numeric_invalid() {
        assert!(!is_numeric("abc"));
        assert!(!is_numeric("12.34.56"));
        assert!(!is_numeric(""));
    }

    #[test]
    fn test_is_sequence_id_valid() {
        assert!(is_sequence_id("1"));
        assert!(is_sequence_id("100"));
        assert!(is_sequence_id("999999"));
    }

    #[test]
    fn test_is_sequence_id_invalid() {
        assert!(!is_sequence_id("0")); // Must be > 0
        assert!(!is_sequence_id("-1")); // Negative
        assert!(!is_sequence_id("abc"));
    }

    #[test]
    fn test_is_person_name_valid() {
        assert!(is_person_name("Smith"));
        assert!(is_person_name("O'Brien"));
        assert!(is_person_name("Smith-Jones"));
        assert!(is_person_name("Dr. Smith"));
        assert!(is_person_name("Smith^John"));
    }

    #[test]
    fn test_is_person_name_invalid() {
        assert!(!is_person_name("Smith123")); // Contains digits
        assert!(!is_person_name("Smith@John")); // Invalid character
    }

    #[test]
    fn test_is_phone_number_valid() {
        assert!(is_phone_number("1234567"));
        assert!(is_phone_number("1234567890"));
        assert!(is_phone_number("(555) 123-4567"));
        assert!(is_phone_number("555-123-4567"));
        assert!(is_phone_number("+1 555 123 4567"));
    }

    #[test]
    fn test_is_phone_number_invalid() {
        assert!(!is_phone_number("123")); // Too short (only 3 digits)
        assert!(!is_phone_number("123456")); // Too short (only 6 digits)
        assert!(!is_phone_number("1234567890123456")); // Too long (16 digits)
    }

    #[test]
    fn test_is_email_valid() {
        assert!(is_email("test@example.com"));
        assert!(is_email("user.name@domain.org"));
        assert!(is_email("user+tag@example.com"));
        assert!(is_email("a@b.co"));
    }

    #[test]
    fn test_is_email_invalid() {
        assert!(!is_email("invalid"));
        assert!(!is_email("@example.com"));
        assert!(!is_email("test@"));
        assert!(!is_email("test@example")); // No TLD
    }

    #[test]
    fn test_is_ssn_valid() {
        assert!(is_ssn("123-45-6789"));
        assert!(is_ssn("123456789"));
        assert!(is_ssn("123 45 6789")); // Spaces are stripped
    }

    #[test]
    fn test_is_ssn_invalid_area() {
        assert!(!is_ssn("000-45-6789")); // Area 000
        assert!(!is_ssn("666-45-6789")); // Area 666
        assert!(!is_ssn("900-45-6789")); // Area 900+
    }

    #[test]
    fn test_is_ssn_invalid_group() {
        assert!(!is_ssn("123-00-6789")); // Group 00
    }

    #[test]
    fn test_is_ssn_invalid_serial() {
        assert!(!is_ssn("123-45-0000")); // Serial 0000
    }

    #[test]
    fn test_is_ssn_wrong_length() {
        assert!(!is_ssn("123-45-678")); // Too short
        assert!(!is_ssn("123-45-67890")); // Too long
    }

    #[test]
    fn test_is_address_valid() {
        assert!(is_address("123 Main St"));
        assert!(is_address("Apt 4B, 456 Oak Ave"));
        assert!(is_address("P.O. Box 123"));
    }

    #[test]
    fn test_is_address_invalid() {
        assert!(!is_address("Line\nBreak")); // Control char
    }

    #[test]
    fn test_is_valid_birth_date() {
        let current_year = chrono::Utc::now().year();
        let past_date = format!("{}0101", current_year - 30);
        let future_date = format!("{}0101", current_year + 1);

        assert!(is_valid_birth_date(&past_date));
        assert!(!is_valid_birth_date(&future_date));
        assert!(!is_valid_birth_date("invalid"));
    }

    #[test]
    fn test_is_valid_age_range() {
        assert!(is_valid_age_range("19900101", "20250128"));
        assert!(is_valid_age_range("20250101", "20250101")); // Same day
        assert!(!is_valid_age_range("20250128", "19900101")); // Birth after reference
        assert!(!is_valid_age_range("invalid", "20250128"));
    }

    #[test]
    fn test_is_within_range() {
        assert!(is_within_range("5", "1", "10"));
        assert!(is_within_range("1", "1", "10")); // Min boundary
        assert!(is_within_range("10", "1", "10")); // Max boundary
        assert!(!is_within_range("0", "1", "10")); // Below min
        assert!(!is_within_range("11", "1", "10")); // Above max
        assert!(!is_within_range("abc", "1", "10")); // Non-numeric
    }
}

// ============================================================================
// Luhn Checksum Tests
// ============================================================================

#[cfg(test)]
mod luhn_tests {
    use super::*;

    #[test]
    fn test_luhn_valid_numbers() {
        // Test credit card numbers (these are test numbers, not real)
        assert!(validate_luhn_checksum("4532015112830366")); // Visa
        assert!(validate_luhn_checksum("6011111111111117")); // Discover
        assert!(validate_luhn_checksum("378282246310005")); // Amex
        assert!(validate_luhn_checksum("4111111111111111")); // Visa test
    }

    #[test]
    fn test_luhn_invalid_numbers() {
        assert!(!validate_luhn_checksum("4532015112830367")); // One digit off
        assert!(!validate_luhn_checksum("1234567890123456")); // Random invalid
    }

    #[test]
    fn test_luhn_with_formatting() {
        // Formatting characters are stripped
        assert!(validate_luhn_checksum("4532-0151-1283-0366"));
        assert!(validate_luhn_checksum("4532 0151 1283 0366"));
    }

    #[test]
    fn test_luhn_too_short() {
        assert!(!validate_luhn_checksum("1"));
        assert!(!validate_luhn_checksum(""));
    }

    #[test]
    fn test_mod10_same_as_luhn() {
        // Mod10 should behave identically to Luhn
        assert_eq!(
            validate_mod10_checksum("4532015112830366"),
            validate_luhn_checksum("4532015112830366")
        );
        assert_eq!(
            validate_mod10_checksum("4532015112830367"),
            validate_luhn_checksum("4532015112830367")
        );
    }
}

// ============================================================================
// Format Matching Tests
// ============================================================================

#[cfg(test)]
mod format_matching_tests {
    use super::*;

    #[test]
    fn test_matches_format_date_yyyy_mm_dd_valid() {
        assert!(matches_format("2025-01-28", "YYYY-MM-DD", "DT"));
        assert!(matches_format("1900-12-31", "YYYY-MM-DD", "DT"));
    }

    #[test]
    fn test_matches_format_date_yyyy_mm_dd_invalid() {
        assert!(!matches_format("2025-13-28", "YYYY-MM-DD", "DT")); // Invalid month
        assert!(!matches_format("2025-01-32", "YYYY-MM-DD", "DT")); // Invalid day
        assert!(!matches_format("2025/01/28", "YYYY-MM-DD", "DT")); // Wrong separator
        assert!(!matches_format("20250128", "YYYY-MM-DD", "DT")); // No separators
        assert!(!matches_format("25-01-28", "YYYY-MM-DD", "DT")); // 2-digit year
    }

    #[test]
    fn test_matches_format_time_hh_mm_ss_valid() {
        assert!(matches_format("15:23:12", "HH:MM:SS", "TM"));
        assert!(matches_format("00:00:00", "HH:MM:SS", "TM"));
        assert!(matches_format("23:59:59", "HH:MM:SS", "TM"));
    }

    #[test]
    fn test_matches_format_time_hh_mm_ss_invalid() {
        assert!(!matches_format("24:00:00", "HH:MM:SS", "TM")); // Invalid hour
        assert!(!matches_format("23:60:00", "HH:MM:SS", "TM")); // Invalid minute
        assert!(!matches_format("23:59:60", "HH:MM:SS", "TM")); // Invalid second
        assert!(!matches_format("152312", "HH:MM:SS", "TM")); // No separators
    }

    #[test]
    fn test_matches_format_unknown() {
        // Unknown formats should return true (permissive)
        assert!(matches_format("anything", "UNKNOWN", "ST"));
    }
}

// ============================================================================
// Error Type Tests
// ============================================================================

#[cfg(test)]
mod error_tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = DataTypeError::InvalidDataType {
            datatype: "XX".to_string(),
            reason: "Unknown type".to_string(),
        };
        assert!(err.to_string().contains("XX"));
        assert!(err.to_string().contains("Unknown type"));

        let err = DataTypeError::TooShort { length: 3, min: 5 };
        assert!(err.to_string().contains("3"));
        assert!(err.to_string().contains("5"));

        let err = DataTypeError::TooLong { length: 10, max: 5 };
        assert!(err.to_string().contains("10"));
        assert!(err.to_string().contains("5"));

        let err = DataTypeError::PatternMismatch {
            value: "abc".to_string(),
            pattern: r"^\d+$".to_string(),
        };
        assert!(err.to_string().contains("abc"));
        assert!(err.to_string().contains(r"^\d+$"));

        let err = DataTypeError::NotInAllowedSet {
            value: "X".to_string(),
        };
        assert!(err.to_string().contains("X"));

        let err = DataTypeError::ChecksumFailed;
        assert!(err.to_string().contains("Checksum"));
    }

    #[test]
    fn test_error_clone() {
        let err = DataTypeError::TooShort { length: 3, min: 5 };
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    #[test]
    fn test_error_partial_eq() {
        let err1 = DataTypeError::TooShort { length: 3, min: 5 };
        let err2 = DataTypeError::TooShort { length: 3, min: 5 };
        let err3 = DataTypeError::TooShort { length: 4, min: 5 };

        assert_eq!(err1, err2);
        assert_ne!(err1, err3);
    }
}

// ============================================================================
// ChecksumAlgorithm Tests
// ============================================================================

#[cfg(test)]
mod checksum_algorithm_tests {
    use super::*;

    #[test]
    fn test_checksum_algorithm_equality() {
        assert_eq!(ChecksumAlgorithm::Luhn, ChecksumAlgorithm::Luhn);
        assert_eq!(ChecksumAlgorithm::Mod10, ChecksumAlgorithm::Mod10);
        assert_ne!(ChecksumAlgorithm::Luhn, ChecksumAlgorithm::Mod10);
    }

    #[test]
    fn test_checksum_algorithm_clone() {
        let algo = ChecksumAlgorithm::Luhn;
        let cloned = algo;
        assert_eq!(algo, cloned);
    }
}
