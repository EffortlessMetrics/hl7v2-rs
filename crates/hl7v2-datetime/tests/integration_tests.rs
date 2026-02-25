//! Integration tests for hl7v2-datetime crate
//!
//! Tests cover real-world scenarios and cross-function interactions

use hl7v2_datetime::*;
use chrono::{Datelike, Timelike};

// ============================================================================
// Real-World HL7 Message Timestamp Tests
// ============================================================================

#[test]
fn test_msh_timestamp_parsing() {
    // Typical MSH-7 timestamp from ADT^A01 message
    let msh_timestamp = "20250128152312";
    let ts = parse_hl7_ts_with_precision(msh_timestamp).unwrap();
    
    assert_eq!(ts.precision, TimestampPrecision::Second);
    assert_eq!(ts.datetime.year(), 2025);
    assert_eq!(ts.datetime.month(), 1);
    assert_eq!(ts.datetime.day(), 28);
    assert_eq!(ts.datetime.hour(), 15);
    assert_eq!(ts.datetime.minute(), 23);
    assert_eq!(ts.datetime.second(), 12);
}

#[test]
fn test_various_msh_timestamp_formats() {
    // Different valid MSH-7 formats
    let test_cases = vec![
        ("20250128152312", TimestampPrecision::Second),
        ("202501281523", TimestampPrecision::Minute),
        ("2025012815", TimestampPrecision::Hour),
        ("20250128", TimestampPrecision::Day),
    ];
    
    for (timestamp, expected_precision) in test_cases {
        let ts = parse_hl7_ts_with_precision(timestamp).unwrap();
        assert_eq!(ts.precision, expected_precision, "Failed for timestamp: {}", timestamp);
    }
}

// ============================================================================
// Date/Time Component Extraction Tests
// ============================================================================

#[test]
fn test_extract_date_components() {
    let date = parse_hl7_dt("19850615").unwrap();
    assert_eq!(date.year(), 1985);
    assert_eq!(date.month(), 6);
    assert_eq!(date.day(), 15);
}

#[test]
fn test_extract_time_components() {
    let (h, m, s, f) = parse_hl7_tm("143052").unwrap();
    assert_eq!(h, 14);
    assert_eq!(m, 30);
    assert_eq!(s, 52);
    assert_eq!(f, None);
}

#[test]
fn test_extract_timestamp_components() {
    let ts = parse_hl7_ts("20250128152345").unwrap();
    assert_eq!(ts.year(), 2025);
    assert_eq!(ts.month(), 1);
    assert_eq!(ts.day(), 28);
    assert_eq!(ts.hour(), 15);
    assert_eq!(ts.minute(), 23);
    assert_eq!(ts.second(), 45);
}

// ============================================================================
// Timestamp Comparison Scenarios
// ============================================================================

#[test]
fn test_message_ordering_by_timestamp() {
    // Simulate ordering messages by timestamp
    let timestamps = vec![
        "20250128100000",
        "20250128080000",
        "20250128120000",
        "20250128090000",
    ];
    
    let mut parsed: Vec<_> = timestamps
        .iter()
        .map(|t| parse_hl7_ts_with_precision(t).unwrap())
        .collect();
    
    parsed.sort_by(|a, b| a.datetime.cmp(&b.datetime));
    
    assert_eq!(parsed[0].datetime.hour(), 8);
    assert_eq!(parsed[1].datetime.hour(), 9);
    assert_eq!(parsed[2].datetime.hour(), 10);
    assert_eq!(parsed[3].datetime.hour(), 12);
}

#[test]
fn test_same_day_comparison() {
    // Messages on the same day but different times
    let ts1 = parse_hl7_ts_with_precision("20250128080000").unwrap();
    let ts2 = parse_hl7_ts_with_precision("20250128170000").unwrap();
    
    assert!(ts1.is_same_day(&ts2));
    assert!(ts1.is_before(&ts2));
    assert!(ts2.is_after(&ts1));
}

#[test]
fn test_different_day_comparison() {
    let ts1 = parse_hl7_ts_with_precision("20250128080000").unwrap();
    let ts2 = parse_hl7_ts_with_precision("20250129080000").unwrap();
    
    assert!(!ts1.is_same_day(&ts2));
    assert!(ts1.is_before(&ts2));
}

// ============================================================================
// Format Validation Tests
// ============================================================================

#[test]
fn test_valid_hl7_timestamps_in_message() {
    // Valid timestamps that might appear in various HL7 fields
    let valid_timestamps = vec![
        "20250128",           // MSH-7 date only
        "202501281523",       // MSH-7 with minute precision
        "20250128152312",     // MSH-7 with second precision
        "19850615",           // PID-7 birth date
        "20250128100000",     // EVN-2 event datetime
    ];
    
    for ts in valid_timestamps {
        assert!(is_valid_hl7_timestamp(ts), "Should be valid: {}", ts);
    }
}

#[test]
fn test_invalid_hl7_timestamps() {
    let invalid_timestamps = vec![
        "2025",               // Too short
        "20251301",           // Invalid month
        "20250132",           // Invalid day
        "2025012825",         // Invalid hour (25)
        "notadate",           // Non-numeric
        "2025-01-28",         // ISO format, not HL7
    ];
    
    for ts in invalid_timestamps {
        assert!(!is_valid_hl7_timestamp(ts), "Should be invalid: {}", ts);
    }
}

// ============================================================================
// Precision Handling Tests
// ============================================================================

#[test]
fn test_precision_based_comparison() {
    // When comparing timestamps with different precisions
    let day_precision = parse_hl7_ts_with_precision("20250128").unwrap();
    let second_precision = parse_hl7_ts_with_precision("20250128120000").unwrap();
    
    // Day precision timestamp is at midnight
    // Second precision at noon is after midnight
    assert!(day_precision.is_before(&second_precision));
}

#[test]
fn test_precision_roundtrip() {
    // Parse and format should be symmetric
    let test_cases = vec![
        "2025",
        "202501",
        "20250128",
        "2025012815",
        "202501281523",
        "20250128152312",
    ];
    
    for original in test_cases {
        let ts = parse_hl7_ts_with_precision(original).unwrap();
        let formatted = ts.to_hl7_string();
        assert_eq!(original, formatted, "Roundtrip failed for: {}", original);
    }
}

// ============================================================================
// Edge Cases from Real HL7 Messages
// ============================================================================

#[test]
fn test_midnight_timestamp() {
    let ts = parse_hl7_ts("20250128000000").unwrap();
    assert_eq!(ts.hour(), 0);
    assert_eq!(ts.minute(), 0);
    assert_eq!(ts.second(), 0);
}

#[test]
fn test_end_of_day_timestamp() {
    let ts = parse_hl7_ts("20250128235959").unwrap();
    assert_eq!(ts.hour(), 23);
    assert_eq!(ts.minute(), 59);
    assert_eq!(ts.second(), 59);
}

#[test]
fn test_year_boundary() {
    // New Year's Eve
    let ts1 = parse_hl7_ts_with_precision("20241231235959").unwrap();
    // New Year's Day
    let ts2 = parse_hl7_ts_with_precision("20250101000000").unwrap();
    
    assert!(ts1.is_before(&ts2));
    assert!(!ts1.is_same_day(&ts2));
}

#[test]
fn test_leap_year_scenarios() {
    // Feb 29, 2024 (leap year)
    let ts = parse_hl7_ts("20240229120000").unwrap();
    assert_eq!(ts.month(), 2);
    assert_eq!(ts.day(), 29);
    
    // Feb 28, 2025 (non-leap year)
    let ts = parse_hl7_ts("20250228120000").unwrap();
    assert_eq!(ts.month(), 2);
    assert_eq!(ts.day(), 28);
}

// ============================================================================
// Fractional Seconds Tests
// ============================================================================

#[test]
fn test_fractional_seconds_parsing() {
    let (h, m, s, f) = parse_hl7_tm("120000.123").unwrap();
    assert_eq!(h, 12);
    assert_eq!(m, 0);
    assert_eq!(s, 0);
    assert_eq!(f, Some(123000)); // Padded to microseconds
}

#[test]
fn test_fractional_seconds_precision() {
    let ts = parse_hl7_ts_with_precision("20250128152312.123456").unwrap();
    assert_eq!(ts.precision, TimestampPrecision::FractionalSecond);
    assert_eq!(ts.fractional_seconds, Some(123456));
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_error_types_for_invalid_dates() {
    let result = parse_hl7_dt("invalid");
    assert!(result.is_err());
    match result.unwrap_err() {
        DateTimeError::InvalidDateFormat(_) => (),
        _ => panic!("Expected InvalidDateFormat error"),
    }
}

#[test]
fn test_error_types_for_invalid_times() {
    let result = parse_hl7_tm("99");
    assert!(result.is_err());
    match result.unwrap_err() {
        DateTimeError::InvalidTimeFormat(_) => (),
        _ => panic!("Expected InvalidTimeFormat error"),
    }
}

#[test]
fn test_error_types_for_out_of_range_times() {
    let result = parse_hl7_tm("2500");
    assert!(result.is_err());
    match result.unwrap_err() {
        DateTimeError::TimeOutOfRange(_) => (),
        _ => panic!("Expected TimeOutOfRange error"),
    }
}

// ============================================================================
// Current Time Functions Tests
// ============================================================================

#[test]
fn test_current_timestamp_format() {
    let now = now_hl7();
    // Should be parseable as a valid timestamp
    assert!(is_valid_hl7_timestamp(&now));
    
    // Should be 14 characters (YYYYMMDDHHMMSS)
    assert_eq!(now.len(), 14);
}

#[test]
fn test_current_date_format() {
    let today = today_hl7();
    // Should be parseable as a valid date
    assert!(is_valid_hl7_date(&today));
    
    // Should be 8 characters (YYYYMMDD)
    assert_eq!(today.len(), 8);
}

// ============================================================================
// Timestamp Arithmetic Scenarios
// ============================================================================

#[test]
fn test_timestamps_within_24_hours() {
    let ts1 = parse_hl7_ts_with_precision("20250128080000").unwrap();
    let ts2 = parse_hl7_ts_with_precision("20250128160000").unwrap();
    let ts3 = parse_hl7_ts_with_precision("20250129070000").unwrap();
    
    // ts1 and ts2 are same day
    assert!(ts1.is_same_day(&ts2));
    
    // ts1 and ts3 are different days (even though < 24 hours apart)
    assert!(!ts1.is_same_day(&ts3));
    
    // Ordering is correct
    assert!(ts1.is_before(&ts2));
    assert!(ts2.is_before(&ts3));
}

// ============================================================================
// Birth Date Validation Scenarios
// ============================================================================

#[test]
fn test_birth_date_formats() {
    // Common birth date formats in HL7
    let birth_dates = vec![
        "19850615",
        "19500101",
        "20231231",
        "19000101",
    ];
    
    for date in birth_dates {
        assert!(is_valid_hl7_date(date), "Should be valid birth date: {}", date);
    }
}

#[test]
fn test_invalid_birth_dates() {
    let invalid_dates = vec![
        "19851315",  // Invalid month
        "19850632",  // Invalid day
        "19000230",  // Feb 30 doesn't exist
    ];
    
    for date in invalid_dates {
        assert!(!is_valid_hl7_date(date), "Should be invalid: {}", date);
    }
}
