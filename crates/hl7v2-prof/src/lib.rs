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
    pub cross_field_rules: Vec<CrossFieldRule>,
    #[serde(default)]
    pub custom_rules: Vec<CustomRule>,
    #[serde(default)]
    pub hl7_tables: Vec<HL7Table>,
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

/// HL7 Table definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HL7Table {
    pub id: String,        // Table ID like "HL70001"
    pub name: String,      // Table name like "Administrative Sex"
    pub version: String,   // HL7 version like "2.5.1"
    pub codes: Vec<HL7TableEntry>,
}

/// Entry in an HL7 table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HL7TableEntry {
    pub value: String,     // The code value
    pub description: String, // Description of the code
    #[serde(default)]
    pub status: String,    // "A" (active), "D" (deprecated), "R" (restricted)
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
fn load_profile_with_inheritance_recursive<F>(parent_name: &str, profile_loader: &F) -> Result<Profile, Error>
where
    F: Fn(&str) -> Result<Profile, Error>,
{
    let parent_profile = profile_loader(parent_name)?;
    
    // If the parent also has a parent, recursively load and merge it
    if let Some(grandparent_name) = &parent_profile.parent {
        let grandparent_profile = load_profile_with_inheritance_recursive(grandparent_name, profile_loader)?;
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
        cross_field_rules: merge_cross_field_rules(parent.cross_field_rules, child.cross_field_rules),
        custom_rules: merge_custom_rules(parent.custom_rules, child.custom_rules),
        hl7_tables: merge_hl7_tables(parent.hl7_tables, child.hl7_tables),
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
fn merge_length_constraints(parent: Vec<LengthConstraint>, child: Vec<LengthConstraint>) -> Vec<LengthConstraint> {
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
fn merge_datatype_constraints(parent: Vec<DataTypeConstraint>, child: Vec<DataTypeConstraint>) -> Vec<DataTypeConstraint> {
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

/// Merge cross-field rules, with child rules overriding parent rules with same ID
fn merge_cross_field_rules(parent: Vec<CrossFieldRule>, child: Vec<CrossFieldRule>) -> Vec<CrossFieldRule> {
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
    
    // Validate length constraints
    for length in &profile.lengths {
        validate_length_constraint(msg, length, &mut issues);
    }
    
    // Validate HL7 tables
    for table in &profile.hl7_tables {
        validate_hl7_table(msg, table, profile, &mut issues);
    }
    
    // Validate cross-field rules
    for rule in &profile.cross_field_rules {
        validate_cross_field_rule(msg, rule, &mut issues);
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
fn validate_field_in_constraint(msg: &Message, path: &str, allowed_values: &[String], issues: &mut Vec<Issue>) {
    if let Some(value) = hl7v2_core::get(msg, path) {
        if !allowed_values.contains(&value.to_string()) {
            issues.push(Issue {
                code: "VALUE_NOT_IN_CONSTRAINT",
                severity: Severity::Error,
                path: Some(path.to_string()),
                detail: format!("Value '{}' for {} is not in allowed constraint values: {:?}", 
                               value, path, allowed_values),
            });
        }
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

/// Validate that a field value is in the allowed HL7 table
fn validate_hl7_table(msg: &Message, table: &HL7Table, profile: &Profile, issues: &mut Vec<Issue>) {
    // Check value sets that reference this table by name
    for valueset in &profile.valuesets {
        if valueset.name == table.id {
            if let Some(value) = hl7v2_core::get(msg, &valueset.path) {
                // Only validate if the field is not empty
                if !value.is_empty() {
                    // Check if the value exists in the table
                    let is_valid = table.codes.iter().any(|entry| {
                        entry.value == value && 
                        (entry.status.is_empty() || entry.status == "A" || entry.status == "active")
                    });
                    
                    if !is_valid {
                        issues.push(Issue {
                            code: "VALUE_NOT_IN_HL7_TABLE",
                            severity: Severity::Error,
                            path: Some(valueset.path.clone()),
                            detail: format!("Value '{}' for {} is not in HL7 table {} ({})", 
                                           value, valueset.path, table.id, table.name),
                        });
                    }
                }
            }
        }
    }
}

/// Validate cross-field rule
fn validate_cross_field_rule(msg: &Message, rule: &CrossFieldRule, issues: &mut Vec<Issue>) {
    // Check if all conditions are met
    let conditions_met = rule.conditions.iter().all(|condition| {
        check_rule_condition(msg, condition)
    });
    
    // If conditions are met, execute actions
    if conditions_met {
        for action in &rule.actions {
            execute_rule_action(msg, action, rule, issues);
        }
    }
}

/// Check if a rule condition is met
fn check_rule_condition(msg: &Message, condition: &RuleCondition) -> bool {
    match condition.operator.as_str() {
        "eq" => {
            if let Some(value) = hl7v2_core::get(msg, &condition.field) {
                if let Some(expected) = &condition.value {
                    return value == expected;
                }
            }
            false
        },
        "ne" => {
            if let Some(value) = hl7v2_core::get(msg, &condition.field) {
                if let Some(expected) = &condition.value {
                    return value != expected;
                }
            }
            false
        },
        "gt" => {
            if let Some(value) = hl7v2_core::get(msg, &condition.field) {
                if let Some(expected) = &condition.value {
                    // Try to compare as numbers first
                    if let (Ok(val_num), Ok(exp_num)) = (value.parse::<f64>(), expected.parse::<f64>()) {
                        return val_num > exp_num;
                    }
                    // For date strings, try to compare lexicographically
                    // This works for YYYYMMDD format
                    return value > expected;
                }
            }
            false
        },
        "lt" => {
            if let Some(value) = hl7v2_core::get(msg, &condition.field) {
                if let Some(expected) = &condition.value {
                    // Try to compare as numbers first
                    if let (Ok(val_num), Ok(exp_num)) = (value.parse::<f64>(), expected.parse::<f64>()) {
                        return val_num < exp_num;
                    }
                    // For date strings, try to compare lexicographically
                    // This works for YYYYMMDD format
                    return value < expected;
                }
            }
            false
        },
        "ge" => {
            if let Some(value) = hl7v2_core::get(msg, &condition.field) {
                if let Some(expected) = &condition.value {
                    // Try to compare as numbers first
                    if let (Ok(val_num), Ok(exp_num)) = (value.parse::<f64>(), expected.parse::<f64>()) {
                        return val_num >= exp_num;
                    }
                    // For date strings, try to compare lexicographically
                    // This works for YYYYMMDD format
                    return value >= expected;
                }
            }
            false
        },
        "le" => {
            if let Some(value) = hl7v2_core::get(msg, &condition.field) {
                if let Some(expected) = &condition.value {
                    // Try to compare as numbers first
                    if let (Ok(val_num), Ok(exp_num)) = (value.parse::<f64>(), expected.parse::<f64>()) {
                        return val_num <= exp_num;
                    }
                    // For date strings, try to compare lexicographically
                    // This works for YYYYMMDD format
                    return value <= expected;
                }
            }
            false
        },
        "in" => {
            if let Some(value) = hl7v2_core::get(msg, &condition.field) {
                if let Some(values) = &condition.values {
                    return values.contains(&value.to_string());
                }
            }
            false
        },
        "contains" => {
            if let Some(value) = hl7v2_core::get(msg, &condition.field) {
                if let Some(expected) = &condition.value {
                    return value.contains(expected);
                }
            }
            false
        },
        "exists" => {
            hl7v2_core::get(msg, &condition.field).is_some()
        },
        "missing" => {
            hl7v2_core::get(msg, &condition.field).is_none()
        },
        _ => false, // Unknown operator
    }
}

/// Execute a rule action
fn execute_rule_action(msg: &Message, action: &RuleAction, rule: &CrossFieldRule, issues: &mut Vec<Issue>) {
    match action.action.as_str() {
        "require" => {
            // Check if the required field exists and is not empty
            if let Some(value) = hl7v2_core::get(msg, &action.field) {
                if value.is_empty() {
                    issues.push(Issue {
                        code: "CROSS_FIELD_VALIDATION_ERROR",
                        severity: Severity::Error,
                        path: Some(action.field.clone()),
                        detail: action.message.clone().unwrap_or_else(|| 
                            format!("Field {} is required by cross-field rule {}", 
                                   action.field, rule.id)),
                    });
                }
            } else {
                issues.push(Issue {
                        code: "CROSS_FIELD_VALIDATION_ERROR",
                        severity: Severity::Error,
                        path: Some(action.field.clone()),
                        detail: action.message.clone().unwrap_or_else(|| 
                            format!("Field {} is required by cross-field rule {}", 
                               action.field, rule.id)),
                });
            }
        },
        "prohibit" => {
            // Check if the prohibited field exists and is not empty
            if let Some(value) = hl7v2_core::get(msg, &action.field) {
                if !value.is_empty() {
                    issues.push(Issue {
                        code: "CROSS_FIELD_VALIDATION_ERROR",
                        severity: Severity::Error,
                        path: Some(action.field.clone()),
                        detail: action.message.clone().unwrap_or_else(|| 
                            format!("Field {} is prohibited by cross-field rule {}", 
                                   action.field, rule.id)),
                    });
                }
            }
            // If the field doesn't exist at all, that's fine (it's not present)
        },
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
                        if let Some(valueset) = find_valueset_by_name(rule, valueset_name) {
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
        },
        _ => {
            // Unknown action, ignore
        }
    }
}

/// Find a value set by name within a rule's context
fn find_valueset_by_name(_rule: &CrossFieldRule, _name: &str) -> Option<ValueSet> {
    // In a more complete implementation, this would search for the value set
    // For now, we'll return None to indicate not found
    None
}

/// Find an HL7 table by ID
fn find_hl7_table_by_id<'a>(profile: &'a Profile, table_id: &str) -> Option<&'a HL7Table> {
    profile.hl7_tables.iter().find(|table| table.id == table_id)
}

/// Validate custom rule
fn validate_custom_rule(msg: &Message, rule: &CustomRule, issues: &mut Vec<Issue>) {
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
                                format!("Field {} length {} is not greater than {}", 
                                       path, value.len(), required_length)
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
                                format!("Field {} value '{}' is not in allowed set {:?}", 
                                       path, value, allowed_values)
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
                                format!("Field {} value '{}' does not match pattern '{}'", 
                                       path, value, pattern)
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