use super::*;

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
        let mut inheritance_stack = Vec::new();
        let parent_profile = load_profile_with_inheritance_recursive(
            parent_name,
            &profile_loader,
            &mut inheritance_stack,
        )?;
        return Ok(merge_profiles(parent_profile, profile));
    }

    Ok(profile)
}

/// Recursively load parent profiles
fn load_profile_with_inheritance_recursive<F>(
    parent_name: &str,
    profile_loader: &F,
    inheritance_stack: &mut Vec<String>,
) -> Result<Profile, ProfileLoadError>
where
    F: Fn(&str) -> Result<Profile, ProfileLoadError>,
{
    if let Some(first_seen) = inheritance_stack
        .iter()
        .position(|seen_parent| seen_parent == parent_name)
    {
        let mut cycle = inheritance_stack[first_seen..].to_vec();
        cycle.push(parent_name.to_string());
        return Err(ProfileLoadError::InheritanceCycle(cycle.join(" -> ")));
    }

    inheritance_stack.push(parent_name.to_string());
    let result = (|| {
        let parent_profile = profile_loader(parent_name)?;

        // If the parent also has a parent, recursively load and merge it
        if let Some(grandparent_name) = parent_profile.parent.clone() {
            let grandparent_profile = load_profile_with_inheritance_recursive(
                &grandparent_name,
                profile_loader,
                inheritance_stack,
            )?;
            return Ok(merge_profiles(grandparent_profile, parent_profile));
        }

        Ok(parent_profile)
    })();
    inheritance_stack.pop();
    result
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

/// Merge segment specifications, with child specs overriding parent specs by ID
fn merge_segment_specs(parent: Vec<SegmentSpec>, child: Vec<SegmentSpec>) -> Vec<SegmentSpec> {
    let mut result: Vec<SegmentSpec> = parent;

    // Add child segments, replacing any with the same ID.
    for child_segment in child {
        if let Some(pos) = result.iter().position(|s| s.id == child_segment.id) {
            result[pos] = child_segment;
        } else {
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
