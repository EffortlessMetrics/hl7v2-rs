use regex::Regex;
use hl7v2_core::Message;
use hl7v2_validation::{
    Issue, is_email, is_phone_number, is_ssn, is_valid_age_range, is_valid_birth_date, is_within_range,
};
use crate::model::CustomRule;

/// Evaluate custom rule script with proper expression parsing
pub fn evaluate_custom_rule_script(
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
        let re = Regex::new(r#"field\(([^)]+)\)\.matches_regex\('([^']+)'\)"#).map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path = &captures[1];
            let pattern = &captures[2];

            if let Some(value) = hl7v2_core::get(msg, path) {
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
        let re = Regex::new(r#"field\(([^)]+)\)\.starts_with\('([^']+)'\)"#).map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path = &captures[1];
            let prefix = &captures[2];

            if let Some(value) = hl7v2_core::get(msg, path) {
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
        let re = Regex::new(r#"field\(([^)]+)\)\.ends_with\('([^']+)'\)"#).map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path = &captures[1];
            let suffix = &captures[2];

            if let Some(value) = hl7v2_core::get(msg, path) {
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
        let re = Regex::new(r#"field\(([^)]+)\)\.is_numeric\(\)"#).map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path = &captures[1];

            if let Some(value) = hl7v2_core::get(msg, path) {
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
        let re = Regex::new(r#"field\(([^)]+)\)\s*==\s*field\(([^)]+)\)"#).map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path1 = &captures[1];
            let path2 = &captures[2];

            if let (Some(value1), Some(value2)) =
                (hl7v2_core::get(msg, path1), hl7v2_core::get(msg, path2))
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
        let re = Regex::new(r#"field\(([^)]+)\)\.is_phone_number\(\)"#).map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path = &captures[1];

            if let Some(value) = hl7v2_core::get(msg, path) {
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
        let re = Regex::new(r#"field\(([^)]+)\)\.is_email\(\)"#).map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path = &captures[1];

            if let Some(value) = hl7v2_core::get(msg, path) {
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
        let re = Regex::new(r#"field\(([^)]+)\)\.is_ssn\(\)"#).map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path = &captures[1];

            if let Some(value) = hl7v2_core::get(msg, path) {
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
        let re = Regex::new(r#"field\(([^)]+)\)\.is_valid_birth_date\(\)"#).map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path = &captures[1];

            if let Some(value) = hl7v2_core::get(msg, path) {
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
        let re = Regex::new(r#"is_valid_age_range\(field\(([^)]+)\),\s*field\(([^)]+)\)\)"#)
            .map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path1 = &captures[1];
            let path2 = &captures[2];

            if let (Some(value1), Some(value2)) =
                (hl7v2_core::get(msg, path1), hl7v2_core::get(msg, path2))
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
        let re = Regex::new(r#"field\(([^)]+)\)\s+between\s+([^\s]+)\s+and\s+([^\s]+)"#)
            .map_err(|_| ())?;
        if let Some(captures) = re.captures(script) {
            let path = &captures[1];
            let min_val = &captures[2];
            let max_val = &captures[3];

            if let Some(value) = hl7v2_core::get(msg, path) {
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
pub fn evaluate_custom_rule_simple(msg: &Message, rule: &CustomRule, issues: &mut Vec<Issue>) {
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
            if let Some(value) = hl7v2_core::get(msg, path) {
                // Extract the allowed values
                let values_part = &rule.script[path_end + 7..];
                if let Some(values_str) = values_part.strip_suffix("]") {
                    // Split by comma and remove quotes
                    let allowed_values: Vec<&str> = values_str
                        .split(',')
                        .map(|s| s.trim())
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
            if let Some(value) = hl7v2_core::get(msg, path) {
                // Extract the regex pattern
                let pattern_part = &rule.script[path_end + 15..];
                if pattern_part.starts_with("'") && pattern_part.ends_with("')") {
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
