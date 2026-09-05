//! HL7 v2 date/time parsing and validation.
//!
//! This crate provides comprehensive date/time handling for HL7 v2 messages,
//! supporting various HL7 timestamp formats and precision levels.
//!
//! # Supported Formats
//!
//! - `DT` (Date): YYYYMMDD
//! - `TM` (Time): HHMM\[SS\[.S\[S\[S\[S\]\]\]\]\]
//! - `TS` (Timestamp): YYYYMMDD\[HHMM\[SS\[.S\[S\[S\[S\]\]\]\]\]\]
//!
//! # Example
//!
//! ```
//! use hl7v2::conformance::datatype::datetime::{
//!     parse_hl7_dt, parse_hl7_tm, parse_hl7_ts, parse_hl7_ts_with_precision, TimestampPrecision,
//! };
//! use chrono::Datelike;
//!
//! // Parse date (DT)
//! let date = parse_hl7_dt("20250128").unwrap();
//! assert_eq!(date.year(), 2025);
//! assert_eq!(date.month(), 1);
//! assert_eq!(date.day(), 28);
//!
//! // Parse timestamp (TS) with precision
//! let ts = parse_hl7_ts_with_precision("20250128152312").unwrap();
//! assert_eq!(ts.precision, TimestampPrecision::Second);
//!
//! // Compare timestamps with different precisions
//! let ts1 = parse_hl7_ts_with_precision("20250128").unwrap();
//! let ts2 = parse_hl7_ts_with_precision("20250128120000").unwrap();
//! assert!(ts1.is_same_day(&ts2));
//! ```

use chrono::{Datelike, NaiveDate, NaiveDateTime, Timelike};

/// Error type for date/time parsing
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum DateTimeError {
    /// Date text is not a valid `YYYYMMDD` value.
    #[error("Invalid date format: {0}")]
    InvalidDateFormat(String),

    /// Time text is not a valid HL7 `TM` value.
    #[error("Invalid time format: {0}")]
    InvalidTimeFormat(String),

    /// Timestamp text is not a valid HL7 `TS` value.
    #[error("Invalid timestamp format: {0}")]
    InvalidTimestampFormat(String),

    /// Parsed date/time is outside the supported range.
    #[error("Date out of range: {0}")]
    DateOutOfRange(String),

    /// Parsed time component is outside the supported range.
    #[error("Time out of range: {0}")]
    TimeOutOfRange(String),
}

/// Precision levels for HL7 timestamps
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TimestampPrecision {
    /// Year only (YYYY)
    Year,
    /// Year and month (YYYYMM)
    Month,
    /// Full date (YYYYMMDD)
    Day,
    /// Date with hour (YYYYMMDDHH)
    Hour,
    /// Date with hour and minute (YYYYMMDDHHMM)
    Minute,
    /// Full precision to second (YYYYMMDDHHMMSS)
    Second,
    /// With fractional seconds
    FractionalSecond,
}

/// Parsed HL7 timestamp with precision information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedTimestamp {
    /// The parsed datetime
    pub datetime: NaiveDateTime,
    /// The precision of the timestamp
    pub precision: TimestampPrecision,
    /// Fractional seconds, right-padded to six digits for storage when present.
    pub fractional_seconds: Option<u32>,
}

impl ParsedTimestamp {
    /// Create a new parsed timestamp
    pub fn new(datetime: NaiveDateTime, precision: TimestampPrecision) -> Self {
        Self {
            datetime,
            precision,
            fractional_seconds: None,
        }
    }

    /// Create with fractional seconds
    pub fn with_fractional(datetime: NaiveDateTime, fractional: u32) -> Self {
        Self {
            datetime,
            precision: TimestampPrecision::FractionalSecond,
            fractional_seconds: Some(fractional),
        }
    }

    /// Check if two timestamps are on the same day
    pub fn is_same_day(&self, other: &ParsedTimestamp) -> bool {
        self.datetime.date() == other.datetime.date()
    }

    /// Check if this timestamp is before another (strictly less than)
    pub fn is_before(&self, other: &ParsedTimestamp) -> bool {
        // For timestamps with different precisions, compare at the finer precision
        if self.precision != other.precision {
            // Compare full datetime values - a date-only timestamp at midnight
            // is considered equal to a datetime at midnight on that same day
            return self.datetime < other.datetime;
        }
        self.datetime < other.datetime
    }

    /// Check if this timestamp is after another
    pub fn is_after(&self, other: &ParsedTimestamp) -> bool {
        other.is_before(self)
    }

    /// Check if this timestamp is equal to another (considering precision)
    pub fn is_equal(&self, other: &ParsedTimestamp) -> bool {
        let min_precision = std::cmp::min(self.precision, other.precision);
        let truncated_self = truncate_to_precision(&self.datetime, min_precision);
        let truncated_other = truncate_to_precision(&other.datetime, min_precision);
        truncated_self == truncated_other
    }

    /// Format as HL7 TS string
    pub fn to_hl7_string(&self) -> String {
        match self.precision {
            TimestampPrecision::Year => self.datetime.format("%Y").to_string(),
            TimestampPrecision::Month => self.datetime.format("%Y%m").to_string(),
            TimestampPrecision::Day => self.datetime.format("%Y%m%d").to_string(),
            TimestampPrecision::Hour => self.datetime.format("%Y%m%d%H").to_string(),
            TimestampPrecision::Minute => self.datetime.format("%Y%m%d%H%M").to_string(),
            TimestampPrecision::Second => self.datetime.format("%Y%m%d%H%M%S").to_string(),
            TimestampPrecision::FractionalSecond => {
                if let Some(frac) = self.fractional_seconds {
                    let frac = format!("{frac:06}");
                    let frac = frac.trim_end_matches('0');
                    let frac = if frac.is_empty() { "0" } else { frac };
                    format!("{}.{}", self.datetime.format("%Y%m%d%H%M%S"), frac)
                } else {
                    self.datetime.format("%Y%m%d%H%M%S").to_string()
                }
            }
        }
    }
}

/// Parse HL7 date (DT format: YYYYMMDD)
///
/// # Errors
///
/// Returns [`DateTimeError::InvalidDateFormat`] when the text is not exactly
/// eight ASCII digits or does not represent a valid calendar date.
pub fn parse_hl7_dt(s: &str) -> Result<NaiveDate, DateTimeError> {
    let s = s.trim();

    if s.len() != 8 {
        return Err(DateTimeError::InvalidDateFormat(format!(
            "Expected 8 characters, got {}",
            s.len()
        )));
    }

    if !s.chars().all(|c| c.is_ascii_digit()) {
        return Err(DateTimeError::InvalidDateFormat(
            "Contains non-digit characters".to_string(),
        ));
    }

    NaiveDate::parse_from_str(s, "%Y%m%d")
        .map_err(|e| DateTimeError::InvalidDateFormat(e.to_string()))
}

/// Parse HL7 time (TM format: HHMM[SS[.S...]], 1 to 4 fractional digits).
///
/// # Errors
///
/// Returns [`DateTimeError::InvalidTimeFormat`] when the text is too short or
/// contains non-ASCII bytes. Returns [`DateTimeError::TimeOutOfRange`] when a
/// parsed hour, minute, or second is outside the HL7 `TM` range.
pub fn parse_hl7_tm(s: &str) -> Result<(u32, u32, u32, Option<u32>), DateTimeError> {
    let s = s.trim();

    if s.len() < 4 {
        return Err(DateTimeError::InvalidTimeFormat(format!(
            "Expected at least 4 characters, got {}",
            s.len()
        )));
    }

    if !s.is_ascii() {
        return Err(DateTimeError::InvalidTimeFormat(
            "Non-ASCII characters".into(),
        ));
    }

    let hour = parse_u32_part(s, 0..2, || {
        DateTimeError::TimeOutOfRange("Invalid hour".to_string())
    })?;
    let minute = parse_u32_part(s, 2..4, || {
        DateTimeError::TimeOutOfRange("Invalid minute".to_string())
    })?;

    // Validate hour and minute
    if hour > 23 {
        return Err(DateTimeError::TimeOutOfRange(format!(
            "Hour {hour} out of range"
        )));
    }
    if minute > 59 {
        return Err(DateTimeError::TimeOutOfRange(format!(
            "Minute {minute} out of range"
        )));
    }

    // Parse seconds (optional)
    let (second, fractional) = if s.len() > 4 {
        // Check for fractional seconds
        let time_tail = s
            .get(4..)
            .ok_or_else(|| DateTimeError::InvalidTimeFormat("Missing time tail".to_string()))?;
        let (sec_part, frac_part) = if let Some(dot_pos) = time_tail.find('.') {
            let frac_start = dot_pos
                .checked_add(1)
                .ok_or_else(|| DateTimeError::InvalidTimeFormat("Invalid fraction".to_string()))?;
            let sec = time_tail
                .get(..dot_pos)
                .ok_or_else(|| DateTimeError::InvalidTimeFormat("Invalid seconds".to_string()))?;
            let frac = time_tail.get(frac_start..).ok_or_else(|| {
                DateTimeError::InvalidTimeFormat("Invalid fractional seconds".to_string())
            })?;
            (sec, Some(frac))
        } else {
            (time_tail, None)
        };

        // HHMM[SS[.S[S[S[S]]]]] permits seconds only as a complete
        // two-digit field. Reject a partial tail before numeric parsing so TM,
        // TS, and precision-aware TS validation agree.
        if sec_part.len() != 2 {
            return Err(DateTimeError::InvalidTimeFormat(format!(
                "Seconds must be two digits, got '{sec_part}'"
            )));
        }

        let sec: u32 = sec_part
            .parse()
            .map_err(|_err| DateTimeError::TimeOutOfRange("Invalid second".to_string()))?;
        if sec > 59 {
            return Err(DateTimeError::TimeOutOfRange(format!(
                "Second {sec} out of range"
            )));
        }

        let frac = if let Some(f) = frac_part {
            Some(parse_fractional_seconds(
                f,
                DateTimeError::InvalidTimeFormat,
            )?)
        } else {
            None
        };

        (sec, frac)
    } else {
        (0, None)
    };

    Ok((hour, minute, second, fractional))
}

/// Parse HL7 timestamp (TS format: YYYYMMDD[HHMM[SS[.S...]]])
///
/// # Errors
///
/// Returns [`DateTimeError::InvalidTimestampFormat`] when the text is too short
/// or contains non-ASCII bytes. Returns date/time errors from the contained
/// `DT` and `TM` components when either component is invalid.
pub fn parse_hl7_ts(s: &str) -> Result<NaiveDateTime, DateTimeError> {
    let s = s.trim();

    if s.len() < 8 {
        return Err(DateTimeError::InvalidTimestampFormat(format!(
            "Expected at least 8 characters, got {}",
            s.len()
        )));
    }

    if !s.is_ascii() {
        return Err(DateTimeError::InvalidTimestampFormat(
            "Non-ASCII characters".into(),
        ));
    }

    // Parse date part
    let date_part = s
        .get(0..8)
        .ok_or_else(|| DateTimeError::InvalidTimestampFormat("Missing date".to_string()))?;
    let date = parse_hl7_dt(date_part)?;

    // If only date, return with midnight time
    if s.len() == 8 {
        return midnight(date);
    }

    // Parse time part
    let time_str = s
        .get(8..)
        .ok_or_else(|| DateTimeError::InvalidTimestampFormat("Missing time".to_string()))?;
    let (hour, minute, second, _) = parse_hl7_tm(time_str)?;

    date_time(date, hour, minute, second)
}

/// Parse HL7 timestamp with precision information
///
/// # Errors
///
/// Returns [`DateTimeError::InvalidTimestampFormat`] when the timestamp length
/// does not map to an HL7 precision or when the text contains non-ASCII bytes.
/// Returns date/time errors from the parsed precision components when they are
/// out of range.
pub fn parse_hl7_ts_with_precision(s: &str) -> Result<ParsedTimestamp, DateTimeError> {
    let s = s.trim();

    if !s.is_ascii() {
        return Err(DateTimeError::InvalidTimestampFormat(
            "Non-ASCII characters".into(),
        ));
    }

    // Determine precision from length
    let precision = match s.len() {
        4 => TimestampPrecision::Year,
        6 => TimestampPrecision::Month,
        8 => TimestampPrecision::Day,
        10 => TimestampPrecision::Hour,
        12 => TimestampPrecision::Minute,
        14 => TimestampPrecision::Second,
        n if n > 14
            && s.get(14..)
                .is_some_and(|fractional| fractional.starts_with('.')) =>
        {
            TimestampPrecision::FractionalSecond
        }
        _ => {
            return Err(DateTimeError::InvalidTimestampFormat(format!(
                "Invalid length: {}",
                s.len()
            )));
        }
    };

    // Parse based on precision
    match precision {
        TimestampPrecision::Year => {
            let year: i32 = s
                .parse()
                .map_err(|_err| DateTimeError::InvalidDateFormat("Invalid year".into()))?;
            let date = NaiveDate::from_ymd_opt(year, 1, 1)
                .ok_or_else(|| DateTimeError::DateOutOfRange("Invalid year".into()))?;
            Ok(ParsedTimestamp::new(midnight(date)?, precision))
        }
        TimestampPrecision::Month => {
            let year = parse_i32_part(s, 0..4, || {
                DateTimeError::InvalidDateFormat("Invalid year".into())
            })?;
            let month = parse_u32_part(s, 4..6, || {
                DateTimeError::InvalidDateFormat("Invalid month".into())
            })?;
            let date = NaiveDate::from_ymd_opt(year, month, 1)
                .ok_or_else(|| DateTimeError::DateOutOfRange("Invalid month".into()))?;
            Ok(ParsedTimestamp::new(midnight(date)?, precision))
        }
        TimestampPrecision::Day => {
            let date = parse_hl7_dt(s)?;
            Ok(ParsedTimestamp::new(midnight(date)?, precision))
        }
        TimestampPrecision::Hour => {
            let date = parse_hl7_dt(part(s, 0..8, "Missing date")?)?;
            let hour = parse_u32_part(s, 8..10, || {
                DateTimeError::TimeOutOfRange("Invalid hour".into())
            })?;
            Ok(ParsedTimestamp::new(
                date_time(date, hour, 0, 0)?,
                precision,
            ))
        }
        TimestampPrecision::Minute => {
            let date = parse_hl7_dt(part(s, 0..8, "Missing date")?)?;
            let hour = parse_u32_part(s, 8..10, || {
                DateTimeError::TimeOutOfRange("Invalid hour".into())
            })?;
            let minute = parse_u32_part(s, 10..12, || {
                DateTimeError::TimeOutOfRange("Invalid minute".into())
            })?;
            Ok(ParsedTimestamp::new(
                date_time(date, hour, minute, 0)?,
                precision,
            ))
        }
        TimestampPrecision::Second => {
            let dt = parse_hl7_ts(s)?;
            Ok(ParsedTimestamp::new(dt, precision))
        }
        TimestampPrecision::FractionalSecond => {
            // Parse base timestamp
            let dt = parse_hl7_ts(part(s, 0..14, "Missing timestamp")?)?;
            // Parse fractional part
            let frac_str = part(s, 15..s.len(), "Missing fractional seconds")?; // Skip the dot
            let fractional =
                parse_fractional_seconds(frac_str, DateTimeError::InvalidTimestampFormat)?;
            Ok(ParsedTimestamp::with_fractional(dt, fractional))
        }
    }
}

fn parse_fractional_seconds<F>(fractional: &str, error: F) -> Result<u32, DateTimeError>
where
    F: Fn(String) -> DateTimeError,
{
    if fractional.is_empty() {
        return Err(error("Missing fractional seconds".to_string()));
    }

    if fractional.len() > 4 {
        return Err(error(format!(
            "Fractional seconds must contain 1 to 4 digits, got {}",
            fractional.len()
        )));
    }

    if !fractional.chars().all(|c| c.is_ascii_digit()) {
        return Err(error(
            "Fractional seconds contain non-digit characters".to_string(),
        ));
    }

    let mut value = 0_u32;
    for digit in fractional.bytes() {
        let digit = digit
            .checked_sub(b'0')
            .ok_or_else(|| error("Invalid fractional seconds".to_string()))?;
        value = value
            .checked_mul(10)
            .and_then(|acc| acc.checked_add(u32::from(digit)))
            .ok_or_else(|| error("Invalid fractional seconds".to_string()))?;
    }

    let scale = match fractional.len() {
        1 => 100_000,
        2 => 10_000,
        3 => 1_000,
        4 => 100,
        _ => return Err(error("Invalid fractional seconds".to_string())),
    };

    value
        .checked_mul(scale)
        .ok_or_else(|| error("Invalid fractional seconds".to_string()))
}

fn part<'a>(
    value: &'a str,
    range: std::ops::Range<usize>,
    message: &str,
) -> Result<&'a str, DateTimeError> {
    value
        .get(range)
        .ok_or_else(|| DateTimeError::InvalidTimestampFormat(message.to_string()))
}

fn parse_u32_part<F>(
    value: &str,
    range: std::ops::Range<usize>,
    error: F,
) -> Result<u32, DateTimeError>
where
    F: Fn() -> DateTimeError,
{
    value
        .get(range)
        .ok_or_else(&error)?
        .parse()
        .map_err(|_err| error())
}

fn parse_i32_part<F>(
    value: &str,
    range: std::ops::Range<usize>,
    error: F,
) -> Result<i32, DateTimeError>
where
    F: Fn() -> DateTimeError,
{
    value
        .get(range)
        .ok_or_else(&error)?
        .parse()
        .map_err(|_err| error())
}

fn midnight(date: NaiveDate) -> Result<NaiveDateTime, DateTimeError> {
    date.and_hms_opt(0, 0, 0)
        .ok_or_else(|| DateTimeError::TimeOutOfRange("Invalid midnight".to_string()))
}

fn date_time(
    date: NaiveDate,
    hour: u32,
    minute: u32,
    second: u32,
) -> Result<NaiveDateTime, DateTimeError> {
    date.and_hms_opt(hour, minute, second)
        .ok_or_else(|| DateTimeError::TimeOutOfRange("Invalid time combination".to_string()))
}

/// Truncate a datetime to a specific precision
fn truncate_to_precision(dt: &NaiveDateTime, precision: TimestampPrecision) -> NaiveDateTime {
    match precision {
        TimestampPrecision::Year => NaiveDate::from_ymd_opt(dt.year(), 1, 1)
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .unwrap_or(*dt),
        TimestampPrecision::Month => NaiveDate::from_ymd_opt(dt.year(), dt.month(), 1)
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .unwrap_or(*dt),
        TimestampPrecision::Day => dt.date().and_hms_opt(0, 0, 0).unwrap_or(*dt),
        TimestampPrecision::Hour => dt
            .with_minute(0)
            .and_then(|d| d.with_second(0))
            .unwrap_or(*dt),
        TimestampPrecision::Minute => dt.with_second(0).unwrap_or(*dt),
        TimestampPrecision::Second | TimestampPrecision::FractionalSecond => *dt,
    }
}

/// Check if a string is a valid HL7 date (DT)
pub fn is_valid_hl7_date(s: &str) -> bool {
    parse_hl7_dt(s).is_ok()
}

/// Check if a string is a valid HL7 time (TM)
pub fn is_valid_hl7_time(s: &str) -> bool {
    parse_hl7_tm(s).is_ok()
}

/// Check if a string is a valid HL7 timestamp (TS)
pub fn is_valid_hl7_timestamp(s: &str) -> bool {
    parse_hl7_ts(s).is_ok()
}

/// Get current timestamp in HL7 format
pub fn now_hl7() -> String {
    chrono::Utc::now().format("%Y%m%d%H%M%S").to_string()
}

/// Get current date in HL7 format
pub fn today_hl7() -> String {
    chrono::Utc::now().format("%Y%m%d").to_string()
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::expect_used,
        reason = "inline unit tests use expect with descriptive messages for clarity"
    )]

    use super::{
        DateTimeError, ParsedTimestamp, TimestampPrecision, is_valid_hl7_date, is_valid_hl7_time,
        is_valid_hl7_timestamp, now_hl7, parse_hl7_dt, parse_hl7_tm, parse_hl7_ts,
        parse_hl7_ts_with_precision, today_hl7, truncate_to_precision,
    };
    use chrono::{Datelike, NaiveDate, Timelike};

    // ------------------------------------------------------------------
    // parse_hl7_dt
    // ------------------------------------------------------------------
    #[test]
    fn parse_hl7_dt_accepts_valid_date() {
        let d = parse_hl7_dt("20250128").expect("valid date");
        assert_eq!(d.year(), 2025);
        assert_eq!(d.month(), 1);
        assert_eq!(d.day(), 28);
    }

    #[test]
    fn parse_hl7_dt_trims_whitespace() {
        let d = parse_hl7_dt("  20250128  ").expect("valid trimmed date");
        assert_eq!(d.year(), 2025);
    }

    #[test]
    fn parse_hl7_dt_leap_year_feb_29() {
        // 2024 is a leap year — Feb 29 is valid
        let d = parse_hl7_dt("20240229").expect("leap day");
        assert_eq!(d.day(), 29);

        // 2023 is not a leap year — Feb 29 is invalid
        assert!(matches!(
            parse_hl7_dt("20230229"),
            Err(DateTimeError::InvalidDateFormat(_))
        ));
    }

    #[test]
    fn parse_hl7_dt_year_2000_is_leap_year_2100_is_not() {
        // 2000 is divisible by 400 — is a leap year
        parse_hl7_dt("20000229").expect("Feb 29 2000 is valid");
        // 2100 is divisible by 100 but not 400 — is NOT a leap year
        parse_hl7_dt("21000229").expect_err("Feb 29 2100 is invalid");
    }

    #[test]
    fn parse_hl7_dt_rejects_wrong_length() {
        let err = parse_hl7_dt("2025012").expect_err("too short");
        assert!(matches!(err, DateTimeError::InvalidDateFormat(_)));
        let err = parse_hl7_dt("202501288").expect_err("too long");
        assert!(matches!(err, DateTimeError::InvalidDateFormat(_)));
    }

    #[test]
    fn parse_hl7_dt_rejects_non_digit() {
        let err = parse_hl7_dt("2025-128").expect_err("non-digit");
        assert!(matches!(err, DateTimeError::InvalidDateFormat(_)));
    }

    #[test]
    fn parse_hl7_dt_rejects_invalid_month_day() {
        parse_hl7_dt("20251301").expect_err("month 13 invalid");
        parse_hl7_dt("20250132").expect_err("day 32 invalid");
        parse_hl7_dt("20250000").expect_err("month 0 day 0 invalid");
    }

    // ------------------------------------------------------------------
    // parse_hl7_tm
    // ------------------------------------------------------------------
    #[test]
    fn parse_hl7_tm_hour_minute_only() {
        let (h, m, s, f) = parse_hl7_tm("1530").expect("valid HHMM");
        assert_eq!(h, 15);
        assert_eq!(m, 30);
        assert_eq!(s, 0);
        assert_eq!(f, None);
    }

    #[test]
    fn parse_hl7_tm_with_seconds() {
        let (h, m, s, f) = parse_hl7_tm("153045").expect("valid HHMMSS");
        assert_eq!(h, 15);
        assert_eq!(m, 30);
        assert_eq!(s, 45);
        assert_eq!(f, None);
    }

    #[test]
    fn parse_hl7_tm_fractional_one_to_four_digits() {
        // 1 digit
        let (.., f1) = parse_hl7_tm("153045.5").expect("1-digit fraction");
        assert_eq!(f1, Some(500000));
        // 2 digits
        let (.., f2) = parse_hl7_tm("153045.50").expect("2-digit fraction");
        assert_eq!(f2, Some(500000));
        // 3 digits (millisecond)
        let (.., f3) = parse_hl7_tm("153045.123").expect("3-digit fraction");
        assert_eq!(f3, Some(123000));
        // 4 digits (HL7 TS/TM maximum fractional precision documented here)
        let (.., f4) = parse_hl7_tm("153045.1234").expect("4-digit fraction");
        assert_eq!(f4, Some(123400));
        // Leading zeroes are significant when scaling to microseconds.
        let (.., f4_with_leading_zeroes) =
            parse_hl7_tm("153045.0001").expect("4-digit fraction with leading zeroes");
        assert_eq!(f4_with_leading_zeroes, Some(100));
    }

    #[test]
    fn parse_hl7_tm_rejects_invalid_fractional_seconds() {
        for bad in ["153045.", "153045.abc", "153045.12345"] {
            let err = parse_hl7_tm(bad).expect_err("invalid fraction");
            assert!(
                matches!(err, DateTimeError::InvalidTimeFormat(_)),
                "{bad} returned {err:?}"
            );
        }
    }

    #[test]
    fn parse_hl7_tm_boundary_2359_59() {
        let (h, m, s, _) = parse_hl7_tm("235959").expect("end of day");
        assert_eq!((h, m, s), (23, 59, 59));
    }

    #[test]
    fn parse_hl7_tm_midnight() {
        let (h, m, s, _) = parse_hl7_tm("0000").expect("midnight");
        assert_eq!((h, m, s), (0, 0, 0));
    }

    #[test]
    fn parse_hl7_tm_rejects_too_short() {
        let err = parse_hl7_tm("12").expect_err("too short");
        assert!(matches!(err, DateTimeError::InvalidTimeFormat(_)));
    }

    #[test]
    fn parse_hl7_tm_rejects_non_ascii() {
        let err = parse_hl7_tm("12\u{00e9}0").expect_err("non-ascii");
        assert!(matches!(err, DateTimeError::InvalidTimeFormat(_)));
    }

    #[test]
    fn parse_hl7_tm_rejects_hour_out_of_range() {
        let err = parse_hl7_tm("2400").expect_err("hour 24");
        assert!(matches!(err, DateTimeError::TimeOutOfRange(_)));
    }

    #[test]
    fn parse_hl7_tm_rejects_minute_out_of_range() {
        let err = parse_hl7_tm("1260").expect_err("minute 60");
        assert!(matches!(err, DateTimeError::TimeOutOfRange(_)));
    }

    #[test]
    fn parse_hl7_tm_rejects_second_out_of_range() {
        let err = parse_hl7_tm("125960").expect_err("second 60");
        assert!(matches!(err, DateTimeError::TimeOutOfRange(_)));
    }

    #[test]
    fn parse_hl7_tm_rejects_partial_seconds_and_misplaced_fraction() {
        assert!(matches!(
            parse_hl7_tm("12305"),
            Err(DateTimeError::InvalidTimeFormat(_))
        ));
        parse_hl7_tm("12.5").expect_err("fraction before complete seconds is invalid");
        assert!(!is_valid_hl7_time("12305"));
        assert!(!is_valid_hl7_time("1230."));
        assert!(!is_valid_hl7_time("123045.."));

        parse_hl7_tm("123045").expect("complete seconds are valid");
        assert!(is_valid_hl7_time("123045.5"));
        assert!(is_valid_hl7_time("123045.1234"));

        parse_hl7_ts("2025010112305").expect_err("partial TS seconds are invalid");
        parse_hl7_ts_with_precision("2025010112305")
            .expect_err("partial precision-aware TS seconds are invalid");
        assert!(!is_valid_hl7_timestamp("2025010112305"));
        assert!(is_valid_hl7_timestamp("20250101123045"));
    }

    #[test]
    fn parse_hl7_tm_rejects_non_numeric_hours() {
        let err = parse_hl7_tm("ab30").expect_err("non-numeric hour");
        assert!(matches!(err, DateTimeError::TimeOutOfRange(_)));
    }

    #[test]
    fn parse_hl7_tm_rejects_non_numeric_seconds() {
        let err = parse_hl7_tm("1530ab").expect_err("non-numeric seconds");
        assert!(matches!(err, DateTimeError::TimeOutOfRange(_)));
    }

    // ------------------------------------------------------------------
    // parse_hl7_ts
    // ------------------------------------------------------------------
    #[test]
    fn parse_hl7_ts_date_only_midnight() {
        let dt = parse_hl7_ts("20250128").expect("date-only ts");
        assert_eq!(
            dt.date(),
            NaiveDate::from_ymd_opt(2025, 1, 28).expect("ymd")
        );
        assert_eq!(dt.hour(), 0);
        assert_eq!(dt.minute(), 0);
        assert_eq!(dt.second(), 0);
    }

    #[test]
    fn parse_hl7_ts_with_time() {
        let dt = parse_hl7_ts("20250128152312").expect("ts with time");
        assert_eq!(dt.hour(), 15);
        assert_eq!(dt.minute(), 23);
        assert_eq!(dt.second(), 12);
    }

    #[test]
    fn parse_hl7_ts_rejects_too_short() {
        let err = parse_hl7_ts("2025").expect_err("too short");
        assert!(matches!(err, DateTimeError::InvalidTimestampFormat(_)));
    }

    #[test]
    fn parse_hl7_ts_rejects_non_ascii() {
        let err = parse_hl7_ts("20250128\u{00e9}").expect_err("non-ascii");
        assert!(matches!(err, DateTimeError::InvalidTimestampFormat(_)));
    }

    #[test]
    fn parse_hl7_ts_rejects_invalid_date_part() {
        // 13-month date should propagate the date error
        let err = parse_hl7_ts("20251301").expect_err("bad month");
        assert!(matches!(err, DateTimeError::InvalidDateFormat(_)));
    }

    #[test]
    fn parse_hl7_ts_rejects_invalid_fractional_time_part() {
        for bad in [
            "20250715103456.",
            "20250715103456.abc",
            "20250715103456.12345",
        ] {
            let err = parse_hl7_ts(bad).expect_err("invalid timestamp fraction");
            assert!(
                matches!(err, DateTimeError::InvalidTimeFormat(_)),
                "{bad} returned {err:?}"
            );
        }
    }

    // ------------------------------------------------------------------
    // parse_hl7_ts_with_precision — every precision arm
    // ------------------------------------------------------------------
    #[test]
    fn parse_hl7_ts_with_precision_year() {
        let ts = parse_hl7_ts_with_precision("2025").expect("year precision");
        assert_eq!(ts.precision, TimestampPrecision::Year);
        assert_eq!(ts.datetime.year(), 2025);
        assert_eq!(ts.datetime.month(), 1);
        assert_eq!(ts.datetime.day(), 1);
    }

    #[test]
    fn parse_hl7_ts_with_precision_month() {
        let ts = parse_hl7_ts_with_precision("202507").expect("month precision");
        assert_eq!(ts.precision, TimestampPrecision::Month);
        assert_eq!(ts.datetime.month(), 7);
        assert_eq!(ts.datetime.day(), 1);
    }

    #[test]
    fn parse_hl7_ts_with_precision_day() {
        let ts = parse_hl7_ts_with_precision("20250715").expect("day precision");
        assert_eq!(ts.precision, TimestampPrecision::Day);
        assert_eq!(ts.datetime.day(), 15);
    }

    #[test]
    fn parse_hl7_ts_with_precision_hour() {
        let ts = parse_hl7_ts_with_precision("2025071510").expect("hour precision");
        assert_eq!(ts.precision, TimestampPrecision::Hour);
        assert_eq!(ts.datetime.hour(), 10);
        assert_eq!(ts.datetime.minute(), 0);
    }

    #[test]
    fn parse_hl7_ts_with_precision_minute() {
        let ts = parse_hl7_ts_with_precision("202507151034").expect("minute precision");
        assert_eq!(ts.precision, TimestampPrecision::Minute);
        assert_eq!(ts.datetime.minute(), 34);
        assert_eq!(ts.datetime.second(), 0);
    }

    #[test]
    fn parse_hl7_ts_with_precision_second() {
        let ts = parse_hl7_ts_with_precision("20250715103456").expect("second precision");
        assert_eq!(ts.precision, TimestampPrecision::Second);
        assert_eq!(ts.datetime.second(), 56);
    }

    #[test]
    fn parse_hl7_ts_with_precision_fractional() {
        let ts = parse_hl7_ts_with_precision("20250715103456.123").expect("fractional precision");
        assert_eq!(ts.precision, TimestampPrecision::FractionalSecond);
        assert_eq!(ts.fractional_seconds, Some(123000));
    }

    #[test]
    fn parse_hl7_ts_with_precision_fractional_scales_leading_zeroes() {
        let ts = parse_hl7_ts_with_precision("20250715103456.0001")
            .expect("fractional precision with leading zeroes");
        assert_eq!(ts.precision, TimestampPrecision::FractionalSecond);
        assert_eq!(ts.fractional_seconds, Some(100));
    }

    #[test]
    fn parse_hl7_ts_with_precision_rejects_invalid_fractional_seconds() {
        for bad in [
            "20250715103456.",
            "20250715103456.x",
            "20250715103456.12345",
        ] {
            let err = parse_hl7_ts_with_precision(bad).expect_err("invalid fractional precision");
            assert!(
                matches!(err, DateTimeError::InvalidTimestampFormat(_)),
                "{bad} returned {err:?}"
            );
        }
    }

    #[test]
    fn parse_hl7_ts_with_precision_invalid_length() {
        let err = parse_hl7_ts_with_precision("202").expect_err("3 chars");
        assert!(matches!(err, DateTimeError::InvalidTimestampFormat(_)));
        // 15 chars without a dot at position 14 is also invalid
        let err = parse_hl7_ts_with_precision("202507151034567").expect_err("15 chars no dot");
        assert!(matches!(err, DateTimeError::InvalidTimestampFormat(_)));
    }

    #[test]
    fn parse_hl7_ts_with_precision_rejects_non_ascii() {
        let err = parse_hl7_ts_with_precision("2025\u{00e9}").expect_err("non-ascii");
        assert!(matches!(err, DateTimeError::InvalidTimestampFormat(_)));
    }

    #[test]
    fn parse_hl7_ts_with_precision_invalid_year_value() {
        // Non-numeric 4-char input
        let err = parse_hl7_ts_with_precision("abcd").expect_err("non-numeric year");
        assert!(matches!(err, DateTimeError::InvalidDateFormat(_)));
    }

    #[test]
    fn parse_hl7_ts_with_precision_invalid_month_value() {
        let err = parse_hl7_ts_with_precision("202513").expect_err("month 13");
        assert!(matches!(err, DateTimeError::DateOutOfRange(_)));
    }

    #[test]
    fn parse_hl7_ts_with_precision_invalid_hour_value() {
        let err = parse_hl7_ts_with_precision("2025071524").expect_err("hour 24");
        assert!(matches!(err, DateTimeError::TimeOutOfRange(_)));
    }

    #[test]
    fn parse_hl7_ts_with_precision_invalid_minute_value() {
        let err = parse_hl7_ts_with_precision("202507151060").expect_err("minute 60");
        assert!(matches!(err, DateTimeError::TimeOutOfRange(_)));
    }

    // ------------------------------------------------------------------
    // is_valid_* helpers
    // ------------------------------------------------------------------
    #[test]
    fn is_valid_hl7_date_accept_and_reject() {
        assert!(is_valid_hl7_date("20250128"));
        assert!(!is_valid_hl7_date("2025-01-28"));
        assert!(!is_valid_hl7_date(""));
    }

    #[test]
    fn is_valid_hl7_time_accept_and_reject() {
        assert!(is_valid_hl7_time("1530"));
        assert!(is_valid_hl7_time("153045"));
        assert!(is_valid_hl7_time("153045.123"));
        assert!(!is_valid_hl7_time("25"));
        assert!(!is_valid_hl7_time("2400"));
    }

    #[test]
    fn is_valid_hl7_timestamp_accept_and_reject() {
        assert!(is_valid_hl7_timestamp("20250128"));
        assert!(is_valid_hl7_timestamp("20250128120000"));
        assert!(!is_valid_hl7_timestamp(""));
        assert!(!is_valid_hl7_timestamp("not-a-ts"));
    }

    // ------------------------------------------------------------------
    // now_hl7 / today_hl7 — format & length only (time-dependent)
    // ------------------------------------------------------------------
    #[test]
    fn now_hl7_returns_14_ascii_digits() {
        let now = now_hl7();
        assert_eq!(now.len(), 14);
        assert!(now.chars().all(|c| c.is_ascii_digit()));
        // and is parseable
        assert!(is_valid_hl7_timestamp(&now));
    }

    #[test]
    fn today_hl7_returns_8_ascii_digits() {
        let today = today_hl7();
        assert_eq!(today.len(), 8);
        assert!(today.chars().all(|c| c.is_ascii_digit()));
        assert!(is_valid_hl7_date(&today));
    }

    // ------------------------------------------------------------------
    // ParsedTimestamp helpers
    // ------------------------------------------------------------------
    #[test]
    fn parsed_timestamp_new_sets_no_fractional() {
        let dt = NaiveDate::from_ymd_opt(2025, 1, 28)
            .and_then(|d| d.and_hms_opt(12, 0, 0))
            .expect("valid dt");
        let ts = ParsedTimestamp::new(dt, TimestampPrecision::Second);
        assert_eq!(ts.precision, TimestampPrecision::Second);
        assert_eq!(ts.fractional_seconds, None);
    }

    #[test]
    fn parsed_timestamp_with_fractional_sets_fields() {
        let dt = NaiveDate::from_ymd_opt(2025, 1, 28)
            .and_then(|d| d.and_hms_opt(12, 0, 0))
            .expect("valid dt");
        let ts = ParsedTimestamp::with_fractional(dt, 123_456);
        assert_eq!(ts.precision, TimestampPrecision::FractionalSecond);
        assert_eq!(ts.fractional_seconds, Some(123_456));
    }

    #[test]
    fn parsed_timestamp_is_same_day() {
        let a = parse_hl7_ts_with_precision("20250128").expect("a");
        let b = parse_hl7_ts_with_precision("20250128120000").expect("b");
        let c = parse_hl7_ts_with_precision("20250129").expect("c");
        assert!(a.is_same_day(&b));
        assert!(!a.is_same_day(&c));
    }

    #[test]
    fn parsed_timestamp_is_before_and_after() {
        let a = parse_hl7_ts_with_precision("20250128").expect("a");
        let b = parse_hl7_ts_with_precision("20250129").expect("b");
        assert!(a.is_before(&b));
        assert!(b.is_after(&a));
        assert!(!a.is_after(&b));
        assert!(!b.is_before(&a));

        // Same precision branch
        let c = parse_hl7_ts_with_precision("20250128").expect("c");
        assert!(!a.is_before(&c));
    }

    #[test]
    fn parsed_timestamp_is_equal_with_mixed_precision() {
        let day = parse_hl7_ts_with_precision("20250128").expect("day");
        let sec = parse_hl7_ts_with_precision("20250128120000").expect("sec");
        // Different precisions — equal at the coarser (Day) level
        assert!(day.is_equal(&sec));
        let other_day = parse_hl7_ts_with_precision("20250129").expect("other day");
        assert!(!day.is_equal(&other_day));
    }

    #[test]
    fn parsed_timestamp_to_hl7_string_per_precision() {
        let year = parse_hl7_ts_with_precision("2025").expect("year");
        assert_eq!(year.to_hl7_string(), "2025");

        let month = parse_hl7_ts_with_precision("202507").expect("month");
        assert_eq!(month.to_hl7_string(), "202507");

        let day = parse_hl7_ts_with_precision("20250715").expect("day");
        assert_eq!(day.to_hl7_string(), "20250715");

        let hour = parse_hl7_ts_with_precision("2025071510").expect("hour");
        assert_eq!(hour.to_hl7_string(), "2025071510");

        let minute = parse_hl7_ts_with_precision("202507151034").expect("minute");
        assert_eq!(minute.to_hl7_string(), "202507151034");

        let sec = parse_hl7_ts_with_precision("20250715103456").expect("sec");
        assert_eq!(sec.to_hl7_string(), "20250715103456");

        let frac = parse_hl7_ts_with_precision("20250715103456.123").expect("frac");
        assert_eq!(frac.to_hl7_string(), "20250715103456.123");
    }

    #[test]
    fn parsed_timestamp_to_hl7_string_fractional_without_frac_value() {
        let dt = NaiveDate::from_ymd_opt(2025, 7, 15)
            .and_then(|d| d.and_hms_opt(10, 34, 56))
            .expect("dt");
        // Construct a FractionalSecond precision but with no fractional_seconds
        // (covers the else branch of to_hl7_string).
        let ts = ParsedTimestamp {
            datetime: dt,
            precision: TimestampPrecision::FractionalSecond,
            fractional_seconds: None,
        };
        assert_eq!(ts.to_hl7_string(), "20250715103456");
    }

    // ------------------------------------------------------------------
    // truncate_to_precision (pub(crate) helper)
    // ------------------------------------------------------------------
    #[test]
    fn truncate_to_precision_all_levels() {
        let dt = NaiveDate::from_ymd_opt(2025, 7, 15)
            .and_then(|d| d.and_hms_opt(10, 34, 56))
            .expect("dt");

        let year = truncate_to_precision(&dt, TimestampPrecision::Year);
        assert_eq!(year.month(), 1);
        assert_eq!(year.day(), 1);
        assert_eq!(year.hour(), 0);

        let month = truncate_to_precision(&dt, TimestampPrecision::Month);
        assert_eq!(month.day(), 1);
        assert_eq!(month.hour(), 0);

        let day = truncate_to_precision(&dt, TimestampPrecision::Day);
        assert_eq!(day.hour(), 0);
        assert_eq!(day.minute(), 0);

        let hour = truncate_to_precision(&dt, TimestampPrecision::Hour);
        assert_eq!(hour.hour(), 10);
        assert_eq!(hour.minute(), 0);
        assert_eq!(hour.second(), 0);

        let minute = truncate_to_precision(&dt, TimestampPrecision::Minute);
        assert_eq!(minute.minute(), 34);
        assert_eq!(minute.second(), 0);

        let sec = truncate_to_precision(&dt, TimestampPrecision::Second);
        assert_eq!(sec, dt);
        let frac = truncate_to_precision(&dt, TimestampPrecision::FractionalSecond);
        assert_eq!(frac, dt);
    }

    // ------------------------------------------------------------------
    // DateTimeError display
    // ------------------------------------------------------------------
    #[test]
    fn datetime_error_display_messages() {
        let e = DateTimeError::InvalidDateFormat("x".to_string());
        assert!(format!("{e}").contains("Invalid date"));
        let e = DateTimeError::InvalidTimeFormat("x".to_string());
        assert!(format!("{e}").contains("Invalid time"));
        let e = DateTimeError::InvalidTimestampFormat("x".to_string());
        assert!(format!("{e}").contains("Invalid timestamp"));
        let e = DateTimeError::DateOutOfRange("x".to_string());
        assert!(format!("{e}").contains("Date out"));
        let e = DateTimeError::TimeOutOfRange("x".to_string());
        assert!(format!("{e}").contains("Time out"));
    }
}
