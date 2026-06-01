#![cfg(feature = "profile")]

use chrono::{Datelike, Timelike};
use hl7v2::conformance::datatype::datetime::{
    DateTimeError, TimestampPrecision, parse_hl7_dt, parse_hl7_tm, parse_hl7_ts,
    parse_hl7_ts_with_precision,
};
use hl7v2::{
    ProfileFixtureExpectation, ProfileTestCaseReport, ProfileTestReport, Severity,
    ValidationReport, explain_profile, lint_profile_yaml, load_profile_checked, parse,
    run_profile_fixture_tests, validate,
};
use std::error::Error;
use std::fmt::Debug;
use std::fs;

fn require_eq<T>(actual: T, expected: T, label: &str) -> Result<(), Box<dyn Error>>
where
    T: PartialEq + Debug,
{
    if actual == expected {
        Ok(())
    } else {
        Err(std::io::Error::other(format!("{label}: expected {expected:?}, got {actual:?}")).into())
    }
}

fn require(condition: bool, message: &'static str) -> Result<(), Box<dyn Error>> {
    if condition {
        Ok(())
    } else {
        Err(std::io::Error::other(message).into())
    }
}

#[test]
fn datetime_facade_preserves_precision_and_fractional_seconds() -> Result<(), Box<dyn Error>> {
    let timestamp = parse_hl7_ts_with_precision("20250128152312.1234")?;

    require_eq(
        timestamp.precision,
        TimestampPrecision::FractionalSecond,
        "timestamp precision",
    )?;
    require_eq(
        timestamp.fractional_seconds,
        Some(123400),
        "fractional seconds",
    )?;
    require_eq(
        timestamp.to_hl7_string(),
        "20250128152312.1234".to_string(),
        "HL7 string",
    )?;
    require_eq(timestamp.datetime.year(), 2025, "year")?;
    require_eq(timestamp.datetime.month(), 1, "month")?;
    require_eq(timestamp.datetime.day(), 28, "day")?;
    require_eq(timestamp.datetime.hour(), 15, "hour")?;

    Ok(())
}

#[test]
fn datetime_facade_reports_specific_error_variants() -> Result<(), Box<dyn Error>> {
    require(
        matches!(
            parse_hl7_dt("notadate"),
            Err(DateTimeError::InvalidDateFormat(_))
        ),
        "expected InvalidDateFormat",
    )?;
    require(
        matches!(parse_hl7_tm("2500"), Err(DateTimeError::TimeOutOfRange(_))),
        "expected TimeOutOfRange",
    )?;
    require(
        matches!(
            parse_hl7_ts("bad"),
            Err(DateTimeError::InvalidTimestampFormat(_))
        ),
        "expected InvalidTimestampFormat",
    )?;

    Ok(())
}

#[test]
fn profile_facade_rejects_invalid_valueset_values() -> Result<(), Box<dyn Error>> {
    let profile = load_profile_checked(
        r#"
message_structure: "ADT_A01"
version: "2.5"
segments:
  - id: "MSH"
  - id: "PID"
valuesets:
  - path: "PID.8"
    name: "AdministrativeSex"
    codes: ["M", "F"]
"#,
    )?;
    let message = parse(
        b"MSH|^~\\&|SEND|FAC|RECV|RF|202605030101||ADT^A01|CTRL123|P|2.5\r\
PID|1||123456^^^HOSP^MR||Doe^John||19700101|X\r",
    )?;
    let issues = validate(&message, &profile);

    require(
        issues.iter().any(|issue| {
            issue.severity == Severity::Error
                && issue.path.as_deref() == Some("PID.8")
                && issue.code == "VALUE_NOT_IN_SET"
        }),
        "expected PID.8 value set violation",
    )?;

    Ok(())
}

#[test]
fn profile_facade_rejects_empty_required_msh_fields() -> Result<(), Box<dyn Error>> {
    let profile = load_profile_checked(
        r#"
message_structure: "ADT_A01"
version: "2.5"
segments:
  - id: "MSH"
constraints:
  - path: "MSH.10"
    required: true
"#,
    )?;
    let message = parse(b"MSH|^~\\&|SEND|FAC|RECV|RF|202605030101||ADT^A01||P|2.5\r")?;
    let issues = validate(&message, &profile);

    require(
        issues.iter().any(|issue| {
            issue.severity == Severity::Error
                && issue.path.as_deref() == Some("MSH.10")
                && issue.code == "MISSING_REQUIRED_FIELD"
        }),
        "expected empty MSH.10 to fail required validation",
    )?;

    Ok(())
}

#[test]
fn profile_facade_accepts_required_composite_fields_with_later_components()
-> Result<(), Box<dyn Error>> {
    let profile = load_profile_checked(
        r#"
message_structure: "ORU_R01"
version: "2.5"
segments:
  - id: "MSH"
  - id: "OBX"
constraints:
  - path: "OBX[1]-5"
    required: true
"#,
    )?;
    let message = parse(
        b"MSH|^~\\&|LAB|FAC|EHR|RF|202605030101||ORU^R01|CTRL123|P|2.5\r\
OBX|1|XPN|NAME^Patient name||^Jane^A\r",
    )?;
    let issues = validate(&message, &profile);

    require(
        !issues.iter().any(|issue| {
            issue.severity == Severity::Error
                && issue.path.as_deref() == Some("OBX[1]-5")
                && issue.code == "MISSING_REQUIRED_FIELD"
        }),
        "expected later OBX-5 components to satisfy required field validation",
    )?;

    Ok(())
}

#[test]
fn profile_facade_reports_length_constraint_failures() -> Result<(), Box<dyn Error>> {
    let profile = load_profile_checked(
        r#"
message_structure: "ADT_A01"
version: "2.5"
segments:
  - id: "MSH"
  - id: "PID"
lengths:
  - path: "PID.5.1"
    max: 3
    policy: "no-truncate"
"#,
    )?;
    let message = parse(
        b"MSH|^~\\&|SEND|FAC|RECV|RF|202605030101||ADT^A01|CTRL123|P|2.5\r\
PID|1||123456^^^HOSP^MR||Longname^John||19700101|M\r",
    )?;
    let issues = validate(&message, &profile);

    require(
        issues
            .iter()
            .any(|issue| issue.path.as_deref() == Some("PID.5.1")),
        "expected PID.5.1 length issue",
    )?;

    Ok(())
}

#[test]
fn profile_facade_accepts_diagnostic_segment_repetition_paths() -> Result<(), Box<dyn Error>> {
    let yaml = r#"
message_structure: "ORU_R01"
version: "2.5"
segments:
  - id: "MSH"
  - id: "OBX"
constraints:
  - path: "OBX[3]-5"
    required: true
valuesets:
  - path: "OBX[3]-2"
    name: "ThirdObservationValueType"
    codes: ["NM"]
"#;
    let lint = lint_profile_yaml(yaml);
    require(lint.valid, "expected diagnostic paths to pass profile lint")?;

    let profile = load_profile_checked(yaml)?;
    let message = parse(
        b"MSH|^~\\&|SEND|FAC|RECV|RF|202605030101||ORU^R01|CTRL123|P|2.5\r\
OBX|1|NM|CODE1^First||1\r\
OBX|2|NM|CODE2^Second||2\r\
OBX|3|ST|CODE3^Third||third\r",
    )?;
    let issues = validate(&message, &profile);

    require(
        issues.iter().any(|issue| {
            issue.severity == Severity::Error
                && issue.path.as_deref() == Some("OBX[3]-2")
                && issue.code == "VALUE_NOT_IN_SET"
        }),
        "expected diagnostic path value set violation",
    )?;

    Ok(())
}

#[test]
fn profile_facade_explains_profile_contract_shape() -> Result<(), Box<dyn Error>> {
    let yaml = r#"
message_structure: "ADT_A01"
version: "2.5"
message_type: "ADT^A01"
segments:
  - id: "MSH"
  - id: "PID"
constraints:
  - path: "PID.3"
    required: true
valuesets:
  - path: "PID.8"
    name: "AdministrativeSex"
    codes: ["M", "F"]
"#;
    let profile = load_profile_checked(yaml)?;
    let lint = lint_profile_yaml(yaml);
    let report = explain_profile("profiles/adt.yaml", yaml, &profile, &lint);

    require_eq(report.profile.as_str(), "profiles/adt.yaml", "profile")?;
    require_eq(report.message_structure.as_str(), "ADT_A01", "structure")?;
    require_eq(report.summary.segment_count, 2, "segment count")?;
    require_eq(
        report.summary.required_field_count,
        1,
        "required field count",
    )?;
    let value_set = report
        .value_sets
        .first()
        .ok_or_else(|| std::io::Error::other("expected value set"))?;
    require_eq(value_set.source.as_str(), "inline", "value set source")?;
    require(
        report.profile_sha256.len() == 64,
        "expected SHA-256 hex profile hash",
    )?;

    let v2 = report.to_v2("test-surface", "0.0.0");
    require_eq(v2.schema_version.as_str(), "2", "schema version")?;
    require_eq(v2.tool_name.as_str(), "test-surface", "tool name")?;
    require_eq(v2.tool_version.as_str(), "0.0.0", "tool version")?;

    Ok(())
}

#[test]
fn profile_facade_owns_profile_test_report_contract() -> Result<(), Box<dyn Error>> {
    let message = parse(
        b"MSH|^~\\&|SEND|FAC|RECV|RF|202605030101||ADT^A01|CTRL123|P|2.5\r\
PID|1||123456^^^HOSP^MR||Doe^John||19700101|M\r",
    )?;
    let validation_report =
        ValidationReport::from_issues(&message, Some("profile.yaml".into()), vec![]);
    let report = ProfileTestReport {
        profile: "profile.yaml".into(),
        fixtures: "fixtures".into(),
        valid: true,
        case_count: 1,
        passed_count: 1,
        failed_count: 0,
        cases: vec![ProfileTestCaseReport {
            name: "valid/valid.hl7".into(),
            path: "fixtures/valid/valid.hl7".into(),
            expectation: ProfileFixtureExpectation::Valid,
            passed: true,
            message: "expected valid and report was valid".into(),
            validation_report: Some(validation_report),
            expected_report: None,
        }],
    };

    let case = report
        .cases
        .first()
        .ok_or_else(|| std::io::Error::other("expected profile test case"))?;
    require_eq(case.expectation.as_str(), "valid", "expectation")?;
    let v2 = report.to_v2("test-surface", "0.0.0");
    require_eq(v2.schema_version.as_str(), "2", "schema version")?;
    require_eq(v2.tool_name.as_str(), "test-surface", "tool name")?;
    require_eq(v2.report.case_count, 1, "case count")?;

    Ok(())
}

#[test]
fn profile_facade_runs_profile_fixture_tests() -> Result<(), Box<dyn Error>> {
    let profile_yaml = r#"
message_structure: "ADT_A01"
version: "2.5"
segments:
  - id: "MSH"
  - id: "PID"
constraints:
  - path: "PID.3"
    required: true
  - path: "PID.13"
    required: true
"#;
    let profile = load_profile_checked(profile_yaml)?;
    let dir = tempfile::tempdir()?;
    let fixtures = dir.path().join("fixtures");
    fs::create_dir_all(fixtures.join("valid"))?;
    fs::create_dir_all(fixtures.join("invalid"))?;
    fs::create_dir_all(fixtures.join("expected"))?;
    fs::write(
        fixtures.join("valid").join("adt.hl7"),
        b"MSH|^~\\&|SEND|FAC|RECV|RF|202605030101||ADT^A01|CTRL123|P|2.5\r\
PID|1||123456^^^HOSP^MR||Doe^John||19700101|M|||742 Evergreen Terrace||5558675309\r",
    )?;
    fs::write(
        fixtures.join("invalid").join("missing_pid3.hl7"),
        b"MSH|^~\\&|SEND|FAC|RECV|RF|202605030101||ADT^A01|CTRL999|P|2.5\r\
PID|1||||Doe^John||19700101|M\r",
    )?;
    fs::write(
        fixtures.join("expected").join("missing_pid3.report.json"),
        r#"{
  "valid": false,
  "issues": [
    {
      "code": "missing_required_field",
      "severity": "error",
      "path": "PID.3"
    }
  ]
}"#,
    )?;

    let report = run_profile_fixture_tests("profile.yaml", &fixtures, &profile)?;
    require(report.valid, "expected profile fixture tests to pass")?;
    require_eq(report.case_count, 2, "case count")?;
    require_eq(report.passed_count, 2, "passed count")?;
    require(
        report.cases.iter().any(|case| {
            case.expectation == ProfileFixtureExpectation::Invalid
                && case
                    .expected_report
                    .as_ref()
                    .is_some_and(|expected| expected.matched)
        }),
        "expected invalid fixture report comparison to match",
    )?;

    Ok(())
}
