use regex::Regex;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

use super::*;

pub fn lint_profile_yaml(yaml: &str) -> ProfileLintReport {
    let root = match serde_yaml::from_str::<serde_yaml::Value>(yaml) {
        Ok(root) => root,
        Err(err) => {
            return ProfileLintReport::from_issues(vec![ProfileLintIssue::error(
                "yaml_parse_error",
                None,
                profile_lint_yaml_error_message("profile YAML could not be parsed", err.location()),
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
                profile_lint_yaml_error_message(
                    "profile YAML could not be loaded as a profile",
                    err.location(),
                ),
            ));
            return ProfileLintReport::from_issues(issues);
        }
    };

    lint_profile(&profile, &mut issues);
    ProfileLintReport::from_issues(issues)
}

fn profile_lint_yaml_error_message(
    summary: &'static str,
    location: Option<serde_yaml::Location>,
) -> String {
    match location {
        Some(location) => format!(
            "{summary} at line {}, column {}",
            location.line(),
            location.column()
        ),
        None => summary.to_string(),
    }
}

/// Build a structured explanation report for a loaded profile.
///
/// `profile_name` is recorded in the report as producer-supplied identity,
/// typically a relative path or display name. The source YAML is used only for
/// the profile hash; raw profile text is not embedded in the report.
#[must_use]
pub fn explain_profile(
    profile_name: impl Into<String>,
    profile_yaml: &str,
    profile: &Profile,
    lint_report: &ProfileLintReport,
) -> ProfileExplainReport {
    let required_fields: Vec<ProfileExplainRequiredField> = profile
        .constraints
        .iter()
        .filter(|constraint| constraint.required)
        .map(|constraint| ProfileExplainRequiredField {
            path: constraint.path.clone(),
            conditional: constraint.when.is_some(),
        })
        .collect();

    let table_code_counts: HashMap<&str, usize> = profile
        .hl7_tables
        .iter()
        .map(|table| (table.id.as_str(), table.codes.len()))
        .collect();

    let datatype_rules = profile
        .datatypes
        .iter()
        .map(|datatype| ProfileExplainDatatypeRule {
            path: datatype.path.clone(),
            datatype: datatype.r#type.clone(),
            kind: "simple".to_string(),
            pattern: None,
            min_length: None,
            max_length: None,
            format: None,
            checksum: None,
        })
        .chain(
            profile
                .advanced_datatypes
                .iter()
                .map(|datatype| ProfileExplainDatatypeRule {
                    path: datatype.path.clone(),
                    datatype: datatype.r#type.clone(),
                    kind: "advanced".to_string(),
                    pattern: datatype.pattern.clone(),
                    min_length: datatype.min_length,
                    max_length: datatype.max_length,
                    format: datatype.format.clone(),
                    checksum: datatype.checksum.clone(),
                }),
        )
        .collect();

    ProfileExplainReport {
        profile: profile_name.into(),
        profile_sha256: compute_profile_sha256(profile_yaml),
        message_structure: profile.message_structure.clone(),
        version: profile.version.clone(),
        message_type: profile.message_type.clone(),
        parent: profile.parent.clone(),
        summary: ProfileExplainSummary {
            segment_count: profile.segments.len(),
            required_field_count: required_fields.len(),
            field_constraint_count: profile.constraints.len(),
            length_rule_count: profile.lengths.len(),
            datatype_rule_count: profile.datatypes.len(),
            advanced_datatype_rule_count: profile.advanced_datatypes.len(),
            value_set_count: profile.valuesets.len(),
            cross_field_rule_count: profile.cross_field_rules.len(),
            temporal_rule_count: profile.temporal_rules.len(),
            contextual_rule_count: profile.contextual_rules.len(),
            custom_rule_count: profile.custom_rules.len(),
            hl7_table_count: profile.hl7_tables.len(),
        },
        segments: profile
            .segments
            .iter()
            .map(|segment| ProfileExplainSegment {
                id: segment.id.clone(),
            })
            .collect(),
        required_fields,
        field_constraints: profile
            .constraints
            .iter()
            .map(|constraint| {
                let (component_min, component_max) = constraint
                    .components
                    .as_ref()
                    .map(|components| (components.min, components.max))
                    .unwrap_or((None, None));
                let allowed_values = constraint.r#in.clone().unwrap_or_default();
                ProfileExplainConstraint {
                    path: constraint.path.clone(),
                    required: constraint.required,
                    conditional: constraint.when.is_some(),
                    component_min,
                    component_max,
                    allowed_value_count: allowed_values.len(),
                    allowed_values,
                    pattern: constraint.pattern.clone(),
                }
            })
            .collect(),
        length_rules: profile
            .lengths
            .iter()
            .map(|length| ProfileExplainLengthRule {
                path: length.path.clone(),
                max: length.max,
                policy: length.policy.clone(),
            })
            .collect(),
        datatype_rules,
        value_sets: profile
            .valuesets
            .iter()
            .map(|valueset| {
                let table_code_count = table_code_counts
                    .get(valueset.name.as_str())
                    .copied()
                    .unwrap_or(0);
                let source = if !valueset.codes.is_empty() {
                    "inline"
                } else if table_code_count > 0 {
                    "hl7_table"
                } else {
                    "empty"
                };
                ProfileExplainValueSet {
                    name: valueset.name.clone(),
                    path: valueset.path.clone(),
                    source: source.to_string(),
                    inline_code_count: valueset.codes.len(),
                    table_code_count,
                }
            })
            .collect(),
        rules: ProfileExplainRules {
            cross_field: profile
                .cross_field_rules
                .iter()
                .map(|rule| ProfileExplainRule {
                    id: rule.id.clone(),
                    description: rule.description.clone(),
                })
                .collect(),
            temporal: profile
                .temporal_rules
                .iter()
                .map(|rule| ProfileExplainRule {
                    id: rule.id.clone(),
                    description: rule.description.clone(),
                })
                .collect(),
            contextual: profile
                .contextual_rules
                .iter()
                .map(|rule| ProfileExplainRule {
                    id: rule.id.clone(),
                    description: rule.description.clone(),
                })
                .collect(),
            custom: profile
                .custom_rules
                .iter()
                .map(|rule| ProfileExplainRule {
                    id: rule.id.clone(),
                    description: rule.description.clone(),
                })
                .collect(),
        },
        hl7_tables: profile
            .hl7_tables
            .iter()
            .map(|table| ProfileExplainTable {
                id: table.id.clone(),
                name: table.name.clone(),
                version: table.version.clone(),
                code_count: table.codes.len(),
            })
            .collect(),
        table_precedence: profile.table_precedence.clone(),
        expression_guardrails: ProfileExplainExpressionGuardrails {
            max_depth: profile.expression_guardrails.max_depth,
            max_length: profile.expression_guardrails.max_length,
            allow_custom_scripts: profile.expression_guardrails.allow_custom_scripts,
        },
        lint: ProfileExplainLintSummary {
            valid: lint_report.valid,
            error_count: lint_report.error_count,
            warning_count: lint_report.warning_count,
            issue_count: lint_report.issue_count,
            ignored_or_unsupported: lint_report
                .issues
                .iter()
                .filter(|issue| profile_lint_issue_is_ignored_or_unsupported(issue))
                .cloned()
                .collect(),
        },
    }
}

fn profile_lint_issue_is_ignored_or_unsupported(issue: &ProfileLintIssue) -> bool {
    issue.code.starts_with("unknown_")
        || issue.code.contains("unsupported")
        || issue.message.contains("ignored")
}

fn compute_profile_sha256(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("{:x}", hasher.finalize())
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
