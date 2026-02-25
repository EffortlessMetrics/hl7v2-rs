//! HL7 v2 data type validation.
//!
//! This crate provides validation functions for HL7 v2 data types,
//! including primitive types (ST, ID, DT, TM, TS, NM, etc.) and
//! commonly used validation patterns.
//!
//! # Supported Data Types
//!
//! - `ST` - String Data
//! - `ID` - Coded values for HL7 tables
//! - `IS` - Coded value for user-defined tables
//! - `DT` - Date
//! - `TM` - Time
//! - `TS` - Timestamp
//! - `NM` - Numeric
//! - `SI` - Sequence ID
//! - `TX` - Text Data
//! - `FT` - Formatted Text Data
//! - `PN` - Person Name
//! - `CX` - Extended Composite ID
//! - `HD` - Hierarchic Designator
//!
//! # Example
//!
//! ```
//! use hl7v2_datatype::{validate_datatype, DataType, DataTypeValidator};
//!
//! // Validate a date
//! assert!(validate_datatype("20250128", "DT"));
//! assert!(!validate_datatype("20251328", "DT")); // Invalid month
//!
//! // Validate a person name
//! assert!(validate_datatype("Smith^John", "PN"));
//!
//! // Use the validator builder
//! let validator = DataTypeValidator::new()
//!     .with_min_length(1)
//!     .with_max_length(50);
//! assert!(validator.validate("Test Value"));
//! ```

use regex::Regex;
pub use hl7v2_datetime;

/// Error type for data type validation
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum DataTypeError {
    #[error("Invalid data type '{datatype}': {reason}")]
    InvalidDataType {
        datatype: String,
        reason: String,
    },
    
    #[error("Value too short: {length} < {min}")]
    TooShort { length: usize, min: usize },
    
    #[error("Value too long: {length} > {max}")]
    TooLong { length: usize, max: usize },
    
    #[error("Pattern mismatch: value '{value}' does not match pattern '{pattern}'")]
    PatternMismatch { value: String, pattern: String },
    
    #[error("Value not in allowed set: {value}")]
    NotInAllowedSet { value: String },
    
    #[error("Checksum validation failed")]
    ChecksumFailed,
}

/// Result type for data type validation
pub type ValidationResult = Result<(), DataTypeError>;

/// HL7 data types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataType {
    /// String Data
    ST,
    /// Coded values for HL7 tables
    ID,
    /// Coded value for user-defined tables
    IS,
    /// Date
    DT,
    /// Time
    TM,
    /// Timestamp
    TS,
    /// Numeric
    NM,
    /// Sequence ID
    SI,
    /// Text Data
    TX,
    /// Formatted Text Data
    FT,
    /// Person Name
    PN,
    /// Extended Composite ID
    CX,
    /// Hierarchic Designator
    HD,
    /// Address
    AD,
    /// Phone Number
    XTN,
}

impl DataType {
    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "ST" => Some(Self::ST),
            "ID" => Some(Self::ID),
            "IS" => Some(Self::IS),
            "DT" => Some(Self::DT),
            "TM" => Some(Self::TM),
            "TS" => Some(Self::TS),
            "NM" => Some(Self::NM),
            "SI" => Some(Self::SI),
            "TX" => Some(Self::TX),
            "FT" => Some(Self::FT),
            "PN" => Some(Self::PN),
            "CX" => Some(Self::CX),
            "HD" => Some(Self::HD),
            "AD" => Some(Self::AD),
            "XTN" => Some(Self::XTN),
            _ => None,
        }
    }
}

/// Validator for data types with configurable constraints
#[derive(Debug, Clone, Default)]
pub struct DataTypeValidator {
    /// Minimum length constraint
    pub min_length: Option<usize>,
    /// Maximum length constraint
    pub max_length: Option<usize>,
    /// Regex pattern constraint
    pub pattern: Option<String>,
    /// Allowed values constraint
    pub allowed_values: Option<Vec<String>>,
    /// Checksum algorithm
    pub checksum: Option<ChecksumAlgorithm>,
}

/// Checksum algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChecksumAlgorithm {
    /// Luhn algorithm (for credit cards, etc.)
    Luhn,
    /// Mod 10
    Mod10,
}

impl DataTypeValidator {
    /// Create a new validator
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set minimum length
    pub fn with_min_length(mut self, min: usize) -> Self {
        self.min_length = Some(min);
        self
    }
    
    /// Set maximum length
    pub fn with_max_length(mut self, max: usize) -> Self {
        self.max_length = Some(max);
        self
    }
    
    /// Set regex pattern
    pub fn with_pattern(mut self, pattern: &str) -> Self {
        self.pattern = Some(pattern.to_string());
        self
    }
    
    /// Set allowed values
    pub fn with_allowed_values(mut self, values: Vec<String>) -> Self {
        self.allowed_values = Some(values);
        self
    }
    
    /// Set checksum algorithm
    pub fn with_checksum(mut self, algorithm: ChecksumAlgorithm) -> Self {
        self.checksum = Some(algorithm);
        self
    }
    
    /// Validate a value
    pub fn validate(&self, value: &str) -> bool {
        self.validate_detailed(value).is_ok()
    }
    
    /// Validate a value with detailed error information
    pub fn validate_detailed(&self, value: &str) -> ValidationResult {
        // Check minimum length
        if let Some(min) = self.min_length {
            if value.len() < min {
                return Err(DataTypeError::TooShort {
                    length: value.len(),
                    min,
                });
            }
        }
        
        // Check maximum length
        if let Some(max) = self.max_length {
            if value.len() > max {
                return Err(DataTypeError::TooLong {
                    length: value.len(),
                    max,
                });
            }
        }
        
        // Check pattern
        if let Some(pattern) = &self.pattern {
            if let Ok(regex) = Regex::new(pattern) {
                if !regex.is_match(value) {
                    return Err(DataTypeError::PatternMismatch {
                        value: value.to_string(),
                        pattern: pattern.clone(),
                    });
                }
            }
        }
        
        // Check allowed values
        if let Some(allowed) = &self.allowed_values {
            if !allowed.contains(&value.to_string()) {
                return Err(DataTypeError::NotInAllowedSet {
                    value: value.to_string(),
                });
            }
        }
        
        // Check checksum
        if let Some(algorithm) = self.checksum {
            match algorithm {
                ChecksumAlgorithm::Luhn | ChecksumAlgorithm::Mod10 => {
                    if !validate_luhn_checksum(value) {
                        return Err(DataTypeError::ChecksumFailed);
                    }
                }
            }
        }
        
        Ok(())
    }
}

/// Validate a value against an HL7 data type
pub fn validate_datatype(value: &str, datatype: &str) -> bool {
    match datatype {
        "ST" => is_string(value),
        "ID" => is_identifier(value),
        "IS" => is_coded_value(value),
        "DT" => is_date(value),
        "TM" => is_time(value),
        "TS" => is_timestamp(value),
        "NM" => is_numeric(value),
        "SI" => is_sequence_id(value),
        "TX" => is_text_data(value),
        "FT" => is_formatted_text(value),
        "PN" => is_person_name(value),
        "CX" => is_extended_id(value),
        "HD" => is_hierarchic_designator(value),
        "AD" => is_address(value),
        "XTN" => is_phone_number(value),
        _ => true, // Unknown data type, assume valid
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

/// Check if value is a valid coded value (alphanumeric + special characters)
pub fn is_coded_value(value: &str) -> bool {
    // Similar to identifier
    is_identifier(value)
}

/// Check if value is a valid date (YYYYMMDD format)
pub fn is_date(value: &str) -> bool {
    hl7v2_datetime::is_valid_hl7_date(value)
}

/// Check if value is a valid time (HHMM[SS[.S[S[S[S]]]]] format)
pub fn is_time(value: &str) -> bool {
    hl7v2_datetime::is_valid_hl7_time(value)
}

/// Check if value is a valid timestamp (YYYYMMDD[HHMM[SS[.S[S[S[S]]]]]] format)
pub fn is_timestamp(value: &str) -> bool {
    hl7v2_datetime::is_valid_hl7_timestamp(value)
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

/// Check if value is a person name (contains letters, spaces, hyphens, apostrophes)
pub fn is_person_name(value: &str) -> bool {
    value
        .chars()
        .all(|c| c.is_alphabetic() || c.is_whitespace() || c == '-' || c == '\'' || c == '.' || c == '^')
}

/// Check if value is an extended ID (contains identifier characters)
pub fn is_extended_id(value: &str) -> bool {
    is_identifier(value)
}

/// Check if value is a hierarchic designator (contains identifier characters)
pub fn is_hierarchic_designator(value: &str) -> bool {
    is_identifier(value)
}

/// Check if value is a valid address
pub fn is_address(value: &str) -> bool {
    // Address can contain most printable characters
    value.chars().all(|c| c.is_ascii() && !c.is_control())
}

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
    validate_luhn_checksum(value)
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

/// Validate format specification
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
            if month < 1 || month > 12 {
                return false;
            }
            // Check day (2 digits)
            if parts[2].len() != 2 || !parts[2].chars().all(|c| c.is_ascii_digit()) {
                return false;
            }
            let day: u32 = parts[2].parse().unwrap_or(0);
            if day < 1 || day > 31 {
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

#[cfg(test)]
mod tests;