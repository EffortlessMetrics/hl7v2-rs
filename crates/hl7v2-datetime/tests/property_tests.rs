//! Property-based tests for hl7v2-datetime crate
//!
//! Uses proptest to verify properties hold across a wide range of inputs

use proptest::prelude::*;
use hl7v2_datetime::*;
use chrono::{Datelike, Timelike};

// ============================================================================
// Strategies for Generating Valid HL7 Date/Time Values
// ============================================================================

/// Generate a valid year (1-9999)
fn year_strategy() -> impl Strategy<Value = i32> {
    1i32..=9999
}

/// Generate a valid month (1-12)
fn month_strategy() -> impl Strategy<Value = u32> {
    1u32..=12
}

/// Generate a valid day (1-31, further validation done in test)
fn day_strategy() -> impl Strategy<Value = u32> {
    1u32..=31
}

/// Generate a valid hour (0-23)
fn hour_strategy() -> impl Strategy<Value = u32> {
    0u32..=23
}

/// Generate a valid minute (0-59)
fn minute_strategy() -> impl Strategy<Value = u32> {
    0u32..=59
}

/// Generate a valid second (0-59)
fn second_strategy() -> impl Strategy<Value = u32> {
    0u32..=59
}

/// Generate a valid HL7 date string (YYYYMMDD)
fn hl7_date_strategy() -> impl Strategy<Value = String> {
    (year_strategy(), month_strategy(), day_strategy())
        .prop_map(|(y, m, d)| format!("{:04}{:02}{:02}", y, m, d))
}

/// Generate a valid HL7 time string (HHMMSS)
fn hl7_time_strategy() -> impl Strategy<Value = String> {
    (hour_strategy(), minute_strategy(), second_strategy())
        .prop_map(|(h, m, s)| format!("{:02}{:02}{:02}", h, m, s))
}

/// Generate a valid HL7 timestamp string (YYYYMMDDHHMMSS)
fn hl7_timestamp_strategy() -> impl Strategy<Value = String> {
    (hl7_date_strategy(), hl7_time_strategy())
        .prop_map(|(d, t)| format!("{}{}", d, t))
}

// ============================================================================
// Date Property Tests
// ============================================================================

proptest! {
    #[test]
    fn test_valid_date_roundtrip(
        year in 1i32..=9999,
        month in 1u32..=12,
        day in 1u32..=28  // Use 28 to avoid month-specific day issues
    ) {
        let date_str = format!("{:04}{:02}{:02}", year, month, day);
        
        if let Ok(date) = parse_hl7_dt(&date_str) {
            // Parsed date should match input
            prop_assert_eq!(date.year(), year);
            prop_assert_eq!(date.month(), month);
            prop_assert_eq!(date.day(), day);
            
            // is_valid_hl7_date should return true
            prop_assert!(is_valid_hl7_date(&date_str));
        }
    }
    
    #[test]
    fn test_date_validation_consistent(date_str in "[0-9]{8}") {
        // parse_hl7_dt and is_valid_hl7_date should agree
        let parsed = parse_hl7_dt(&date_str);
        let is_valid = is_valid_hl7_date(&date_str);
        
        prop_assert_eq!(parsed.is_ok(), is_valid);
    }
    
    #[test]
    fn test_date_only_digits(date_str in "\\PC*") {
        // Non-digit characters should always fail
        if !date_str.chars().all(|c| c.is_ascii_digit()) {
            prop_assert!(parse_hl7_dt(&date_str).is_err());
            prop_assert!(!is_valid_hl7_date(&date_str));
        }
    }
}

// ============================================================================
// Time Property Tests
// ============================================================================

proptest! {
    #[test]
    fn test_valid_time_roundtrip(
        hour in 0u32..=23,
        minute in 0u32..=59,
        second in 0u32..=59
    ) {
        let time_str = format!("{:02}{:02}{:02}", hour, minute, second);
        
        if let Ok((h, m, s, _)) = parse_hl7_tm(&time_str) {
            prop_assert_eq!(h, hour);
            prop_assert_eq!(m, minute);
            prop_assert_eq!(s, second);
            prop_assert!(is_valid_hl7_time(&time_str));
        }
    }
    
    #[test]
    fn test_time_hour_minute_only(
        hour in 0u32..=23,
        minute in 0u32..=59
    ) {
        let time_str = format!("{:02}{:02}", hour, minute);
        
        if let Ok((h, m, s, _)) = parse_hl7_tm(&time_str) {
            prop_assert_eq!(h, hour);
            prop_assert_eq!(m, minute);
            prop_assert_eq!(s, 0); // Seconds default to 0
            prop_assert!(is_valid_hl7_time(&time_str));
        }
    }
    
    #[test]
    fn test_time_validation_consistent(time_str in "[0-9]{4,6}") {
        let parsed = parse_hl7_tm(&time_str);
        let is_valid = is_valid_hl7_time(&time_str);
        
        prop_assert_eq!(parsed.is_ok(), is_valid);
    }
    
    #[test]
    fn test_invalid_hour_rejected(hour in 24u32..=99) {
        let time_str = format!("{:02}0000", hour);
        prop_assert!(parse_hl7_tm(&time_str).is_err());
        prop_assert!(!is_valid_hl7_time(&time_str));
    }
    
    #[test]
    fn test_invalid_minute_rejected(minute in 60u32..=99) {
        let time_str = format!("00{:02}00", minute);
        prop_assert!(parse_hl7_tm(&time_str).is_err());
        prop_assert!(!is_valid_hl7_time(&time_str));
    }
    
    #[test]
    fn test_invalid_second_rejected(second in 60u32..=99) {
        let time_str = format!("0000{:02}", second);
        prop_assert!(parse_hl7_tm(&time_str).is_err());
        prop_assert!(!is_valid_hl7_time(&time_str));
    }
}

// ============================================================================
// Timestamp Property Tests
// ============================================================================

proptest! {
    #[test]
    fn test_valid_timestamp_roundtrip(
        year in 1i32..=9999,
        month in 1u32..=12,
        day in 1u32..=28,
        hour in 0u32..=23,
        minute in 0u32..=59,
        second in 0u32..=59
    ) {
        let ts_str = format!("{:04}{:02}{:02}{:02}{:02}{:02}", year, month, day, hour, minute, second);
        
        if let Ok(ts) = parse_hl7_ts(&ts_str) {
            prop_assert_eq!(ts.year(), year);
            prop_assert_eq!(ts.month(), month);
            prop_assert_eq!(ts.day(), day);
            prop_assert_eq!(ts.hour(), hour);
            prop_assert_eq!(ts.minute(), minute);
            prop_assert_eq!(ts.second(), second);
            prop_assert!(is_valid_hl7_timestamp(&ts_str));
        }
    }
    
    #[test]
    fn test_timestamp_validation_consistent(ts_str in "[0-9]{8,14}") {
        let parsed = parse_hl7_ts(&ts_str);
        let is_valid = is_valid_hl7_timestamp(&ts_str);
        
        prop_assert_eq!(parsed.is_ok(), is_valid);
    }
    
    #[test]
    fn test_timestamp_date_only(
        year in 1i32..=9999,
        month in 1u32..=12,
        day in 1u32..=28
    ) {
        let ts_str = format!("{:04}{:02}{:02}", year, month, day);
        
        if let Ok(ts) = parse_hl7_ts(&ts_str) {
            prop_assert_eq!(ts.year(), year);
            prop_assert_eq!(ts.month(), month);
            prop_assert_eq!(ts.day(), day);
            // Time should default to midnight
            prop_assert_eq!(ts.hour(), 0);
            prop_assert_eq!(ts.minute(), 0);
            prop_assert_eq!(ts.second(), 0);
        }
    }
}

// ============================================================================
// Precision Property Tests
// ============================================================================

proptest! {
    #[test]
    fn test_precision_year(year in 1i32..=9999) {
        let ts_str = format!("{:04}", year);
        
        if let Ok(ts) = parse_hl7_ts_with_precision(&ts_str) {
            prop_assert_eq!(ts.precision, TimestampPrecision::Year);
            prop_assert_eq!(ts.datetime.year(), year);
            // Month and day should be 1
            prop_assert_eq!(ts.datetime.month(), 1);
            prop_assert_eq!(ts.datetime.day(), 1);
        }
    }
    
    #[test]
    fn test_precision_month(year in 1i32..=9999, month in 1u32..=12) {
        let ts_str = format!("{:04}{:02}", year, month);
        
        if let Ok(ts) = parse_hl7_ts_with_precision(&ts_str) {
            prop_assert_eq!(ts.precision, TimestampPrecision::Month);
            prop_assert_eq!(ts.datetime.year(), year);
            prop_assert_eq!(ts.datetime.month(), month);
            prop_assert_eq!(ts.datetime.day(), 1);
        }
    }
    
    #[test]
    fn test_precision_day(
        year in 1i32..=9999,
        month in 1u32..=12,
        day in 1u32..=28
    ) {
        let ts_str = format!("{:04}{:02}{:02}", year, month, day);
        
        if let Ok(ts) = parse_hl7_ts_with_precision(&ts_str) {
            prop_assert_eq!(ts.precision, TimestampPrecision::Day);
            prop_assert_eq!(ts.datetime.year(), year);
            prop_assert_eq!(ts.datetime.month(), month);
            prop_assert_eq!(ts.datetime.day(), day);
        }
    }
    
    #[test]
    fn test_precision_hour(
        year in 1i32..=9999,
        month in 1u32..=12,
        day in 1u32..=28,
        hour in 0u32..=23
    ) {
        let ts_str = format!("{:04}{:02}{:02}{:02}", year, month, day, hour);
        
        if let Ok(ts) = parse_hl7_ts_with_precision(&ts_str) {
            prop_assert_eq!(ts.precision, TimestampPrecision::Hour);
            prop_assert_eq!(ts.datetime.hour(), hour);
            prop_assert_eq!(ts.datetime.minute(), 0);
            prop_assert_eq!(ts.datetime.second(), 0);
        }
    }
    
    #[test]
    fn test_precision_minute(
        year in 1i32..=9999,
        month in 1u32..=12,
        day in 1u32..=28,
        hour in 0u32..=23,
        minute in 0u32..=59
    ) {
        let ts_str = format!("{:04}{:02}{:02}{:02}{:02}", year, month, day, hour, minute);
        
        if let Ok(ts) = parse_hl7_ts_with_precision(&ts_str) {
            prop_assert_eq!(ts.precision, TimestampPrecision::Minute);
            prop_assert_eq!(ts.datetime.hour(), hour);
            prop_assert_eq!(ts.datetime.minute(), minute);
            prop_assert_eq!(ts.datetime.second(), 0);
        }
    }
    
    #[test]
    fn test_precision_second(
        year in 1i32..=9999,
        month in 1u32..=12,
        day in 1u32..=28,
        hour in 0u32..=23,
        minute in 0u32..=59,
        second in 0u32..=59
    ) {
        let ts_str = format!("{:04}{:02}{:02}{:02}{:02}{:02}", year, month, day, hour, minute, second);
        
        if let Ok(ts) = parse_hl7_ts_with_precision(&ts_str) {
            prop_assert_eq!(ts.precision, TimestampPrecision::Second);
            prop_assert_eq!(ts.datetime.hour(), hour);
            prop_assert_eq!(ts.datetime.minute(), minute);
            prop_assert_eq!(ts.datetime.second(), second);
        }
    }
}

// ============================================================================
// Comparison Property Tests
// ============================================================================

proptest! {
    #[test]
    fn test_comparison_reflexivity(
        year in 1i32..=9999,
        month in 1u32..=12,
        day in 1u32..=28
    ) {
        let ts_str = format!("{:04}{:02}{:02}120000", year, month, day);
        
        if let Ok(ts) = parse_hl7_ts_with_precision(&ts_str) {
            // A timestamp is equal to itself
            prop_assert!(ts.is_equal(&ts));
            // A timestamp is not before itself
            prop_assert!(!ts.is_before(&ts));
            // A timestamp is not after itself
            prop_assert!(!ts.is_after(&ts));
        }
    }
    
    #[test]
    fn test_comparison_transitivity(
        year in 2020i32..=2025,
        month1 in 1u32..=12,
        day1 in 1u32..=28,
        day2 in 1u32..=28,
        day3 in 1u32..=28
    ) {
        let ts1_str = format!("{:04}{:02}{:02}100000", year, month1, day1);
        let ts2_str = format!("{:04}{:02}{:02}120000", year, month1, day2);
        let ts3_str = format!("{:04}{:02}{:02}140000", year, month1, day3);
        
        if let (Ok(ts1), Ok(ts2), Ok(ts3)) = (
            parse_hl7_ts_with_precision(&ts1_str),
            parse_hl7_ts_with_precision(&ts2_str),
            parse_hl7_ts_with_precision(&ts3_str),
        ) {
            // If ts1 < ts2 and ts2 < ts3, then ts1 < ts3
            if ts1.is_before(&ts2) && ts2.is_before(&ts3) {
                prop_assert!(ts1.is_before(&ts3));
            }
        }
    }
    
    #[test]
    fn test_comparison_symmetry(
        year in 2020i32..=2025,
        month in 1u32..=12,
        day1 in 1u32..=28,
        day2 in 1u32..=28
    ) {
        let ts1_str = format!("{:04}{:02}{:02}120000", year, month, day1);
        let ts2_str = format!("{:04}{:02}{:02}120000", year, month, day2);
        
        if let (Ok(ts1), Ok(ts2)) = (
            parse_hl7_ts_with_precision(&ts1_str),
            parse_hl7_ts_with_precision(&ts2_str),
        ) {
            // If ts1 is before ts2, then ts2 is after ts1
            if ts1.is_before(&ts2) {
                prop_assert!(ts2.is_after(&ts1));
            }
            if ts2.is_before(&ts1) {
                prop_assert!(ts1.is_after(&ts2));
            }
        }
    }
    
    #[test]
    fn test_same_day_symmetry(
        year in 2020i32..=2025,
        month in 1u32..=12,
        day in 1u32..=28
    ) {
        let ts1_str = format!("{:04}{:02}{:02}100000", year, month, day);
        let ts2_str = format!("{:04}{:02}{:02}150000", year, month, day);
        
        if let (Ok(ts1), Ok(ts2)) = (
            parse_hl7_ts_with_precision(&ts1_str),
            parse_hl7_ts_with_precision(&ts2_str),
        ) {
            // is_same_day should be symmetric
            prop_assert_eq!(ts1.is_same_day(&ts2), ts2.is_same_day(&ts1));
        }
    }
}

// ============================================================================
// String Format Property Tests
// ============================================================================

proptest! {
    #[test]
    fn test_to_hl7_string_format(ts_str in "[0-9]{14}") {
        if let Ok(ts) = parse_hl7_ts_with_precision(&ts_str) {
            let formatted = ts.to_hl7_string();
            // Formatted string should be all digits
            prop_assert!(formatted.chars().all(|c| c.is_ascii_digit()));
            // Should be able to parse the formatted string
            prop_assert!(parse_hl7_ts_with_precision(&formatted).is_ok());
        }
    }
}

#[test]
fn test_now_hl7_format_valid() {
    let now = now_hl7();
    assert!(now.len() == 14);
    assert!(now.chars().all(|c| c.is_ascii_digit()));
    assert!(is_valid_hl7_timestamp(&now));
}

#[test]
fn test_today_hl7_format_valid() {
    let today = today_hl7();
    assert!(today.len() == 8);
    assert!(today.chars().all(|c| c.is_ascii_digit()));
    assert!(is_valid_hl7_date(&today));
}

// ============================================================================
// Whitespace Handling Property Tests
// ============================================================================

proptest! {
    #[test]
    fn test_whitespace_handling(
        leading_ws in " *",
        trailing_ws in " *",
        year in 2020i32..=2025,
        month in 1u32..=12,
        day in 1u32..=28
    ) {
        let date_str = format!("{}{:04}{:02}{:02}{}", leading_ws, year, month, day, trailing_ws);
        
        // Whitespace should be trimmed
        if let Ok(date) = parse_hl7_dt(&date_str) {
            prop_assert_eq!(date.year(), year);
            prop_assert_eq!(date.month(), month);
            prop_assert_eq!(date.day(), day);
        }
    }
}
