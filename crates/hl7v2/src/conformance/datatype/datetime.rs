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
    /// Fractional seconds (if present)
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
                    format!("{}{frac:06}", self.datetime.format("%Y%m%d%H%M%S"))
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

/// Parse HL7 time (TM format: HHMM[SS[.S...]])
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

        let sec: u32 = sec_part
            .parse()
            .map_err(|_err| DateTimeError::TimeOutOfRange("Invalid second".to_string()))?;
        if sec > 59 {
            return Err(DateTimeError::TimeOutOfRange(format!(
                "Second {sec} out of range"
            )));
        }

        let frac = if let Some(f) = frac_part {
            // Parse fractional seconds (up to 6 digits for microseconds)
            let padded = format!("{:0<6}", f.chars().take(6).collect::<String>());
            Some(padded.parse::<u32>().unwrap_or(0))
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
            let padded = format!("{:0<6}", frac_str.chars().take(6).collect::<String>());
            let fractional: u32 = padded.parse().unwrap_or(0);
            Ok(ParsedTimestamp::with_fractional(dt, fractional))
        }
    }
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
