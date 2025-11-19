//! Profile validation for HL7 v2 messages.
//!
//! This crate provides functionality for loading and applying
//! conformance profiles to HL7 v2 messages.

use chrono::{NaiveDate, NaiveDateTime};
use hl7v2_core::{Error, Message};
use regex::Regex;
use serde::{Deserialize, Serialize};

/// A conformance profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub message_structure: String,
    pub version: String,
    #[serde(default)]
    pub message_type: Option<String>,
    #[serde(default)]
    pub parent: Option<String>, // Reference to parent profile by name
    pub segments: Vec<SegmentSpec>,
    #[serde(default)]
    pub constraints: Vec<Constraint>,
    #[serde(default)]
    pub lengths: Vec<LengthConstraint>,
    #[serde(default)]
    pub valuesets: Vec<ValueSet>,
    #[serde(default)]
    pub datatypes: Vec<DataTypeConstraint>,
    #[serde(default)]
    pub advanced_datatypes: Vec<AdvancedDataTypeConstraint>, // New field for advanced data type validation
    #[serde(default)]
    pub cross_field_rules: Vec<CrossFieldRule>,
    #[serde(default)]
    pub temporal_rules: Vec<TemporalRule>, // New field for temporal validation
    #[serde(default)]
    pub contextual_rules: Vec<ContextualRule>, // New field for contextual validation
    #[serde(default)]
    pub custom_rules: Vec<CustomRule>,
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
    /// Codes can be defined inline OR reference an HL7 table by name
    #[serde(default)]
    pub codes: Vec<String>,
}

/// Data type constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataTypeConstraint {
    pub path: String,
    pub r#type: String, // HL7 data type like "ST", "ID", "DT", etc.
}

/// Advanced data type constraint with complex validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedDataTypeConstraint {
    pub path: String,
    pub r#type: String, // HL7 data type like "ST", "ID", "DT", etc.
    #[serde(default)]
    pub pattern: Option<String>, // Regex pattern for additional validation
    #[serde(default)]
    pub min_length: Option<usize>, // Minimum length constraint
    #[serde(default)]
    pub max_length: Option<usize>, // Maximum length constraint
    #[serde(default)]
    pub format: Option<String>, // Format specification (e.g., "YYYY-MM-DD" for dates)
    #[serde(default)]
    pub checksum: Option<String>, // Checksum algorithm (e.g., "luhn" for credit cards)
}

/// Temporal validation rule for date/time relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalRule {
    pub id: String,
    pub description: String,
    pub before: String, // Path to field that should be before another
    pub after: String,  // Path to field that should be after another
    #[serde(default)]
    pub allow_equal: bool, // Whether equal times are allowed
    #[serde(default)]
    pub tolerance: Option<String>, // Tolerance for comparison (e.g., "1d" for 1 day)
}

/// Contextual validation rule based on message context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextualRule {
    pub id: String,
    pub description: String,
    pub context_field: String,   // Field that determines the context
    pub context_value: String,   // Value that triggers this rule
    pub target_field: String,    // Field to validate
    pub validation_type: String, // Type of validation to apply
    #[serde(default)]
    pub parameters: std::collections::HashMap<String, String>, // Additional parameters
}

/// HL7 Table definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HL7Table {
    pub id: String,      // Table ID like "HL70001"
    pub name: String,    // Table name like "Administrative Sex"
    pub version: String, // HL7 version like "2.5.1"
    pub codes: Vec<HL7TableEntry>,
}

/// Entry in an HL7 table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HL7TableEntry {
    pub value: String,       // The code value
    pub description: String, // Description of the code
    #[serde(default)]
    pub status: String, // "A" (active), "D" (deprecated), "R" (restricted)
}

/// Cross-field validation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossFieldRule {
    pub id: String,
    pub description: String,
    pub conditions: Vec<RuleCondition>,
    pub actions: Vec<RuleAction>,
}

/// Condition for a cross-field rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleCondition {
    pub field: String,
    pub operator: String, // "eq", "ne", "gt", "lt", "ge", "le", "in", "contains", "exists", "missing"
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub values: Option<Vec<String>>,
}

/// Action to take when a cross-field rule is violated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleAction {
    pub field: String,
    pub action: String, // "require", "prohibit", "validate"
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub datatype: Option<String>,
    #[serde(default)]
    pub valueset: Option<String>,
}

/// Custom validation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomRule {
    pub id: String,
    pub description: String,
    pub script: String, // Could be a simple expression or reference to external logic
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
pub fn load_profile_with_inheritance<F>(yaml: &str, profile_loader: F) -> Result<Profile, Error>
where
    F: Fn(&str) -> Result<Profile, Error>,
{
    let profile = load_profile(yaml)?;

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
) -> Result<Profile, Error>
where
    F: Fn(&str) -> Result<Profile, Error>,
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
        table_precedence: if child.table_precedence.is_empty() { parent.table_precedence } else { child.table_precedence },
        expression_guardrails: if child.expression_guardrails == ExpressionGuardrails::default() { parent.expression_guardrails } else { child.expression_guardrails },
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
        validate_data_type(msg, datatype, &mut issues);
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

/// Validate that a field value is in the allowed values
fn validate_field_in_constraint(
    msg: &Message,
    path: &str,
    allowed_values: &[String],
    issues: &mut Vec<Issue>,
) {
    if let Some(value) = hl7v2_core::get(msg, path) {
        if !allowed_values.contains(&value.to_string()) {
            issues.push(Issue {
                code: "VALUE_NOT_IN_CONSTRAINT",
                severity: Severity::Error,
                path: Some(path.to_string()),
                detail: format!(
                    "Value '{}' for {} is not in allowed constraint values: {:?}",
                    value, path, allowed_values
                ),
            });
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

    if let Some(value) = hl7v2_core::get(msg, &valueset.path) {
        if !valueset.codes.contains(&value.to_string()) {
            issues.push(Issue {
                code: "VALUE_NOT_IN_SET",
                severity: Severity::Error,
                path: Some(valueset.path.clone()),
                detail: format!(
                    "Value '{}' for {} is not in allowed set: {:?}",
                    value, valueset.path, valueset.codes
                ),
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
                detail: format!(
                    "Value '{}' for {} does not match expected data type {}",
                    value, datatype.path, datatype.r#type
                ),
            });
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
    if let Some(value) = hl7v2_core::get(msg, &datatype.path) {
        // First check basic data type
        if !matches_data_type(value, &datatype.r#type) {
            issues.push(Issue {
                code: "INVALID_DATA_TYPE",
                severity: Severity::Error,
                path: Some(datatype.path.clone()),
                detail: format!(
                    "Value '{}' for {} does not match expected data type {}",
                    value, datatype.path, datatype.r#type
                ),
            });
            return;
        }

        // Check length constraints
        if let Some(min_length) = datatype.min_length {
            if value.len() < min_length {
                issues.push(Issue {
                    code: "VALUE_TOO_SHORT",
                    severity: Severity::Error,
                    path: Some(datatype.path.clone()),
                    detail: format!(
                        "Value '{}' for {} is shorter than minimum length of {} characters",
                        value, datatype.path, min_length
                    ),
                });
            }
        }

        if let Some(max_length) = datatype.max_length {
            if value.len() > max_length {
                issues.push(Issue {
                    code: "VALUE_TOO_LONG",
                    severity: Severity::Error,
                    path: Some(datatype.path.clone()),
                    detail: format!(
                        "Value '{}' for {} exceeds maximum length of {} characters",
                        value, datatype.path, max_length
                    ),
                });
            }
        }

        // Check regex pattern if specified
        if let Some(pattern) = &datatype.pattern {
            if let Ok(regex) = Regex::new(pattern) {
                if !regex.is_match(value) {
                    issues.push(Issue {
                        code: "PATTERN_MISMATCH",
                        severity: Severity::Error,
                        path: Some(datatype.path.clone()),
                        detail: format!(
                            "Value '{}' for {} does not match required pattern '{}'",
                            value, datatype.path, pattern
                        ),
                    });
                }
            }
        }

        // Check format if specified
        if let Some(format) = &datatype.format {
            if !matches_format(value, format, &datatype.r#type) {
                issues.push(Issue {
                    code: "FORMAT_MISMATCH",
                    severity: Severity::Error,
                    path: Some(datatype.path.clone()),
                    detail: format!(
                        "Value '{}' for {} does not match required format '{}'",
                        value, datatype.path, format
                    ),
                });
            }
        }

        // Check checksum if specified
        if let Some(checksum) = &datatype.checksum {
            if !validate_checksum(value, checksum) {
                issues.push(Issue {
                    code: "CHECKSUM_MISMATCH",
                    severity: Severity::Error,
                    path: Some(datatype.path.clone()),
                    detail: format!("Checksum validation failed for {}", datatype.path),
                });
            }
        }
    }
}

/// Validate HL7 tables with precedence support
fn validate_hl7_tables_with_precedence(msg: &Message, profile: &Profile, issues: &mut Vec<Issue>) {
    // Create a mapping of value set names to HL7 tables
    let mut table_map: std::collections::HashMap<&str, &HL7Table> = std::collections::HashMap::new();
    for table in &profile.hl7_tables {
        table_map.insert(&table.id, table);
    }
    
    // Validate value sets with table precedence
    for valueset in &profile.valuesets {
        if let Some(table_id) = table_map.get(valueset.name.as_str()) {
            if let Some(value) = hl7v2_core::get(msg, &valueset.path) {
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
                        issues.push(Issue {
                            code: "VALUE_NOT_IN_HL7_TABLE",
                            severity: Severity::Error,
                            path: Some(valueset.path.clone()),
                            detail: format!(
                                "Value '{}' for {} is not in HL7 table {} ({})",
                                value, valueset.path, table_id.id, table_id.name
                            ),
                        });
                    }
                }
            }
        }
    }
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
                    detail: format!(
                        "Value '{}' for {} exceeds maximum length of {} characters",
                        value, length.path, max_length
                    ),
                });
            }
        }
    }
    // Note: We don't report an error if the field is missing but has a length constraint
    // That would be handled by a separate presence constraint if needed
}

/// Validate that a field value is in the allowed HL7 table
fn validate_hl7_table(msg: &Message, table: &HL7Table, profile: &Profile, issues: &mut Vec<Issue>) {
    // This function is kept for backward compatibility but the new
    // validate_hl7_tables_with_precedence function should be used instead
    // when table precedence is important
    
    // Check value sets that reference this table by name
    for valueset in &profile.valuesets {
        if valueset.name == table.id {
            if let Some(value) = hl7v2_core::get(msg, &valueset.path) {
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
                        issues.push(Issue {
                            code: "VALUE_NOT_IN_HL7_TABLE",
                            severity: Severity::Error,
                            path: Some(valueset.path.clone()),
                            detail: format!(
                                "Value '{}' for {} is not in HL7 table {} ({})",
                                value, valueset.path, table.id, table.name
                            ),
                        });
                    }
                }
            }
        }
    }
}

/// Validate temporal rule (date/time relationships)
fn validate_temporal_rule(msg: &Message, rule: &TemporalRule, issues: &mut Vec<Issue>) {
    if let (Some(before_value), Some(after_value)) = (
        hl7v2_core::get(msg, &rule.before),
        hl7v2_core::get(msg, &rule.after),
    ) {
        // Parse the date/time values
        if let (Some(before_time), Some(after_time)) =
            (parse_datetime(&before_value), parse_datetime(&after_value))
        {
            // Check if before_time should be before after_time
            let is_valid = if rule.allow_equal {
                before_time <= after_time
            } else {
                before_time < after_time
            };

            if !is_valid {
                issues.push(Issue {
                    code: "TEMPORAL_RULE_VIOLATION",
                    severity: Severity::Error,
                    path: Some(rule.before.clone()),
                    detail: format!(
                        "Value '{}' for {} should be before {} for {}",
                        before_value, rule.before, after_value, rule.after
                    ),
                });
            }
        } else {
            // Handle the case where the date/time parsing fails
            issues.push(Issue {
                code: "INVALID_DATETIME",
                severity: Severity::Error,
                path: Some(rule.before.clone()),
                detail: format!(
                    "Invalid date/time value for {} or {}",
                    rule.before, rule.after
                ),
            });
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
        let re = Regex::new(r#"field\(([^)]+)\)\.length\(\)\s*>\s*(\d+)"#).map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path = &captures[1];
            let required_length: usize = captures[2].parse().map_err(|_| ())?;

            if let Some(value) = hl7v2_core::get(msg, path) {
                if value.len() <= required_length {
                    issues.push(Issue {
                        code: "CUSTOM_RULE_VIOLATION",
                        severity: Severity::Error,
                        path: Some(path.to_string()),
                        detail: if rule.description.is_empty() {
                            format!(
                                "Field {} length {} is not greater than {}",
                                path,
                                value.len(),
                                required_length
                            )
                        } else {
                            rule.description.clone()
                        },
                    });
                }
            }
            return Ok(());
        }
    }

    // Pattern: "field(PATH) in ['A', 'B', 'C']"
    if script.contains(" in [") {
        let re = Regex::new(r#"field\(([^)]+)\)\s+in\s+\[([^\]]+)\]"#).map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path = &captures[1];
            let values_str = &captures[2];

            if let Some(value) = hl7v2_core::get(msg, path) {
                // Parse the allowed values
                let allowed_values: Vec<&str> = values_str
                    .split(',')
                    .map(|s| s.trim())
                    .map(|s| s.trim_matches('\''))
                    .collect();

                if !allowed_values.contains(&value) {
                    issues.push(Issue {
                        code: "CUSTOM_RULE_VIOLATION",
                        severity: Severity::Error,
                        path: Some(path.to_string()),
                        detail: if rule.description.is_empty() {
                            format!(
                                "Field {} value '{}' is not in allowed set {:?}",
                                path, value, allowed_values
                            )
                        } else {
                            rule.description.clone()
                        },
                    });
                }
            }
            return Ok(());
        }
    }

    // Pattern: "field(PATH).matches_regex('PATTERN')"
    if script.contains(".matches_regex(") {
        let re = Regex::new(r#"field\(([^)]+)\)\.matches_regex\('([^']+)'\)"#).map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path = &captures[1];
            let pattern = &captures[2];

            if let Some(value) = hl7v2_core::get(msg, path) {
                let regex = Regex::new(pattern).map_err(|_| ())?;
                if !regex.is_match(value) {
                    issues.push(Issue {
                        code: "CUSTOM_RULE_VIOLATION",
                        severity: Severity::Error,
                        path: Some(path.to_string()),
                        detail: if rule.description.is_empty() {
                            format!(
                                "Field {} value '{}' does not match pattern '{}'",
                                path, value, pattern
                            )
                        } else {
                            rule.description.clone()
                        },
                    });
                }
            }
            return Ok(());
        }
    }

    // Pattern: "field(PATH).starts_with('PREFIX')"
    if script.contains(".starts_with(") {
        let re = Regex::new(r#"field\(([^)]+)\)\.starts_with\('([^']+)'\)"#).map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path = &captures[1];
            let prefix = &captures[2];

            if let Some(value) = hl7v2_core::get(msg, path) {
                if !value.starts_with(prefix) {
                    issues.push(Issue {
                        code: "CUSTOM_RULE_VIOLATION",
                        severity: Severity::Error,
                        path: Some(path.to_string()),
                        detail: if rule.description.is_empty() {
                            format!(
                                "Field {} value '{}' does not start with '{}'",
                                path, value, prefix
                            )
                        } else {
                            rule.description.clone()
                        },
                    });
                }
            }
            return Ok(());
        }
    }

    // Pattern: "field(PATH).ends_with('SUFFIX')"
    if script.contains(".ends_with(") {
        let re = Regex::new(r#"field\(([^)]+)\)\.ends_with\('([^']+)'\)"#).map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path = &captures[1];
            let suffix = &captures[2];

            if let Some(value) = hl7v2_core::get(msg, path) {
                if !value.ends_with(suffix) {
                    issues.push(Issue {
                        code: "CUSTOM_RULE_VIOLATION",
                        severity: Severity::Error,
                        path: Some(path.to_string()),
                        detail: if rule.description.is_empty() {
                            format!(
                                "Field {} value '{}' does not end with '{}'",
                                path, value, suffix
                            )
                        } else {
                            rule.description.clone()
                        },
                    });
                }
            }
            return Ok(());
        }
    }

    // Pattern: "field(PATH).is_numeric()"
    if script.contains(".is_numeric()") {
        let re = Regex::new(r#"field\(([^)]+)\)\.is_numeric\(\)"#).map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path = &captures[1];

            if let Some(value) = hl7v2_core::get(msg, path) {
                if !value.chars().all(|c| c.is_ascii_digit()) {
                    issues.push(Issue {
                        code: "CUSTOM_RULE_VIOLATION",
                        severity: Severity::Error,
                        path: Some(path.to_string()),
                        detail: if rule.description.is_empty() {
                            format!("Field {} value '{}' is not numeric", path, value)
                        } else {
                            rule.description.clone()
                        },
                    });
                }
            }
            return Ok(());
        }
    }

    // Pattern: "field(PATH1) == field(PATH2)"
    if script.contains(" == field(") {
        let re = Regex::new(r#"field\(([^)]+)\)\s*==\s*field\(([^)]+)\)"#).map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path1 = &captures[1];
            let path2 = &captures[2];

            if let (Some(value1), Some(value2)) =
                (hl7v2_core::get(msg, path1), hl7v2_core::get(msg, path2))
            {
                if value1 != value2 {
                    issues.push(Issue {
                        code: "CUSTOM_RULE_VIOLATION",
                        severity: Severity::Error,
                        path: Some(path1.to_string()),
                        detail: if rule.description.is_empty() {
                            format!(
                                "Field {} value '{}' does not equal field {} value '{}'",
                                path1, value1, path2, value2
                            )
                        } else {
                            rule.description.clone()
                        },
                    });
                }
            }
            return Ok(());
        }
    }

    // Pattern: "field(PATH).is_phone_number()"
    if script.contains(".is_phone_number()") {
        let re = Regex::new(r#"field\(([^)]+)\)\.is_phone_number\(\)"#).map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path = &captures[1];

            if let Some(value) = hl7v2_core::get(msg, path) {
                if !is_phone_number(value) {
                    issues.push(Issue {
                        code: "CUSTOM_RULE_VIOLATION",
                        severity: Severity::Error,
                        path: Some(path.to_string()),
                        detail: if rule.description.is_empty() {
                            format!(
                                "Field {} value '{}' is not a valid phone number",
                                path, value
                            )
                        } else {
                            rule.description.clone()
                        },
                    });
                }
            }
            return Ok(());
        }
    }

    // Pattern: "field(PATH).is_email()"
    if script.contains(".is_email()") {
        let re = Regex::new(r#"field\(([^)]+)\)\.is_email\(\)"#).map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path = &captures[1];

            if let Some(value) = hl7v2_core::get(msg, path) {
                if !is_email(value) {
                    issues.push(Issue {
                        code: "CUSTOM_RULE_VIOLATION",
                        severity: Severity::Error,
                        path: Some(path.to_string()),
                        detail: if rule.description.is_empty() {
                            format!(
                                "Field {} value '{}' is not a valid email address",
                                path, value
                            )
                        } else {
                            rule.description.clone()
                        },
                    });
                }
            }
            return Ok(());
        }
    }

    // Pattern: "field(PATH).is_ssn()"
    if script.contains(".is_ssn()") {
        let re = Regex::new(r#"field\(([^)]+)\)\.is_ssn\(\)"#).map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path = &captures[1];

            if let Some(value) = hl7v2_core::get(msg, path) {
                if !is_ssn(value) {
                    issues.push(Issue {
                        code: "CUSTOM_RULE_VIOLATION",
                        severity: Severity::Error,
                        path: Some(path.to_string()),
                        detail: if rule.description.is_empty() {
                            format!("Field {} value '{}' is not a valid SSN", path, value)
                        } else {
                            rule.description.clone()
                        },
                    });
                }
            }
            return Ok(());
        }
    }

    // Pattern: "field(PATH).is_valid_birth_date()"
    if script.contains(".is_valid_birth_date()") {
        let re = Regex::new(r#"field\(([^)]+)\)\.is_valid_birth_date\(\)"#).map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path = &captures[1];

            if let Some(value) = hl7v2_core::get(msg, path) {
                if !is_valid_birth_date(value) {
                    issues.push(Issue {
                        code: "CUSTOM_RULE_VIOLATION",
                        severity: Severity::Error,
                        path: Some(path.to_string()),
                        detail: if rule.description.is_empty() {
                            format!("Field {} value '{}' is not a valid birth date", path, value)
                        } else {
                            rule.description.clone()
                        },
                    });
                }
            }
            return Ok(());
        }
    }

    // Pattern: "is_valid_age_range(field(PATH1), field(PATH2))"
    if script.contains("is_valid_age_range(") {
        let re = Regex::new(r#"is_valid_age_range\(field\(([^)]+)\),\s*field\(([^)]+)\)\)"#)
            .map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path1 = &captures[1];
            let path2 = &captures[2];

            if let (Some(value1), Some(value2)) =
                (hl7v2_core::get(msg, path1), hl7v2_core::get(msg, path2))
            {
                if !is_valid_age_range(value1, value2) {
                    issues.push(Issue {
                        code: "CUSTOM_RULE_VIOLATION",
                        severity: Severity::Error,
                        path: Some(path1.to_string()),
                        detail: if rule.description.is_empty() {
                            format!("Age range between {} and {} is not valid", path1, path2)
                        } else {
                            rule.description.clone()
                        },
                    });
                }
            }
            return Ok(());
        }
    }

    // Pattern: "field(PATH) between VALUE1 and VALUE2"
    if script.contains(" between ") && script.contains(" and ") {
        let re = Regex::new(r#"field\(([^)]+)\)\s+between\s+([^\s]+)\s+and\s+([^\s]+)"#)
            .map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path = &captures[1];
            let min_val = &captures[2];
            let max_val = &captures[3];

            if let Some(value) = hl7v2_core::get(msg, path) {
                if !is_within_range(value, min_val, max_val) {
                    issues.push(Issue {
                        code: "CUSTOM_RULE_VIOLATION",
                        severity: Severity::Error,
                        path: Some(path.to_string()),
                        detail: if rule.description.is_empty() {
                            format!(
                                "Field {} value '{}' is not between {} and {}",
                                path, value, min_val, max_val
                            )
                        } else {
                            rule.description.clone()
                        },
                    });
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
            if let Some(value) = hl7v2_core::get(msg, path) {
                let length_str = &rule.script[path_end + 13..];
                if let Ok(required_length) = length_str.parse::<usize>() {
                    if value.len() <= required_length {
                        issues.push(Issue {
                            code: "CUSTOM_RULE_VIOLATION",
                            severity: Severity::Error,
                            path: Some(path.to_string()),
                            detail: if rule.description.is_empty() {
                                format!(
                                    "Field {} length {} is not greater than {}",
                                    path,
                                    value.len(),
                                    required_length
                                )
                            } else {
                                rule.description.clone()
                            },
                        });
                    }
                }
            }
        }
    } else if rule.script.starts_with("field(") && rule.script.contains(") in [") {
        // Pattern: "field(PATH) in ['A', 'B', 'C']"
        if let Some(path_end) = rule.script.find(") in [") {
            let path = &rule.script[6..path_end];
            if let Some(value) = hl7v2_core::get(msg, path) {
                // Extract the allowed values
                let values_part = &rule.script[path_end + 7..];
                if values_part.ends_with("]") {
                    let values_str = &values_part[..values_part.len() - 1];
                    // Split by comma and remove quotes
                    let allowed_values: Vec<&str> = values_str
                        .split(',')
                        .map(|s| s.trim())
                        .map(|s| s.trim_matches('\''))
                        .collect();

                    if !allowed_values.contains(&value) {
                        issues.push(Issue {
                            code: "CUSTOM_RULE_VIOLATION",
                            severity: Severity::Error,
                            path: Some(path.to_string()),
                            detail: if rule.description.is_empty() {
                                format!(
                                    "Field {} value '{}' is not in allowed set {:?}",
                                    path, value, allowed_values
                                )
                            } else {
                                rule.description.clone()
                            },
                        });
                    }
                }
            }
        }
    } else if rule.script.starts_with("field(") && rule.script.contains(").matches_regex(") {
        // Pattern: "field(PATH).matches_regex('PATTERN')"
        if let Some(path_end) = rule.script.find(").matches_regex(") {
            let path = &rule.script[6..path_end];
            if let Some(value) = hl7v2_core::get(msg, path) {
                // Extract the regex pattern
                let pattern_part = &rule.script[path_end + 15..];
                if pattern_part.starts_with("'") && pattern_part.ends_with("')") {
                    let pattern = &pattern_part[1..pattern_part.len() - 2];
                    // Simple regex matching (in a real implementation, we would use regex crate)
                    if !value.contains(pattern) && pattern != ".*" {
                        // This is a very simplified check - just for demonstration
                        issues.push(Issue {
                            code: "CUSTOM_RULE_VIOLATION",
                            severity: Severity::Error,
                            path: Some(path.to_string()),
                            detail: if rule.description.is_empty() {
                                format!(
                                    "Field {} value '{}' does not match pattern '{}'",
                                    path, value, pattern
                                )
                            } else {
                                rule.description.clone()
                            },
                        });
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

    // If conditions are met, execute actions
    if conditions_met {
        for action in &rule.actions {
            execute_rule_action(msg, action, rule, profile, issues);
        }
    }
}

/// Check if a rule condition is met
fn check_rule_condition(msg: &Message, condition: &RuleCondition) -> bool {
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
            if let Some(l) = lhs {
                rhs_list.iter().any(|r| l == *r)
            } else {
                false
            }
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
                // Debug output
                println!("DEBUG: before operator - lhs: {:?}, rhs_first: {:?}", lhs, rhs_first);
                
                // Right-hand side can be either a literal value or a field path
                let rhs_value = if let Some(rhs_field) = rhs_first {
                    // Check if rhs_field is a valid field path by trying to get its value
                    println!("DEBUG: before operator - trying to get field: {}", rhs_field);
                    if let Some(rhs_val) = get_nonempty(msg, rhs_field) {
                        println!("DEBUG: before operator - found field value: {}", rhs_val);
                        Some(rhs_val)
                    } else {
                        // Treat as literal value
                        println!("DEBUG: before operator - field not found, treating as literal");
                        Some(rhs_field)
                    }
                } else {
                    None
                };
                
                // Try to parse right-hand side
                if let Some(rhs_ts) = rhs_value.and_then(parse_hl7_ts_with_precision) {
                    let result = compare_timestamps_for_before(&lhs_ts, &rhs_ts);
                    // Debug output
                    println!("DEBUG: before operator - lhs_ts: {:?}, rhs_value: {:?}, rhs_ts: {:?}, result: {}", 
                             lhs_ts, rhs_value, rhs_ts, result);
                    result
                } else {
                    println!("DEBUG: before operator - failed to parse rhs");
                    false
                }
            } else {
                println!("DEBUG: before operator - failed to parse lhs");
                false
            }
        },
        // numeric range over integers OR date range over TS
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
            if let (Some(l), Ok(lo), Ok(hi)) = (lhs, a.parse::<i64>(), b.parse::<i64>()) {
                if let Ok(li) = l.parse::<i64>() {
                    return li >= lo && li <= hi;
                }
            }
            false
        }
        _ => {
            // Unknown operator, ignore
            false
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
            if let Some(value) = hl7v2_core::get(msg, &action.field) {
                if value.is_empty() {
                    issues.push(Issue {
                        code: "CROSS_FIELD_VALIDATION_ERROR",
                        severity: Severity::Error,
                        path: Some(action.field.clone()),
                        detail: action.message.clone().unwrap_or_else(|| {
                            format!(
                                "Field {} is required by cross-field rule {}",
                                action.field, rule.id
                            )
                        }),
                    });
                }
            } else {
                issues.push(Issue {
                    code: "CROSS_FIELD_VALIDATION_ERROR",
                    severity: Severity::Error,
                    path: Some(action.field.clone()),
                    detail: action.message.clone().unwrap_or_else(|| {
                        format!(
                            "Field {} is required by cross-field rule {}",
                            action.field, rule.id
                        )
                    }),
                });
            }
        }
        "prohibit" => {
            // Check if the prohibited field exists and is not empty
            if let Some(value) = hl7v2_core::get(msg, &action.field) {
                if !value.is_empty() {
                    issues.push(Issue {
                        code: "CROSS_FIELD_VALIDATION_ERROR",
                        severity: Severity::Error,
                        path: Some(action.field.clone()),
                        detail: action.message.clone().unwrap_or_else(|| {
                            format!(
                                "Field {} is prohibited by cross-field rule {}",
                                action.field, rule.id
                            )
                        }),
                    });
                }
            }
            // If the field doesn't exist at all, that's fine (it's not present)
        }
        "validate" => {
            // Apply additional validation based on action parameters
            if let Some(value) = hl7v2_core::get(msg, &action.field) {
                // Only validate if the field is not empty
                if !value.is_empty() {
                    // Validate data type if specified
                    if let Some(datatype) = &action.datatype {
                        if !matches_data_type(value, datatype) {
                            issues.push(Issue {
                                code: "CROSS_FIELD_VALIDATION_ERROR",
                                severity: Severity::Error,
                                path: Some(action.field.clone()),
                                detail: action.message.clone().unwrap_or_else(||
                                    format!("Field {} does not match data type {} required by cross-field rule {}",
                                           action.field, datatype, rule.id)),
                            });
                        }
                    }

                    // Validate against value set if specified
                    if let Some(valueset_name) = &action.valueset {
                        // Find the value set in the profile
                        if let Some(valueset) = find_valueset_by_name(profile, valueset_name) {
                            if !valueset.codes.contains(&value.to_string()) {
                                issues.push(Issue {
                                    code: "CROSS_FIELD_VALIDATION_ERROR",
                                    severity: Severity::Error,
                                    path: Some(action.field.clone()),
                                    detail: action.message.clone().unwrap_or_else(||
                                        format!("Value '{}' for {} is not in value set {} required by cross-field rule {}",
                                               value, action.field, valueset_name, rule.id)),
                                });
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
    if let Some(context_value) = hl7v2_core::get(msg, &rule.context_field) {
        if context_value == rule.context_value {
            // Apply the validation based on validation_type
            match rule.validation_type.as_str() {
                "require" => {
                    // Check if the target field exists and is not empty
                    if let Some(value) = hl7v2_core::get(msg, &rule.target_field) {
                        if value.is_empty() {
                            issues.push(Issue {
                                code: "CONTEXTUAL_VALIDATION_ERROR",
                                severity: Severity::Error,
                                path: Some(rule.target_field.clone()),
                                detail: if rule.description.is_empty() {
                                    format!(
                                        "Field {} is required when {} equals {}",
                                        rule.target_field, rule.context_field, rule.context_value
                                    )
                                } else {
                                    rule.description.clone()
                                },
                            });
                        }
                    } else {
                        issues.push(Issue {
                            code: "CONTEXTUAL_VALIDATION_ERROR",
                            severity: Severity::Error,
                            path: Some(rule.target_field.clone()),
                            detail: if rule.description.is_empty() {
                                format!(
                                    "Field {} is required when {} equals {}",
                                    rule.target_field, rule.context_field, rule.context_value
                                )
                            } else {
                                rule.description.clone()
                            },
                        });
                    }
                }
                "prohibit" => {
                    // Check if the target field exists and is not empty
                    if let Some(value) = hl7v2_core::get(msg, &rule.target_field) {
                        if !value.is_empty() {
                            issues.push(Issue {
                                code: "CONTEXTUAL_VALIDATION_ERROR",
                                severity: Severity::Error,
                                path: Some(rule.target_field.clone()),
                                detail: if rule.description.is_empty() {
                                    format!(
                                        "Field {} is prohibited when {} equals {}",
                                        rule.target_field, rule.context_field, rule.context_value
                                    )
                                } else {
                                    rule.description.clone()
                                },
                            });
                        }
                    }
                    // If the field doesn't exist at all, that's fine (it's not present)
                }
                "validate_datatype" => {
                    // Validate target field against specified data type
                    if let Some(datatype) = rule.parameters.get("datatype") {
                        if let Some(value) = hl7v2_core::get(msg, &rule.target_field) {
                            if !matches_data_type(value, datatype) {
                                issues.push(Issue {
                                    code: "CONTEXTUAL_VALIDATION_ERROR",
                                    severity: Severity::Error,
                                    path: Some(rule.target_field.clone()),
                                    detail: if rule.description.is_empty() {
                                        format!("Field {} does not match data type {} required when {} equals {}", 
                                               rule.target_field, datatype, rule.context_field, rule.context_value)
                                    } else {
                                        rule.description.clone()
                                    },
                                });
                            }
                        }
                    }
                }
                "validate_valueset" => {
                    // Validate target field against specified value set
                    if let Some(valueset_name) = rule.parameters.get("valueset") {
                        if let Some(value) = hl7v2_core::get(msg, &rule.target_field) {
                            // Find the value set in the profile
                            if let Some(valueset) = find_valueset_by_name(profile, valueset_name) {
                                if !valueset.codes.contains(&value.to_string()) {
                                    issues.push(Issue {
                                        code: "CONTEXTUAL_VALIDATION_ERROR",
                                        severity: Severity::Error,
                                        path: Some(rule.target_field.clone()),
                                        detail: if rule.description.is_empty() {
                                            format!("Value '{}' for {} is not in value set {} required when {} equals {}", 
                                                   value, rule.target_field, valueset_name, rule.context_field, rule.context_value)
                                        } else {
                                            rule.description.clone()
                                        },
                                    });
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

/// Check if value matches the specified format
fn matches_format(value: &str, format: &str, datatype: &str) -> bool {
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

/// Validate checksum for a value
fn validate_checksum(value: &str, algorithm: &str) -> bool {
    match algorithm {
        "luhn" => validate_luhn_checksum(value),
        "mod10" => validate_mod10_checksum(value),
        _ => true, // Unknown algorithm, assume valid
    }
}

/// Validate Luhn checksum (used for credit cards, etc.)
fn validate_luhn_checksum(value: &str) -> bool {
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
fn validate_mod10_checksum(value: &str) -> bool {
    // This is essentially the same as Luhn for our purposes
    validate_luhn_checksum(value)
}

/// Find a value set by name within a profile
fn find_valueset_by_name<'a>(profile: &'a Profile, name: &str) -> Option<&'a ValueSet> {
    profile
        .valuesets
        .iter()
        .find(|valueset| valueset.name == name)
}

/// Return HL7 value only if non-empty after trim.
#[inline]
fn get_nonempty<'a>(msg: &'a Message, path: &str) -> Option<&'a str> {
    hl7v2_core::get(msg, path).and_then(|s| {
        let t = s.trim();
        if t.is_empty() { None } else { Some(t) }
    })
}

/// Minimal HL7 TS parser supporting: YYYYMMDDHHMMSS, YYYYMMDDHHMM, YYYYMMDD (no TZ), and YYYYMMDD.
fn parse_hl7_ts(s: &str) -> Option<NaiveDateTime> {
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
    if s.len() == 8 {
        if let Ok(d) = NaiveDate::parse_from_str(s, "%Y%m%d") {
            return Some(d.and_hms_opt(0, 0, 0)?);
        }
    }
    None
}

/// Parse datetime with precision information
#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedTimestamp {
    datetime: NaiveDateTime,
    precision: TimestampPrecision,
}

/// Precision levels for timestamps
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum TimestampPrecision {
    Year,
    Month,
    Day,
    Hour,
    Minute,
    Second,
}

/// Parse HL7 TS with precision information
fn parse_hl7_ts_with_precision(s: &str) -> Option<ParsedTimestamp> {
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
    if s.len() == 8 {
        if let Ok(date) = NaiveDate::parse_from_str(s, "%Y%m%d") {
            return Some(ParsedTimestamp {
                datetime: date.and_hms_opt(0, 0, 0)?,
                precision: TimestampPrecision::Day,
            });
        }
    }
    
    // Try year-month format
    if s.len() == 6 {
        if let Ok(date) = NaiveDate::parse_from_str(&format!("{}01", s), "%Y%m%d") {
            return Some(ParsedTimestamp {
                datetime: date.and_hms_opt(0, 0, 0)?,
                precision: TimestampPrecision::Month,
            });
        }
    }
    
    // Try year only format
    if s.len() == 4 {
        if let Ok(date) = NaiveDate::parse_from_str(&format!("{}0101", s), "%Y%m%d") {
            return Some(ParsedTimestamp {
                datetime: date.and_hms_opt(0, 0, 0)?,
                precision: TimestampPrecision::Year,
            });
        }
    }
    
    None
}

/// Compare two timestamps with partial precision handling
/// For "before" comparisons with partial precision:
/// - If comparing 20230101 (date) with 20230101120000 (datetime), 
///   we should consider them "equal" for the date part, not treat the date as 00:00:00
fn compare_timestamps_for_before(a: &ParsedTimestamp, b: &ParsedTimestamp) -> bool {
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
fn truncate_to_precision(dt: &NaiveDateTime, precision: TimestampPrecision) -> NaiveDateTime {
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
fn parse_datetime(value: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    // Try YYYYMMDDHHMMSS format
    if value.len() == 14 {
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(value, "%Y%m%d%H%M%S") {
            return Some(dt.and_utc());
        }
    }

    // Try YYYYMMDD format
    if value.len() == 8 {
        if let Ok(date) = chrono::NaiveDate::parse_from_str(value, "%Y%m%d") {
            return Some(date.and_hms_opt(0, 0, 0)?.and_utc());
        }
    }

    // Try YYYY-MM-DD format
    if value.len() == 10 {
        if let Ok(date) = chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d") {
            return Some(date.and_hms_opt(0, 0, 0)?.and_utc());
        }
    }

    None
}

/// Check if a value matches the expected HL7 data type
fn matches_data_type(value: &str, datatype: &str) -> bool {
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
    value
        .chars()
        .all(|c| c.is_alphabetic() || c.is_whitespace() || c == '-' || c == '\'' || c == '.')
}

/// Check if value is an extended ID (contains identifier characters)
fn is_extended_id(value: &str) -> bool {
    is_identifier(value)
}

/// Check if value is a hierarchic designator (contains identifier characters)
fn is_hierarchic_designator(value: &str) -> bool {
    is_identifier(value)
}

/// Check if value is a valid phone number (basic validation)
fn is_phone_number(value: &str) -> bool {
    // Remove common phone number formatting characters
    let cleaned: String = value.chars().filter(|c| c.is_ascii_digit()).collect();

    // Basic phone number validation (7-15 digits)
    cleaned.len() >= 7 && cleaned.len() <= 15 && cleaned.chars().all(|c| c.is_ascii_digit())
}

/// Check if value is a valid email address (basic validation)
fn is_email(value: &str) -> bool {
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
fn is_ssn(value: &str) -> bool {
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
fn is_valid_birth_date(value: &str) -> bool {
    if !is_date(value) {
        return false;
    }

    // Check if date is not in the future
    let current_date = chrono::Utc::now().format("%Y%m%d").to_string();
    value <= current_date.as_str()
}

/// Check if two dates represent a valid age range (e.g., birth date vs admission date)
fn is_valid_age_range(birth_date: &str, reference_date: &str) -> bool {
    if !is_date(birth_date) || !is_date(reference_date) {
        return false;
    }

    // Birth date should be before or equal to reference date
    birth_date <= reference_date
}

/// Check if a value matches a complex pattern with multiple conditions
fn matches_complex_pattern(value: &str, patterns: &[&str]) -> bool {
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
fn validate_mathematical_relationship(value1: &str, value2: &str, operator: &str) -> bool {
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

/// Check if a value is within a specified range (inclusive)
fn is_within_range(value: &str, min: &str, max: &str) -> bool {
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

#[cfg(test)]
mod tests;
