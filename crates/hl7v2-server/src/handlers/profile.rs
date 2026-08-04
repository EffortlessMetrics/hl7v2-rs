use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Serialize;
use std::sync::Arc;

use crate::handlers::error::AppError;
use crate::models::{ProfileExplainRequest, ProfileLintRequest, ProfileTestRequest};
use crate::server::AppState;
use axum::extract::State;

use super::schema_versions::{
    requested_profile_explain_report_schema_version, requested_profile_lint_report_schema_version,
    requested_profile_test_report_schema_version,
};

const SERVER_TOOL_NAME: &str = "hl7v2-server";
const INLINE_PROFILE: &str = "<inline-profile>";
const INLINE_FIXTURES: &str = "<inline-fixtures>";

pub(crate) async fn profile_lint_handler(
    Json(request): Json<ProfileLintRequest>,
) -> Result<impl IntoResponse, AppError> {
    let report_schema_version =
        requested_profile_lint_report_schema_version(request.report_schema_version)?;
    let report = hl7v2::lint_profile_yaml(&request.profile);
    let response = if report_schema_version == 2 {
        evidence_json(
            &report.to_v2(SERVER_TOOL_NAME, env!("CARGO_PKG_VERSION")),
            "profile lint report",
        )?
    } else {
        evidence_json(&report, "profile lint report")?
    };

    Ok((StatusCode::OK, Json(response)))
}

pub(crate) async fn profile_explain_handler(
    Json(request): Json<ProfileExplainRequest>,
) -> Result<impl IntoResponse, AppError> {
    let report_schema_version =
        requested_profile_explain_report_schema_version(request.report_schema_version)?;
    let profile_name = safe_profile_name(request.profile_name.as_deref())?;
    let profile = hl7v2::load_profile_checked(&request.profile).map_err(AppError::from)?;
    let lint_report = hl7v2::lint_profile_yaml(&request.profile);
    let report = hl7v2::explain_profile(profile_name, &request.profile, &profile, &lint_report);
    let response = if report_schema_version == 2 {
        evidence_json(
            &report.to_v2(SERVER_TOOL_NAME, env!("CARGO_PKG_VERSION")),
            "profile explain report",
        )?
    } else {
        evidence_json(&report, "profile explain report")?
    };

    Ok((StatusCode::OK, Json(response)))
}

pub(crate) async fn profile_test_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ProfileTestRequest>,
) -> Result<impl IntoResponse, AppError> {
    let report_schema_version =
        requested_profile_test_report_schema_version(request.report_schema_version)?;
    let profile = hl7v2::load_profile_checked(&request.profile).map_err(AppError::from)?;
    for fixture in &request.fixtures {
        if let Err(error) = super::enforce_decoded_message_size_if_valid_mllp(
            fixture.message.as_bytes(),
            fixture.mllp_framed,
            state.max_message_size,
        ) {
            return Ok(error.into_response());
        }
    }
    let report = profile_test_report_from_inline_fixtures(&request.fixtures, &profile)?;
    let response = if report_schema_version == 2 {
        evidence_json(
            &report.to_v2(SERVER_TOOL_NAME, env!("CARGO_PKG_VERSION")),
            "profile test report",
        )?
    } else {
        evidence_json(&report, "profile test report")?
    };

    Ok((StatusCode::OK, Json(response)).into_response())
}

fn evidence_json<T: Serialize>(value: &T, label: &str) -> Result<serde_json::Value, AppError> {
    serde_json::to_value(value)
        .map_err(|error| AppError::Internal(format!("could not serialize {label}: {error}")))
}

fn safe_profile_name(profile_name: Option<&str>) -> Result<String, AppError> {
    let Some(profile_name) = profile_name else {
        return Ok(INLINE_PROFILE.to_string());
    };
    validate_safe_flat_label(profile_name, "profile_name")?;
    Ok(profile_name.to_string())
}

fn validate_safe_flat_label(value: &str, field: &str) -> Result<(), AppError> {
    validate_label_common(value, field)?;
    if value
        .bytes()
        .any(|byte| matches!(byte, b'/' | b'\\' | b':'))
    {
        return Err(AppError::Validation(format!(
            "{field} must be a short safe label without path separators"
        )));
    }
    Ok(())
}

fn safe_fixture_name(index: usize, value: Option<&str>) -> Result<String, AppError> {
    let Some(value) = value else {
        return Ok(format!("fixture-{}", index.saturating_add(1)));
    };
    if value.is_empty() {
        return Ok(format!("fixture-{}", index.saturating_add(1)));
    }
    validate_label_common(value, "fixture name")?;
    if value.bytes().any(|byte| matches!(byte, b'\\' | b':')) {
        return Err(AppError::Validation(
            "fixture name must not contain path separators or drive prefixes".to_string(),
        ));
    }
    if value.starts_with('/') || value.ends_with('/') || value.contains("//") {
        return Err(AppError::Validation(
            "fixture name must be a relative safe label".to_string(),
        ));
    }
    if value
        .split('/')
        .any(|segment| segment == "." || segment == "..")
    {
        return Err(AppError::Validation(
            "fixture name must not contain relative path segments".to_string(),
        ));
    }
    Ok(value.to_string())
}

fn validate_label_common(value: &str, field: &str) -> Result<(), AppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation(format!("{field} must not be empty")));
    }
    if trimmed != value {
        return Err(AppError::Validation(format!(
            "{field} must not include leading or trailing whitespace"
        )));
    }
    if value == "." || value == ".." {
        return Err(AppError::Validation(format!(
            "{field} must be a short safe label"
        )));
    }
    if value.len() > 128 {
        return Err(AppError::Validation(format!(
            "{field} must be 128 characters or fewer"
        )));
    }
    if !value.bytes().all(|byte| {
        matches!(
            byte,
            b'a'..=b'z'
                | b'A'..=b'Z'
                | b'0'..=b'9'
                | b'.'
                | b'-'
                | b'_'
                | b'/'
        )
    }) {
        return Err(AppError::Validation(format!(
            "{field} may contain only ASCII letters, numbers, '.', '-', '_', and '/'"
        )));
    }
    Ok(())
}

fn profile_test_report_from_inline_fixtures(
    fixtures: &[crate::models::ProfileTestFixtureInput],
    profile: &hl7v2::Profile,
) -> Result<hl7v2::ProfileTestReport, AppError> {
    if fixtures.is_empty() {
        return Err(AppError::Validation(
            "fixtures must contain at least one fixture".to_string(),
        ));
    }

    let cases = fixtures
        .iter()
        .enumerate()
        .map(|(index, fixture)| profile_test_case_from_inline_fixture(index, fixture, profile))
        .collect::<Result<Vec<_>, _>>()?;
    let passed_count = cases.iter().filter(|case| case.passed).count();
    let case_count = cases.len();
    let failed_count = case_count.saturating_sub(passed_count);

    Ok(hl7v2::ProfileTestReport {
        profile: INLINE_PROFILE.to_string(),
        fixtures: INLINE_FIXTURES.to_string(),
        valid: failed_count == 0,
        case_count,
        passed_count,
        failed_count,
        cases,
    })
}

fn profile_test_case_from_inline_fixture(
    index: usize,
    fixture: &crate::models::ProfileTestFixtureInput,
    profile: &hl7v2::Profile,
) -> Result<hl7v2::ProfileTestCaseReport, AppError> {
    let name = safe_fixture_name(index, fixture.name.as_deref())?;
    let message_bytes = if fixture.mllp_framed {
        match hl7v2::unwrap_mllp(fixture.message.as_bytes()) {
            Ok(message) => message,
            Err(error) => {
                return Ok(profile_test_case_without_report(
                    name,
                    fixture.expectation,
                    format!("fixture did not unwrap as MLLP: {error}"),
                ));
            }
        }
    } else {
        fixture.message.as_bytes()
    };

    let message = match hl7v2::parse(message_bytes) {
        Ok(message) => message,
        Err(error) => {
            return Ok(profile_test_case_without_report(
                name,
                fixture.expectation,
                format!("fixture did not parse as HL7: {error}"),
            ));
        }
    };

    let issues = hl7v2::validate(&message, profile);
    let validation_report =
        hl7v2::ValidationReport::from_issues(&message, Some(INLINE_PROFILE.to_string()), issues);
    let expected_valid = fixture.expectation == hl7v2::ProfileFixtureExpectation::Valid;
    let mut passed = validation_report.valid == expected_valid;
    let mut case_message = if passed {
        format!(
            "expected {} and report was {}",
            fixture.expectation.as_str(),
            validation_state_label(validation_report.valid)
        )
    } else {
        format!(
            "expected {} but report was {}",
            fixture.expectation.as_str(),
            validation_state_label(validation_report.valid)
        )
    };

    let expected_report = fixture
        .expected_report_json
        .as_deref()
        .map(|expected| compare_inline_expected_report_json(&name, expected, &validation_report));
    if let Some(comparison) = &expected_report {
        if comparison.matched {
            case_message.push_str("; expected report matched");
        } else {
            passed = false;
            let detail = comparison
                .message
                .as_deref()
                .unwrap_or("expected report did not match");
            case_message.push_str(&format!("; {detail}"));
        }
    }

    Ok(hl7v2::ProfileTestCaseReport {
        name: name.clone(),
        path: name,
        expectation: fixture.expectation,
        passed,
        message: case_message,
        validation_report: Some(validation_report),
        expected_report,
    })
}

fn profile_test_case_without_report(
    name: String,
    expectation: hl7v2::ProfileFixtureExpectation,
    message: String,
) -> hl7v2::ProfileTestCaseReport {
    hl7v2::ProfileTestCaseReport {
        name: name.clone(),
        path: name,
        expectation,
        passed: false,
        message,
        validation_report: None,
        expected_report: None,
    }
}

fn validation_state_label(valid: bool) -> &'static str {
    if valid { "valid" } else { "invalid" }
}

fn compare_inline_expected_report_json(
    fixture_name: &str,
    expected_json: &str,
    actual_report: &hl7v2::ValidationReport,
) -> hl7v2::ExpectedReportComparison {
    let path = format!("{fixture_name}.expected-report.json");
    let expected = match serde_json::from_str::<serde_json::Value>(expected_json) {
        Ok(expected) => expected,
        Err(error) => {
            return hl7v2::ExpectedReportComparison {
                path,
                matched: false,
                message: Some(format!("expected report is not valid JSON: {error}")),
            };
        }
    };
    let actual = match serde_json::to_value(actual_report) {
        Ok(actual) => actual,
        Err(error) => {
            return hl7v2::ExpectedReportComparison {
                path,
                matched: false,
                message: Some(format!("actual report could not be serialized: {error}")),
            };
        }
    };

    match json_subset_matches(&expected, &actual, "$") {
        Ok(()) => hl7v2::ExpectedReportComparison {
            path,
            matched: true,
            message: None,
        },
        Err(message) => hl7v2::ExpectedReportComparison {
            path,
            matched: false,
            message: Some(message),
        },
    }
}

fn json_subset_matches(
    expected: &serde_json::Value,
    actual: &serde_json::Value,
    path: &str,
) -> Result<(), String> {
    match (expected, actual) {
        (serde_json::Value::Object(expected), serde_json::Value::Object(actual)) => {
            for (key, expected_value) in expected {
                let actual_value = actual
                    .get(key)
                    .ok_or_else(|| format!("{path}.{key} was missing from actual report"))?;
                json_subset_matches(expected_value, actual_value, &format!("{path}.{key}"))?;
            }
            Ok(())
        }
        (serde_json::Value::Array(expected), serde_json::Value::Array(actual)) => {
            for (index, expected_value) in expected.iter().enumerate() {
                let matched = actual.iter().any(|actual_value| {
                    json_subset_matches(expected_value, actual_value, &format!("{path}[{index}]"))
                        .is_ok()
                });
                if !matched {
                    return Err(format!(
                        "{path}[{index}] did not match any actual report item"
                    ));
                }
            }
            Ok(())
        }
        _ if expected == actual => Ok(()),
        _ => Err(format!("{path} did not match actual report")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_profile_name_rejects_path_like_values() {
        assert_eq!(
            safe_profile_name(None).expect("default profile name should be safe"),
            INLINE_PROFILE
        );
        assert_eq!(
            safe_profile_name(Some("adt-a01")).expect("simple profile name should be safe"),
            "adt-a01"
        );

        for name in ["../adt", "C:/adt", "adt/a01", "adt\\a01", " adt"] {
            assert!(
                safe_profile_name(Some(name)).is_err(),
                "expected {name:?} to be rejected"
            );
        }
    }

    #[test]
    fn fixture_names_allow_relative_groups_but_reject_traversal() {
        assert_eq!(
            safe_fixture_name(0, None).expect("missing fixture name should default"),
            "fixture-1"
        );
        assert_eq!(
            safe_fixture_name(0, Some("valid/adt.hl7"))
                .expect("relative fixture label should be safe"),
            "valid/adt.hl7"
        );

        for name in [
            "../adt.hl7",
            "/adt.hl7",
            "valid//adt.hl7",
            "valid\\adt.hl7",
            " valid",
        ] {
            assert!(
                safe_fixture_name(0, Some(name)).is_err(),
                "expected {name:?} to be rejected"
            );
        }
    }
}
