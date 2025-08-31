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
    serde_yaml::from_str(yaml).map_err(|_| Error::InvalidEscapeToken) // TODO: Better error mapping
}

/// Validate message against profile
pub fn validate(msg: &Message, profile: &Profile) -> Vec<Issue> {
    // Implementation will be added later
    vec![]
}