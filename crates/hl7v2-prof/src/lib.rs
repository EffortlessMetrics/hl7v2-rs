//! Profile validation for HL7 v2 messages.
//!
//! This crate provides functionality for loading and applying
//! conformance profiles to HL7 v2 messages.

use hl7v2_core::{Message, Error};
use serde::{Deserialize, Serialize};

/// A conformance profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub message_structure: String,
    pub version: String,
    #[serde(default)]
    pub message_type: Option<String>,
    pub segments: Vec<SegmentSpec>,
    #[serde(default)]
    pub constraints: Vec<Constraint>,
    #[serde(default)]
    pub lengths: Vec<LengthConstraint>,
    #[serde(default)]
    pub valuesets: Vec<ValueSet>,
    #[serde(default)]
    pub datatypes: Vec<DataTypeConstraint>,
}

/// Specification for a segment in a profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentSpec {
    pub id: String,
}

/// Constraint on a field path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    pub path: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub components: Option<ComponentConstraint>,
    #[serde(default)]
    pub r#in: Option<Vec<String>>,
    #[serde(default)]
    pub when: Option<Condition>,
    #[serde(default)]
    pub pattern: Option<String>,
}

/// Component constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentConstraint {
    pub min: Option<usize>,
    pub max: Option<usize>,
}

/// Conditional constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    #[serde(default)]
    pub eq: Option<Vec<String>>,
    #[serde(default)]
    pub any: Option<Vec<Condition>>,
}

/// Length constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LengthConstraint {
    pub path: String,
    pub max: Option<usize>,
    pub policy: Option<String>, // "no-truncate" or "may-truncate"
}

/// Value set constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueSet {
    pub path: String,
    pub name: String,
    pub codes: Vec<String>,
}

/// Data type constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataTypeConstraint {
    pub path: String,
    pub r#type: String, // HL7 data type like "ST", "ID", "DT", etc.
}

/// Severity of validation issues
#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Error,
    Warning,
}

/// Validation issue
#[derive(Debug, Clone, PartialEq)]
pub struct Issue {
    pub code: &'static str,
    pub severity: Severity,
    pub path: Option<String>,
    pub detail: String,
}

/// Load profile from YAML
pub fn load_profile(yaml: &str) -> Result<Profile, Error> {
    serde_yaml::from_str(yaml).map_err(|_e| Error::InvalidEscapeToken) // TODO: Better error mapping
}

/// Validate message against profile
pub fn validate(msg: &Message, profile: &Profile) -> Vec<Issue> {
    let mut issues = Vec::new();
    
    // Validate constraints (including conditional ones)
    for constraint in &profile.constraints {
        if should_validate_constraint(msg, constraint) {
            if constraint.required {
                if let Some(path) = &constraint.path.strip_prefix("MSH.") {
                    // Special handling for MSH segment
                    validate_msh_field_required(msg, path, &mut issues);
                } else {
                    validate_field_required(msg, &constraint.path, &mut issues);
                }
            }
        }
    }
    
    // Validate value sets
    for valueset in &profile.valuesets {
        validate_value_set(msg, valueset, &mut issues);
    }
    
    // Validate data types
    for datatype in &profile.datatypes {
        validate_data_type(msg, datatype, &mut issues);
    }
    
    // Validate length constraints
    for length in &profile.lengths {
        validate_length_constraint(msg, length, &mut issues);
    }
    
    issues
}

/// Validate that a required field is present
fn validate_field_required(msg: &Message, path: &str, issues: &mut Vec<Issue>) {
    // Use the get function from hl7v2-core to retrieve the value
    if let Some(value) = hl7v2_core::get(msg, path) {
        // If we get a value, check if it's empty
        if value.is_empty() {
            issues.push(Issue {
                code: "MISSING_REQUIRED_FIELD",
                severity: Severity::Error,
                path: Some(path.to_string()),
                detail: format!("Required field {} is missing", path),
            });
        }
    } else {
        // If we get None, the field is truly missing
        issues.push(Issue {
            code: "MISSING_REQUIRED_FIELD",
            severity: Severity::Error,
            path: Some(path.to_string()),
            detail: format!("Required field {} is missing", path),
        });
    }
}

/// Determine if a constraint should be validated based on its conditions
fn should_validate_constraint(msg: &Message, constraint: &Constraint) -> bool {
    // If there's no condition, always validate
    let condition = match &constraint.when {
        Some(cond) => cond,
        None => return true,
    };
    
    // Check if any condition is met
    check_condition(msg, condition)
}

/// Check if a condition is met
fn check_condition(msg: &Message, condition: &Condition) -> bool {
    // Check equality conditions
    if let Some(eq_conditions) = &condition.eq {
        if eq_conditions.len() == 2 {
            let field_path = &eq_conditions[0];
            let expected_value = &eq_conditions[1];
            
            if let Some(actual_value) = hl7v2_core::get(msg, field_path) {
                return actual_value == expected_value;
            }
            return false;
        }
    }
    
    // Check any conditions (OR logic)
    if let Some(any_conditions) = &condition.any {
        for cond in any_conditions {
            if check_condition(msg, cond) {
                return true;
            }
        }
        return false;
    }
    
    // If no conditions match, don't validate
    false
}

/// Validate that a required MSH field is present
fn validate_msh_field_required(msg: &Message, path: &str, issues: &mut Vec<Issue>) {
    let full_path = format!("MSH.{}", path);
    // Use the get function from hl7v2-core to retrieve the value
    if hl7v2_core::get(msg, &full_path).is_none() {
        issues.push(Issue {
            code: "MISSING_REQUIRED_FIELD",
            severity: Severity::Error,
            path: Some(full_path),
            detail: format!("Required MSH field {} is missing", path),
        });
    }
}

/// Validate that a field value is in the allowed value set
fn validate_value_set(msg: &Message, valueset: &ValueSet, issues: &mut Vec<Issue>) {
    if let Some(value) = hl7v2_core::get(msg, &valueset.path) {
        if !valueset.codes.contains(&value.to_string()) {
            issues.push(Issue {
                code: "VALUE_NOT_IN_SET",
                severity: Severity::Error,
                path: Some(valueset.path.clone()),
                detail: format!("Value '{}' for {} is not in allowed set: {:?}", 
                               value, valueset.path, valueset.codes),
            });
        }
    }
    // Note: We don't report an error if the field is missing but has a value set constraint
    // That would be handled by a separate presence constraint if needed
}

/// Validate that a field value matches the expected data type
fn validate_data_type(msg: &Message, datatype: &DataTypeConstraint, issues: &mut Vec<Issue>) {
    if let Some(value) = hl7v2_core::get(msg, &datatype.path) {
        if !matches_data_type(value, &datatype.r#type) {
            issues.push(Issue {
                code: "INVALID_DATA_TYPE",
                severity: Severity::Error,
                path: Some(datatype.path.clone()),
                detail: format!("Value '{}' for {} does not match expected data type {}", 
                               value, datatype.path, datatype.r#type),
            });
        }
    }
    // Note: We don't report an error if the field is missing but has a data type constraint
    // That would be handled by a separate presence constraint if needed
}

/// Validate that a field value does not exceed the maximum length
fn validate_length_constraint(msg: &Message, length: &LengthConstraint, issues: &mut Vec<Issue>) {
    if let Some(value) = hl7v2_core::get(msg, &length.path) {
        if let Some(max_length) = length.max {
            if value.len() > max_length {
                issues.push(Issue {
                    code: "VALUE_TOO_LONG",
                    severity: Severity::Error,
                    path: Some(length.path.clone()),
                    detail: format!("Value '{}' for {} exceeds maximum length of {} characters", 
                                   value, length.path, max_length),
                });
            }
        }
    }
    // Note: We don't report an error if the field is missing but has a length constraint
    // That would be handled by a separate presence constraint if needed
}

/// Check if a value matches the expected HL7 data type
fn matches_data_type(value: &str, datatype: &str) -> bool {
    match datatype {
        "ST" => is_string(value), // String Data
        "ID" => is_identifier(value), // Coded values for HL7 tables
        "DT" => is_date(value), // Date
        "TM" => is_time(value), // Time
        "TS" => is_timestamp(value), // Time Stamp
        "NM" => is_numeric(value), // Numeric
        "SI" => is_sequence_id(value), // Sequence ID
        "TX" => is_text_data(value), // Text Data
        "FT" => is_formatted_text(value), // Formatted Text Data
        "IS" => is_coded_value(value), // Coded value for user-defined tables
        "PN" => is_person_name(value), // Person name
        "CX" => is_extended_id(value), // Extended composite ID with check digit
        "HD" => is_hierarchic_designator(value), // Hierarchic designator
        _ => true, // Unknown data type, assume valid
    }
}

/// Check if value is a valid string (always true for parsed values)
fn is_string(_value: &str) -> bool {
    true
}

/// Check if value is a valid identifier (alphanumeric + special characters)
fn is_identifier(value: &str) -> bool {
    // HL7 identifiers can contain alphanumeric characters and some special characters
    // For simplicity, we'll check if it contains only printable ASCII characters
    value.chars().all(|c| c.is_ascii() && !c.is_control())
}

/// Check if value is a valid date (YYYYMMDD format)
fn is_date(value: &str) -> bool {
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
    if month < "01" || month > "12" {
        return false;
    }
    
    if day < "01" || day > "31" {
        return false;
    }
    
    true
}

/// Check if value is a valid time (HHMM[SS[.S[S[S[S]]]]] format)
fn is_time(value: &str) -> bool {
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
fn is_timestamp(value: &str) -> bool {
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
fn is_numeric(value: &str) -> bool {
    // Can be integer or decimal
    value.parse::<f64>().is_ok()
}

/// Check if value is a sequence ID (positive integer)
fn is_sequence_id(value: &str) -> bool {
    match value.parse::<u32>() {
        Ok(num) => num > 0,
        Err(_) => false,
    }
}

/// Check if value is text data (always true for parsed values)
fn is_text_data(_value: &str) -> bool {
    true
}

/// Check if value is formatted text (always true for parsed values)
fn is_formatted_text(_value: &str) -> bool {
    true
}

/// Check if value is a coded value (alphanumeric + special characters)
fn is_coded_value(value: &str) -> bool {
    // Similar to identifier
    value.chars().all(|c| c.is_ascii() && !c.is_control())
}

/// Check if value is a person name (contains letters, spaces, hyphens, apostrophes)
fn is_person_name(value: &str) -> bool {
    value.chars().all(|c| c.is_alphabetic() || c.is_whitespace() || c == '-' || c == '\'' || c == '.')
}

/// Check if value is an extended ID (contains identifier characters)
fn is_extended_id(value: &str) -> bool {
    is_identifier(value)
}

/// Check if value is a hierarchic designator (contains identifier characters)
fn is_hierarchic_designator(value: &str) -> bool {
    is_identifier(value)
}

#[cfg(test)]
mod tests;