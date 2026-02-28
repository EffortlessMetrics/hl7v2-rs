//! HL7 v2 Message Validation
//!
//! This crate provides validation functionality for HL7 v2 messages.
//! It can be used standalone for basic validation or integrated with
//! profile-based validation through the `hl7v2-prof` crate.
//!
//! # Features
//!
//! - Data type validation (ST, ID, DT, TM, TS, NM, etc.)
//! - Format validation (phone numbers, emails, SSN, etc.)
//! - Checksum validation (Luhn, Mod10)
//! - Temporal validation (date/time comparisons)
//! - Cross-field validation rules
//! - Contextual validation rules
//! - Custom validation rules
//!
//! # Example
//!
//! ```
//! use hl7v2_validation::{Severity, Issue, validate_data_type};
//!
//! let value = "20230101";
//! let is_valid = validate_data_type(value, "DT");
//! assert!(is_valid);
//! ```

use chrono::{NaiveDate, NaiveDateTime};
use hl7v2_core::Message;
use regex::Regex;
use serde::{Deserialize, Serialize};

/// Severity of validation issues
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Severity {
    /// Error-level issue (validation failure)
    #[default]
    Error,
    /// Warning-level issue (potential problem)
    Warning,
}

/// Validation issue
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Issue {
    /// Issue code (e.g., "MISSING_REQUIRED_FIELD", "INVALID_DATA_TYPE")
    pub code: String,
    /// Severity of the issue
    pub severity: Severity,
    /// Path to the field with the issue (e.g., "PID.5.1")
    pub path: Option<String>,
    /// Detailed description of the issue
    pub detail: String,
}

impl Issue {
    /// Create a new validation issue
    pub fn new(code: &str, severity: Severity, path: Option<String>, detail: String) -> Self {
        Issue {
            code: code.to_string(),
            severity,
            path,
            detail,
        }
    }

    /// Create an error-level issue
    pub fn error(code: &str, path: Option<String>, detail: String) -> Self {
        Issue::new(code, Severity::Error, path, detail)
    }

    /// Create a warning-level issue
    pub fn warning(code: &str, path: Option<String>, detail: String) -> Self {
        Issue::new(code, Severity::Warning, path, detail)
    }
}

/// Validation result type
pub type ValidationResult = Vec<Issue>;

/// Trait for validating HL7 v2 messages
pub trait Validator {
    /// Validate a message and return any issues found
    fn validate(&self, msg: &Message) -> ValidationResult;
}

// ============================================================================
// Data Type Validation
// ============================================================================

/// Check if a value matches the expected HL7 data type
pub fn validate_data_type(value: &str, datatype: &str) -> bool {
    match datatype {
        "ST" => is_string(value),                // String Data
        "ID" => is_identifier(value),            // Coded values for HL7 tables
        "DT" => is_date(value),                  // Date
        "TM" => is_time(value),                  // Time
        "TS" => is_timestamp(value),             // Time Stamp
        "NM" => is_numeric(value),               // Numeric
        "SI" => is_sequence_id(value),           // Sequence ID
        "TX" => is_text_data(value),             // Text Data
        "FT" => is_formatted_text(value),        // Formatted Text Data
        "IS" => is_coded_value(value),           // Coded value for user-defined tables
        "PN" => is_person_name(value),           // Person name
        "CX" => is_extended_id(value),           // Extended composite ID with check digit
        "HD" => is_hierarchic_designator(value), // Hierarchic designator
        _ => true,                               // Unknown data type, assume valid
    }
}

/// Check if value is a valid string (always true for parsed values)
pub fn is_string(_value: &str) -> bool {
    true
}

/// Check if value is a valid identifier (alphanumeric + special characters)
pub fn is_identifier(value: &str) -> bool {
    // HL7 identifiers can contain alphanumeric characters and some special characters
    // For simplicity, we'll check if it contains only printable ASCII characters
    value.chars().all(|c| c.is_ascii() && !c.is_control())
}

/// Check if value is a valid date (YYYYMMDD format)
pub fn is_date(value: &str) -> bool {
    if value.len() != 8 {
        return false;
    }

    // Check if all characters are digits
    if !value.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }

    // Extract year, month, day
    let _year = &value[0..4];
    let month = &value[4..6];
    let day = &value[6..8];

    // Basic validation
    if !("01"..="12").contains(&month) {
        return false;
    }

    if !("01"..="31").contains(&day) {
        return false;
    }

    true
}

/// Check if value is a valid time (HHMM[SS[.S[S[S[S]]]]] format)
pub fn is_time(value: &str) -> bool {
    if value.is_empty() || value.len() > 16 {
        return false;
    }

    // Check if all characters are valid (digits, period)
    if !value.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return false;
    }

    // Must start with at least 4 digits (HHMM)
    if value.len() < 4 {
        return false;
    }

    // Extract hour and minute
    let hour = &value[0..2];
    let minute = &value[2..4];

    // Basic validation
    if hour > "23" {
        return false;
    }

    if minute > "59" {
        return false;
    }

    // If seconds are present
    if value.len() >= 6 {
        let second = &value[4..6];
        if second > "59" {
            return false;
        }
    }

    true
}

/// Check if value is a valid timestamp (YYYYMMDD[HHMM[SS[.S[S[S[S]]]]]] format)
pub fn is_timestamp(value: &str) -> bool {
    if value.len() < 8 {
        return false;
    }

    // First 8 characters should be a valid date
    let date_part = &value[0..8];
    if !is_date(date_part) {
        return false;
    }

    // If time part is present
    if value.len() > 8 {
        let time_part = &value[8..];
        if !is_time(time_part) {
            return false;
        }
    }

    true
}

/// Check if value is numeric
pub fn is_numeric(value: &str) -> bool {
    // Can be integer or decimal
    value.parse::<f64>().is_ok()
}

/// Check if value is a sequence ID (positive integer)
pub fn is_sequence_id(value: &str) -> bool {
    match value.parse::<u32>() {
        Ok(num) => num > 0,
        Err(_) => false,
    }
}

/// Check if value is text data (always true for parsed values)
pub fn is_text_data(_value: &str) -> bool {
    true
}

/// Check if value is formatted text (always true for parsed values)
pub fn is_formatted_text(_value: &str) -> bool {
    true
}

/// Check if value is a coded value (alphanumeric + special characters)
pub fn is_coded_value(value: &str) -> bool {
    // Similar to identifier
    value.chars().all(|c| c.is_ascii() && !c.is_control())
}

/// Check if value is a person name (contains letters, spaces, hyphens, apostrophes)
pub fn is_person_name(value: &str) -> bool {
    value
        .chars()
        .all(|c| c.is_alphabetic() || c.is_whitespace() || c == '-' || c == '\'' || c == '.')
}

/// Check if value is an extended ID (contains identifier characters)
pub fn is_extended_id(value: &str) -> bool {
    is_identifier(value)
}

/// Check if value is a hierarchic designator (contains identifier characters)
pub fn is_hierarchic_designator(value: &str) -> bool {
    is_identifier(value)
}

// ============================================================================
// Format Validation
// ============================================================================

/// Check if value is a valid phone number (basic validation)
pub fn is_phone_number(value: &str) -> bool {
    // Remove common phone number formatting characters
    let cleaned: String = value.chars().filter(|c| c.is_ascii_digit()).collect();

    // Basic phone number validation (7-15 digits)
    cleaned.len() >= 7 && cleaned.len() <= 15 && cleaned.chars().all(|c| c.is_ascii_digit())
}

/// Check if value is a valid email address (basic validation)
pub fn is_email(value: &str) -> bool {
    // Basic email validation - contains @ and has characters before and after
    if !value.contains('@') {
        return false;
    }

    let parts: Vec<&str> = value.split('@').collect();
    if parts.len() != 2 {
        return false;
    }

    let local_part = parts[0];
    let domain_part = parts[1];

    // Check that both parts are non-empty
    if local_part.is_empty() || domain_part.is_empty() {
        return false;
    }

    // Check that domain contains at least one dot
    if !domain_part.contains('.') {
        return false;
    }

    true
}

/// Check if value is a valid SSN (Social Security Number) format
pub fn is_ssn(value: &str) -> bool {
    // Remove dashes and spaces
    let cleaned: String = value.chars().filter(|c| c.is_ascii_digit()).collect();

    // SSN should be exactly 9 digits
    if cleaned.len() != 9 {
        return false;
    }

    // First 3 digits cannot be 000, 666, or 900-999
    let area = &cleaned[0..3];
    if area == "000" || area == "666" || area.starts_with('9') {
        return false;
    }

    // Next 2 digits cannot be 00
    let group = &cleaned[3..5];
    if group == "00" {
        return false;
    }

    // Last 4 digits cannot be 0000
    let serial = &cleaned[5..9];
    if serial == "0000" {
        return false;
    }

    true
}

/// Check if a date is valid and not in the future
pub fn is_valid_birth_date(value: &str) -> bool {
    if !is_date(value) {
        return false;
    }

    // Check if date is not in the future
    let current_date = chrono::Utc::now().format("%Y%m%d").to_string();
    value <= current_date.as_str()
}

/// Check if two dates represent a valid age range (e.g., birth date vs admission date)
pub fn is_valid_age_range(birth_date: &str, reference_date: &str) -> bool {
    if !is_date(birth_date) || !is_date(reference_date) {
        return false;
    }

    // Birth date should be before or equal to reference date
    birth_date <= reference_date
}

/// Check if a value is within a specified range (inclusive)
pub fn is_within_range(value: &str, min: &str, max: &str) -> bool {
    // Parse all values as numbers
    let val: f64 = match value.parse() {
        Ok(n) => n,
        Err(_) => return false,
    };

    let min_val: f64 = match min.parse() {
        Ok(n) => n,
        Err(_) => return false,
    };

    let max_val: f64 = match max.parse() {
        Ok(n) => n,
        Err(_) => return false,
    };

    val >= min_val && val <= max_val
}

/// Check if value matches a complex pattern with multiple conditions
pub fn matches_complex_pattern(value: &str, patterns: &[&str]) -> bool {
    // All patterns must match
    patterns.iter().all(|pattern| {
        if let Ok(regex) = Regex::new(pattern) {
            regex.is_match(value)
        } else {
            false
        }
    })
}

/// Validate that a field value satisfies a mathematical relationship with another field
pub fn validate_mathematical_relationship(value1: &str, value2: &str, operator: &str) -> bool {
    // Parse both values as numbers
    let num1: f64 = match value1.parse() {
        Ok(n) => n,
        Err(_) => return false,
    };

    let num2: f64 = match value2.parse() {
        Ok(n) => n,
        Err(_) => return false,
    };

    match operator {
        "gt" => num1 > num2,
        "lt" => num1 < num2,
        "ge" => num1 >= num2,
        "le" => num1 <= num2,
        "eq" => (num1 - num2).abs() < f64::EPSILON,
        "ne" => (num1 - num2).abs() >= f64::EPSILON,
        _ => false,
    }
}

// ============================================================================
// Checksum Validation
// ============================================================================

/// Validate checksum for a value
pub fn validate_checksum(value: &str, algorithm: &str) -> bool {
    match algorithm {
        "luhn" => validate_luhn_checksum(value),
        "mod10" => validate_mod10_checksum(value),
        _ => true, // Unknown algorithm, assume valid
    }
}

/// Validate Luhn checksum (used for credit cards, etc.)
pub fn validate_luhn_checksum(value: &str) -> bool {
    // Remove any non-digit characters
    let digits: String = value.chars().filter(|c| c.is_ascii_digit()).collect();

    if digits.len() < 2 {
        return false;
    }

    let mut sum = 0;
    let mut double = false;

    // Process digits from right to left
    for digit_char in digits.chars().rev() {
        let digit = digit_char.to_digit(10).unwrap_or(0);

        if double {
            let doubled = digit * 2;
            sum += if doubled > 9 { doubled - 9 } else { doubled };
        } else {
            sum += digit;
        }

        double = !double;
    }

    sum % 10 == 0
}

/// Validate Mod10 checksum
pub fn validate_mod10_checksum(value: &str) -> bool {
    // This is essentially the same as Luhn for our purposes
    validate_luhn_checksum(value)
}

// ============================================================================
// Format Matching
// ============================================================================

/// Check if value matches the specified format
pub fn matches_format(value: &str, format: &str, datatype: &str) -> bool {
    match (datatype, format) {
        ("DT", "YYYY-MM-DD") => {
            // Check if value matches YYYY-MM-DD format
            if value.len() != 10 {
                return false;
            }
            let parts: Vec<&str> = value.split('-').collect();
            if parts.len() != 3 {
                return false;
            }
            // Check year (4 digits)
            if parts[0].len() != 4 || !parts[0].chars().all(|c| c.is_ascii_digit()) {
                return false;
            }
            // Check month (2 digits)
            if parts[1].len() != 2 || !parts[1].chars().all(|c| c.is_ascii_digit()) {
                return false;
            }
            let month: u32 = parts[1].parse().unwrap_or(0);
            if !(1..=12).contains(&month) {
                return false;
            }
            // Check day (2 digits)
            if parts[2].len() != 2 || !parts[2].chars().all(|c| c.is_ascii_digit()) {
                return false;
            }
            let day: u32 = parts[2].parse().unwrap_or(0);
            if !(1..=31).contains(&day) {
                return false;
            }
            true
        }
        ("TM", "HH:MM:SS") => {
            // Check if value matches HH:MM:SS format
            if value.len() != 8 {
                return false;
            }
            let parts: Vec<&str> = value.split(':').collect();
            if parts.len() != 3 {
                return false;
            }
            // Check hour (2 digits)
            if parts[0].len() != 2 || !parts[0].chars().all(|c| c.is_ascii_digit()) {
                return false;
            }
            let hour: u32 = parts[0].parse().unwrap_or(0);
            if hour > 23 {
                return false;
            }
            // Check minute (2 digits)
            if parts[1].len() != 2 || !parts[1].chars().all(|c| c.is_ascii_digit()) {
                return false;
            }
            let minute: u32 = parts[1].parse().unwrap_or(0);
            if minute > 59 {
                return false;
            }
            // Check second (2 digits)
            if parts[2].len() != 2 || !parts[2].chars().all(|c| c.is_ascii_digit()) {
                return false;
            }
            let second: u32 = parts[2].parse().unwrap_or(0);
            if second > 59 {
                return false;
            }
            true
        }
        _ => true, // Unknown format, assume valid
    }
}

// ============================================================================
// Temporal Validation
// ============================================================================

/// Precision levels for timestamps
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
    /// Full precision (YYYYMMDDHHMMSS)
    Second,
}

/// Parsed timestamp with precision information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedTimestamp {
    /// The parsed datetime
    pub datetime: NaiveDateTime,
    /// The precision of the timestamp
    pub precision: TimestampPrecision,
}

/// Parse HL7 TS (timestamp) value
pub fn parse_hl7_ts(s: &str) -> Option<NaiveDateTime> {
    let s = s.trim();
    // longest first
    let fmts = &[
        "%Y%m%d%H%M%S", // 14
        "%Y%m%d%H%M",   // 12
        "%Y%m%d%H",     // 10
    ];
    for f in fmts {
        if let Ok(dt) = NaiveDateTime::parse_from_str(s, f) {
            return Some(dt);
        }
    }
    if s.len() == 8
        && let Ok(d) = NaiveDate::parse_from_str(s, "%Y%m%d")
    {
        return d.and_hms_opt(0, 0, 0);
    }
    None
}

/// Parse HL7 TS with precision information
pub fn parse_hl7_ts_with_precision(s: &str) -> Option<ParsedTimestamp> {
    let s = s.trim();

    // Try full datetime formats first
    let formats = &[
        ("%Y%m%d%H%M%S", TimestampPrecision::Second), // 14 chars
        ("%Y%m%d%H%M", TimestampPrecision::Minute),   // 12 chars
        ("%Y%m%d%H", TimestampPrecision::Hour),       // 10 chars
    ];

    for (format, precision) in formats {
        if let Ok(dt) = NaiveDateTime::parse_from_str(s, format) {
            return Some(ParsedTimestamp {
                datetime: dt,
                precision: *precision,
            });
        }
    }

    // Try date only format
    if s.len() == 8
        && let Ok(date) = NaiveDate::parse_from_str(s, "%Y%m%d")
    {
        return Some(ParsedTimestamp {
            datetime: date.and_hms_opt(0, 0, 0)?,
            precision: TimestampPrecision::Day,
        });
    }

    // Try year-month format
    if s.len() == 6
        && let Ok(date) = NaiveDate::parse_from_str(&format!("{}01", s), "%Y%m%d")
    {
        return Some(ParsedTimestamp {
            datetime: date.and_hms_opt(0, 0, 0)?,
            precision: TimestampPrecision::Month,
        });
    }

    // Try year only format
    if s.len() == 4
        && let Ok(date) = NaiveDate::parse_from_str(&format!("{}0101", s), "%Y%m%d")
    {
        return Some(ParsedTimestamp {
            datetime: date.and_hms_opt(0, 0, 0)?,
            precision: TimestampPrecision::Year,
        });
    }

    None
}

/// Compare two timestamps with partial precision handling
/// For "before" comparisons with partial precision:
/// - If comparing 20230101 (date) with 20230101120000 (datetime),
///   we should consider them "equal" for the date part, not treat the date as 00:00:00
pub fn compare_timestamps_for_before(a: &ParsedTimestamp, b: &ParsedTimestamp) -> bool {
    // If both have the same precision, compare directly
    if a.precision == b.precision {
        return a.datetime < b.datetime;
    }

    // For different precisions, we need to truncate the more precise one
    // to match the less precise one's precision
    let min_precision = std::cmp::min(a.precision, b.precision);

    // Truncate both timestamps to the minimum precision
    let truncated_a = truncate_to_precision(&a.datetime, min_precision);
    let truncated_b = truncate_to_precision(&b.datetime, min_precision);

    // Now compare the truncated versions
    truncated_a < truncated_b
}

/// Truncate a datetime to a specific precision
pub fn truncate_to_precision(dt: &NaiveDateTime, precision: TimestampPrecision) -> NaiveDateTime {
    use chrono::{Datelike, Timelike};

    match precision {
        TimestampPrecision::Year => NaiveDate::from_ymd_opt(dt.year(), 1, 1)
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .unwrap_or(*dt),
        TimestampPrecision::Month => NaiveDate::from_ymd_opt(dt.year(), dt.month(), 1)
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .unwrap_or(*dt),
        TimestampPrecision::Day => dt.date().and_hms_opt(0, 0, 0).unwrap_or(*dt),
        TimestampPrecision::Hour => dt.with_minute(0).and_then(|d| d.with_second(0)).unwrap_or(*dt),
        TimestampPrecision::Minute => dt.with_second(0).unwrap_or(*dt),
        TimestampPrecision::Second => *dt,
    }
}

/// Parse datetime string (supports various HL7 formats)
pub fn parse_datetime(value: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    // Try YYYYMMDDHHMMSS format
    if value.len() == 14
        && let Ok(dt) = chrono::NaiveDateTime::parse_from_str(value, "%Y%m%d%H%M%S")
    {
        return Some(dt.and_utc());
    }

    // Try YYYYMMDD format
    if value.len() == 8
        && let Ok(date) = chrono::NaiveDate::parse_from_str(value, "%Y%m%d")
    {
        return Some(date.and_hms_opt(0, 0, 0)?.and_utc());
    }

    // Try YYYY-MM-DD format
    if value.len() == 10
        && let Ok(date) = chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
    {
        return Some(date.and_hms_opt(0, 0, 0)?.and_utc());
    }

    None
}

// ============================================================================
// Field Value Helpers
// ============================================================================

/// Return HL7 value only if non-empty after trim.
#[inline]
pub fn get_nonempty<'a>(msg: &'a Message, path: &str) -> Option<&'a str> {
    hl7v2_core::get(msg, path).and_then(|s| {
        let t = s.trim();
        if t.is_empty() {
            None
        } else {
            Some(t)
        }
    })
}

// ============================================================================
// Validation Rule Types (for profile-based validation)
// ============================================================================

/// Condition operator types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum ConditionOperator {
    /// Equal
    #[default]
    Eq,
    /// Not equal
    Ne,
    /// Greater than
    Gt,
    /// Less than
    Lt,
    /// Greater than or equal
    Ge,
    /// Less than or equal
    Le,
    /// Value in list
    In,
    /// Contains substring
    Contains,
    /// Field exists
    Exists,
    /// Field missing
    Missing,
    /// Matches regex
    MatchesRegex,
    /// Is a valid date
    IsDate,
    /// Before (temporal)
    Before,
    /// Within range
    WithinRange,
}

/// Rule condition for cross-field validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleCondition {
    /// Field path
    pub field: String,
    /// Comparison operator
    pub operator: String,
    /// Expected value (single)
    #[serde(default)]
    pub value: Option<String>,
    /// Expected values (list)
    #[serde(default)]
    pub values: Option<Vec<String>>,
}

/// Rule action for cross-field validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleAction {
    /// Target field path
    pub field: String,
    /// Action type (require, prohibit, validate)
    pub action: String,
    /// Custom error message
    #[serde(default)]
    pub message: Option<String>,
    /// Data type to validate against
    #[serde(default)]
    pub datatype: Option<String>,
    /// Value set to validate against
    #[serde(default)]
    pub valueset: Option<String>,
}

/// Check if a rule condition is met
pub fn check_rule_condition(msg: &Message, condition: &RuleCondition) -> bool {
    // Left-hand side (path) value:
    let lhs = get_nonempty(msg, &condition.field);

    // Right-hand value(s):
    let rhs_first = condition.value.as_deref();
    let rhs_list: Vec<&str> = condition
        .values
        .as_ref()
        .map_or(Vec::new(), |v| v.iter().map(|s| s.as_str()).collect());

    match condition.operator.as_str() {
        // value/string ops
        "eq" => match (lhs, rhs_first) {
            (Some(l), Some(r)) => l == r,
            (None, Some(r)) => r.is_empty(), // treat empty LHS equal to empty RHS
            (Some(l), None) => l.is_empty(),
            (None, None) => true,
        },
        "ne" => match (lhs, rhs_first) {
            (Some(l), Some(r)) => l != r,
            (None, Some(r)) => !r.is_empty(),
            (Some(l), None) => !l.is_empty(),
            (None, None) => false,
        },
        "contains" => {
            let needle = rhs_first.unwrap_or_default();
            lhs.map(|l| l.contains(needle)).unwrap_or(false)
        }
        "in" => {
            lhs.map(|l| rhs_list.contains(&l)).unwrap_or(false)
        }
        "matches_regex" => {
            if let (Some(l), Some(pat)) = (lhs, rhs_first) {
                // compile per-call for simplicity; optimize later with a cache if needed
                Regex::new(pat).map(|re| re.is_match(l)).unwrap_or(false)
            } else {
                false
            }
        }

        // existence
        "exists" => lhs.is_some(),
        "not_exists" => lhs.is_none(),

        // temporal: accepts HL7 TS or YYYYMMDD
        "is_date" => lhs.and_then(parse_hl7_ts_with_precision).is_some(),
        "before" => {
            // Try to parse left-hand side
            if let Some(lhs_ts) = lhs.and_then(parse_hl7_ts_with_precision) {
                // Right-hand side can be either a literal value or a field path
                let rhs_value = if let Some(rhs_field) = rhs_first {
                    // Check if rhs_field is a valid field path by trying to get its value
                    if let Some(rhs_val) = get_nonempty(msg, rhs_field) {
                        Some(rhs_val)
                    } else {
                        // Treat as literal value
                        Some(rhs_field)
                    }
                } else {
                    None
                };

                // Try to parse right-hand side
                if let Some(rhs_ts) = rhs_value.and_then(parse_hl7_ts_with_precision) {
                    compare_timestamps_for_before(&lhs_ts, &rhs_ts)
                } else {
                    false
                }
            } else {
                false
            }
        }
        // numeric range over integers OR date range over TS
        "within_range" => {
            if rhs_list.len() != 2 {
                return false;
            }
            let a = rhs_list[0];
            let b = rhs_list[1];
            // Try dates first
            if let (Some(l), Some(lo), Some(hi)) =
                (lhs.and_then(parse_hl7_ts), parse_hl7_ts(a), parse_hl7_ts(b))
            {
                return l >= lo && l <= hi;
            }
            // Fallback to integer range
            if let (Some(l), Ok(lo), Ok(hi)) = (lhs, a.parse::<i64>(), b.parse::<i64>())
                && let Ok(li) = l.parse::<i64>()
            {
                return li >= lo && li <= hi;
            }
            false
        }
        _ => {
            // Unknown operator, ignore
            false
        }
    }
}

// ============================================================================
// Test Modules
// ============================================================================

#[cfg(test)]
pub mod tests;

// Legacy tests kept for backward compatibility
#[cfg(test)]
mod legacy_tests {
    use super::*;

    #[test]
    fn test_is_date() {
        assert!(is_date("20230101"));
        assert!(is_date("19991231"));
        assert!(!is_date("20231301")); // Invalid month
        assert!(!is_date("20230132")); // Invalid day
        assert!(!is_date("2023010"));  // Too short
        assert!(!is_date("202301011")); // Too long
    }

    #[test]
    fn test_is_time() {
        assert!(is_time("1200"));
        assert!(is_time("235959"));
        assert!(is_time("0000"));
        assert!(!is_time("2400"));     // Invalid hour
        assert!(!is_time("1260"));     // Invalid minute
        assert!(!is_time("123"));      // Too short
    }

    #[test]
    fn test_is_timestamp() {
        assert!(is_timestamp("20230101"));
        assert!(is_timestamp("202301011200"));
        assert!(is_timestamp("20230101120000"));
        assert!(!is_timestamp("2023")); // Too short
    }

    #[test]
    fn test_is_numeric() {
        assert!(is_numeric("123"));
        assert!(is_numeric("123.45"));
        assert!(is_numeric("-123"));
        assert!(!is_numeric("abc"));
    }

    #[test]
    fn test_is_email() {
        assert!(is_email("test@example.com"));
        assert!(is_email("user.name@domain.org"));
        assert!(!is_email("invalid"));
        assert!(!is_email("@domain.com"));
        assert!(!is_email("user@"));
    }

    #[test]
    fn test_is_ssn() {
        assert!(is_ssn("123456789"));
        assert!(is_ssn("123-45-6789"));
        assert!(!is_ssn("000123456"));   // Invalid area
        assert!(!is_ssn("666123456"));   // Invalid area
        assert!(!is_ssn("123450000"));   // Invalid serial
    }

    #[test]
    fn test_validate_luhn_checksum() {
        assert!(validate_luhn_checksum("4532015112830366")); // Valid test card
        assert!(!validate_luhn_checksum("4532015112830367")); // Invalid
    }

    #[test]
    fn test_parse_hl7_ts() {
        assert!(parse_hl7_ts("20230101").is_some());
        assert!(parse_hl7_ts("202301011200").is_some());
        assert!(parse_hl7_ts("20230101120000").is_some());
        assert!(parse_hl7_ts("invalid").is_none());
    }

    #[test]
    fn test_issue_creation() {
        let issue = Issue::error("TEST_CODE", Some("PID.5".to_string()), "Test detail".to_string());
        assert_eq!(issue.code, "TEST_CODE");
        assert_eq!(issue.severity, Severity::Error);
        assert_eq!(issue.path, Some("PID.5".to_string()));
    }
}
