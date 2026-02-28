//! Unit tests for hl7v2-datetime crate
//!
//! Tests cover:
//! - HL7 date format parsing (DT)
//! - HL7 time format parsing (TM)
//! - HL7 timestamp format parsing (TS)
//! - Precision handling
//! - Timestamp comparisons
//! - Validation functions

use super::*;

// ============================================================================
// Date (DT) Tests
// ============================================================================

#[cfg(test)]
mod date_tests {
    use super::*;

    #[test]
    fn test_parse_valid_date() {
        let date = parse_hl7_dt("20250128").unwrap();
        assert_eq!(date.year(), 2025);
        assert_eq!(date.month(), 1);
        assert_eq!(date.day(), 28);
    }

    #[test]
    fn test_parse_date_leap_year() {
        // Feb 29, 2024 is valid (leap year)
        let date = parse_hl7_dt("20240229").unwrap();
        assert_eq!(date.year(), 2024);
        assert_eq!(date.month(), 2);
        assert_eq!(date.day(), 29);
    }

    #[test]
    fn test_parse_date_boundary_values() {
        // January 1
        let date = parse_hl7_dt("20250101").unwrap();
        assert_eq!(date.month(), 1);
        assert_eq!(date.day(), 1);

        // December 31
        let date = parse_hl7_dt("20251231").unwrap();
        assert_eq!(date.month(), 12);
        assert_eq!(date.day(), 31);
    }

    #[test]
    fn test_parse_date_with_whitespace() {
        let date = parse_hl7_dt("  20250128  ").unwrap();
        assert_eq!(date.year(), 2025);
    }

    #[test]
    fn test_parse_date_invalid_month() {
        let result = parse_hl7_dt("20251301");
        assert!(result.is_err());
        match result.unwrap_err() {
            DateTimeError::InvalidDateFormat(_) => (),
            _ => panic!("Expected InvalidDateFormat error"),
        }
    }

    #[test]
    fn test_parse_date_invalid_day() {
        // Day 32
        assert!(parse_hl7_dt("20250132").is_err());
        // Day 0
        assert!(parse_hl7_dt("20250100").is_err());
        // Feb 30
        assert!(parse_hl7_dt("20250230").is_err());
    }

    #[test]
    fn test_parse_date_invalid_length() {
        // Too short
        assert!(parse_hl7_dt("2025").is_err());
        assert!(parse_hl7_dt("2025012").is_err());
        // Too long
        assert!(parse_hl7_dt("202501281").is_err());
    }

    #[test]
    fn test_parse_date_non_digit() {
        assert!(parse_hl7_dt("2025012a").is_err());
        assert!(parse_hl7_dt("abcdefgh").is_err());
        assert!(parse_hl7_dt("2025-01-28").is_err());
    }

    #[test]
    fn test_parse_date_empty() {
        assert!(parse_hl7_dt("").is_err());
        assert!(parse_hl7_dt("   ").is_err());
    }

    #[test]
    fn test_is_valid_hl7_date_valid() {
        assert!(is_valid_hl7_date("20250128"));
        assert!(is_valid_hl7_date("20240229")); // Leap year
        assert!(is_valid_hl7_date("20000101"));
        assert!(is_valid_hl7_date("20991231"));
    }

    #[test]
    fn test_is_valid_hl7_date_invalid() {
        assert!(!is_valid_hl7_date("20251328")); // Invalid month
        assert!(!is_valid_hl7_date("20250132")); // Invalid day
        assert!(!is_valid_hl7_date("2025")); // Too short
        assert!(!is_valid_hl7_date("2025012a")); // Non-digit
    }
}

// ============================================================================
// Time (TM) Tests
// ============================================================================

#[cfg(test)]
mod time_tests {
    use super::*;

    #[test]
    fn test_parse_valid_time_hour_minute() {
        let (h, m, s, f) = parse_hl7_tm("1523").unwrap();
        assert_eq!(h, 15);
        assert_eq!(m, 23);
        assert_eq!(s, 0);
        assert_eq!(f, None);
    }

    #[test]
    fn test_parse_valid_time_with_seconds() {
        let (h, m, s, f) = parse_hl7_tm("152312").unwrap();
        assert_eq!(h, 15);
        assert_eq!(m, 23);
        assert_eq!(s, 12);
        assert_eq!(f, None);
    }

    #[test]
    fn test_parse_valid_time_with_fractional_seconds() {
        let (h, m, s, f) = parse_hl7_tm("152312.123").unwrap();
        assert_eq!(h, 15);
        assert_eq!(m, 23);
        assert_eq!(s, 12);
        assert_eq!(f, Some(123000));
    }

    #[test]
    fn test_parse_time_fractional_various_lengths() {
        // Single digit fractional
        let (_h, _m, _s, f) = parse_hl7_tm("120000.1").unwrap();
        assert_eq!(f, Some(100000));

        // Two digit fractional
        let (_h, _m, _s, f) = parse_hl7_tm("120000.12").unwrap();
        assert_eq!(f, Some(120000));

        // Four digit fractional (padded to 6)
        let (_h, _m, _s, f) = parse_hl7_tm("120000.1234").unwrap();
        assert_eq!(f, Some(123400));

        // Six digit fractional
        let (_h, _m, _s, f) = parse_hl7_tm("120000.123456").unwrap();
        assert_eq!(f, Some(123456));
    }

    #[test]
    fn test_parse_time_boundary_values() {
        // Midnight
        let (h, m, _s, _f) = parse_hl7_tm("0000").unwrap();
        assert_eq!(h, 0);
        assert_eq!(m, 0);

        // Just before midnight
        let (h, m, s, _f) = parse_hl7_tm("235959").unwrap();
        assert_eq!(h, 23);
        assert_eq!(m, 59);
        assert_eq!(s, 59);

        // Noon
        let (h, _m, _s, _f) = parse_hl7_tm("120000").unwrap();
        assert_eq!(h, 12);
    }

    #[test]
    fn test_parse_time_invalid_hour() {
        assert!(parse_hl7_tm("2400").is_err()); // Hour 24
        assert!(parse_hl7_tm("2500").is_err()); // Hour 25
    }

    #[test]
    fn test_parse_time_invalid_minute() {
        assert!(parse_hl7_tm("2360").is_err()); // Minute 60
        assert!(parse_hl7_tm("2399").is_err()); // Minute 99
    }

    #[test]
    fn test_parse_time_invalid_second() {
        assert!(parse_hl7_tm("120060").is_err()); // Second 60
        assert!(parse_hl7_tm("120099").is_err()); // Second 99
    }

    #[test]
    fn test_parse_time_too_short() {
        assert!(parse_hl7_tm("").is_err());
        assert!(parse_hl7_tm("1").is_err());
        assert!(parse_hl7_tm("12").is_err());
        assert!(parse_hl7_tm("123").is_err());
    }

    #[test]
    fn test_parse_time_with_whitespace() {
        let (h, m, _s, _f) = parse_hl7_tm("  1523  ").unwrap();
        assert_eq!(h, 15);
        assert_eq!(m, 23);
    }

    #[test]
    fn test_is_valid_hl7_time_valid() {
        assert!(is_valid_hl7_time("0000"));
        assert!(is_valid_hl7_time("2359"));
        assert!(is_valid_hl7_time("120000"));
        assert!(is_valid_hl7_time("152312.123"));
    }

    #[test]
    fn test_is_valid_hl7_time_invalid() {
        assert!(!is_valid_hl7_time("2400"));
        assert!(!is_valid_hl7_time("1260"));
        assert!(!is_valid_hl7_time("12")); // Too short
    }
}

// ============================================================================
// Timestamp (TS) Tests
// ============================================================================

#[cfg(test)]
mod timestamp_tests {
    use super::*;

    #[test]
    fn test_parse_valid_timestamp_full() {
        let ts = parse_hl7_ts("20250128152312").unwrap();
        assert_eq!(ts.year(), 2025);
        assert_eq!(ts.month(), 1);
        assert_eq!(ts.day(), 28);
        assert_eq!(ts.hour(), 15);
        assert_eq!(ts.minute(), 23);
        assert_eq!(ts.second(), 12);
    }

    #[test]
    fn test_parse_timestamp_date_only() {
        let ts = parse_hl7_ts("20250128").unwrap();
        assert_eq!(ts.year(), 2025);
        assert_eq!(ts.month(), 1);
        assert_eq!(ts.day(), 28);
        // Time should default to midnight
        assert_eq!(ts.hour(), 0);
        assert_eq!(ts.minute(), 0);
        assert_eq!(ts.second(), 0);
    }

    #[test]
    fn test_parse_timestamp_with_hour_only() {
        // Note: parse_hl7_ts requires at least HHMM for time part
        // For hour-only precision, use parse_hl7_ts_with_precision
        let ts = parse_hl7_ts_with_precision("2025012815").unwrap();
        assert_eq!(ts.datetime.hour(), 15);
        assert_eq!(ts.datetime.minute(), 0);
        assert_eq!(ts.datetime.second(), 0);
        assert_eq!(ts.precision, TimestampPrecision::Hour);
    }

    #[test]
    fn test_parse_timestamp_with_hour_minute() {
        let ts = parse_hl7_ts("202501281523").unwrap();
        assert_eq!(ts.hour(), 15);
        assert_eq!(ts.minute(), 23);
        assert_eq!(ts.second(), 0);
    }

    #[test]
    fn test_parse_timestamp_invalid() {
        // Too short
        assert!(parse_hl7_ts("2025").is_err());
        assert!(parse_hl7_ts("").is_err());
        // Invalid date
        assert!(parse_hl7_ts("20251328").is_err());
        // Invalid time
        assert!(parse_hl7_ts("202501282500").is_err());
    }

    #[test]
    fn test_is_valid_hl7_timestamp_valid() {
        assert!(is_valid_hl7_timestamp("20250128"));
        assert!(is_valid_hl7_timestamp("20250128152312"));
        assert!(is_valid_hl7_timestamp("202501281523"));
    }

    #[test]
    fn test_is_valid_hl7_timestamp_invalid() {
        assert!(!is_valid_hl7_timestamp("2025")); // Too short for TS
        assert!(!is_valid_hl7_timestamp("20251328")); // Invalid date
    }
}

// ============================================================================
// Precision Tests
// ============================================================================

#[cfg(test)]
mod precision_tests {
    use super::*;

    #[test]
    fn test_parse_precision_year() {
        let ts = parse_hl7_ts_with_precision("2025").unwrap();
        assert_eq!(ts.precision, TimestampPrecision::Year);
        assert_eq!(ts.datetime.year(), 2025);
        assert_eq!(ts.datetime.month(), 1);
        assert_eq!(ts.datetime.day(), 1);
    }

    #[test]
    fn test_parse_precision_month() {
        let ts = parse_hl7_ts_with_precision("202501").unwrap();
        assert_eq!(ts.precision, TimestampPrecision::Month);
        assert_eq!(ts.datetime.year(), 2025);
        assert_eq!(ts.datetime.month(), 1);
        assert_eq!(ts.datetime.day(), 1);
    }

    #[test]
    fn test_parse_precision_day() {
        let ts = parse_hl7_ts_with_precision("20250128").unwrap();
        assert_eq!(ts.precision, TimestampPrecision::Day);
        assert_eq!(ts.datetime.year(), 2025);
        assert_eq!(ts.datetime.month(), 1);
        assert_eq!(ts.datetime.day(), 28);
    }

    #[test]
    fn test_parse_precision_hour() {
        let ts = parse_hl7_ts_with_precision("2025012815").unwrap();
        assert_eq!(ts.precision, TimestampPrecision::Hour);
        assert_eq!(ts.datetime.hour(), 15);
        assert_eq!(ts.datetime.minute(), 0);
    }

    #[test]
    fn test_parse_precision_minute() {
        let ts = parse_hl7_ts_with_precision("202501281523").unwrap();
        assert_eq!(ts.precision, TimestampPrecision::Minute);
        assert_eq!(ts.datetime.hour(), 15);
        assert_eq!(ts.datetime.minute(), 23);
        assert_eq!(ts.datetime.second(), 0);
    }

    #[test]
    fn test_parse_precision_second() {
        let ts = parse_hl7_ts_with_precision("20250128152312").unwrap();
        assert_eq!(ts.precision, TimestampPrecision::Second);
        assert_eq!(ts.datetime.hour(), 15);
        assert_eq!(ts.datetime.minute(), 23);
        assert_eq!(ts.datetime.second(), 12);
    }

    #[test]
    fn test_parse_precision_fractional() {
        let ts = parse_hl7_ts_with_precision("20250128152312.123456").unwrap();
        assert_eq!(ts.precision, TimestampPrecision::FractionalSecond);
        assert_eq!(ts.fractional_seconds, Some(123456));
    }

    #[test]
    fn test_precision_ordering() {
        assert!(TimestampPrecision::Year < TimestampPrecision::Month);
        assert!(TimestampPrecision::Month < TimestampPrecision::Day);
        assert!(TimestampPrecision::Day < TimestampPrecision::Hour);
        assert!(TimestampPrecision::Hour < TimestampPrecision::Minute);
        assert!(TimestampPrecision::Minute < TimestampPrecision::Second);
        assert!(TimestampPrecision::Second < TimestampPrecision::FractionalSecond);
    }
}

// ============================================================================
// ParsedTimestamp Tests
// ============================================================================

#[cfg(test)]
mod parsed_timestamp_tests {
    use super::*;

    #[test]
    fn test_is_same_day_true() {
        let ts1 = parse_hl7_ts_with_precision("20250128").unwrap();
        let ts2 = parse_hl7_ts_with_precision("20250128120000").unwrap();
        assert!(ts1.is_same_day(&ts2));
        assert!(ts2.is_same_day(&ts1));
    }

    #[test]
    fn test_is_same_day_false() {
        let ts1 = parse_hl7_ts_with_precision("20250128").unwrap();
        let ts2 = parse_hl7_ts_with_precision("20250129120000").unwrap();
        assert!(!ts1.is_same_day(&ts2));
    }

    #[test]
    fn test_is_before() {
        let ts1 = parse_hl7_ts_with_precision("20250128100000").unwrap();
        let ts2 = parse_hl7_ts_with_precision("20250128120000").unwrap();
        assert!(ts1.is_before(&ts2));
        assert!(!ts2.is_before(&ts1));
    }

    #[test]
    fn test_is_after() {
        let ts1 = parse_hl7_ts_with_precision("20250128100000").unwrap();
        let ts2 = parse_hl7_ts_with_precision("20250128120000").unwrap();
        assert!(ts2.is_after(&ts1));
        assert!(!ts1.is_after(&ts2));
    }

    #[test]
    fn test_is_equal_same_precision() {
        let ts1 = parse_hl7_ts_with_precision("20250128152312").unwrap();
        let ts2 = parse_hl7_ts_with_precision("20250128152312").unwrap();
        assert!(ts1.is_equal(&ts2));
    }

    #[test]
    fn test_is_equal_different_precision() {
        // Day precision vs second precision - should compare at day level
        let ts1 = parse_hl7_ts_with_precision("20250128").unwrap();
        let ts2 = parse_hl7_ts_with_precision("20250128120000").unwrap();
        // When compared at minimum precision (day), both truncate to midnight
        // so they ARE equal at the day level
        assert!(ts1.is_equal(&ts2));

        // But different days are NOT equal
        let ts3 = parse_hl7_ts_with_precision("20250129").unwrap();
        assert!(!ts1.is_equal(&ts3));
    }

    #[test]
    fn test_to_hl7_string_year() {
        let ts = parse_hl7_ts_with_precision("2025").unwrap();
        assert_eq!(ts.to_hl7_string(), "2025");
    }

    #[test]
    fn test_to_hl7_string_month() {
        let ts = parse_hl7_ts_with_precision("202501").unwrap();
        assert_eq!(ts.to_hl7_string(), "202501");
    }

    #[test]
    fn test_to_hl7_string_day() {
        let ts = parse_hl7_ts_with_precision("20250128").unwrap();
        assert_eq!(ts.to_hl7_string(), "20250128");
    }

    #[test]
    fn test_to_hl7_string_hour() {
        let ts = parse_hl7_ts_with_precision("2025012815").unwrap();
        assert_eq!(ts.to_hl7_string(), "2025012815");
    }

    #[test]
    fn test_to_hl7_string_minute() {
        let ts = parse_hl7_ts_with_precision("202501281523").unwrap();
        assert_eq!(ts.to_hl7_string(), "202501281523");
    }

    #[test]
    fn test_to_hl7_string_second() {
        let ts = parse_hl7_ts_with_precision("20250128152312").unwrap();
        assert_eq!(ts.to_hl7_string(), "20250128152312");
    }

    #[test]
    fn test_to_hl7_string_fractional() {
        let ts = parse_hl7_ts_with_precision("20250128152312.123456").unwrap();
        // Note: The format includes fractional seconds
        assert!(ts.to_hl7_string().starts_with("20250128152312"));
    }

    #[test]
    fn test_new_parsed_timestamp() {
        let dt = NaiveDate::from_ymd_opt(2025, 1, 28)
            .unwrap()
            .and_hms_opt(15, 23, 12)
            .unwrap();
        let ts = ParsedTimestamp::new(dt, TimestampPrecision::Second);
        assert_eq!(ts.precision, TimestampPrecision::Second);
        assert_eq!(ts.fractional_seconds, None);
    }

    #[test]
    fn test_with_fractional() {
        let dt = NaiveDate::from_ymd_opt(2025, 1, 28)
            .unwrap()
            .and_hms_opt(15, 23, 12)
            .unwrap();
        let ts = ParsedTimestamp::with_fractional(dt, 123456);
        assert_eq!(ts.precision, TimestampPrecision::FractionalSecond);
        assert_eq!(ts.fractional_seconds, Some(123456));
    }
}

// ============================================================================
// Current Time Tests
// ============================================================================

#[cfg(test)]
mod current_time_tests {
    use super::*;

    #[test]
    fn test_now_hl7_format() {
        let now = now_hl7();
        // Should be 14 digits
        assert_eq!(now.len(), 14);
        // Should be all digits
        assert!(now.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn test_today_hl7_format() {
        let today = today_hl7();
        // Should be 8 digits (YYYYMMDD)
        assert_eq!(today.len(), 8);
        // Should be all digits
        assert!(today.chars().all(|c| c.is_ascii_digit()));
        // Should be a valid date
        assert!(is_valid_hl7_date(&today));
    }

    #[test]
    fn test_now_hl7_is_valid_timestamp() {
        let now = now_hl7();
        assert!(is_valid_hl7_timestamp(&now));
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
        let err = DateTimeError::InvalidDateFormat("test".to_string());
        assert!(err.to_string().contains("test"));

        let err = DateTimeError::InvalidTimeFormat("test".to_string());
        assert!(err.to_string().contains("test"));

        let err = DateTimeError::InvalidTimestampFormat("test".to_string());
        assert!(err.to_string().contains("test"));

        let err = DateTimeError::DateOutOfRange("test".to_string());
        assert!(err.to_string().contains("test"));

        let err = DateTimeError::TimeOutOfRange("test".to_string());
        assert!(err.to_string().contains("test"));
    }

    #[test]
    fn test_error_clone() {
        let err = DateTimeError::InvalidDateFormat("test".to_string());
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_century_leap_year() {
        // 2000 is a leap year (divisible by 400)
        let date = parse_hl7_dt("20000229").unwrap();
        assert_eq!(date.day(), 29);
    }

    #[test]
    fn test_century_non_leap_year() {
        // 1900 was not a leap year (divisible by 100 but not 400)
        assert!(parse_hl7_dt("19000229").is_err());
    }

    #[test]
    fn test_year_boundaries() {
        // Year 0001
        let date = parse_hl7_dt("00010101").unwrap();
        assert_eq!(date.year(), 1);

        // Far future
        let date = parse_hl7_dt("99991231").unwrap();
        assert_eq!(date.year(), 9999);
    }

    #[test]
    fn test_timestamp_with_leading_zeros() {
        let ts = parse_hl7_ts("20250101000000").unwrap();
        assert_eq!(ts.month(), 1);
        assert_eq!(ts.day(), 1);
        assert_eq!(ts.hour(), 0);
        assert_eq!(ts.minute(), 0);
        assert_eq!(ts.second(), 0);
    }
}
