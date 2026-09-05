use crate::model::{Atom, Comp, Field, Message, Rep, Segment};
use regex::Regex;

use super::*;

pub fn validate(msg: &Message, profile: &Profile) -> Vec<Issue> {
    let mut issues = Vec::new();

    validate_segment_specs(msg, profile, &mut issues);

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

            if let Some(pattern) = &constraint.pattern {
                validate_field_pattern_constraint(msg, &constraint.path, pattern, &mut issues);
            }

            if let Some(components) = &constraint.components {
                validate_field_component_constraint(msg, &constraint.path, components, &mut issues);
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

fn validate_segment_specs(msg: &Message, profile: &Profile, issues: &mut Vec<Issue>) {
    for segment in &profile.segments {
        let mut occurrence_count = 0;

        for _ in msg
            .segments
            .iter()
            .filter(|actual| actual.id_str() == segment.id)
        {
            occurrence_count += 1;
            if occurrence_count > 1 && !segment.repetition {
                issues.push(Issue::error(
                    "SEGMENT_REPETITION_NOT_ALLOWED",
                    Some(format!("{}[{occurrence_count}]", segment.id)),
                    format!(
                        "Segment {} repeats but the profile does not permit repetition",
                        segment.id
                    ),
                ));
            }
        }

        if segment.required && occurrence_count == 0 {
            issues.push(Issue::error(
                "MISSING_REQUIRED_SEGMENT",
                Some(segment.id.clone()),
                format!("Required segment {} is missing", segment.id),
            ));
        }
    }
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RepetitionSemanticsCase {
    validation_path: &'static str,
    collector: &'static str,
}

#[cfg(test)]
const PROFILE_REPETITION_SEMANTICS: &[RepetitionSemanticsCase] = &[
    RepetitionSemanticsCase {
        validation_path: "segments.repetition",
        collector: "segment_occurrence_count",
    },
    RepetitionSemanticsCase {
        validation_path: "constraints.required",
        collector: "required_path_has_value",
    },
    RepetitionSemanticsCase {
        validation_path: "constraints.when",
        collector: "condition_text_values",
    },
    RepetitionSemanticsCase {
        validation_path: "constraints.in",
        collector: "path_text_values",
    },
    RepetitionSemanticsCase {
        validation_path: "constraints.pattern",
        collector: "path_text_values",
    },
    RepetitionSemanticsCase {
        validation_path: "constraints.components",
        collector: "field_component_counts",
    },
    RepetitionSemanticsCase {
        validation_path: "valuesets.inline",
        collector: "path_text_values",
    },
    RepetitionSemanticsCase {
        validation_path: "valuesets.hl7_tables_with_precedence",
        collector: "path_text_values",
    },
    RepetitionSemanticsCase {
        validation_path: "valuesets.legacy_hl7_table",
        collector: "path_text_values",
    },
    RepetitionSemanticsCase {
        validation_path: "datatypes.basic",
        collector: "path_text_values",
    },
    RepetitionSemanticsCase {
        validation_path: "datatypes.advanced",
        collector: "path_text_values",
    },
    RepetitionSemanticsCase {
        validation_path: "lengths",
        collector: "path_text_values",
    },
    RepetitionSemanticsCase {
        validation_path: "cross_field.conditions",
        collector: "check_rule_condition",
    },
    RepetitionSemanticsCase {
        validation_path: "cross_field.actions.require",
        collector: "required_path_has_value",
    },
    RepetitionSemanticsCase {
        validation_path: "cross_field.actions.prohibit",
        collector: "path_text_values",
    },
    RepetitionSemanticsCase {
        validation_path: "cross_field.actions.validate",
        collector: "path_text_values",
    },
    RepetitionSemanticsCase {
        validation_path: "temporal_rules",
        collector: "first_condition_pair_matching",
    },
    RepetitionSemanticsCase {
        validation_path: "contextual.context",
        collector: "condition_text_values",
    },
    RepetitionSemanticsCase {
        validation_path: "contextual.require",
        collector: "required_path_has_value",
    },
    RepetitionSemanticsCase {
        validation_path: "contextual.prohibit",
        collector: "path_text_values",
    },
    RepetitionSemanticsCase {
        validation_path: "contextual.validate_datatype",
        collector: "path_text_values",
    },
    RepetitionSemanticsCase {
        validation_path: "contextual.validate_valueset",
        collector: "path_text_values",
    },
    RepetitionSemanticsCase {
        validation_path: "custom_rules.unary_predicates",
        collector: "first_condition_value_matching",
    },
    RepetitionSemanticsCase {
        validation_path: "custom_rules.pair_predicates",
        collector: "first_condition_pair_matching",
    },
    // MSH-1 is delimiter metadata, not a stored repeated field.
    RepetitionSemanticsCase {
        validation_path: "helpers.condition_text_values.msh_field_separator",
        collector: "justified_msh_field_separator_scalar_lookup",
    },
    RepetitionSemanticsCase {
        validation_path: "helpers.path_text_values.msh_field_separator",
        collector: "justified_msh_field_separator_scalar_lookup",
    },
];

/// Validate that a required field is present
fn validate_field_required(msg: &Message, path: &str, issues: &mut Vec<Issue>) {
    if !required_path_has_value(msg, path) {
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

            return condition_text_values(msg, field_path)
                .into_iter()
                .any(|actual_value| actual_value == expected_value);
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
    if !required_path_has_value(msg, &full_path) {
        issues.push(Issue::error(
            "MISSING_REQUIRED_FIELD",
            Some(full_path),
            format!("Required MSH field {} is missing", path),
        ));
    }
}

fn required_path_has_value(msg: &Message, path: &str) -> bool {
    let Ok(path) = crate::query::path::parse_located_path(path) else {
        return false;
    };
    let Some(segment) = required_segment(msg, &path) else {
        return false;
    };

    if path.path.is_msh() && path.path.field == 1 {
        return true;
    }

    let Some(field) = required_field(segment, &path.path) else {
        return false;
    };

    field_has_required_value(
        field,
        path.path.repetition,
        path.path.component,
        path.path.subcomponent,
    )
}

fn required_segment<'a>(
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

fn required_field<'a>(segment: &'a Segment, path: &crate::query::path::Path) -> Option<&'a Field> {
    let field_index = if path.is_msh() {
        path.msh_stored_field_index()?
    } else {
        path.field.checked_sub(1)?
    };

    segment.fields.get(field_index)
}

fn field_has_required_value(
    field: &Field,
    repetition: Option<usize>,
    component: Option<usize>,
    subcomponent: Option<usize>,
) -> bool {
    if let Some(repetition) = repetition {
        return repetition
            .checked_sub(1)
            .and_then(|index| field.reps.get(index))
            .is_some_and(|rep| rep_has_required_value(rep, component, subcomponent));
    }

    field
        .reps
        .iter()
        .any(|rep| rep_has_required_value(rep, component, subcomponent))
}

fn rep_has_required_value(
    rep: &Rep,
    component: Option<usize>,
    subcomponent: Option<usize>,
) -> bool {
    if let Some(component) = component {
        return component
            .checked_sub(1)
            .and_then(|index| rep.comps.get(index))
            .is_some_and(|comp| comp_has_required_value(comp, subcomponent));
    }

    rep.comps
        .iter()
        .any(|comp| comp_has_required_value(comp, subcomponent))
}

fn comp_has_required_value(comp: &Comp, subcomponent: Option<usize>) -> bool {
    if let Some(subcomponent) = subcomponent {
        return subcomponent
            .checked_sub(1)
            .and_then(|index| comp.subs.get(index))
            .is_some_and(atom_has_required_value);
    }

    comp.subs.iter().any(atom_has_required_value)
}

fn atom_has_required_value(atom: &Atom) -> bool {
    matches!(atom, Atom::Text(text) if !text.is_empty())
}

fn condition_text_values<'a>(msg: &'a Message, path: &str) -> Vec<&'a str> {
    let Ok(path) = crate::query::path::parse_located_path(path) else {
        return Vec::new();
    };

    if path.path.is_msh() && path.path.field == 1 {
        return crate::query::get_located(msg, &path).into_iter().collect();
    }

    let Some(segment) = required_segment(msg, &path) else {
        return Vec::new();
    };
    let Some(field) = required_field(segment, &path.path) else {
        return Vec::new();
    };

    field_condition_text_values(
        field,
        path.path.repetition,
        path.path.component,
        path.path.subcomponent,
    )
}

fn first_condition_value_matching<'a>(
    msg: &'a Message,
    path: &str,
    mut predicate: impl FnMut(&str) -> bool,
) -> Option<&'a str> {
    condition_text_values(msg, path)
        .into_iter()
        .find(|value| predicate(value))
}

fn first_condition_pair_matching<'a>(
    msg: &'a Message,
    path1: &str,
    path2: &str,
    mut predicate: impl FnMut(&str, &str) -> bool,
) -> Option<(&'a str, &'a str)> {
    let values1 = condition_text_values(msg, path1);
    let values2 = condition_text_values(msg, path2);

    values1
        .into_iter()
        .zip(values2)
        .find(|(value1, value2)| predicate(value1, value2))
}

fn field_condition_text_values(
    field: &Field,
    repetition: Option<usize>,
    component: Option<usize>,
    subcomponent: Option<usize>,
) -> Vec<&str> {
    if let Some(repetition) = repetition {
        return repetition
            .checked_sub(1)
            .and_then(|index| field.reps.get(index))
            .and_then(|rep| rep_condition_text_value(rep, component, subcomponent))
            .into_iter()
            .collect();
    }

    field
        .reps
        .iter()
        .filter_map(|rep| rep_condition_text_value(rep, component, subcomponent))
        .collect()
}

fn rep_condition_text_value(
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

/// Validate that a field value is in the allowed values
fn validate_field_in_constraint(
    msg: &Message,
    path: &str,
    allowed_values: &[String],
    issues: &mut Vec<Issue>,
) {
    if let Some(value) = path_text_values(msg, path).into_iter().find(|value| {
        !allowed_values
            .iter()
            .any(|allowed| allowed.as_str() == *value)
    }) {
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

/// Validate that a field value matches a regex pattern constraint
fn validate_field_pattern_constraint(
    msg: &Message,
    path: &str,
    pattern: &str,
    issues: &mut Vec<Issue>,
) {
    let Some(regex) = crate::conformance::regex_cache::get(pattern) else {
        return;
    };

    if let Some(value) = path_text_values(msg, path)
        .into_iter()
        .find(|value| !regex.is_match(value))
    {
        issues.push(Issue::error(
            "PATTERN_MISMATCH",
            Some(path.to_string()),
            format!(
                "Value '{}' for {} does not match required pattern '{}'",
                value, path, pattern
            ),
        ));
    }
}

fn validate_field_component_constraint(
    msg: &Message,
    path: &str,
    components: &ComponentConstraint,
    issues: &mut Vec<Issue>,
) {
    let Ok(path_ref) = crate::query::path::parse_located_path(path) else {
        return;
    };
    let Some(segment) = required_segment(msg, &path_ref) else {
        return;
    };
    let Some(field) = required_field(segment, &path_ref.path) else {
        return;
    };

    if let Some((count, min)) = field_component_counts(field, path_ref.path.repetition)
        .into_iter()
        .find_map(|count| {
            components
                .min
                .filter(|min| count < *min)
                .map(|min| (count, min))
        })
    {
        issues.push(Issue::error(
            "TOO_FEW_COMPONENTS",
            Some(path.to_string()),
            format!(
                "Field {} has {} components, fewer than minimum of {}",
                path, count, min
            ),
        ));
        return;
    }

    if let Some((count, max)) = field_component_counts(field, path_ref.path.repetition)
        .into_iter()
        .find_map(|count| {
            components
                .max
                .filter(|max| count > *max)
                .map(|max| (count, max))
        })
    {
        issues.push(Issue::error(
            "TOO_MANY_COMPONENTS",
            Some(path.to_string()),
            format!(
                "Field {} has {} components, more than maximum of {}",
                path, count, max
            ),
        ));
    }
}

fn field_component_counts(field: &Field, repetition: Option<usize>) -> Vec<usize> {
    if let Some(repetition) = repetition {
        return repetition
            .checked_sub(1)
            .and_then(|index| field.reps.get(index))
            .map(|rep| vec![rep.comps.len()])
            .unwrap_or_default();
    }

    field.reps.iter().map(|rep| rep.comps.len()).collect()
}

/// Validate that a field value is in the allowed value set
fn validate_value_set(msg: &Message, valueset: &ValueSet, issues: &mut Vec<Issue>) {
    // If codes is empty, this valueset references an HL7 table
    // Validation will happen in validate_hl7_tables_with_precedence instead
    if valueset.codes.is_empty() {
        return;
    }

    if let Some(value) = path_text_values(msg, &valueset.path)
        .into_iter()
        .find(|value| !valueset.codes.iter().any(|code| code.as_str() == *value))
    {
        issues.push(Issue::error(
            "VALUE_NOT_IN_SET",
            Some(valueset.path.clone()),
            format!(
                "Value '{}' for {} is not in allowed set: {:?}",
                value, valueset.path, valueset.codes
            ),
        ));
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
    if let Some(value) = path_text_values(msg, &datatype.path)
        .into_iter()
        .find(|value| !validate_data_type(value, &datatype.r#type))
    {
        issues.push(Issue::error(
            "INVALID_DATA_TYPE",
            Some(datatype.path.clone()),
            format!(
                "Value '{}' for {} does not match expected data type {}",
                value, datatype.path, datatype.r#type
            ),
        ));
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
    let values = path_text_values(msg, &datatype.path);

    // First check basic data type
    if let Some(value) = values
        .iter()
        .copied()
        .find(|value| !validate_data_type(value, &datatype.r#type))
    {
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
    if let Some(min_length) = datatype.min_length
        && let Some(value) = values
            .iter()
            .copied()
            .find(|value| value.len() < min_length)
    {
        issues.push(Issue::error(
            "VALUE_TOO_SHORT",
            Some(datatype.path.clone()),
            format!(
                "Value '{}' for {} is shorter than minimum length of {} characters",
                value, datatype.path, min_length
            ),
        ));
    }

    if let Some(max_length) = datatype.max_length
        && let Some(value) = values
            .iter()
            .copied()
            .find(|value| value.len() > max_length)
    {
        issues.push(Issue::error(
            "VALUE_TOO_LONG",
            Some(datatype.path.clone()),
            format!(
                "Value '{}' for {} exceeds maximum length of {} characters",
                value, datatype.path, max_length
            ),
        ));
    }

    // Check regex pattern if specified
    if let Some(pattern) = &datatype.pattern
        && let Some(regex) = crate::conformance::regex_cache::get(pattern)
        && let Some(value) = values.iter().copied().find(|value| !regex.is_match(value))
    {
        issues.push(Issue::error(
            "PATTERN_MISMATCH",
            Some(datatype.path.clone()),
            format!(
                "Value '{}' for {} does not match required pattern '{}'",
                value, datatype.path, pattern
            ),
        ));
    }

    // Check format if specified
    if let Some(format) = &datatype.format
        && let Some(value) = values
            .iter()
            .copied()
            .find(|value| !matches_format(value, format, &datatype.r#type))
    {
        issues.push(Issue::error(
            "FORMAT_MISMATCH",
            Some(datatype.path.clone()),
            format!(
                "Value '{}' for {} does not match required format '{}'",
                value, datatype.path, format
            ),
        ));
    }

    // Check checksum if specified
    if let Some(checksum) = &datatype.checksum
        && let Some(_value) = values
            .iter()
            .copied()
            .find(|value| !validate_checksum(value, checksum))
    {
        issues.push(Issue::error(
            "CHECKSUM_MISMATCH",
            Some(datatype.path.clone()),
            format!("Checksum validation failed for {}", datatype.path),
        ));
    }
}

/// Validate HL7 tables with precedence support
fn validate_hl7_tables_with_precedence(msg: &Message, profile: &Profile, issues: &mut Vec<Issue>) {
    // Create a mapping of HL7 table IDs to table definitions.
    let mut table_map: std::collections::HashMap<&str, &HL7Table> =
        std::collections::HashMap::new();
    for table in &profile.hl7_tables {
        table_map.insert(&table.id, table);
    }

    // Validate value sets with table precedence
    for valueset in &profile.valuesets {
        if let Some(table_id) = table_map.get(valueset_hl7_table_id(valueset)) {
            if let Some(value) = path_text_values(msg, &valueset.path)
                .into_iter()
                .filter(|value| !value.is_empty())
                .find(|value| {
                    !table_id.codes.iter().any(|entry| {
                        entry.value == *value
                            && (entry.status.is_empty()
                                || entry.status == "A"
                                || entry.status == "active")
                    })
                })
            {
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

fn valueset_hl7_table_id(valueset: &ValueSet) -> &str {
    valueset
        .hl7_table
        .as_deref()
        .filter(|table_id| !table_id.trim().is_empty())
        .unwrap_or(valueset.name.as_str())
}

/// Validate that a field value does not exceed the maximum length
fn validate_length_constraint(msg: &Message, length: &LengthConstraint, issues: &mut Vec<Issue>) {
    if let Some(max_length) = length.max {
        if let Some(value) = path_text_values(msg, &length.path)
            .into_iter()
            .find(|value| value.len() > max_length)
        {
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
    // Note: We don't report an error if the field is missing but has a length constraint
    // That would be handled by a separate presence constraint if needed
}

fn path_text_values<'a>(msg: &'a Message, path: &str) -> Vec<&'a str> {
    let Ok(path) = crate::query::path::parse_located_path(path) else {
        return Vec::new();
    };

    if path.path.is_msh() && path.path.field == 1 {
        return crate::query::get_located(msg, &path).into_iter().collect();
    }

    let Some(segment) = required_segment(msg, &path) else {
        return Vec::new();
    };
    let Some(field) = required_field(segment, &path.path) else {
        return Vec::new();
    };

    field_text_values(
        field,
        path.path.repetition,
        path.path.component,
        path.path.subcomponent,
    )
}

fn field_text_values(
    field: &Field,
    repetition: Option<usize>,
    component: Option<usize>,
    subcomponent: Option<usize>,
) -> Vec<&str> {
    let mut values = Vec::new();

    if let Some(repetition) = repetition {
        if let Some(rep) = repetition
            .checked_sub(1)
            .and_then(|index| field.reps.get(index))
        {
            collect_rep_text_values(&mut values, rep, component, subcomponent);
        }
        return values;
    }

    for rep in &field.reps {
        collect_rep_text_values(&mut values, rep, component, subcomponent);
    }

    values
}

fn collect_rep_text_values<'a>(
    values: &mut Vec<&'a str>,
    rep: &'a Rep,
    component: Option<usize>,
    subcomponent: Option<usize>,
) {
    if let Some(component) = component {
        if let Some(comp) = component
            .checked_sub(1)
            .and_then(|index| rep.comps.get(index))
        {
            collect_comp_text_values(values, comp, subcomponent);
        }
        return;
    }

    for comp in &rep.comps {
        collect_comp_text_values(values, comp, subcomponent);
    }
}

fn collect_comp_text_values<'a>(
    values: &mut Vec<&'a str>,
    comp: &'a Comp,
    subcomponent: Option<usize>,
) {
    if let Some(subcomponent) = subcomponent {
        if let Some(Atom::Text(text)) = subcomponent
            .checked_sub(1)
            .and_then(|index| comp.subs.get(index))
        {
            values.push(text.as_str());
        }
        return;
    }

    for atom in &comp.subs {
        if let Atom::Text(text) = atom {
            values.push(text.as_str());
        }
    }
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
            if let Some(value) = path_text_values(msg, &valueset.path)
                .into_iter()
                .find(|value| {
                    !value.is_empty()
                        && !table.codes.iter().any(|entry| {
                            entry.value == *value
                                && (entry.status.is_empty()
                                    || entry.status == "A"
                                    || entry.status == "active")
                        })
                })
            {
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

/// Parse a temporal tolerance string such as `30s`, `5m`, `2h`, or `1d` into a
/// [`chrono::Duration`].
///
/// The accepted grammar is a non-negative integer followed by a single-character
/// unit suffix (`s`econds, `m`inutes, `h`ours, `d`ays). Returns `None` for
/// empty, negative, non-numeric, unknown-unit, or overflowing values so callers
/// can decide how to treat a malformed value.
pub(super) fn parse_tolerance(raw: &str) -> Option<chrono::Duration> {
    // Split off the final *character* (not byte) so a multi-byte unit such as
    // "5€" cannot land mid-codepoint and panic in a byte-indexed split.
    let mut chars = raw.trim().chars();
    let unit = chars.next_back()?;
    let amount: i64 = chars.as_str().parse().ok()?;
    if amount < 0 {
        return None;
    }
    match unit {
        's' => chrono::Duration::try_seconds(amount),
        'm' => chrono::Duration::try_minutes(amount),
        'h' => chrono::Duration::try_hours(amount),
        'd' => chrono::Duration::try_days(amount),
        _ => None,
    }
}

/// Validate temporal rule (date/time relationships)
fn validate_temporal_rule(msg: &Message, rule: &TemporalRule, issues: &mut Vec<Issue>) {
    // A configured tolerance widens the allowed window: `before` may be up to
    // `tolerance` later than `after` (for example, to absorb clock skew between
    // two feeds) and still be considered compliant. A malformed tolerance is
    // treated as zero here; `lint_temporal_rule` surfaces the bad value at
    // authoring time so it is not silently ignored.
    let tolerance = rule
        .tolerance
        .as_deref()
        .and_then(parse_tolerance)
        .unwrap_or_else(chrono::Duration::zero);

    // Is this ordered pair a violation? A reverse delta up to and *including*
    // the tolerance is compliant; `allow_equal` governs only exact equality at
    // `after` (the zero-delta baseline), independent of the tolerance window.
    // Shared by the search predicate and the final check so the two cannot
    // drift. If adding the tolerance would overflow the calendar, the deadline
    // is clamped to the maximum representable instant so the full tolerance is
    // preserved rather than silently dropped.
    let violates = |before_time: chrono::DateTime<chrono::Utc>,
                    after_time: chrono::DateTime<chrono::Utc>| {
        let deadline = after_time
            .checked_add_signed(tolerance)
            .unwrap_or(chrono::DateTime::<chrono::Utc>::MAX_UTC);
        before_time > deadline || (!rule.allow_equal && before_time == after_time)
    };

    if let Some((before_value, after_value)) = first_condition_pair_matching(
        msg,
        &rule.before,
        &rule.after,
        |before_value, after_value| {
            let (Some(before_time), Some(after_time)) =
                (parse_datetime(before_value), parse_datetime(after_value))
            else {
                return true;
            };

            violates(before_time, after_time)
        },
    ) {
        let (Some(before_time), Some(after_time)) =
            (parse_datetime(before_value), parse_datetime(after_value))
        else {
            issues.push(Issue::error(
                "INVALID_DATETIME",
                Some(rule.before.clone()),
                format!(
                    "Invalid date/time value '{}' for {} or '{}' for {}",
                    before_value, rule.before, after_value, rule.after
                ),
            ));
            return;
        };

        if violates(before_time, after_time) {
            issues.push(Issue::error(
                "TEMPORAL_RULE_VIOLATION",
                Some(rule.before.clone()),
                format!(
                    "Value '{}' for {} should be before {} for {}",
                    before_value, rule.before, after_value, rule.after
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

            if let Some(value) =
                first_condition_value_matching(msg, path, |value| value.len() <= required_length)
            {
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
            return Ok(());
        }
    }

    // Pattern: "field(PATH) in ['A', 'B', 'C']"
    if script.contains(" in [") {
        let re = Regex::new(r"field\(([^)]+)\)\s+in\s+\[([^\]]+)\]").map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path = &captures[1];
            let values_str = &captures[2];

            // Parse the allowed values
            let allowed_values: Vec<&str> = values_str
                .split(',')
                .map(str::trim)
                .map(|s| s.trim_matches('\''))
                .collect();

            if let Some(value) =
                first_condition_value_matching(msg, path, |value| !allowed_values.contains(&value))
            {
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
            return Ok(());
        }
    }

    // Pattern: "field(PATH).matches_regex('PATTERN')"
    if script.contains(".matches_regex(") {
        let re = Regex::new(r"field\(([^)]+)\)\.matches_regex\('([^']+)'\)").map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path = &captures[1];
            let pattern = &captures[2];

            let regex = crate::conformance::regex_cache::get(pattern).ok_or(())?;
            if let Some(value) =
                first_condition_value_matching(msg, path, |value| !regex.is_match(value))
            {
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
            return Ok(());
        }
    }

    // Pattern: "field(PATH).starts_with('PREFIX')"
    if script.contains(".starts_with(") {
        let re = Regex::new(r"field\(([^)]+)\)\.starts_with\('([^']+)'\)").map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path = &captures[1];
            let prefix = &captures[2];

            if let Some(value) =
                first_condition_value_matching(msg, path, |value| !value.starts_with(prefix))
            {
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
            return Ok(());
        }
    }

    // Pattern: "field(PATH).ends_with('SUFFIX')"
    if script.contains(".ends_with(") {
        let re = Regex::new(r"field\(([^)]+)\)\.ends_with\('([^']+)'\)").map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path = &captures[1];
            let suffix = &captures[2];

            if let Some(value) =
                first_condition_value_matching(msg, path, |value| !value.ends_with(suffix))
            {
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
            return Ok(());
        }
    }

    // Pattern: "field(PATH).is_numeric()"
    if script.contains(".is_numeric()") {
        let re = Regex::new(r"field\(([^)]+)\)\.is_numeric\(\)").map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path = &captures[1];

            if let Some(value) = first_condition_value_matching(msg, path, |value| {
                !value.chars().all(|c| c.is_ascii_digit())
            }) {
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
            return Ok(());
        }
    }

    // Pattern: "field(PATH1) == field(PATH2)"
    if script.contains(" == field(") {
        let re = Regex::new(r"field\(([^)]+)\)\s*==\s*field\(([^)]+)\)").map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path1 = &captures[1];
            let path2 = &captures[2];

            if let Some((value1, value2)) =
                first_condition_pair_matching(msg, path1, path2, |value1, value2| value1 != value2)
            {
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
            return Ok(());
        }
    }

    // Pattern: "field(PATH).is_phone_number()"
    if script.contains(".is_phone_number()") {
        let re = Regex::new(r"field\(([^)]+)\)\.is_phone_number\(\)").map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path = &captures[1];

            if let Some(value) =
                first_condition_value_matching(msg, path, |value| !is_phone_number(value))
            {
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
            return Ok(());
        }
    }

    // Pattern: "field(PATH).is_email()"
    if script.contains(".is_email()") {
        let re = Regex::new(r"field\(([^)]+)\)\.is_email\(\)").map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path = &captures[1];

            if let Some(value) = first_condition_value_matching(msg, path, |value| !is_email(value))
            {
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
            return Ok(());
        }
    }

    // Pattern: "field(PATH).is_ssn()"
    if script.contains(".is_ssn()") {
        let re = Regex::new(r"field\(([^)]+)\)\.is_ssn\(\)").map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path = &captures[1];

            if let Some(value) = first_condition_value_matching(msg, path, |value| !is_ssn(value)) {
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
            return Ok(());
        }
    }

    // Pattern: "field(PATH).is_valid_birth_date()"
    if script.contains(".is_valid_birth_date()") {
        let re = Regex::new(r"field\(([^)]+)\)\.is_valid_birth_date\(\)").map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path = &captures[1];

            if let Some(value) =
                first_condition_value_matching(msg, path, |value| !is_valid_birth_date(value))
            {
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

            if let Some((value1, value2)) =
                first_condition_pair_matching(msg, path1, path2, |value1, value2| {
                    !is_valid_age_range(value1, value2)
                })
            {
                issues.push(Issue::error(
                    "CUSTOM_RULE_VIOLATION",
                    Some(path1.to_string()),
                    if rule.description.is_empty() {
                        format!(
                            "Age range between {} value '{}' and {} value '{}' is not valid",
                            path1, value1, path2, value2
                        )
                    } else {
                        rule.description.clone()
                    },
                ));
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

            if let Some(value) = first_condition_value_matching(msg, path, |value| {
                !is_within_range(value, min_val, max_val)
            }) {
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
            let length_str = &rule.script[path_end + 13..];
            if let Ok(required_length) = length_str.parse::<usize>()
                && let Some(value) = first_condition_value_matching(msg, path, |value| {
                    value.len() <= required_length
                })
            {
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
    } else if rule.script.starts_with("field(") && rule.script.contains(") in [") {
        // Pattern: "field(PATH) in ['A', 'B', 'C']"
        if let Some(path_end) = rule.script.find(") in [") {
            let path = &rule.script[6..path_end];
            // Extract the allowed values
            let values_part = &rule.script[path_end + 7..];
            if let Some(values_str) = values_part.strip_suffix("]") {
                // Split by comma and remove quotes
                let allowed_values: Vec<&str> = values_str
                    .split(',')
                    .map(str::trim)
                    .map(|s| s.trim_matches('\''))
                    .collect();

                if let Some(value) = first_condition_value_matching(msg, path, |value| {
                    !allowed_values.contains(&value)
                }) {
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
    } else if rule.script.starts_with("field(") && rule.script.contains(").matches_regex(") {
        // Pattern: "field(PATH).matches_regex('PATTERN')"
        if let Some(path_end) = rule.script.find(").matches_regex(") {
            let path = &rule.script[6..path_end];
            // `).matches_regex(` is 16 bytes. Skip the complete marker so
            // `pattern_part` begins at the opening quote rather than `(`.
            let pattern_part = &rule.script[path_end + 16..];
            // A degenerate `pattern_part == "')"` satisfies both delimiter
            // checks but cannot yield the interior slice. `get` fails closed.
            if pattern_part.starts_with('\'')
                && pattern_part.ends_with("')")
                && let Some(pattern) = pattern_part.get(1..pattern_part.len() - 2)
            {
                // Simple regex matching (in a real implementation, we would use regex crate)
                if let Some(value) = first_condition_value_matching(msg, path, |value| {
                    !value.contains(pattern) && pattern != ".*"
                }) {
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
        "require" if !required_path_has_value(msg, &action.field) => {
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
        "prohibit"
            if path_text_values(msg, &action.field)
                .into_iter()
                .any(|value| !value.is_empty()) =>
        {
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
        "require" | "prohibit" => {}
        "validate" => {
            // Apply additional validation based on action parameters
            let values = path_text_values(msg, &action.field);

            // Validate data type if specified
            if let Some(datatype) = &action.datatype
                && let Some(_value) = values
                    .iter()
                    .copied()
                    .find(|value| !value.is_empty() && !validate_data_type(value, datatype))
            {
                issues.push(Issue::error(
                    "CROSS_FIELD_VALIDATION_ERROR",
                    Some(action.field.clone()),
                    action.message.clone().unwrap_or_else(|| {
                        format!(
                            "Field {} does not match data type {} required by cross-field rule {}",
                            action.field, datatype, rule.id
                        )
                    }),
                ));
            }

            // Validate against value set if specified
            if let Some(valueset_name) = &action.valueset
                && let Some(valueset) = find_valueset_by_name(profile, valueset_name)
                && let Some(value) = values.iter().copied().find(|value| {
                    !value.is_empty() && !valueset.codes.iter().any(|code| code.as_str() == *value)
                })
            {
                issues.push(Issue::error(
                    "CROSS_FIELD_VALIDATION_ERROR",
                    Some(action.field.clone()),
                    action.message.clone().unwrap_or_else(|| {
                        format!(
                            "Value '{}' for {} is not in value set {} required by cross-field rule {}",
                            value, action.field, valueset_name, rule.id
                        )
                    }),
                ));
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
    if !condition_text_values(msg, &rule.context_field)
        .into_iter()
        .any(|context_value| context_value == rule.context_value)
    {
        return;
    }

    match rule.validation_type.as_str() {
        "require" if !required_path_has_value(msg, &rule.target_field) => {
            issues.push(contextual_issue(
                rule,
                format!(
                    "Field {} is required when {} equals {}",
                    rule.target_field, rule.context_field, rule.context_value
                ),
            ));
        }
        "prohibit"
            if path_text_values(msg, &rule.target_field)
                .into_iter()
                .any(|value| !value.is_empty()) =>
        {
            issues.push(contextual_issue(
                rule,
                format!(
                    "Field {} is prohibited when {} equals {}",
                    rule.target_field, rule.context_field, rule.context_value
                ),
            ));
        }
        "require" | "prohibit" => {}
        "validate_datatype" => {
            if let Some(datatype) = rule.parameters.get("datatype")
                && path_text_values(msg, &rule.target_field)
                    .into_iter()
                    .any(|value| !validate_data_type(value, datatype))
            {
                issues.push(contextual_issue(
                    rule,
                    format!(
                        "Field {} does not match data type {} required when {} equals {}",
                        rule.target_field, datatype, rule.context_field, rule.context_value
                    ),
                ));
            }
        }
        "validate_valueset" => {
            if let Some(valueset_name) = rule.parameters.get("valueset")
                && let Some(valueset) = find_valueset_by_name(profile, valueset_name)
                && let Some(value) = path_text_values(msg, &rule.target_field)
                    .into_iter()
                    .find(|value| !valueset.codes.iter().any(|code| code.as_str() == *value))
            {
                issues.push(contextual_issue(
                    rule,
                    format!(
                        "Value '{}' for {} is not in value set {} required when {} equals {}",
                        value,
                        rule.target_field,
                        valueset_name,
                        rule.context_field,
                        rule.context_value
                    ),
                ));
            }
        }
        _ => {}
    }
}

fn contextual_issue(rule: &ContextualRule, fallback: String) -> Issue {
    Issue::error(
        "CONTEXTUAL_VALIDATION_ERROR",
        Some(rule.target_field.clone()),
        if rule.description.is_empty() {
            fallback
        } else {
            rule.description.clone()
        },
    )
}

/// Find a value set by name within a profile
fn find_valueset_by_name<'a>(profile: &'a Profile, name: &str) -> Option<&'a ValueSet> {
    profile
        .valuesets
        .iter()
        .find(|valueset| valueset.name == name)
}

#[cfg(test)]
mod repetition_semantics_tests {
    use std::collections::BTreeSet;

    use super::PROFILE_REPETITION_SEMANTICS;

    #[test]
    fn profile_repetition_semantics_matrix_is_explicit() {
        const ALLOWED_COLLECTORS: &[&str] = &[
            "path_text_values",
            "condition_text_values",
            "first_condition_value_matching",
            "first_condition_pair_matching",
            "required_path_has_value",
            "field_component_counts",
            "check_rule_condition",
            "segment_occurrence_count",
            "justified_msh_field_separator_scalar_lookup",
        ];
        const EXPECTED_PATHS: &[&str] = &[
            "segments.repetition",
            "constraints.required",
            "constraints.when",
            "constraints.in",
            "constraints.pattern",
            "constraints.components",
            "valuesets.inline",
            "valuesets.hl7_tables_with_precedence",
            "valuesets.legacy_hl7_table",
            "datatypes.basic",
            "datatypes.advanced",
            "lengths",
            "cross_field.conditions",
            "cross_field.actions.require",
            "cross_field.actions.prohibit",
            "cross_field.actions.validate",
            "temporal_rules",
            "contextual.context",
            "contextual.require",
            "contextual.prohibit",
            "contextual.validate_datatype",
            "contextual.validate_valueset",
            "custom_rules.unary_predicates",
            "custom_rules.pair_predicates",
            "helpers.condition_text_values.msh_field_separator",
            "helpers.path_text_values.msh_field_separator",
        ];

        let mut seen = BTreeSet::new();
        for case in PROFILE_REPETITION_SEMANTICS {
            assert!(
                ALLOWED_COLLECTORS.contains(&case.collector),
                "unexpected collector {} for {}",
                case.collector,
                case.validation_path
            );
            assert_ne!(
                case.collector, "crate::query::get",
                "scalar query lookup is not a profile repetition collector"
            );
            assert_ne!(
                case.collector, "scalar_query_get",
                "scalar query lookup is not a profile repetition collector"
            );
            assert!(
                seen.insert(case.validation_path),
                "duplicate repetition semantics path {}",
                case.validation_path
            );
        }

        for &path in EXPECTED_PATHS {
            assert!(
                seen.contains(path),
                "missing repetition semantics path {path}"
            );
        }
    }

    #[test]
    fn profile_validation_query_lookups_are_classified() {
        let source = include_str!("validation.rs");
        let scalar_query_get = concat!("crate::query::", "get(");

        assert!(
            !source.contains(scalar_query_get),
            "profile validation must use repetition-aware collectors, not scalar query get"
        );

        let located_query_get = concat!("crate::query::", "get_located(");
        let located_lookup_count = source.matches(located_query_get).count();
        let justified_lookup_count = PROFILE_REPETITION_SEMANTICS
            .iter()
            .filter(|case| case.collector == "justified_msh_field_separator_scalar_lookup")
            .count();

        assert_eq!(
            located_lookup_count, justified_lookup_count,
            "profile validation get_located fallbacks must be classified in the repetition semantics matrix"
        );
    }
}
