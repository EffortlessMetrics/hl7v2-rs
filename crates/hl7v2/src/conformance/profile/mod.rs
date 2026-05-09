//! Profile validation for HL7 v2 messages.
//!
//! This module provides functionality for loading and applying
//! conformance profiles to HL7 v2 messages. It builds on
//! `hl7v2::conformance::validation` for core validation logic.

#![expect(
    clippy::collapsible_if,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::manual_let_else,
    clippy::map_err_ignore,
    clippy::missing_errors_doc,
    clippy::string_slice,
    clippy::uninlined_format_args,
    reason = "Pre-existing profile implementation debt moved from hl7v2-prof; cleanup is separate from this behavior-preserving module collapse."
)]
//!
//! # Features
//!
//! - Profile loading from YAML
//! - Profile inheritance and merging
//! - Profile-based message validation
//! - Cross-field validation rules
//! - Temporal validation rules
//! - Contextual validation rules
//!
//! # Example
//!
//! ```ignore
//! use hl7v2::{load_profile, validate, Profile};
//!
//! let yaml = r#"
//! message_structure: ADT_A01
//! version: "2.5.1"
//! segments:
//!   - id: MSH
//! constraints:
//!   - path: MSH.9
//!     required: true
//! "#;
//!
//! let profile = load_profile(yaml)?;
//! let issues = validate(&message, &profile);
//! ```

// Re-export validation types for compatibility with the old profile facade.
pub use super::validation::{
    Issue, ParsedTimestamp, RuleAction, RuleCondition, Severity, TimestampPrecision,
    ValidationResult, Validator, check_rule_condition, compare_timestamps_for_before, get_nonempty,
    is_coded_value, is_date, is_email, is_extended_id, is_formatted_text, is_hierarchic_designator,
    is_identifier, is_numeric, is_person_name, is_phone_number, is_sequence_id, is_ssn, is_string,
    is_text_data, is_time, is_timestamp, is_valid_age_range, is_valid_birth_date, is_within_range,
    matches_complex_pattern, matches_format, parse_datetime, parse_hl7_ts,
    parse_hl7_ts_with_precision, truncate_to_precision, validate_checksum, validate_data_type,
    validate_luhn_checksum, validate_mathematical_relationship, validate_mod10_checksum,
};

use crate::model::{Error, Message};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Profile loading error types.
///
/// These errors provide detailed information about profile loading failures,
/// making it easier to diagnose configuration and parsing issues.
#[derive(Debug, thiserror::Error)]
pub enum ProfileLoadError {
    /// YAML syntax error during parsing.
    #[error("YAML parse error: {0}")]
    YamlParse(String),

    /// Required field is missing from the profile.
    #[error("Missing required field: {field}")]
    MissingField {
        /// The name of the missing field.
        field: String,
    },

    /// Invalid field value in the profile.
    #[error("Invalid value for field '{field}': {details}")]
    InvalidValue {
        /// The name of the field with an invalid value.
        field: String,
        /// Details about why the value is invalid.
        details: String,
    },

    /// IO error during profile file reading.
    #[error("IO error: {0}")]
    Io(String),

    /// Profile inheritance cycle detected.
    #[error("Profile inheritance cycle detected: {0}")]
    InheritanceCycle(String),

    /// Parent profile not found.
    #[error("Parent profile not found: {0}")]
    ParentNotFound(String),

    /// Network error during remote profile loading.
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// Profile not found in cache or local filesystem.
    #[error("Profile not found: {0}")]
    NotFound(String),

    /// Invalid URL scheme for remote loading.
    #[error("Invalid URL scheme: {0}. Only http and https are supported.")]
    InvalidScheme(String),

    /// Cache operation failed.
    #[error("Cache error: {0}")]
    Cache(String),

    /// Core HL7 library error.
    #[error("Core error: {0}")]
    Core(String),
}

impl From<serde_yaml::Error> for ProfileLoadError {
    fn from(err: serde_yaml::Error) -> Self {
        ProfileLoadError::YamlParse(err.to_string())
    }
}

impl From<std::io::Error> for ProfileLoadError {
    fn from(err: std::io::Error) -> Self {
        ProfileLoadError::Io(err.to_string())
    }
}

impl From<Error> for ProfileLoadError {
    fn from(err: Error) -> Self {
        ProfileLoadError::Core(err.to_string())
    }
}

/// A conformance profile
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Profile {
    /// HL7 message structure name (for example, `ADT_A01`).
    pub message_structure: String,
    /// HL7 version string (for example, `2.5.1`).
    pub version: String,
    /// Optional message type override.
    #[serde(default)]
    pub message_type: Option<String>,
    /// Reference to parent profile by name for profile inheritance.
    #[serde(default)]
    pub parent: Option<String>,
    /// Segment specifications for this profile.
    pub segments: Vec<SegmentSpec>,
    /// Field and primitive constraints.
    #[serde(default)]
    pub constraints: Vec<Constraint>,
    /// Field length constraints.
    #[serde(default)]
    pub lengths: Vec<LengthConstraint>,
    /// HL7 value sets referenced by the profile.
    #[serde(default)]
    pub valuesets: Vec<ValueSet>,
    /// Simple datatype constraints.
    #[serde(default)]
    pub datatypes: Vec<DataTypeConstraint>,
    /// Advanced datatype constraints for richer checks.
    #[serde(default)]
    pub advanced_datatypes: Vec<AdvancedDataTypeConstraint>,
    /// Cross-field validation rules.
    #[serde(default)]
    pub cross_field_rules: Vec<CrossFieldRule>,
    /// Temporal validation rules for date/time comparisons.
    #[serde(default)]
    pub temporal_rules: Vec<TemporalRule>,
    /// Contextual validation rules based on message context.
    #[serde(default)]
    pub contextual_rules: Vec<ContextualRule>,
    /// Custom extension validation rules.
    #[serde(default)]
    pub custom_rules: Vec<CustomRule>,
    /// HL7 table definitions used by value-set checks.
    #[serde(default)]
    pub hl7_tables: Vec<HL7Table>,
    /// Table precedence order - defines the order in which tables should be checked
    /// when multiple tables could apply to a field
    #[serde(default)]
    pub table_precedence: Vec<String>,
    /// Expression guardrails - rules that limit how expressions can be used in profiles
    #[serde(default)]
    pub expression_guardrails: ExpressionGuardrails,
}

/// Specification for a segment in a profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentSpec {
    /// Segment identifier (for example, `MSH`).
    pub id: String,
}

/// Constraint on a field path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    /// Path of the constrained field.
    pub path: String,
    /// Whether the field is mandatory.
    #[serde(default)]
    pub required: bool,
    /// Optional component-level constraint.
    #[serde(default)]
    pub components: Option<ComponentConstraint>,
    /// Allowed literal values.
    #[serde(default)]
    pub r#in: Option<Vec<String>>,
    /// Condition under which this constraint is active.
    #[serde(default)]
    pub when: Option<Condition>,
    /// Regex pattern that the field value must match.
    #[serde(default)]
    pub pattern: Option<String>,
}

/// Component constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentConstraint {
    /// Minimum number of values/components.
    pub min: Option<usize>,
    /// Maximum number of values/components.
    pub max: Option<usize>,
}

/// Conditional constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    /// Match exactly one of these values.
    #[serde(default)]
    pub eq: Option<Vec<String>>,
    /// Match when any nested condition is satisfied.
    #[serde(default)]
    pub any: Option<Vec<Condition>>,
}

/// Length constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LengthConstraint {
    /// Path to the constrained field.
    pub path: String,
    /// Optional maximum length.
    pub max: Option<usize>,
    /// Truncation policy (`no-truncate` or `may-truncate`).
    pub policy: Option<String>,
}

/// Value set constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueSet {
    /// Field path where this value set applies.
    pub path: String,
    /// Name of the value set.
    pub name: String,
    /// Codes can be defined inline OR reference an HL7 table by name
    #[serde(default)]
    pub codes: Vec<String>,
}

/// Data type constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataTypeConstraint {
    /// Field path constrained by datatype.
    pub path: String,
    /// HL7 datatype identifier (for example, `ST`, `ID`, `DT`).
    pub r#type: String,
}

/// Advanced data type constraint with complex validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedDataTypeConstraint {
    /// Field path constrained by advanced datatype rules.
    pub path: String,
    /// HL7 datatype identifier (for example, `ST`, `ID`, `DT`).
    pub r#type: String,
    /// Optional regex pattern to validate the field.
    #[serde(default)]
    pub pattern: Option<String>,
    /// Minimum length constraint.
    #[serde(default)]
    pub min_length: Option<usize>,
    /// Maximum length constraint.
    #[serde(default)]
    pub max_length: Option<usize>,
    /// Optional format hint (for example, `YYYY-MM-DD`).
    #[serde(default)]
    pub format: Option<String>,
    /// Optional checksum algorithm name.
    #[serde(default)]
    pub checksum: Option<String>,
}

/// Temporal validation rule for date/time relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalRule {
    /// Rule identifier.
    pub id: String,
    /// Human-readable description.
    pub description: String,
    /// Path expected to be earlier than `after`.
    pub before: String,
    /// Path expected to be later than `before`.
    pub after: String,
    /// Whether equal timestamps are allowed.
    #[serde(default)]
    pub allow_equal: bool,
    /// Optional tolerance for comparison.
    #[serde(default)]
    pub tolerance: Option<String>,
}

/// Contextual validation rule based on message context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextualRule {
    /// Rule identifier.
    pub id: String,
    /// Human-readable description.
    pub description: String,
    /// Field used to determine applicability.
    pub context_field: String,
    /// Required field value to activate this rule.
    pub context_value: String,
    /// Field validated when the context matches.
    pub target_field: String,
    /// Validation type to execute.
    pub validation_type: String,
    /// Parameters passed to the validator.
    #[serde(default)]
    pub parameters: std::collections::HashMap<String, String>,
}

/// HL7 Table definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HL7Table {
    /// HL7 table identifier (for example, `HL70001`).
    pub id: String,
    /// Table display name.
    pub name: String,
    /// Table version.
    pub version: String,
    /// Table values.
    pub codes: Vec<HL7TableEntry>,
}

/// Entry in an HL7 table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HL7TableEntry {
    /// Code value.
    pub value: String,
    /// Code description.
    pub description: String,
    /// Entry status (`A` active, `D` deprecated, `R` restricted).
    #[serde(default)]
    pub status: String,
}

/// Cross-field validation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossFieldRule {
    /// Rule identifier.
    pub id: String,
    /// Human-readable description.
    pub description: String,
    /// Validation mode: "conditional" (default) or "assert"
    /// - "conditional": If conditions are met, execute actions
    /// - "assert": Conditions must be true, fail otherwise
    #[serde(default = "default_validation_mode")]
    pub validation_mode: String,
    /// Conditions that gate this rule.
    pub conditions: Vec<RuleCondition>,
    /// Actions produced when conditions pass.
    pub actions: Vec<RuleAction>,
}

fn default_validation_mode() -> String {
    "conditional".to_string()
}

/// Custom validation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomRule {
    /// Rule identifier.
    pub id: String,
    /// Human-readable description.
    pub description: String,
    /// Rule script or reference to external logic.
    pub script: String,
}

/// Expression guardrails - rules that limit how expressions can be used in profiles
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ExpressionGuardrails {
    /// Maximum depth of nested expressions
    #[serde(default)]
    pub max_depth: Option<usize>,
    /// Maximum length of expression strings
    #[serde(default)]
    pub max_length: Option<usize>,
    /// Whether to allow custom scripts
    #[serde(default)]
    pub allow_custom_scripts: bool,
}

/// Result of linting a profile definition.
///
/// Profile linting checks the profile as configuration, before it is applied to
/// a message. It reports YAML/load failures plus structural issues that the
/// runtime validator would otherwise ignore or only reveal during validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileLintReport {
    /// Whether the profile has no lint errors.
    pub valid: bool,
    /// Number of errors in `issues`.
    pub error_count: usize,
    /// Number of warnings in `issues`.
    pub warning_count: usize,
    /// Total number of issues.
    pub issue_count: usize,
    /// Lint findings.
    pub issues: Vec<ProfileLintIssue>,
}

impl ProfileLintReport {
    fn from_issues(issues: Vec<ProfileLintIssue>) -> Self {
        let error_count = issues
            .iter()
            .filter(|issue| issue.severity == ProfileLintSeverity::Error)
            .count();
        let warning_count = issues
            .iter()
            .filter(|issue| issue.severity == ProfileLintSeverity::Warning)
            .count();
        Self {
            valid: error_count == 0,
            error_count,
            warning_count,
            issue_count: issues.len(),
            issues,
        }
    }

    /// Convert this v1 lint report into the explicit v2 evidence contract shape.
    ///
    /// This preserves the default serialized form of [`ProfileLintReport`].
    /// Producers opt into v2 when they are ready to emit embedded provenance.
    #[must_use]
    pub fn to_v2(
        &self,
        tool_name: impl Into<String>,
        tool_version: impl Into<String>,
    ) -> ProfileLintReportV2 {
        ProfileLintReportV2 {
            schema_version: "2".to_string(),
            tool_name: tool_name.into(),
            tool_version: tool_version.into(),
            report: self.clone(),
        }
    }
}

/// Profile lint report v2 with embedded evidence provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileLintReportV2 {
    /// Evidence artifact schema version.
    pub schema_version: String,
    /// Producer surface that generated this profile lint report.
    pub tool_name: String,
    /// Producer package version.
    pub tool_version: String,
    /// V1 profile lint report fields.
    #[serde(flatten)]
    pub report: ProfileLintReport,
}

/// A single profile lint finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileLintIssue {
    /// Stable lint code.
    pub code: String,
    /// Finding severity.
    pub severity: ProfileLintSeverity,
    /// Profile path or YAML location, when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Human-readable explanation.
    pub message: String,
}

impl ProfileLintIssue {
    fn error(code: &str, path: Option<String>, message: String) -> Self {
        Self {
            code: code.to_string(),
            severity: ProfileLintSeverity::Error,
            path,
            message,
        }
    }

    fn warning(code: &str, path: Option<String>, message: String) -> Self {
        Self {
            code: code.to_string(),
            severity: ProfileLintSeverity::Warning,
            path,
            message,
        }
    }
}

/// Profile lint finding severity.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProfileLintSeverity {
    /// Profile should not be used until fixed.
    Error,
    /// Profile loads, but contains surprising or ignored configuration.
    Warning,
}

impl ProfileLintSeverity {
    /// Return the lowercase severity string used by text output.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
        }
    }
}

/// Lint a profile YAML document.
///
/// This function is intentionally stricter than `load_profile_checked`: it
/// catches ignored keys, malformed paths, invalid regex patterns, unsupported
/// rule strings, and dangling value-set/table references.
pub fn lint_profile_yaml(yaml: &str) -> ProfileLintReport {
    let root = match serde_yaml::from_str::<serde_yaml::Value>(yaml) {
        Ok(root) => root,
        Err(err) => {
            return ProfileLintReport::from_issues(vec![ProfileLintIssue::error(
                "yaml_parse_error",
                None,
                err.to_string(),
            )]);
        }
    };

    let mut issues = Vec::new();
    lint_unknown_profile_keys(&root, &mut issues);

    let profile = match serde_yaml::from_value::<Profile>(root) {
        Ok(profile) => profile,
        Err(err) => {
            issues.push(ProfileLintIssue::error(
                "profile_load_error",
                None,
                err.to_string(),
            ));
            return ProfileLintReport::from_issues(issues);
        }
    };

    lint_profile(&profile, &mut issues);
    ProfileLintReport::from_issues(issues)
}

fn lint_unknown_profile_keys(root: &serde_yaml::Value, issues: &mut Vec<ProfileLintIssue>) {
    let Some(mapping) = root.as_mapping() else {
        return;
    };

    let known_keys = [
        "message_structure",
        "version",
        "message_type",
        "parent",
        "description",
        "segments",
        "constraints",
        "lengths",
        "valuesets",
        "datatypes",
        "advanced_datatypes",
        "cross_field_rules",
        "temporal_rules",
        "contextual_rules",
        "custom_rules",
        "hl7_tables",
        "table_precedence",
        "expression_guardrails",
    ];

    for key in mapping.keys().filter_map(serde_yaml::Value::as_str) {
        if !known_keys.contains(&key) {
            issues.push(ProfileLintIssue::warning(
                "unknown_top_level_key",
                Some(key.to_string()),
                format!("top-level key '{key}' is ignored by the profile loader"),
            ));
        }
    }
}

fn lint_profile(profile: &Profile, issues: &mut Vec<ProfileLintIssue>) {
    lint_profile_identity(profile, issues);
    lint_segments(profile, issues);
    lint_constraints(profile, issues);
    lint_lengths(profile, issues);
    lint_value_sets(profile, issues);
    lint_datatypes(profile, issues);
    lint_rules(profile, issues);
    lint_tables(profile, issues);
    lint_custom_rules(profile, issues);
}

fn lint_profile_identity(profile: &Profile, issues: &mut Vec<ProfileLintIssue>) {
    if profile.message_structure.trim().is_empty() {
        issues.push(ProfileLintIssue::error(
            "empty_message_structure",
            Some("message_structure".to_string()),
            "message_structure must not be empty".to_string(),
        ));
    }

    if profile.version.trim().is_empty() {
        issues.push(ProfileLintIssue::error(
            "empty_version",
            Some("version".to_string()),
            "version must not be empty".to_string(),
        ));
    }
}

fn lint_segments(profile: &Profile, issues: &mut Vec<ProfileLintIssue>) {
    let mut seen = HashSet::new();
    for (index, segment) in profile.segments.iter().enumerate() {
        let path = format!("segments[{index}].id");
        if segment.id.trim().is_empty() {
            issues.push(ProfileLintIssue::error(
                "empty_segment_id",
                Some(path),
                "segment id must not be empty".to_string(),
            ));
        } else if !seen.insert(segment.id.as_str()) {
            issues.push(ProfileLintIssue::warning(
                "duplicate_segment_id",
                Some(path),
                format!("segment '{}' is listed more than once", segment.id),
            ));
        }
    }
}

fn lint_constraints(profile: &Profile, issues: &mut Vec<ProfileLintIssue>) {
    for (index, constraint) in profile.constraints.iter().enumerate() {
        lint_hl7_path(
            &constraint.path,
            format!("constraints[{index}].path"),
            issues,
        );

        if let Some(components) = &constraint.components
            && let (Some(min), Some(max)) = (components.min, components.max)
            && min > max
        {
            issues.push(ProfileLintIssue::error(
                "component_min_exceeds_max",
                Some(format!("constraints[{index}].components")),
                format!("component minimum {min} exceeds maximum {max}"),
            ));
        }

        if let Some(pattern) = &constraint.pattern {
            lint_regex(
                pattern,
                format!("constraints[{index}].pattern"),
                "invalid_constraint_pattern",
                issues,
            );
        }

        if let Some(condition) = &constraint.when {
            lint_condition(condition, format!("constraints[{index}].when"), issues);
        }
    }
}

fn lint_condition(condition: &Condition, base_path: String, issues: &mut Vec<ProfileLintIssue>) {
    if let Some(eq_conditions) = &condition.eq {
        if eq_conditions.len() != 2 {
            issues.push(ProfileLintIssue::error(
                "invalid_condition_eq",
                Some(format!("{base_path}.eq")),
                "condition eq must contain exactly [field_path, expected_value]".to_string(),
            ));
        } else if let Some(field_path) = eq_conditions.first() {
            lint_hl7_path(field_path, format!("{base_path}.eq[0]"), issues);
        }
    }

    if let Some(nested_conditions) = &condition.any {
        for (index, nested) in nested_conditions.iter().enumerate() {
            lint_condition(nested, format!("{base_path}.any[{index}]"), issues);
        }
    }
}

fn lint_lengths(profile: &Profile, issues: &mut Vec<ProfileLintIssue>) {
    for (index, length) in profile.lengths.iter().enumerate() {
        lint_hl7_path(&length.path, format!("lengths[{index}].path"), issues);

        if let Some(policy) = &length.policy
            && policy != "no-truncate"
            && policy != "may-truncate"
        {
            issues.push(ProfileLintIssue::warning(
                "unknown_length_policy",
                Some(format!("lengths[{index}].policy")),
                format!("length policy '{policy}' is not recognized"),
            ));
        }
    }
}

fn lint_value_sets(profile: &Profile, issues: &mut Vec<ProfileLintIssue>) {
    let table_ids: HashSet<&str> = profile
        .hl7_tables
        .iter()
        .map(|table| table.id.as_str())
        .collect();
    let mut names = HashSet::new();

    for (index, valueset) in profile.valuesets.iter().enumerate() {
        lint_hl7_path(&valueset.path, format!("valuesets[{index}].path"), issues);

        if valueset.name.trim().is_empty() {
            issues.push(ProfileLintIssue::error(
                "empty_valueset_name",
                Some(format!("valuesets[{index}].name")),
                "value set name must not be empty".to_string(),
            ));
        } else if !names.insert(valueset.name.as_str()) {
            issues.push(ProfileLintIssue::warning(
                "duplicate_valueset_name",
                Some(format!("valuesets[{index}].name")),
                format!("value set '{}' is defined more than once", valueset.name),
            ));
        }

        if valueset.codes.is_empty() && !table_ids.contains(valueset.name.as_str()) {
            issues.push(ProfileLintIssue::warning(
                "empty_valueset_without_table",
                Some(format!("valuesets[{index}]")),
                format!(
                    "value set '{}' has no inline codes and does not reference an hl7_tables id",
                    valueset.name
                ),
            ));
        }
    }
}

fn lint_datatypes(profile: &Profile, issues: &mut Vec<ProfileLintIssue>) {
    for (index, datatype) in profile.datatypes.iter().enumerate() {
        lint_hl7_path(&datatype.path, format!("datatypes[{index}].path"), issues);
        if datatype.r#type.trim().is_empty() {
            issues.push(ProfileLintIssue::error(
                "empty_datatype",
                Some(format!("datatypes[{index}].type")),
                "datatype must not be empty".to_string(),
            ));
        }
    }

    for (index, datatype) in profile.advanced_datatypes.iter().enumerate() {
        lint_hl7_path(
            &datatype.path,
            format!("advanced_datatypes[{index}].path"),
            issues,
        );

        if datatype.r#type.trim().is_empty() {
            issues.push(ProfileLintIssue::error(
                "empty_advanced_datatype",
                Some(format!("advanced_datatypes[{index}].type")),
                "advanced datatype must not be empty".to_string(),
            ));
        }

        if let (Some(min), Some(max)) = (datatype.min_length, datatype.max_length)
            && min > max
        {
            issues.push(ProfileLintIssue::error(
                "datatype_min_length_exceeds_max",
                Some(format!("advanced_datatypes[{index}]")),
                format!("minimum length {min} exceeds maximum length {max}"),
            ));
        }

        if let Some(pattern) = &datatype.pattern {
            lint_regex(
                pattern,
                format!("advanced_datatypes[{index}].pattern"),
                "invalid_datatype_pattern",
                issues,
            );
        }

        if let Some(checksum) = &datatype.checksum
            && checksum != "luhn"
            && checksum != "mod10"
        {
            issues.push(ProfileLintIssue::warning(
                "unknown_checksum_algorithm",
                Some(format!("advanced_datatypes[{index}].checksum")),
                format!("checksum algorithm '{checksum}' is ignored by validation"),
            ));
        }
    }
}

fn lint_rules(profile: &Profile, issues: &mut Vec<ProfileLintIssue>) {
    let valueset_names: HashSet<&str> = profile
        .valuesets
        .iter()
        .map(|valueset| valueset.name.as_str())
        .collect();

    lint_cross_field_rules(profile, &valueset_names, issues);
    lint_temporal_rules(profile, issues);
    lint_contextual_rules(profile, &valueset_names, issues);
}

fn lint_cross_field_rules(
    profile: &Profile,
    valueset_names: &HashSet<&str>,
    issues: &mut Vec<ProfileLintIssue>,
) {
    let mut ids = HashSet::new();

    for (index, rule) in profile.cross_field_rules.iter().enumerate() {
        let base_path = format!("cross_field_rules[{index}]");
        lint_rule_id(rule.id.as_str(), &mut ids, &base_path, issues);

        if rule.validation_mode != "conditional" && rule.validation_mode != "assert" {
            issues.push(ProfileLintIssue::error(
                "unknown_cross_field_validation_mode",
                Some(format!("{base_path}.validation_mode")),
                format!(
                    "cross-field validation mode '{}' is not supported",
                    rule.validation_mode
                ),
            ));
        }

        for (condition_index, condition) in rule.conditions.iter().enumerate() {
            lint_rule_condition(
                condition,
                format!("{base_path}.conditions[{condition_index}]"),
                issues,
            );
        }

        for (action_index, action) in rule.actions.iter().enumerate() {
            lint_rule_action(
                action,
                format!("{base_path}.actions[{action_index}]"),
                valueset_names,
                issues,
            );
        }
    }
}

fn lint_temporal_rules(profile: &Profile, issues: &mut Vec<ProfileLintIssue>) {
    let mut ids = HashSet::new();

    for (index, rule) in profile.temporal_rules.iter().enumerate() {
        let base_path = format!("temporal_rules[{index}]");
        lint_rule_id(rule.id.as_str(), &mut ids, &base_path, issues);
        lint_hl7_path(&rule.before, format!("{base_path}.before"), issues);
        lint_hl7_path(&rule.after, format!("{base_path}.after"), issues);
    }
}

fn lint_contextual_rules(
    profile: &Profile,
    valueset_names: &HashSet<&str>,
    issues: &mut Vec<ProfileLintIssue>,
) {
    let mut ids = HashSet::new();

    for (index, rule) in profile.contextual_rules.iter().enumerate() {
        let base_path = format!("contextual_rules[{index}]");
        lint_rule_id(rule.id.as_str(), &mut ids, &base_path, issues);
        lint_hl7_path(
            &rule.context_field,
            format!("{base_path}.context_field"),
            issues,
        );
        lint_hl7_path(
            &rule.target_field,
            format!("{base_path}.target_field"),
            issues,
        );

        match rule.validation_type.as_str() {
            "require" | "prohibit" | "validate_datatype" | "validate_valueset" => {}
            validation_type => issues.push(ProfileLintIssue::error(
                "unknown_contextual_validation_type",
                Some(format!("{base_path}.validation_type")),
                format!("contextual validation type '{validation_type}' is not supported"),
            )),
        }

        if rule.validation_type == "validate_datatype" && !rule.parameters.contains_key("datatype")
        {
            issues.push(ProfileLintIssue::error(
                "missing_contextual_datatype_parameter",
                Some(format!("{base_path}.parameters.datatype")),
                "validate_datatype requires a datatype parameter".to_string(),
            ));
        }

        if rule.validation_type == "validate_valueset" {
            match rule.parameters.get("valueset") {
                Some(valueset) if valueset_names.contains(valueset.as_str()) => {}
                Some(valueset) => issues.push(ProfileLintIssue::error(
                    "unknown_contextual_valueset",
                    Some(format!("{base_path}.parameters.valueset")),
                    format!("contextual rule references undefined value set '{valueset}'"),
                )),
                None => issues.push(ProfileLintIssue::error(
                    "missing_contextual_valueset_parameter",
                    Some(format!("{base_path}.parameters.valueset")),
                    "validate_valueset requires a valueset parameter".to_string(),
                )),
            }
        }
    }
}

fn lint_rule_id(
    id: &str,
    seen: &mut HashSet<String>,
    base_path: &str,
    issues: &mut Vec<ProfileLintIssue>,
) {
    if id.trim().is_empty() {
        issues.push(ProfileLintIssue::error(
            "empty_rule_id",
            Some(format!("{base_path}.id")),
            "rule id must not be empty".to_string(),
        ));
    } else if !seen.insert(id.to_string()) {
        issues.push(ProfileLintIssue::warning(
            "duplicate_rule_id",
            Some(format!("{base_path}.id")),
            format!("rule id '{id}' is defined more than once in this rule family"),
        ));
    }
}

fn lint_rule_condition(
    condition: &RuleCondition,
    base_path: String,
    issues: &mut Vec<ProfileLintIssue>,
) {
    lint_hl7_path(&condition.field, format!("{base_path}.field"), issues);

    match condition.operator.as_str() {
        "eq" | "ne" | "contains" | "in" | "exists" | "not_exists" | "is_date" | "before"
        | "within_range" => {}
        "matches_regex" => match &condition.value {
            Some(pattern) => lint_regex(
                pattern,
                format!("{base_path}.value"),
                "invalid_rule_condition_regex",
                issues,
            ),
            None => issues.push(ProfileLintIssue::error(
                "missing_rule_condition_regex",
                Some(format!("{base_path}.value")),
                "matches_regex requires a regex pattern in value".to_string(),
            )),
        },
        operator => issues.push(ProfileLintIssue::error(
            "unknown_rule_condition_operator",
            Some(format!("{base_path}.operator")),
            format!("rule condition operator '{operator}' is not supported"),
        )),
    }

    if condition.operator == "within_range" && condition.values.as_ref().map_or(0, Vec::len) != 2 {
        issues.push(ProfileLintIssue::error(
            "invalid_within_range_values",
            Some(format!("{base_path}.values")),
            "within_range requires exactly two values".to_string(),
        ));
    }
}

fn lint_rule_action(
    action: &RuleAction,
    base_path: String,
    valueset_names: &HashSet<&str>,
    issues: &mut Vec<ProfileLintIssue>,
) {
    lint_hl7_path(&action.field, format!("{base_path}.field"), issues);

    match action.action.as_str() {
        "require" | "prohibit" | "validate" => {}
        action_type => issues.push(ProfileLintIssue::error(
            "unknown_rule_action",
            Some(format!("{base_path}.action")),
            format!("rule action '{action_type}' is not supported"),
        )),
    }

    if let Some(valueset) = &action.valueset
        && !valueset_names.contains(valueset.as_str())
    {
        issues.push(ProfileLintIssue::error(
            "unknown_action_valueset",
            Some(format!("{base_path}.valueset")),
            format!("rule action references undefined value set '{valueset}'"),
        ));
    }
}

fn lint_tables(profile: &Profile, issues: &mut Vec<ProfileLintIssue>) {
    let mut ids = HashSet::new();
    let mut table_by_id: HashMap<&str, usize> = HashMap::new();

    for (index, table) in profile.hl7_tables.iter().enumerate() {
        if table.id.trim().is_empty() {
            issues.push(ProfileLintIssue::error(
                "empty_table_id",
                Some(format!("hl7_tables[{index}].id")),
                "HL7 table id must not be empty".to_string(),
            ));
        } else {
            table_by_id.insert(table.id.as_str(), index);
            if !ids.insert(table.id.as_str()) {
                issues.push(ProfileLintIssue::warning(
                    "duplicate_table_id",
                    Some(format!("hl7_tables[{index}].id")),
                    format!("HL7 table '{}' is defined more than once", table.id),
                ));
            }
        }

        if table.name.trim().is_empty() {
            issues.push(ProfileLintIssue::warning(
                "empty_table_name",
                Some(format!("hl7_tables[{index}].name")),
                format!("HL7 table '{}' has an empty name", table.id),
            ));
        }

        if table.version.trim().is_empty() {
            issues.push(ProfileLintIssue::warning(
                "empty_table_version",
                Some(format!("hl7_tables[{index}].version")),
                format!("HL7 table '{}' has an empty version", table.id),
            ));
        }
    }

    for (index, table_id) in profile.table_precedence.iter().enumerate() {
        if !table_by_id.contains_key(table_id.as_str()) {
            issues.push(ProfileLintIssue::error(
                "unknown_table_precedence_entry",
                Some(format!("table_precedence[{index}]")),
                format!("table precedence references undefined HL7 table '{table_id}'"),
            ));
        }
    }
}

fn lint_custom_rules(profile: &Profile, issues: &mut Vec<ProfileLintIssue>) {
    let mut ids = HashSet::new();

    for (index, rule) in profile.custom_rules.iter().enumerate() {
        let base_path = format!("custom_rules[{index}]");
        lint_rule_id(rule.id.as_str(), &mut ids, &base_path, issues);

        if rule.script.trim().is_empty() {
            issues.push(ProfileLintIssue::error(
                "empty_custom_rule_script",
                Some(format!("{base_path}.script")),
                format!("custom rule '{}' has an empty script", rule.id),
            ));
        }
    }

    if !profile.custom_rules.is_empty() && !profile.expression_guardrails.allow_custom_scripts {
        issues.push(ProfileLintIssue::warning(
            "custom_rules_without_script_guardrail",
            Some("expression_guardrails.allow_custom_scripts".to_string()),
            "custom_rules are present but expression_guardrails.allow_custom_scripts is false"
                .to_string(),
        ));
    }
}

fn lint_hl7_path(path: &str, location: String, issues: &mut Vec<ProfileLintIssue>) {
    if let Err(err) = crate::query::path::parse_path(path) {
        issues.push(ProfileLintIssue::error(
            "invalid_hl7_path",
            Some(location),
            format!("'{path}' is not a valid HL7 field path: {err}"),
        ));
    }
}

fn lint_regex(pattern: &str, location: String, code: &str, issues: &mut Vec<ProfileLintIssue>) {
    if let Err(err) = Regex::new(pattern) {
        issues.push(ProfileLintIssue::error(
            code,
            Some(location),
            format!("regex '{pattern}' failed to compile: {err}"),
        ));
    }
}

/// Load profile from YAML
pub fn load_profile(yaml: &str) -> Result<Profile, ProfileLoadError> {
    serde_yaml::from_str(yaml).map_err(ProfileLoadError::from)
}

/// Load profile from YAML with specific error types.
///
/// This is the preferred function for loading profiles as it provides
/// detailed error information specific to profile loading issues.
///
/// # Arguments
///
/// * `yaml` - The YAML string containing the profile definition
///
/// # Returns
///
/// The parsed Profile, or a ProfileLoadError if parsing fails
///
/// # Example
///
/// ```ignore
/// use hl7v2::conformance::profile::{load_profile_checked, ProfileLoadError};
///
/// let yaml = r#"
/// message_structure: ADT_A01
/// version: "2.5.1"
/// segments:
///   - id: MSH
/// "#;
///
/// let profile = load_profile_checked(yaml)?;
/// assert_eq!(profile.message_structure, "ADT_A01");
/// ```
pub fn load_profile_checked(yaml: &str) -> Result<Profile, ProfileLoadError> {
    serde_yaml::from_str(yaml).map_err(ProfileLoadError::from)
}

/// Load profile from a file path with specific error types.
///
/// # Arguments
///
/// * `path` - The path to the YAML file containing the profile definition
///
/// # Returns
///
/// The parsed Profile, or a ProfileLoadError if loading or parsing fails
///
/// # Example
///
/// ```ignore
/// use hl7v2::conformance::profile::load_profile_from_file;
///
/// let profile = load_profile_from_file("profiles/adt_a01.yaml")?;
/// ```
pub async fn load_profile_from_file(path: &str) -> Result<Profile, ProfileLoadError> {
    let content = tokio::fs::read_to_string(path).await?;
    load_profile_checked(&content)
}

/// Load profile with inheritance resolution
///
/// This function loads a profile and recursively resolves any parent profiles,
/// merging their constraints and rules into a single profile.
///
/// # Arguments
///
/// * `yaml` - The YAML string for the profile
/// * `profile_loader` - A function that can load a parent profile by name
///
/// # Returns
///
/// A fully resolved profile with all inherited constraints merged
pub fn load_profile_with_inheritance<F>(
    yaml: &str,
    profile_loader: F,
) -> Result<Profile, ProfileLoadError>
where
    F: Fn(&str) -> Result<Profile, ProfileLoadError>,
{
    let profile = load_profile_checked(yaml)?;

    // If there's a parent, recursively load and merge it
    if let Some(parent_name) = &profile.parent {
        let parent_profile = load_profile_with_inheritance_recursive(parent_name, &profile_loader)?;
        return Ok(merge_profiles(parent_profile, profile));
    }

    Ok(profile)
}

/// Recursively load parent profiles
fn load_profile_with_inheritance_recursive<F>(
    parent_name: &str,
    profile_loader: &F,
) -> Result<Profile, ProfileLoadError>
where
    F: Fn(&str) -> Result<Profile, ProfileLoadError>,
{
    let parent_profile = profile_loader(parent_name)?;

    // If the parent also has a parent, recursively load and merge it
    if let Some(grandparent_name) = &parent_profile.parent {
        let grandparent_profile =
            load_profile_with_inheritance_recursive(grandparent_name, profile_loader)?;
        return Ok(merge_profiles(grandparent_profile, parent_profile));
    }

    Ok(parent_profile)
}

/// Merge two profiles, with the child profile taking precedence
fn merge_profiles(parent: Profile, child: Profile) -> Profile {
    Profile {
        message_structure: child.message_structure,
        version: child.version,
        message_type: child.message_type.or(parent.message_type),
        parent: child.parent, // Keep child's parent reference
        segments: merge_segment_specs(parent.segments, child.segments),
        constraints: merge_constraints(parent.constraints, child.constraints),
        lengths: merge_length_constraints(parent.lengths, child.lengths),
        valuesets: merge_valuesets(parent.valuesets, child.valuesets),
        datatypes: merge_datatype_constraints(parent.datatypes, child.datatypes),
        advanced_datatypes: merge_advanced_datatype_constraints(
            parent.advanced_datatypes,
            child.advanced_datatypes,
        ),
        cross_field_rules: merge_cross_field_rules(
            parent.cross_field_rules,
            child.cross_field_rules,
        ),
        temporal_rules: merge_temporal_rules(parent.temporal_rules, child.temporal_rules),
        contextual_rules: merge_contextual_rules(parent.contextual_rules, child.contextual_rules),
        custom_rules: merge_custom_rules(parent.custom_rules, child.custom_rules),
        hl7_tables: merge_hl7_tables(parent.hl7_tables, child.hl7_tables),
        table_precedence: if child.table_precedence.is_empty() {
            parent.table_precedence
        } else {
            child.table_precedence
        },
        expression_guardrails: if child.expression_guardrails == ExpressionGuardrails::default() {
            parent.expression_guardrails
        } else {
            child.expression_guardrails
        },
    }
}

/// Merge segment specifications, removing duplicates by ID
fn merge_segment_specs(parent: Vec<SegmentSpec>, child: Vec<SegmentSpec>) -> Vec<SegmentSpec> {
    let mut result: Vec<SegmentSpec> = parent;

    // Add child segments that don't already exist in parent
    for child_segment in child {
        if !result.iter().any(|s| s.id == child_segment.id) {
            result.push(child_segment);
        }
    }

    result
}

/// Merge constraints, with child constraints overriding parent constraints on same path
fn merge_constraints(parent: Vec<Constraint>, child: Vec<Constraint>) -> Vec<Constraint> {
    let mut result: Vec<Constraint> = parent;

    // Add child constraints, replacing any with the same path
    for child_constraint in child {
        if let Some(pos) = result.iter().position(|c| c.path == child_constraint.path) {
            result[pos] = child_constraint;
        } else {
            result.push(child_constraint);
        }
    }

    result
}

/// Merge length constraints, with child constraints overriding parent constraints on same path
fn merge_length_constraints(
    parent: Vec<LengthConstraint>,
    child: Vec<LengthConstraint>,
) -> Vec<LengthConstraint> {
    let mut result: Vec<LengthConstraint> = parent;

    // Add child constraints, replacing any with the same path
    for child_constraint in child {
        if let Some(pos) = result.iter().position(|c| c.path == child_constraint.path) {
            result[pos] = child_constraint;
        } else {
            result.push(child_constraint);
        }
    }

    result
}

/// Merge value sets, with child value sets overriding parent value sets on same path
fn merge_valuesets(parent: Vec<ValueSet>, child: Vec<ValueSet>) -> Vec<ValueSet> {
    let mut result: Vec<ValueSet> = parent;

    // Add child value sets, replacing any with the same path
    for child_valueset in child {
        if let Some(pos) = result.iter().position(|v| v.path == child_valueset.path) {
            result[pos] = child_valueset;
        } else {
            result.push(child_valueset);
        }
    }

    result
}

/// Merge data type constraints, with child constraints overriding parent constraints on same path
fn merge_datatype_constraints(
    parent: Vec<DataTypeConstraint>,
    child: Vec<DataTypeConstraint>,
) -> Vec<DataTypeConstraint> {
    let mut result: Vec<DataTypeConstraint> = parent;

    // Add child constraints, replacing any with the same path
    for child_constraint in child {
        if let Some(pos) = result.iter().position(|d| d.path == child_constraint.path) {
            result[pos] = child_constraint;
        } else {
            result.push(child_constraint);
        }
    }

    result
}

/// Merge advanced data type constraints, with child constraints overriding parent constraints on same path
fn merge_advanced_datatype_constraints(
    parent: Vec<AdvancedDataTypeConstraint>,
    child: Vec<AdvancedDataTypeConstraint>,
) -> Vec<AdvancedDataTypeConstraint> {
    let mut result: Vec<AdvancedDataTypeConstraint> = parent;

    // Add child constraints, replacing any with the same path
    for child_constraint in child {
        if let Some(pos) = result.iter().position(|d| d.path == child_constraint.path) {
            result[pos] = child_constraint;
        } else {
            result.push(child_constraint);
        }
    }

    result
}

/// Merge cross-field rules, with child rules overriding parent rules with same ID
fn merge_cross_field_rules(
    parent: Vec<CrossFieldRule>,
    child: Vec<CrossFieldRule>,
) -> Vec<CrossFieldRule> {
    let mut result: Vec<CrossFieldRule> = parent;

    // Add child rules, replacing any with the same ID
    for child_rule in child {
        if let Some(pos) = result.iter().position(|r| r.id == child_rule.id) {
            result[pos] = child_rule;
        } else {
            result.push(child_rule);
        }
    }

    result
}

/// Merge temporal rules, with child rules overriding parent rules with same ID
fn merge_temporal_rules(parent: Vec<TemporalRule>, child: Vec<TemporalRule>) -> Vec<TemporalRule> {
    let mut result: Vec<TemporalRule> = parent;

    // Add child rules, replacing any with the same ID
    for child_rule in child {
        if let Some(pos) = result.iter().position(|r| r.id == child_rule.id) {
            result[pos] = child_rule;
        } else {
            result.push(child_rule);
        }
    }

    result
}

/// Merge contextual rules, with child rules overriding parent rules with same ID
fn merge_contextual_rules(
    parent: Vec<ContextualRule>,
    child: Vec<ContextualRule>,
) -> Vec<ContextualRule> {
    let mut result: Vec<ContextualRule> = parent;

    // Add child rules, replacing any with the same ID
    for child_rule in child {
        if let Some(pos) = result.iter().position(|r| r.id == child_rule.id) {
            result[pos] = child_rule;
        } else {
            result.push(child_rule);
        }
    }

    result
}

/// Merge custom rules, with child rules overriding parent rules with same ID
fn merge_custom_rules(parent: Vec<CustomRule>, child: Vec<CustomRule>) -> Vec<CustomRule> {
    let mut result: Vec<CustomRule> = parent;

    // Add child rules, replacing any with the same ID
    for child_rule in child {
        if let Some(pos) = result.iter().position(|r| r.id == child_rule.id) {
            result[pos] = child_rule;
        } else {
            result.push(child_rule);
        }
    }

    result
}

/// Merge HL7 tables, with child tables overriding parent tables with same ID
fn merge_hl7_tables(parent: Vec<HL7Table>, child: Vec<HL7Table>) -> Vec<HL7Table> {
    let mut result: Vec<HL7Table> = parent;

    // Add child tables, replacing any with the same ID
    for child_table in child {
        if let Some(pos) = result.iter().position(|t| t.id == child_table.id) {
            result[pos] = child_table;
        } else {
            result.push(child_table);
        }
    }

    result
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

            // Validate 'in' constraints against value sets
            if let Some(allowed_values) = &constraint.r#in {
                validate_field_in_constraint(msg, &constraint.path, allowed_values, &mut issues);
            }
        }
    }

    // Validate value sets
    for valueset in &profile.valuesets {
        validate_value_set(msg, valueset, &mut issues);
    }

    // Validate data types
    for datatype in &profile.datatypes {
        validate_data_type_constraint(msg, datatype, &mut issues);
    }

    // Validate advanced data types
    for datatype in &profile.advanced_datatypes {
        validate_advanced_data_type(msg, datatype, &mut issues);
    }

    // Validate length constraints
    for length in &profile.lengths {
        validate_length_constraint(msg, length, &mut issues);
    }

    // Validate HL7 tables (with precedence support if configured)
    if !profile.hl7_tables.is_empty() || !profile.valuesets.is_empty() {
        validate_hl7_tables_with_precedence(msg, profile, &mut issues);
    }

    // Validate cross-field rules
    for rule in &profile.cross_field_rules {
        validate_cross_field_rule(msg, rule, profile, &mut issues);
    }

    // Validate temporal rules
    for rule in &profile.temporal_rules {
        validate_temporal_rule(msg, rule, &mut issues);
    }

    // Validate contextual rules
    for rule in &profile.contextual_rules {
        validate_contextual_rule(msg, rule, profile, &mut issues);
    }

    // Validate custom rules
    for rule in &profile.custom_rules {
        validate_custom_rule(msg, rule, &mut issues);
    }

    issues
}

/// Validate that a required field is present
fn validate_field_required(msg: &Message, path: &str, issues: &mut Vec<Issue>) {
    // Use the query module to retrieve the value.
    if let Some(value) = crate::query::get(msg, path) {
        // If we get a value, check if it's empty
        if value.is_empty() {
            issues.push(Issue::error(
                "MISSING_REQUIRED_FIELD",
                Some(path.to_string()),
                format!("Required field {} is missing", path),
            ));
        }
    } else {
        // If we get None, the field is truly missing
        issues.push(Issue::error(
            "MISSING_REQUIRED_FIELD",
            Some(path.to_string()),
            format!("Required field {} is missing", path),
        ));
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

            if let Some(actual_value) = crate::query::get(msg, field_path) {
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
    // Use the query module to retrieve the value.
    if crate::query::get(msg, &full_path).is_none() {
        issues.push(Issue::error(
            "MISSING_REQUIRED_FIELD",
            Some(full_path),
            format!("Required MSH field {} is missing", path),
        ));
    }
}

/// Validate that a field value is in the allowed values
fn validate_field_in_constraint(
    msg: &Message,
    path: &str,
    allowed_values: &[String],
    issues: &mut Vec<Issue>,
) {
    if let Some(value) = crate::query::get(msg, path) {
        if !allowed_values.contains(&value.to_string()) {
            issues.push(Issue::error(
                "VALUE_NOT_IN_CONSTRAINT",
                Some(path.to_string()),
                format!(
                    "Value '{}' for {} is not in allowed constraint values: {:?}",
                    value, path, allowed_values
                ),
            ));
        }
    }
}

/// Validate that a field value is in the allowed value set
fn validate_value_set(msg: &Message, valueset: &ValueSet, issues: &mut Vec<Issue>) {
    // If codes is empty, this valueset references an HL7 table
    // Validation will happen in validate_hl7_tables_with_precedence instead
    if valueset.codes.is_empty() {
        return;
    }

    if let Some(value) = crate::query::get(msg, &valueset.path) {
        if !valueset.codes.contains(&value.to_string()) {
            issues.push(Issue::error(
                "VALUE_NOT_IN_SET",
                Some(valueset.path.clone()),
                format!(
                    "Value '{}' for {} is not in allowed set: {:?}",
                    value, valueset.path, valueset.codes
                ),
            ));
        }
    }
    // Note: We don't report an error if the field is missing but has a value set constraint
    // That would be handled by a separate presence constraint if needed
}

/// Validate that a field value matches the expected data type
fn validate_data_type_constraint(
    msg: &Message,
    datatype: &DataTypeConstraint,
    issues: &mut Vec<Issue>,
) {
    if let Some(value) = crate::query::get(msg, &datatype.path) {
        if !validate_data_type(value, &datatype.r#type) {
            issues.push(Issue::error(
                "INVALID_DATA_TYPE",
                Some(datatype.path.clone()),
                format!(
                    "Value '{}' for {} does not match expected data type {}",
                    value, datatype.path, datatype.r#type
                ),
            ));
        }
    }
    // Note: We don't report an error if the field is missing but has a data type constraint
    // That would be handled by a separate presence constraint if needed
}

/// Validate that a field value matches the expected advanced data type
fn validate_advanced_data_type(
    msg: &Message,
    datatype: &AdvancedDataTypeConstraint,
    issues: &mut Vec<Issue>,
) {
    if let Some(value) = crate::query::get(msg, &datatype.path) {
        // First check basic data type
        if !validate_data_type(value, &datatype.r#type) {
            issues.push(Issue::error(
                "INVALID_DATA_TYPE",
                Some(datatype.path.clone()),
                format!(
                    "Value '{}' for {} does not match expected data type {}",
                    value, datatype.path, datatype.r#type
                ),
            ));
            return;
        }

        // Check length constraints
        if let Some(min_length) = datatype.min_length {
            if value.len() < min_length {
                issues.push(Issue::error(
                    "VALUE_TOO_SHORT",
                    Some(datatype.path.clone()),
                    format!(
                        "Value '{}' for {} is shorter than minimum length of {} characters",
                        value, datatype.path, min_length
                    ),
                ));
            }
        }

        if let Some(max_length) = datatype.max_length {
            if value.len() > max_length {
                issues.push(Issue::error(
                    "VALUE_TOO_LONG",
                    Some(datatype.path.clone()),
                    format!(
                        "Value '{}' for {} exceeds maximum length of {} characters",
                        value, datatype.path, max_length
                    ),
                ));
            }
        }

        // Check regex pattern if specified
        if let Some(pattern) = &datatype.pattern {
            if let Ok(regex) = Regex::new(pattern) {
                if !regex.is_match(value) {
                    issues.push(Issue::error(
                        "PATTERN_MISMATCH",
                        Some(datatype.path.clone()),
                        format!(
                            "Value '{}' for {} does not match required pattern '{}'",
                            value, datatype.path, pattern
                        ),
                    ));
                }
            }
        }

        // Check format if specified
        if let Some(format) = &datatype.format {
            if !matches_format(value, format, &datatype.r#type) {
                issues.push(Issue::error(
                    "FORMAT_MISMATCH",
                    Some(datatype.path.clone()),
                    format!(
                        "Value '{}' for {} does not match required format '{}'",
                        value, datatype.path, format
                    ),
                ));
            }
        }

        // Check checksum if specified
        if let Some(checksum) = &datatype.checksum {
            if !validate_checksum(value, checksum) {
                issues.push(Issue::error(
                    "CHECKSUM_MISMATCH",
                    Some(datatype.path.clone()),
                    format!("Checksum validation failed for {}", datatype.path),
                ));
            }
        }
    }
}

/// Validate HL7 tables with precedence support
fn validate_hl7_tables_with_precedence(msg: &Message, profile: &Profile, issues: &mut Vec<Issue>) {
    // Create a mapping of value set names to HL7 tables
    let mut table_map: std::collections::HashMap<&str, &HL7Table> =
        std::collections::HashMap::new();
    for table in &profile.hl7_tables {
        table_map.insert(&table.id, table);
    }

    // Validate value sets with table precedence
    for valueset in &profile.valuesets {
        if let Some(table_id) = table_map.get(valueset.name.as_str()) {
            if let Some(value) = crate::query::get(msg, &valueset.path) {
                // Only validate if the field is not empty
                if !value.is_empty() {
                    // Check if the value exists in the table
                    let is_valid = table_id.codes.iter().any(|entry| {
                        entry.value == value
                            && (entry.status.is_empty()
                                || entry.status == "A"
                                || entry.status == "active")
                    });

                    if !is_valid {
                        issues.push(Issue::error(
                            "VALUE_NOT_IN_HL7_TABLE",
                            Some(valueset.path.clone()),
                            format!(
                                "Value '{}' for {} is not in HL7 table {} ({})",
                                value, valueset.path, table_id.id, table_id.name
                            ),
                        ));
                    }
                }
            }
        }
    }
}

/// Validate that a field value does not exceed the maximum length
fn validate_length_constraint(msg: &Message, length: &LengthConstraint, issues: &mut Vec<Issue>) {
    if let Some(value) = crate::query::get(msg, &length.path) {
        if let Some(max_length) = length.max {
            if value.len() > max_length {
                issues.push(Issue::error(
                    "VALUE_TOO_LONG",
                    Some(length.path.clone()),
                    format!(
                        "Value '{}' for {} exceeds maximum length of {} characters",
                        value, length.path, max_length
                    ),
                ));
            }
        }
    }
    // Note: We don't report an error if the field is missing but has a length constraint
    // That would be handled by a separate presence constraint if needed
}

/// Validate that a field value is in the allowed HL7 table
#[expect(
    dead_code,
    reason = "Legacy table validator is retained for compatibility while the profile implementation is collapsed."
)]
fn validate_hl7_table(msg: &Message, table: &HL7Table, profile: &Profile, issues: &mut Vec<Issue>) {
    // This function is kept for backward compatibility but the new
    // validate_hl7_tables_with_precedence function should be used instead
    // when table precedence is important

    // Check value sets that reference this table by name
    for valueset in &profile.valuesets {
        if valueset.name == table.id {
            if let Some(value) = crate::query::get(msg, &valueset.path) {
                // Only validate if the field is not empty
                if !value.is_empty() {
                    // Check if the value exists in the table
                    let is_valid = table.codes.iter().any(|entry| {
                        entry.value == value
                            && (entry.status.is_empty()
                                || entry.status == "A"
                                || entry.status == "active")
                    });

                    if !is_valid {
                        issues.push(Issue::error(
                            "VALUE_NOT_IN_HL7_TABLE",
                            Some(valueset.path.clone()),
                            format!(
                                "Value '{}' for {} is not in HL7 table {} ({})",
                                value, valueset.path, table.id, table.name
                            ),
                        ));
                    }
                }
            }
        }
    }
}

/// Validate temporal rule (date/time relationships)
fn validate_temporal_rule(msg: &Message, rule: &TemporalRule, issues: &mut Vec<Issue>) {
    if let (Some(before_value), Some(after_value)) = (
        crate::query::get(msg, &rule.before),
        crate::query::get(msg, &rule.after),
    ) {
        // Parse the date/time values
        if let (Some(before_time), Some(after_time)) =
            (parse_datetime(before_value), parse_datetime(after_value))
        {
            // Check if before_time should be before after_time
            let is_valid = if rule.allow_equal {
                before_time <= after_time
            } else {
                before_time < after_time
            };

            if !is_valid {
                issues.push(Issue::error(
                    "TEMPORAL_RULE_VIOLATION",
                    Some(rule.before.clone()),
                    format!(
                        "Value '{}' for {} should be before {} for {}",
                        before_value, rule.before, after_value, rule.after
                    ),
                ));
            }
        } else {
            // Handle the case where the date/time parsing fails
            issues.push(Issue::error(
                "INVALID_DATETIME",
                Some(rule.before.clone()),
                format!(
                    "Invalid date/time value for {} or {}",
                    rule.before, rule.after
                ),
            ));
        }
    }
}

/// Validate custom rule
fn validate_custom_rule(msg: &Message, rule: &CustomRule, issues: &mut Vec<Issue>) {
    // Parse and evaluate the custom rule script
    if let Err(_e) = evaluate_custom_rule_script(msg, rule, issues) {
        // If parsing fails, fall back to the simple pattern matching
        evaluate_custom_rule_simple(msg, rule, issues);
    }
}

/// Evaluate custom rule script with proper expression parsing
fn evaluate_custom_rule_script(
    msg: &Message,
    rule: &CustomRule,
    issues: &mut Vec<Issue>,
) -> Result<(), ()> {
    // This is a simplified expression parser for custom rules
    // In a production implementation, this would be a full expression parser

    // Handle field access patterns like "field(PATH)"
    let script = &rule.script;

    // Pattern: "field(PATH).length() > N"
    if script.contains(".length() > ") {
        let re = Regex::new(r"field\(([^)]+)\)\.length\(\)\s*>\s*(\d+)").map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path = &captures[1];
            let required_length: usize = captures[2].parse().map_err(|_| ())?;

            if let Some(value) = crate::query::get(msg, path) {
                if value.len() <= required_length {
                    issues.push(Issue::error(
                        "CUSTOM_RULE_VIOLATION",
                        Some(path.to_string()),
                        if rule.description.is_empty() {
                            format!(
                                "Field {} length {} is not greater than {}",
                                path,
                                value.len(),
                                required_length
                            )
                        } else {
                            rule.description.clone()
                        },
                    ));
                }
            }
            return Ok(());
        }
    }

    // Pattern: "field(PATH) in ['A', 'B', 'C']"
    if script.contains(" in [") {
        let re = Regex::new(r"field\(([^)]+)\)\s+in\s+\[([^\]]+)\]").map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path = &captures[1];
            let values_str = &captures[2];

            if let Some(value) = crate::query::get(msg, path) {
                // Parse the allowed values
                let allowed_values: Vec<&str> = values_str
                    .split(',')
                    .map(str::trim)
                    .map(|s| s.trim_matches('\''))
                    .collect();

                if !allowed_values.contains(&value) {
                    issues.push(Issue::error(
                        "CUSTOM_RULE_VIOLATION",
                        Some(path.to_string()),
                        if rule.description.is_empty() {
                            format!(
                                "Field {} value '{}' is not in allowed set {:?}",
                                path, value, allowed_values
                            )
                        } else {
                            rule.description.clone()
                        },
                    ));
                }
            }
            return Ok(());
        }
    }

    // Pattern: "field(PATH).matches_regex('PATTERN')"
    if script.contains(".matches_regex(") {
        let re = Regex::new(r"field\(([^)]+)\)\.matches_regex\('([^']+)'\)").map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path = &captures[1];
            let pattern = &captures[2];

            if let Some(value) = crate::query::get(msg, path) {
                let regex = Regex::new(pattern).map_err(|_| ())?;
                if !regex.is_match(value) {
                    issues.push(Issue::error(
                        "CUSTOM_RULE_VIOLATION",
                        Some(path.to_string()),
                        if rule.description.is_empty() {
                            format!(
                                "Field {} value '{}' does not match pattern '{}'",
                                path, value, pattern
                            )
                        } else {
                            rule.description.clone()
                        },
                    ));
                }
            }
            return Ok(());
        }
    }

    // Pattern: "field(PATH).starts_with('PREFIX')"
    if script.contains(".starts_with(") {
        let re = Regex::new(r"field\(([^)]+)\)\.starts_with\('([^']+)'\)").map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path = &captures[1];
            let prefix = &captures[2];

            if let Some(value) = crate::query::get(msg, path) {
                if !value.starts_with(prefix) {
                    issues.push(Issue::error(
                        "CUSTOM_RULE_VIOLATION",
                        Some(path.to_string()),
                        if rule.description.is_empty() {
                            format!(
                                "Field {} value '{}' does not start with '{}'",
                                path, value, prefix
                            )
                        } else {
                            rule.description.clone()
                        },
                    ));
                }
            }
            return Ok(());
        }
    }

    // Pattern: "field(PATH).ends_with('SUFFIX')"
    if script.contains(".ends_with(") {
        let re = Regex::new(r"field\(([^)]+)\)\.ends_with\('([^']+)'\)").map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path = &captures[1];
            let suffix = &captures[2];

            if let Some(value) = crate::query::get(msg, path) {
                if !value.ends_with(suffix) {
                    issues.push(Issue::error(
                        "CUSTOM_RULE_VIOLATION",
                        Some(path.to_string()),
                        if rule.description.is_empty() {
                            format!(
                                "Field {} value '{}' does not end with '{}'",
                                path, value, suffix
                            )
                        } else {
                            rule.description.clone()
                        },
                    ));
                }
            }
            return Ok(());
        }
    }

    // Pattern: "field(PATH).is_numeric()"
    if script.contains(".is_numeric()") {
        let re = Regex::new(r"field\(([^)]+)\)\.is_numeric\(\)").map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path = &captures[1];

            if let Some(value) = crate::query::get(msg, path) {
                if !value.chars().all(|c| c.is_ascii_digit()) {
                    issues.push(Issue::error(
                        "CUSTOM_RULE_VIOLATION",
                        Some(path.to_string()),
                        if rule.description.is_empty() {
                            format!("Field {} value '{}' is not numeric", path, value)
                        } else {
                            rule.description.clone()
                        },
                    ));
                }
            }
            return Ok(());
        }
    }

    // Pattern: "field(PATH1) == field(PATH2)"
    if script.contains(" == field(") {
        let re = Regex::new(r"field\(([^)]+)\)\s*==\s*field\(([^)]+)\)").map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path1 = &captures[1];
            let path2 = &captures[2];

            if let (Some(value1), Some(value2)) =
                (crate::query::get(msg, path1), crate::query::get(msg, path2))
            {
                if value1 != value2 {
                    issues.push(Issue::error(
                        "CUSTOM_RULE_VIOLATION",
                        Some(path1.to_string()),
                        if rule.description.is_empty() {
                            format!(
                                "Field {} value '{}' does not equal field {} value '{}'",
                                path1, value1, path2, value2
                            )
                        } else {
                            rule.description.clone()
                        },
                    ));
                }
            }
            return Ok(());
        }
    }

    // Pattern: "field(PATH).is_phone_number()"
    if script.contains(".is_phone_number()") {
        let re = Regex::new(r"field\(([^)]+)\)\.is_phone_number\(\)").map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path = &captures[1];

            if let Some(value) = crate::query::get(msg, path) {
                if !is_phone_number(value) {
                    issues.push(Issue::error(
                        "CUSTOM_RULE_VIOLATION",
                        Some(path.to_string()),
                        if rule.description.is_empty() {
                            format!(
                                "Field {} value '{}' is not a valid phone number",
                                path, value
                            )
                        } else {
                            rule.description.clone()
                        },
                    ));
                }
            }
            return Ok(());
        }
    }

    // Pattern: "field(PATH).is_email()"
    if script.contains(".is_email()") {
        let re = Regex::new(r"field\(([^)]+)\)\.is_email\(\)").map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path = &captures[1];

            if let Some(value) = crate::query::get(msg, path) {
                if !is_email(value) {
                    issues.push(Issue::error(
                        "CUSTOM_RULE_VIOLATION",
                        Some(path.to_string()),
                        if rule.description.is_empty() {
                            format!(
                                "Field {} value '{}' is not a valid email address",
                                path, value
                            )
                        } else {
                            rule.description.clone()
                        },
                    ));
                }
            }
            return Ok(());
        }
    }

    // Pattern: "field(PATH).is_ssn()"
    if script.contains(".is_ssn()") {
        let re = Regex::new(r"field\(([^)]+)\)\.is_ssn\(\)").map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path = &captures[1];

            if let Some(value) = crate::query::get(msg, path) {
                if !is_ssn(value) {
                    issues.push(Issue::error(
                        "CUSTOM_RULE_VIOLATION",
                        Some(path.to_string()),
                        if rule.description.is_empty() {
                            format!("Field {} value '{}' is not a valid SSN", path, value)
                        } else {
                            rule.description.clone()
                        },
                    ));
                }
            }
            return Ok(());
        }
    }

    // Pattern: "field(PATH).is_valid_birth_date()"
    if script.contains(".is_valid_birth_date()") {
        let re = Regex::new(r"field\(([^)]+)\)\.is_valid_birth_date\(\)").map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path = &captures[1];

            if let Some(value) = crate::query::get(msg, path) {
                if !is_valid_birth_date(value) {
                    issues.push(Issue::error(
                        "CUSTOM_RULE_VIOLATION",
                        Some(path.to_string()),
                        if rule.description.is_empty() {
                            format!("Field {} value '{}' is not a valid birth date", path, value)
                        } else {
                            rule.description.clone()
                        },
                    ));
                }
            }
            return Ok(());
        }
    }

    // Pattern: "is_valid_age_range(field(PATH1), field(PATH2))"
    if script.contains("is_valid_age_range(") {
        let re = Regex::new(r"is_valid_age_range\(field\(([^)]+)\),\s*field\(([^)]+)\)\)")
            .map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path1 = &captures[1];
            let path2 = &captures[2];

            if let (Some(value1), Some(value2)) =
                (crate::query::get(msg, path1), crate::query::get(msg, path2))
            {
                if !is_valid_age_range(value1, value2) {
                    issues.push(Issue::error(
                        "CUSTOM_RULE_VIOLATION",
                        Some(path1.to_string()),
                        if rule.description.is_empty() {
                            format!("Age range between {} and {} is not valid", path1, path2)
                        } else {
                            rule.description.clone()
                        },
                    ));
                }
            }
            return Ok(());
        }
    }

    // Pattern: "field(PATH) between VALUE1 and VALUE2"
    if script.contains(" between ") && script.contains(" and ") {
        let re = Regex::new(r"field\(([^)]+)\)\s+between\s+([^\s]+)\s+and\s+([^\s]+)")
            .map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path = &captures[1];
            let min_val = &captures[2];
            let max_val = &captures[3];

            if let Some(value) = crate::query::get(msg, path) {
                if !is_within_range(value, min_val, max_val) {
                    issues.push(Issue::error(
                        "CUSTOM_RULE_VIOLATION",
                        Some(path.to_string()),
                        if rule.description.is_empty() {
                            format!(
                                "Field {} value '{}' is not between {} and {}",
                                path, value, min_val, max_val
                            )
                        } else {
                            rule.description.clone()
                        },
                    ));
                }
            }
            return Ok(());
        }
    }

    // If we get here, we didn't match any known patterns
    Err(())
}

/// Simple pattern matching fallback for custom rules (original implementation)
fn evaluate_custom_rule_simple(msg: &Message, rule: &CustomRule, issues: &mut Vec<Issue>) {
    // For now, we'll implement a simple expression-based custom rule system
    // The script field can contain simple expressions like:
    // "field(PID.5.1).length() > 5"
    // "field(PID.8) in ['M', 'F']"
    // "field(PID.7).matches_regex('^[0-9]{8}$')"

    // This is a simplified implementation - a full implementation would require
    // a proper expression parser and evaluator

    // For demonstration purposes, let's implement a few basic patterns
    if rule.script.starts_with("field(") && rule.script.contains(").length() > ") {
        // Pattern: "field(PATH).length() > N"
        if let Some(path_end) = rule.script.find(").length() > ") {
            let path = &rule.script[6..path_end];
            if let Some(value) = crate::query::get(msg, path) {
                let length_str = &rule.script[path_end + 13..];
                if let Ok(required_length) = length_str.parse::<usize>() {
                    if value.len() <= required_length {
                        issues.push(Issue::error(
                            "CUSTOM_RULE_VIOLATION",
                            Some(path.to_string()),
                            if rule.description.is_empty() {
                                format!(
                                    "Field {} length {} is not greater than {}",
                                    path,
                                    value.len(),
                                    required_length
                                )
                            } else {
                                rule.description.clone()
                            },
                        ));
                    }
                }
            }
        }
    } else if rule.script.starts_with("field(") && rule.script.contains(") in [") {
        // Pattern: "field(PATH) in ['A', 'B', 'C']"
        if let Some(path_end) = rule.script.find(") in [") {
            let path = &rule.script[6..path_end];
            if let Some(value) = crate::query::get(msg, path) {
                // Extract the allowed values
                let values_part = &rule.script[path_end + 7..];
                if let Some(values_str) = values_part.strip_suffix("]") {
                    // Split by comma and remove quotes
                    let allowed_values: Vec<&str> = values_str
                        .split(',')
                        .map(str::trim)
                        .map(|s| s.trim_matches('\''))
                        .collect();

                    if !allowed_values.contains(&value) {
                        issues.push(Issue::error(
                            "CUSTOM_RULE_VIOLATION",
                            Some(path.to_string()),
                            if rule.description.is_empty() {
                                format!(
                                    "Field {} value '{}' is not in allowed set {:?}",
                                    path, value, allowed_values
                                )
                            } else {
                                rule.description.clone()
                            },
                        ));
                    }
                }
            }
        }
    } else if rule.script.starts_with("field(") && rule.script.contains(").matches_regex(") {
        // Pattern: "field(PATH).matches_regex('PATTERN')"
        if let Some(path_end) = rule.script.find(").matches_regex(") {
            let path = &rule.script[6..path_end];
            if let Some(value) = crate::query::get(msg, path) {
                // Extract the regex pattern
                let pattern_part = &rule.script[path_end + 15..];
                if pattern_part.starts_with('\'') && pattern_part.ends_with("')") {
                    let pattern = &pattern_part[1..pattern_part.len() - 2];
                    // Simple regex matching (in a real implementation, we would use regex crate)
                    if !value.contains(pattern) && pattern != ".*" {
                        // This is a very simplified check - just for demonstration
                        issues.push(Issue::error(
                            "CUSTOM_RULE_VIOLATION",
                            Some(path.to_string()),
                            if rule.description.is_empty() {
                                format!(
                                    "Field {} value '{}' does not match pattern '{}'",
                                    path, value, pattern
                                )
                            } else {
                                rule.description.clone()
                            },
                        ));
                    }
                }
            }
        }
    }
    // Additional custom rule patterns can be added here
}

/// Validate cross-field rule
fn validate_cross_field_rule(
    msg: &Message,
    rule: &CrossFieldRule,
    profile: &Profile,
    issues: &mut Vec<Issue>,
) {
    // Check if all conditions are met
    let conditions_met = rule
        .conditions
        .iter()
        .all(|condition| check_rule_condition(msg, condition));

    match rule.validation_mode.as_str() {
        "assert" => {
            // Assert mode: conditions must be true, fail if they're not
            if !conditions_met {
                issues.push(Issue::error(
                    "CROSS_FIELD_ASSERTION_FAILED",
                    None,
                    format!(
                        "Cross-field assertion failed: {} ({})",
                        rule.description, rule.id
                    ),
                ));
            }
            // If conditions are true, validation passes (no error)
        }
        _ => {
            // Conditional mode (default): if conditions are met, execute actions
            if conditions_met {
                for action in &rule.actions {
                    execute_rule_action(msg, action, rule, profile, issues);
                }
            }
        }
    }
}

/// Execute a rule action
fn execute_rule_action(
    msg: &Message,
    action: &RuleAction,
    rule: &CrossFieldRule,
    profile: &Profile,
    issues: &mut Vec<Issue>,
) {
    match action.action.as_str() {
        "require" => {
            // Check if the required field exists and is not empty
            if let Some(value) = crate::query::get(msg, &action.field) {
                if value.is_empty() {
                    issues.push(Issue::error(
                        "CROSS_FIELD_VALIDATION_ERROR",
                        Some(action.field.clone()),
                        action.message.clone().unwrap_or_else(|| {
                            format!(
                                "Field {} is required by cross-field rule {}",
                                action.field, rule.id
                            )
                        }),
                    ));
                }
            } else {
                issues.push(Issue::error(
                    "CROSS_FIELD_VALIDATION_ERROR",
                    Some(action.field.clone()),
                    action.message.clone().unwrap_or_else(|| {
                        format!(
                            "Field {} is required by cross-field rule {}",
                            action.field, rule.id
                        )
                    }),
                ));
            }
        }
        "prohibit" => {
            // Check if the prohibited field exists and is not empty
            if let Some(value) = crate::query::get(msg, &action.field) {
                if !value.is_empty() {
                    issues.push(Issue::error(
                        "CROSS_FIELD_VALIDATION_ERROR",
                        Some(action.field.clone()),
                        action.message.clone().unwrap_or_else(|| {
                            format!(
                                "Field {} is prohibited by cross-field rule {}",
                                action.field, rule.id
                            )
                        }),
                    ));
                }
            }
            // If the field doesn't exist at all, that's fine (it's not present)
        }
        "validate" => {
            // Apply additional validation based on action parameters
            if let Some(value) = crate::query::get(msg, &action.field) {
                // Only validate if the field is not empty
                if !value.is_empty() {
                    // Validate data type if specified
                    if let Some(datatype) = &action.datatype {
                        if !validate_data_type(value, datatype) {
                            issues.push(Issue::error(
                                "CROSS_FIELD_VALIDATION_ERROR",
                                Some(action.field.clone()),
                                action.message.clone().unwrap_or_else(||
                                    format!("Field {} does not match data type {} required by cross-field rule {}",
                                           action.field, datatype, rule.id)),
                            ));
                        }
                    }

                    // Validate against value set if specified
                    if let Some(valueset_name) = &action.valueset {
                        // Find the value set in the profile
                        if let Some(valueset) = find_valueset_by_name(profile, valueset_name) {
                            if !valueset.codes.contains(&value.to_string()) {
                                issues.push(Issue::error(
                                    "CROSS_FIELD_VALIDATION_ERROR",
                                    Some(action.field.clone()),
                                    action.message.clone().unwrap_or_else(||
                                        format!("Value '{}' for {} is not in value set {} required by cross-field rule {}",
                                               value, action.field, valueset_name, rule.id)),
                                ));
                            }
                        }
                    }
                }
            }
        }
        _ => {
            // Unknown action, ignore
        }
    }
}

/// Validate contextual rule
fn validate_contextual_rule(
    msg: &Message,
    rule: &ContextualRule,
    profile: &Profile,
    issues: &mut Vec<Issue>,
) {
    // Check if the context field has the expected value
    if let Some(context_value) = crate::query::get(msg, &rule.context_field) {
        if context_value == rule.context_value {
            // Apply the validation based on validation_type
            match rule.validation_type.as_str() {
                "require" => {
                    // Check if the target field exists and is not empty
                    if let Some(value) = crate::query::get(msg, &rule.target_field) {
                        if value.is_empty() {
                            issues.push(Issue::error(
                                "CONTEXTUAL_VALIDATION_ERROR",
                                Some(rule.target_field.clone()),
                                if rule.description.is_empty() {
                                    format!(
                                        "Field {} is required when {} equals {}",
                                        rule.target_field, rule.context_field, rule.context_value
                                    )
                                } else {
                                    rule.description.clone()
                                },
                            ));
                        }
                    } else {
                        issues.push(Issue::error(
                            "CONTEXTUAL_VALIDATION_ERROR",
                            Some(rule.target_field.clone()),
                            if rule.description.is_empty() {
                                format!(
                                    "Field {} is required when {} equals {}",
                                    rule.target_field, rule.context_field, rule.context_value
                                )
                            } else {
                                rule.description.clone()
                            },
                        ));
                    }
                }
                "prohibit" => {
                    // Check if the target field exists and is not empty
                    if let Some(value) = crate::query::get(msg, &rule.target_field) {
                        if !value.is_empty() {
                            issues.push(Issue::error(
                                "CONTEXTUAL_VALIDATION_ERROR",
                                Some(rule.target_field.clone()),
                                if rule.description.is_empty() {
                                    format!(
                                        "Field {} is prohibited when {} equals {}",
                                        rule.target_field, rule.context_field, rule.context_value
                                    )
                                } else {
                                    rule.description.clone()
                                },
                            ));
                        }
                    }
                    // If the field doesn't exist at all, that's fine (it's not present)
                }
                "validate_datatype" => {
                    // Validate target field against specified data type
                    if let Some(datatype) = rule.parameters.get("datatype") {
                        if let Some(value) = crate::query::get(msg, &rule.target_field) {
                            if !validate_data_type(value, datatype) {
                                issues.push(Issue::error(
                                    "CONTEXTUAL_VALIDATION_ERROR",
                                    Some(rule.target_field.clone()),
                                    if rule.description.is_empty() {
                                        format!("Field {} does not match data type {} required when {} equals {}", 
                                               rule.target_field, datatype, rule.context_field, rule.context_value)
                                    } else {
                                        rule.description.clone()
                                    },
                                ));
                            }
                        }
                    }
                }
                "validate_valueset" => {
                    // Validate target field against specified value set
                    if let Some(valueset_name) = rule.parameters.get("valueset") {
                        if let Some(value) = crate::query::get(msg, &rule.target_field) {
                            // Find the value set in the profile
                            if let Some(valueset) = find_valueset_by_name(profile, valueset_name) {
                                if !valueset.codes.contains(&value.to_string()) {
                                    issues.push(Issue::error(
                                        "CONTEXTUAL_VALIDATION_ERROR",
                                        Some(rule.target_field.clone()),
                                        if rule.description.is_empty() {
                                            format!("Value '{}' for {} is not in value set {} required when {} equals {}", 
                                                   value, rule.target_field, valueset_name, rule.context_field, rule.context_value)
                                        } else {
                                            rule.description.clone()
                                        },
                                    ));
                                }
                            }
                        }
                    }
                }
                _ => {
                    // Unknown validation type, ignore
                }
            }
        }
    }
}

/// Find a value set by name within a profile
fn find_valueset_by_name<'a>(profile: &'a Profile, name: &str) -> Option<&'a ValueSet> {
    profile
        .valuesets
        .iter()
        .find(|valueset| valueset.name == name)
}

/// Profile loader module with remote loading and caching support
pub mod loader;

/// Persistent profile cache with PostgreSQL backend and two-tier caching
#[cfg(feature = "persistent-cache")]
pub mod persistent_cache;

#[cfg(test)]
mod tests;
