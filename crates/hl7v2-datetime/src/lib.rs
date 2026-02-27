//! HL7 v2 date/time parsing and validation.
//!
//! This crate provides comprehensive date/time handling for HL7 v2 messages,
//! supporting various HL7 timestamp formats and precision levels.
//!
//! # Supported Formats
//!
//! - `DT` (Date): YYYYMMDD
//! - `TM` (Time): HHMM[SS[.S[S[S[S]]]]]
//! - `TS` (Timestamp): YYYYMMDD[HHMM[SS[.S[S[S[S]]]]]]
//!
//! # Example
//!
//! ```
//! use hl7v2_datetime::{parse_hl7_ts, parse_hl7_dt, parse_hl7_tm, parse_hl7_ts_with_precision, TimestampPrecision};
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

use chrono::{NaiveDate, NaiveDateTime, Datelike, Timelike};

/// Error type for date/time parsing
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum DateTimeError {
    #[error("Invalid date format: {0}")]
    InvalidDateFormat(String),
    
    #[error("Invalid time format: {0}")]
    InvalidTimeFormat(String),
    
    #[error("Invalid timestamp format: {0}")]
    InvalidTimestampFormat(String),
    
    #[error("Date out of range: {0}")]
    DateOutOfRange(String),
    
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
                    format!("{}{:06}", self.datetime.format("%Y%m%d%H%M%S"), frac)
                } else {
                    self.datetime.format("%Y%m%d%H%M%S").to_string()
                }
            }
        }
    }
}

/// Parse HL7 date (DT format: YYYYMMDD)
pub fn parse_hl7_dt(s: &str) -> Result<NaiveDate, DateTimeError> {
    let s = s.trim();
    
    if s.len() != 8 {
        return Err(DateTimeError::InvalidDateFormat(
            format!("Expected 8 characters, got {}", s.len())
        ));
    }
    
    if !s.chars().all(|c| c.is_ascii_digit()) {
        return Err(DateTimeError::InvalidDateFormat(
            "Contains non-digit characters".to_string()
        ));
    }
    
    NaiveDate::parse_from_str(s, "%Y%m%d")
        .map_err(|e| DateTimeError::InvalidDateFormat(e.to_string()))
}

/// Parse HL7 time (TM format: HHMM[SS[.S...]])
pub fn parse_hl7_tm(s: &str) -> Result<(u32, u32, u32, Option<u32>), DateTimeError> {
    let s = s.trim();
    
    if s.len() < 4 {
        return Err(DateTimeError::InvalidTimeFormat(
            format!("Expected at least 4 characters, got {}", s.len())
        ));
    }
    
    // Parse hour and minute (required)
    let hour: u32 = s[0..2].parse().map_err(|_| DateTimeError::TimeOutOfRange("Invalid hour".to_string()))?;
    let minute: u32 = s[2..4].parse().map_err(|_| DateTimeError::TimeOutOfRange("Invalid minute".to_string()))?;
    
    // Validate hour and minute
    if hour > 23 {
        return Err(DateTimeError::TimeOutOfRange(format!("Hour {} out of range", hour)));
    }
    if minute > 59 {
        return Err(DateTimeError::TimeOutOfRange(format!("Minute {} out of range", minute)));
    }
    
    // Parse seconds (optional)
    let (second, fractional) = if s.len() > 4 {
        // Check for fractional seconds
        let (sec_part, frac_part) = if let Some(dot_pos) = s[4..].find('.') {
            let sec = &s[4..4+dot_pos];
            let frac = &s[4+dot_pos+1..];
            (sec, Some(frac))
        } else {
            (&s[4..], None)
        };
        
        let sec: u32 = sec_part.parse().map_err(|_| DateTimeError::TimeOutOfRange("Invalid second".to_string()))?;
        if sec > 59 {
            return Err(DateTimeError::TimeOutOfRange(format!("Second {} out of range", sec)));
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
pub fn parse_hl7_ts(s: &str) -> Result<NaiveDateTime, DateTimeError> {
    let s = s.trim();
    
    if s.len() < 8 {
        return Err(DateTimeError::InvalidTimestampFormat(
            format!("Expected at least 8 characters, got {}", s.len())
        ));
    }
    
    // Parse date part
    let date = parse_hl7_dt(&s[0..8])?;
    
    // If only date, return with midnight time
    if s.len() == 8 {
        return Ok(date.and_hms_opt(0, 0, 0).unwrap());
    }
    
    // Parse time part
    let time_str = &s[8..];
    let (hour, minute, second, _) = parse_hl7_tm(time_str)?;
    
    date.and_hms_opt(hour, minute, second)
        .ok_or_else(|| DateTimeError::TimeOutOfRange("Invalid time combination".to_string()))
}

/// Parse HL7 timestamp with precision information
pub fn parse_hl7_ts_with_precision(s: &str) -> Result<ParsedTimestamp, DateTimeError> {
    let s = s.trim();
    
    // Determine precision from length
    let precision = match s.len() {
        4 => TimestampPrecision::Year,
        6 => TimestampPrecision::Month,
        8 => TimestampPrecision::Day,
        10 => TimestampPrecision::Hour,
        12 => TimestampPrecision::Minute,
        14 => TimestampPrecision::Second,
        n if n > 14 && s[14..].starts_with('.') => TimestampPrecision::FractionalSecond,
        _ => return Err(DateTimeError::InvalidTimestampFormat(
            format!("Invalid length: {}", s.len())
        )),
    };
    
    // Parse based on precision
    match precision {
        TimestampPrecision::Year => {
            let year: i32 = s.parse().map_err(|_| DateTimeError::InvalidDateFormat("Invalid year".into()))?;
            let date = NaiveDate::from_ymd_opt(year, 1, 1)
                .ok_or_else(|| DateTimeError::DateOutOfRange("Invalid year".into()))?;
            Ok(ParsedTimestamp::new(date.and_hms_opt(0, 0, 0).unwrap(), precision))
        }
        TimestampPrecision::Month => {
            let year: i32 = s[0..4].parse().map_err(|_| DateTimeError::InvalidDateFormat("Invalid year".into()))?;
            let month: u32 = s[4..6].parse().map_err(|_| DateTimeError::InvalidDateFormat("Invalid month".into()))?;
            let date = NaiveDate::from_ymd_opt(year, month, 1)
                .ok_or_else(|| DateTimeError::DateOutOfRange("Invalid month".into()))?;
            Ok(ParsedTimestamp::new(date.and_hms_opt(0, 0, 0).unwrap(), precision))
        }
        TimestampPrecision::Day => {
            let date = parse_hl7_dt(s)?;
            Ok(ParsedTimestamp::new(date.and_hms_opt(0, 0, 0).unwrap(), precision))
        }
        TimestampPrecision::Hour => {
            let date = parse_hl7_dt(&s[0..8])?;
            let hour: u32 = s[8..10].parse().map_err(|_| DateTimeError::TimeOutOfRange("Invalid hour".into()))?;
            Ok(ParsedTimestamp::new(
                date.and_hms_opt(hour, 0, 0).unwrap(),
                precision
            ))
        }
        TimestampPrecision::Minute => {
            let date = parse_hl7_dt(&s[0..8])?;
            let hour: u32 = s[8..10].parse().map_err(|_| DateTimeError::TimeOutOfRange("Invalid hour".into()))?;
            let minute: u32 = s[10..12].parse().map_err(|_| DateTimeError::TimeOutOfRange("Invalid minute".into()))?;
            Ok(ParsedTimestamp::new(
                date.and_hms_opt(hour, minute, 0).unwrap(),
                precision
            ))
        }
        TimestampPrecision::Second => {
            let dt = parse_hl7_ts(s)?;
            Ok(ParsedTimestamp::new(dt, precision))
        }
        TimestampPrecision::FractionalSecond => {
            // Parse base timestamp
            let dt = parse_hl7_ts(&s[0..14])?;
            // Parse fractional part
            let frac_str = &s[15..]; // Skip the dot
            let padded = format!("{:0<6}", frac_str.chars().take(6).collect::<String>());
            let fractional: u32 = padded.parse().unwrap_or(0);
            Ok(ParsedTimestamp::with_fractional(dt, fractional))
        }
    }
}

/// Truncate a datetime to a specific precision
fn truncate_to_precision(dt: &NaiveDateTime, precision: TimestampPrecision) -> NaiveDateTime {
    match precision {
        TimestampPrecision::Year => {
            NaiveDate::from_ymd_opt(dt.year(), 1, 1)
                .and_then(|d| d.and_hms_opt(0, 0, 0))
                .unwrap_or(*dt)
        }
        TimestampPrecision::Month => {
            NaiveDate::from_ymd_opt(dt.year(), dt.month(), 1)
                .and_then(|d| d.and_hms_opt(0, 0, 0))
                .unwrap_or(*dt)
        }
        TimestampPrecision::Day => {
            dt.date().and_hms_opt(0, 0, 0).unwrap_or(*dt)
        }
        TimestampPrecision::Hour => {
            dt.with_minute(0).and_then(|d| d.with_second(0)).unwrap_or(*dt)
        }
        TimestampPrecision::Minute => {
            dt.with_second(0).unwrap_or(*dt)
        }
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
mod tests;