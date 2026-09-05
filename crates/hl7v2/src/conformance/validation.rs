//! HL7 v2 Message Validation
//!
//! This module provides validation functionality for HL7 v2 messages.
//! It can be used standalone for basic validation or integrated with
//! profile-based validation through `hl7v2::conformance::profile`.
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
//! use hl7v2::conformance::validation::{Severity, Issue, validate_data_type};
//!
//! let value = "20230101";
//! let is_valid = validate_data_type(value, "DT");
//! assert!(is_valid);
//! ```

#![expect(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::uninlined_format_args,
    reason = "Pre-existing validation implementation debt moved during module collapse; cleanup is separate from this behavior-preserving change."
)]

use crate::conformance::regex_cache;
use crate::model::{Atom, Field, Message, Rep, Segment};
use chrono::{NaiveDate, NaiveDateTime};
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

/// Stable severity values used by machine-readable validation reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValidationReportSeverity {
    /// Error-level issue.
    Error,
    /// Warning-level issue.
    Warning,
}

impl ValidationReportSeverity {
    /// Return the stable lowercase string representation.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
        }
    }
}

impl From<&Severity> for ValidationReportSeverity {
    fn from(value: &Severity) -> Self {
        match value {
            Severity::Error => Self::Error,
            Severity::Warning => Self::Warning,
        }
    }
}

/// Machine-readable validation report shared by CLI and service surfaces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationReport {
    /// Whether the message passed validation without error-level issues.
    pub valid: bool,
    /// HL7 trigger event from `MSH.9`, such as `ADT^A01`.
    pub message_type: String,
    /// Profile identifier, usually a path or configured profile name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    /// Number of parsed message segments.
    pub segment_count: usize,
    /// Number of reported validation issues.
    pub issue_count: usize,
    /// Stable validation issue records.
    pub issues: Vec<ValidationReportIssue>,
}

impl ValidationReport {
    /// Build a report from the message, optional profile label, and validation issues.
    pub fn from_issues(message: &Message, profile: Option<String>, issues: Vec<Issue>) -> Self {
        let report_issues: Vec<ValidationReportIssue> = issues
            .into_iter()
            .map(|issue| ValidationReportIssue::from_issue(message, issue))
            .collect();
        let valid = report_issues
            .iter()
            .all(|issue| issue.severity != ValidationReportSeverity::Error);

        Self {
            valid,
            message_type: message_type(message),
            profile,
            segment_count: message.segments.len(),
            issue_count: report_issues.len(),
            issues: report_issues,
        }
    }

    /// Convert this v1 report into the explicit v2 evidence contract shape.
    ///
    /// This does not change the default serialized form of `ValidationReport`.
    /// Producers opt into v2 when they are ready to emit embedded provenance.
    pub fn to_v2(
        &self,
        tool_name: impl Into<String>,
        tool_version: impl Into<String>,
        profile_identity: Option<ValidationReportProfileIdentity>,
    ) -> ValidationReportV2 {
        ValidationReportV2 {
            schema_version: "2".to_string(),
            tool_name: tool_name.into(),
            tool_version: tool_version.into(),
            valid: self.valid,
            message_type: self.message_type.clone(),
            profile: self.profile.clone(),
            profile_identity,
            segment_count: self.segment_count,
            issue_count: self.issue_count,
            issues: self.issues.clone(),
        }
    }
}

/// Reproducible profile identity metadata for validation report v2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationReportProfileIdentity {
    /// Display label for the profile source.
    pub label: String,
    /// Loaded profile message structure when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_structure: Option<String>,
    /// Loaded profile HL7 version when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// SHA-256 digest of the profile bytes when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
}

/// Validation report v2 with embedded evidence provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationReportV2 {
    /// Evidence artifact schema version.
    pub schema_version: String,
    /// Producer surface that generated this report.
    pub tool_name: String,
    /// Semantic version of the producing crate or binding package.
    pub tool_version: String,
    /// Whether the message passed validation without error-level issues.
    pub valid: bool,
    /// HL7 trigger event from `MSH.9`, such as `ADT^A01`.
    pub message_type: String,
    /// Profile identifier, usually a path or configured profile name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    /// Reproducible profile identity metadata when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_identity: Option<ValidationReportProfileIdentity>,
    /// Number of parsed message segments.
    pub segment_count: usize,
    /// Number of reported validation issues.
    pub issue_count: usize,
    /// Stable validation issue records.
    pub issues: Vec<ValidationReportIssue>,
}

/// Stable validation issue record used in machine-readable reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationReportIssue {
    /// Stable snake_case issue code.
    pub code: String,
    /// Error or warning severity.
    pub severity: ValidationReportSeverity,
    /// HL7 path associated with the issue, such as `PID.3`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Stable rule identifier. Today this mirrors `code` for profile-generated issues.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_id: Option<String>,
    /// Human-readable issue message.
    pub message: String,
    /// Zero-based segment index when it can be inferred from the issue path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub segment_index: Option<usize>,
    /// One-based HL7 field index when it can be inferred from the issue path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field_index: Option<usize>,
}

impl ValidationReportIssue {
    /// Convert an internal validation issue into a stable report issue.
    pub fn from_issue(message: &Message, issue: Issue) -> Self {
        let code = stable_issue_code(&issue.code);
        let (segment_index, field_index) = issue.path.as_deref().map_or((None, None), |path| {
            (
                segment_index_for_path(message, path),
                field_index_for_path(path),
            )
        });

        Self {
            rule_id: if code.is_empty() {
                None
            } else {
                Some(code.clone())
            },
            code,
            severity: ValidationReportSeverity::from(&issue.severity),
            path: issue.path,
            message: issue.detail,
            segment_index,
            field_index,
        }
    }
}

/// Validation result type
pub type ValidationResult = Vec<Issue>;

/// Trait for validating HL7 v2 messages
pub trait Validator {
    /// Validate a message and return any issues found
    fn validate(&self, msg: &Message) -> ValidationResult;
}

fn stable_issue_code(code: &str) -> String {
    let mut output = String::new();
    let mut previous_was_separator = false;

    for character in code.chars() {
        if character.is_ascii_alphanumeric() {
            output.push(character.to_ascii_lowercase());
            previous_was_separator = false;
        } else if !previous_was_separator && !output.is_empty() {
            output.push('_');
            previous_was_separator = true;
        }
    }

    if output.ends_with('_') {
        output.pop();
    }

    output
}

fn message_type(message: &Message) -> String {
    let message_code = crate::query::get(message, "MSH.9.1")
        .or_else(|| crate::query::get(message, "MSH.9"))
        .unwrap_or("UNKNOWN");
    let trigger_event = crate::query::get(message, "MSH.9.2");

    trigger_event.map_or_else(
        || message_code.to_string(),
        |event| format!("{}^{}", message_code, event),
    )
}

fn segment_index_for_path(message: &Message, path: &str) -> Option<usize> {
    if let Some((segment, segment_repetition)) = segment_occurrence_for_path(path) {
        return message
            .segments
            .iter()
            .enumerate()
            .filter(|(_index, actual)| actual.id_str() == segment)
            .nth(segment_repetition.checked_sub(1)?)
            .map(|(index, _segment)| index);
    }

    let located = crate::query::path::parse_located_path(path).ok()?;
    let segment_repetition = located.segment_repetition.unwrap_or(1);

    message
        .segments
        .iter()
        .enumerate()
        .filter(|(_index, segment)| segment.id_str() == located.path.segment)
        .nth(segment_repetition.checked_sub(1)?)
        .map(|(index, _segment)| index)
}

fn segment_occurrence_for_path(path: &str) -> Option<(String, usize)> {
    let path = path.trim();
    if path.is_empty() || path.contains('.') || path.contains('-') {
        return None;
    }

    let (segment, repetition) = if let Some(start) = path.find('[') {
        if !path.ends_with(']') {
            return None;
        }
        let segment = &path[..start];
        let repetition = &path[start + 1..path.len().checked_sub(1)?];
        (segment, repetition.parse::<usize>().ok()?)
    } else {
        (path, 1)
    };

    if segment.is_empty()
        || repetition == 0
        || !segment
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
    {
        return None;
    }

    Some((segment.to_ascii_uppercase(), repetition))
}

fn field_index_for_path(path: &str) -> Option<usize> {
    crate::query::path::parse_located_path(path)
        .ok()
        .map(|located| located.path.field)
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
        "AD" => crate::conformance::datatype::validate_datatype(value, "AD"),
        "XTN" => crate::conformance::datatype::validate_datatype(value, "XTN"),
        _ => false, // Unknown data type, reject it
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
    if value.len() != 8 || !value.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }

    crate::conformance::datatype::datetime::parse_hl7_dt(value).is_ok()
}

/// Check if value is a valid time (HHMM\[SS\[.S\[S\[S\[S\]\]\]\]\] format)
pub fn is_time(value: &str) -> bool {
    value.trim() == value && crate::conformance::datatype::datetime::parse_hl7_tm(value).is_ok()
}

/// Check if value is a valid timestamp (YYYYMMDD\[HHMM\[SS\[.S\[S\[S\[S\]\]\]\]\]\] format)
pub fn is_timestamp(value: &str) -> bool {
    if value.len() < 8 {
        return false;
    }

    if !value.is_ascii() {
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
    value.parse::<f64>().is_ok_and(f64::is_finite)
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

/// Check if value is a person name (allows HL7 component separators)
pub fn is_person_name(value: &str) -> bool {
    value.chars().all(|c| {
        c.is_alphabetic() || c.is_whitespace() || c == '-' || c == '\'' || c == '.' || c == '^'
    })
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
    let cleaned: String = value.chars().filter(char::is_ascii_digit).collect();

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
    let cleaned: String = value.chars().filter(char::is_ascii_digit).collect();

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
        if let Some(regex) = regex_cache::get(pattern) {
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
    let digits: String = value.chars().filter(char::is_ascii_digit).collect();

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
            NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok()
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
        TimestampPrecision::Hour => dt
            .with_minute(0)
            .and_then(|d| d.with_second(0))
            .unwrap_or(*dt),
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
    crate::query::get(msg, path).and_then(|s| {
        let t = s.trim();
        if t.is_empty() { None } else { Some(t) }
    })
}

fn get_nonempty_values<'a>(msg: &'a Message, path: &str) -> Vec<&'a str> {
    let Ok(path) = crate::query::path::parse_located_path(path) else {
        return Vec::new();
    };

    if path.path.is_msh() && path.path.field == 1 {
        return get_nonempty(msg, &path.to_path_string())
            .into_iter()
            .collect();
    }

    let Some(segment) = condition_segment(msg, &path) else {
        return Vec::new();
    };
    let Some(field) = condition_field(segment, &path.path) else {
        return Vec::new();
    };

    condition_field_values(
        field,
        path.path.repetition,
        path.path.component,
        path.path.subcomponent,
    )
    .into_iter()
    .filter_map(trim_nonempty)
    .collect()
}

fn condition_segment<'a>(
    msg: &'a Message,
    path: &crate::query::path::LocatedPath,
) -> Option<&'a Segment> {
    let segment_repetition = path.segment_repetition.unwrap_or(1);
    if segment_repetition == 0 {
        return None;
    }

    msg.segments
        .iter()
        .filter(|segment| segment.id_str() == path.path.segment)
        .nth(segment_repetition - 1)
}

fn condition_field<'a>(segment: &'a Segment, path: &crate::query::path::Path) -> Option<&'a Field> {
    let field_index = if path.is_msh() {
        path.msh_stored_field_index()?
    } else {
        path.field.checked_sub(1)?
    };

    segment.fields.get(field_index)
}

fn condition_field_values(
    field: &Field,
    repetition: Option<usize>,
    component: Option<usize>,
    subcomponent: Option<usize>,
) -> Vec<&str> {
    if let Some(repetition) = repetition {
        return repetition
            .checked_sub(1)
            .and_then(|index| field.reps.get(index))
            .and_then(|rep| condition_rep_value(rep, component, subcomponent))
            .into_iter()
            .collect();
    }

    field
        .reps
        .iter()
        .filter_map(|rep| condition_rep_value(rep, component, subcomponent))
        .collect()
}

fn condition_rep_value(
    rep: &Rep,
    component: Option<usize>,
    subcomponent: Option<usize>,
) -> Option<&str> {
    let component_index = component.unwrap_or(1).checked_sub(1)?;
    let subcomponent_index = subcomponent.unwrap_or(1).checked_sub(1)?;
    let atom = rep
        .comps
        .get(component_index)?
        .subs
        .get(subcomponent_index)?;

    match atom {
        Atom::Text(text) => Some(text.as_str()),
        Atom::Null => None,
    }
}

fn trim_nonempty(value: &str) -> Option<&str> {
    let value = value.trim();
    if value.is_empty() { None } else { Some(value) }
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
    let lhs_values = get_nonempty_values(msg, &condition.field);

    // Right-hand value(s):
    let rhs_first = condition.value.as_deref();
    let rhs_list: Vec<&str> = condition.values.as_ref().map_or(Vec::new(), |v| {
        v.iter().map(std::string::String::as_str).collect()
    });

    match condition.operator.as_str() {
        // value/string ops
        "eq" => match rhs_first {
            Some(rhs) => {
                lhs_values.iter().any(|lhs| lhs == &rhs)
                    || (lhs_values.is_empty() && rhs.is_empty())
            }
            None => lhs_values.is_empty(),
        },
        "ne" => match rhs_first {
            Some(rhs) => {
                lhs_values.iter().any(|lhs| lhs != &rhs)
                    || (lhs_values.is_empty() && !rhs.is_empty())
            }
            None => !lhs_values.is_empty(),
        },
        "contains" => {
            let needle = rhs_first.unwrap_or_default();
            lhs_values.iter().any(|lhs| lhs.contains(needle))
        }
        "in" => lhs_values.iter().any(|lhs| rhs_list.contains(lhs)),
        "matches_regex" => {
            if let Some(pat) = rhs_first {
                regex_cache::get(pat)
                    .map(|re| lhs_values.iter().any(|lhs| re.is_match(lhs)))
                    .unwrap_or(false)
            } else {
                false
            }
        }

        // numeric comparisons: true if any left-hand value satisfies the
        // relation against the right-hand value. Non-numeric operands compare
        // false rather than failing open.
        "gt" | "lt" | "ge" | "le" => match rhs_first {
            Some(rhs) => lhs_values.iter().any(|lhs| {
                validate_mathematical_relationship(lhs, rhs, condition.operator.as_str())
            }),
            None => false,
        },

        // existence
        "exists" => !lhs_values.is_empty(),
        "not_exists" | "missing" => lhs_values.is_empty(),

        // temporal: accepts HL7 TS or YYYYMMDD
        "is_date" => lhs_values
            .iter()
            .any(|lhs| parse_hl7_ts_with_precision(lhs).is_some()),
        "before" => check_before_condition(msg, &lhs_values, rhs_first),
        // numeric range over integers OR date range over TS
        "within_range" => {
            if rhs_list.len() != 2 {
                return false;
            }
            let a = rhs_list[0];
            let b = rhs_list[1];
            // Try dates first
            if let (Some(lo), Some(hi)) = (parse_hl7_ts(a), parse_hl7_ts(b))
                && lhs_values
                    .iter()
                    .filter_map(|lhs| parse_hl7_ts(lhs))
                    .any(|lhs| lhs >= lo && lhs <= hi)
            {
                return true;
            }
            // Fallback to integer range
            if let (Ok(lo), Ok(hi)) = (a.parse::<i64>(), b.parse::<i64>())
                && lhs_values
                    .iter()
                    .filter_map(|lhs| lhs.parse::<i64>().ok())
                    .any(|lhs| lhs >= lo && lhs <= hi)
            {
                return true;
            }
            false
        }
        _ => {
            // Unknown operator, ignore
            false
        }
    }
}

fn check_before_condition(msg: &Message, lhs_values: &[&str], rhs_first: Option<&str>) -> bool {
    let Some(rhs) = rhs_first else {
        return false;
    };
    let rhs_values = get_nonempty_values(msg, rhs);

    if rhs_values.is_empty() {
        return lhs_values
            .iter()
            .any(|lhs| is_before_condition_value(lhs, rhs));
    }

    if rhs_values.len() == 1 {
        return lhs_values
            .iter()
            .any(|lhs| is_before_condition_value(lhs, rhs_values[0]));
    }

    lhs_values
        .iter()
        .zip(rhs_values)
        .any(|(lhs, rhs)| is_before_condition_value(lhs, rhs))
}

fn is_before_condition_value(lhs: &str, rhs: &str) -> bool {
    let Some(lhs_ts) = parse_hl7_ts_with_precision(lhs) else {
        return false;
    };
    let Some(rhs_ts) = parse_hl7_ts_with_precision(rhs) else {
        return false;
    };

    compare_timestamps_for_before(&lhs_ts, &rhs_ts)
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
        assert!(!is_date("2023010")); // Too short
        assert!(!is_date("202301011")); // Too long
    }

    #[test]
    fn test_is_time() {
        assert!(is_time("1200"));
        assert!(is_time("235959"));
        assert!(is_time("0000"));
        assert!(!is_time("2400")); // Invalid hour
        assert!(!is_time("1260")); // Invalid minute
        assert!(!is_time("123")); // Too short
    }

    #[test]
    fn test_is_time_uses_canonical_tm_grammar() {
        assert!(!is_time("12.5"));
        assert!(!is_time("1.30"));
        assert!(!is_time("1230.."));
        assert!(!is_time("1230."));
        assert!(!is_time("123045.."));
        assert!(!is_time("12305"));
        assert!(!is_time(" 1230"));
        assert!(!is_time("1230 "));

        assert!(is_time("1230"));
        assert!(is_time("235959"));
        assert!(is_time("123045.5"));
        assert!(is_time("123045.1234"));

        assert!(!is_timestamp("2025010112.5"));
        assert!(!is_timestamp("2025010112305"));
        assert!(is_timestamp("20250101123045.5"));
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
        assert!(!is_ssn("000123456")); // Invalid area
        assert!(!is_ssn("666123456")); // Invalid area
        assert!(!is_ssn("123450000")); // Invalid serial
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
        let issue = Issue::error(
            "TEST_CODE",
            Some("PID.5".to_string()),
            "Test detail".to_string(),
        );
        assert_eq!(issue.code, "TEST_CODE");
        assert_eq!(issue.severity, Severity::Error);
        assert_eq!(issue.path, Some("PID.5".to_string()));
    }

    #[test]
    fn validation_report_normalizes_issue_contract() {
        let message = crate::parser::parse(
            b"MSH|^~\\&|SENDAPP|SENDFAC|RECVAPP|RECVFAC|202605030101||ADT^A01|CTRL123|P|2.5\rPID|1||\r",
        )
        .unwrap_or_default();
        let report = ValidationReport::from_issues(
            &message,
            Some("adt_a01.yaml".to_string()),
            vec![Issue::error(
                "MISSING_REQUIRED_FIELD",
                Some("PID.3".to_string()),
                "PID.3 is required".to_string(),
            )],
        );

        assert!(!report.valid);
        assert_eq!(report.message_type, "ADT^A01");
        assert_eq!(report.profile.as_deref(), Some("adt_a01.yaml"));
        assert_eq!(report.issue_count, 1);
        assert_eq!(report.issues[0].code, "missing_required_field");
        assert_eq!(
            report.issues[0].rule_id.as_deref(),
            Some("missing_required_field")
        );
        assert_eq!(report.issues[0].severity, ValidationReportSeverity::Error);
        assert_eq!(report.issues[0].path.as_deref(), Some("PID.3"));
        assert_eq!(report.issues[0].segment_index, Some(1));
        assert_eq!(report.issues[0].field_index, Some(3));
    }

    #[test]
    fn validation_report_infers_indices_from_diagnostic_paths() {
        let message = crate::parser::parse(
            b"MSH|^~\\&|SENDAPP|SENDFAC|RECVAPP|RECVFAC|202605030101||ORU^R01|CTRL123|P|2.5\r\
PID|1||123456^^^HOSP^MR~987654^^^ALT^MR||Doe^John\r\
OBX|1|ST|CODE1^First||one\r\
OBX|2|ST|CODE2^Second||two\r\
OBX|3|ST|CODE3^Third||\r",
        )
        .unwrap_or_default();
        let report = ValidationReport::from_issues(
            &message,
            Some("oru_r01.yaml".to_string()),
            vec![
                Issue::warning(
                    "ASSIGNING_AUTHORITY_REVIEW",
                    Some("PID-3[2].4".to_string()),
                    "PID-3 second repetition assigning authority should be reviewed".to_string(),
                ),
                Issue::error(
                    "MISSING_OBSERVATION_VALUE",
                    Some("OBX[3]-5".to_string()),
                    "Third OBX observation value is required".to_string(),
                ),
                Issue::error(
                    "SEGMENT_REPETITION_NOT_ALLOWED",
                    Some("OBX[3]".to_string()),
                    "Third OBX segment is not allowed by the profile".to_string(),
                ),
            ],
        );

        assert_eq!(report.issues[0].path.as_deref(), Some("PID-3[2].4"));
        assert_eq!(report.issues[0].segment_index, Some(1));
        assert_eq!(report.issues[0].field_index, Some(3));
        assert_eq!(report.issues[1].path.as_deref(), Some("OBX[3]-5"));
        assert_eq!(report.issues[1].segment_index, Some(4));
        assert_eq!(report.issues[1].field_index, Some(5));
        assert_eq!(report.issues[2].path.as_deref(), Some("OBX[3]"));
        assert_eq!(report.issues[2].segment_index, Some(4));
        assert_eq!(report.issues[2].field_index, None);
    }

    #[test]
    fn validation_report_v2_embeds_provenance_without_changing_v1() {
        let message = crate::parser::parse(
            b"MSH|^~\\&|SENDAPP|SENDFAC|RECVAPP|RECVFAC|202605030101||ADT^A01|CTRL123|P|2.5\rPID|1||\r",
        )
        .unwrap_or_default();
        let report = ValidationReport::from_issues(
            &message,
            Some("adt_a01.yaml".to_string()),
            vec![Issue::error(
                "MISSING_REQUIRED_FIELD",
                Some("PID.3".to_string()),
                "PID.3 is required".to_string(),
            )],
        );
        let v1_json = serde_json::to_value(&report).unwrap_or_default();
        let v2 = report.to_v2(
            "hl7v2",
            "1.5.0",
            Some(ValidationReportProfileIdentity {
                label: "adt_a01.yaml".to_string(),
                message_structure: Some("ADT_A01".to_string()),
                version: Some("2.5.1".to_string()),
                sha256: Some(
                    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
                ),
            }),
        );
        let v2_json = serde_json::to_value(&v2).unwrap_or_default();

        assert!(v1_json.get("schema_version").is_none());
        assert_eq!(v2.schema_version, "2");
        assert_eq!(v2.tool_name, "hl7v2");
        assert_eq!(v2.tool_version, "1.5.0");
        assert_eq!(v2_json["profile_identity"]["message_structure"], "ADT_A01");
        assert_eq!(v2.issue_count, report.issue_count);
        assert_eq!(v2.issues, report.issues);
    }

    #[test]
    fn validation_report_severity_serializes_lowercase() {
        let serialized =
            serde_json::to_string(&ValidationReportSeverity::Warning).unwrap_or_default();

        assert_eq!(serialized, "\"warning\"");
    }
}
