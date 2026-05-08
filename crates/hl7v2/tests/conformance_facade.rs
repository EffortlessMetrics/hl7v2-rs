#![cfg(feature = "profile")]

use chrono::{Datelike, Timelike};
use hl7v2::conformance::datatype::datetime::{
    DateTimeError, TimestampPrecision, parse_hl7_dt, parse_hl7_tm, parse_hl7_ts,
    parse_hl7_ts_with_precision,
};
use hl7v2::{Severity, load_profile_checked, parse, validate};
use std::error::Error;
use std::fmt::Debug;

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
    let timestamp = parse_hl7_ts_with_precision("20250128152312.123456")?;

    require_eq(
        timestamp.precision,
        TimestampPrecision::FractionalSecond,
        "timestamp precision",
    )?;
    require_eq(
        timestamp.fractional_seconds,
        Some(123456),
        "fractional seconds",
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
