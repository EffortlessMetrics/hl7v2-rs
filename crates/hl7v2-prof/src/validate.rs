use hl7v2_core::Message;
use regex::Regex;
use hl7v2_validation::{
    Issue, check_rule_condition, matches_format, parse_datetime, validate_checksum, validate_data_type,
};
use crate::model::*;
use crate::expressions::{evaluate_custom_rule_script, evaluate_custom_rule_simple};

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
    // Use the get function from hl7v2-core to retrieve the value
    if let Some(value) = hl7v2_core::get(msg, path) {
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
    if let Some(value) = hl7v2_core::get(msg, path) {
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

    if let Some(value) = hl7v2_core::get(msg, &valueset.path) {
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
    if let Some(value) = hl7v2_core::get(msg, &datatype.path) {
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
    if let Some(value) = hl7v2_core::get(msg, &datatype.path) {
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
    if let Some(value) = hl7v2_core::get(msg, &length.path) {
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
#[allow(dead_code)]
pub fn validate_hl7_table(msg: &Message, table: &HL7Table, profile: &Profile, issues: &mut Vec<Issue>) {
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
        hl7v2_core::get(msg, &rule.before),
        hl7v2_core::get(msg, &rule.after),
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
    action: &hl7v2_validation::RuleAction,
    rule: &CrossFieldRule,
    profile: &Profile,
    issues: &mut Vec<Issue>,
) {
    match action.action.as_str() {
        "require" => {
            // Check if the required field exists and is not empty
            if let Some(value) = hl7v2_core::get(msg, &action.field) {
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
            if let Some(value) = hl7v2_core::get(msg, &action.field) {
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
            if let Some(value) = hl7v2_core::get(msg, &action.field) {
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
    if let Some(context_value) = hl7v2_core::get(msg, &rule.context_field) {
        if context_value == rule.context_value {
            // Apply the validation based on validation_type
            match rule.validation_type.as_str() {
                "require" => {
                    // Check if the target field exists and is not empty
                    if let Some(value) = hl7v2_core::get(msg, &rule.target_field) {
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
                    if let Some(value) = hl7v2_core::get(msg, &rule.target_field) {
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
                        if let Some(value) = hl7v2_core::get(msg, &rule.target_field) {
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
                        if let Some(value) = hl7v2_core::get(msg, &rule.target_field) {
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
