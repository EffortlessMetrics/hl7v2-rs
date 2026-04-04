//! BDD tests for hl7v2-datetime using Cucumber
//!
//! Run with: cargo test --test bdd_tests

use cucumber::{World, given, then, when};
use hl7v2_datetime::{
    DateTimeError, ParsedTimestamp, TimestampPrecision, is_valid_hl7_date, is_valid_hl7_time,
    is_valid_hl7_timestamp, now_hl7, parse_hl7_dt, parse_hl7_tm, parse_hl7_ts,
    parse_hl7_ts_with_precision, today_hl7,
};

/// Test world for DateTime BDD tests
#[derive(Debug, World)]
#[world(init = Self::new)]
pub struct DateTimeWorld {
    // ---- inputs ----
    date_string: String,
    time_string: String,
    timestamp_string: String,

    // ---- date parse results ----
    date_result: Option<Result<chrono::NaiveDate, DateTimeError>>,

    // ---- time parse results ----
    time_result: Option<Result<(u32, u32, u32, Option<u32>), DateTimeError>>,

    // ---- timestamp parse results ----
    ts_result: Option<Result<chrono::NaiveDateTime, DateTimeError>>,

    // ---- precision parse results ----
    precision_result: Option<Result<ParsedTimestamp, DateTimeError>>,
    parsed_ts: Option<ParsedTimestamp>,

    // ---- comparison pair ----
    first_ts: Option<ParsedTimestamp>,
    second_ts: Option<ParsedTimestamp>,

    // ---- formatted output ----
    hl7_string: String,

    // ---- helper output ----
    helper_output: String,
}

impl DateTimeWorld {
    fn new() -> Self {
        Self {
            date_string: String::new(),
            time_string: String::new(),
            timestamp_string: String::new(),
            date_result: None,
            time_result: None,
            ts_result: None,
            precision_result: None,
            parsed_ts: None,
            first_ts: None,
            second_ts: None,
            hl7_string: String::new(),
            helper_output: String::new(),
        }
    }
}

// ============================================================================
// Given Steps
// ============================================================================

#[given(regex = r#"^the date string "([^"]*)"$"#)]
fn given_date_string(world: &mut DateTimeWorld, input: String) {
    world.date_string = input;
}

#[given(regex = r#"^the time string "([^"]*)"$"#)]
fn given_time_string(world: &mut DateTimeWorld, input: String) {
    world.time_string = input;
}

#[given(regex = r#"^the timestamp string "([^"]*)"$"#)]
fn given_timestamp_string(world: &mut DateTimeWorld, input: String) {
    world.timestamp_string = input;
}

#[given(regex = r#"^a timestamp "([^"]*)" as the first$"#)]
fn given_first_timestamp(world: &mut DateTimeWorld, input: String) {
    world.first_ts = Some(parse_hl7_ts_with_precision(&input).expect("valid first timestamp"));
}

#[given(regex = r#"^a timestamp "([^"]*)" as the second$"#)]
fn given_second_timestamp(world: &mut DateTimeWorld, input: String) {
    world.second_ts = Some(parse_hl7_ts_with_precision(&input).expect("valid second timestamp"));
}

// ============================================================================
// When Steps
// ============================================================================

#[when("I parse the date")]
fn when_parse_date(world: &mut DateTimeWorld) {
    world.date_result = Some(parse_hl7_dt(&world.date_string));
}

#[when("I attempt to parse the date")]
fn when_attempt_parse_date(world: &mut DateTimeWorld) {
    world.date_result = Some(parse_hl7_dt(&world.date_string));
}

#[when("I parse the time")]
fn when_parse_time(world: &mut DateTimeWorld) {
    world.time_result = Some(parse_hl7_tm(&world.time_string));
}

#[when("I attempt to parse the time")]
fn when_attempt_parse_time(world: &mut DateTimeWorld) {
    world.time_result = Some(parse_hl7_tm(&world.time_string));
}

#[when("I parse the timestamp")]
fn when_parse_timestamp(world: &mut DateTimeWorld) {
    world.ts_result = Some(parse_hl7_ts(&world.timestamp_string));
}

#[when("I attempt to parse the timestamp")]
fn when_attempt_parse_timestamp(world: &mut DateTimeWorld) {
    world.ts_result = Some(parse_hl7_ts(&world.timestamp_string));
}

#[when("I parse the timestamp with precision")]
fn when_parse_timestamp_with_precision(world: &mut DateTimeWorld) {
    let result = parse_hl7_ts_with_precision(&world.timestamp_string);
    if let Ok(ref ts) = result {
        world.parsed_ts = Some(ts.clone());
    }
    world.precision_result = Some(result);
}

#[when("I format the timestamp to an HL7 string")]
fn when_format_hl7_string(world: &mut DateTimeWorld) {
    let ts = world.parsed_ts.as_ref().expect("No parsed timestamp");
    world.hl7_string = ts.to_hl7_string();
}

#[when("I call now_hl7")]
fn when_call_now_hl7(world: &mut DateTimeWorld) {
    world.helper_output = now_hl7();
}

#[when("I call today_hl7")]
fn when_call_today_hl7(world: &mut DateTimeWorld) {
    world.helper_output = today_hl7();
}

// ============================================================================
// Then Steps — Date
// ============================================================================

#[then(regex = r"^the year should be (\d+)$")]
fn then_year(world: &mut DateTimeWorld, expected: i32) {
    use chrono::Datelike;
    let date = world
        .date_result
        .as_ref()
        .expect("No date result")
        .as_ref()
        .expect("Date parse failed");
    assert_eq!(date.year(), expected);
}

#[then(regex = r"^the month should be (\d+)$")]
fn then_month(world: &mut DateTimeWorld, expected: u32) {
    use chrono::Datelike;
    let date = world
        .date_result
        .as_ref()
        .expect("No date result")
        .as_ref()
        .expect("Date parse failed");
    assert_eq!(date.month(), expected);
}

#[then(regex = r"^the day should be (\d+)$")]
fn then_day(world: &mut DateTimeWorld, expected: u32) {
    use chrono::Datelike;
    let date = world
        .date_result
        .as_ref()
        .expect("No date result")
        .as_ref()
        .expect("Date parse failed");
    assert_eq!(date.day(), expected);
}

#[then("date parsing should fail")]
fn then_date_parsing_fails(world: &mut DateTimeWorld) {
    assert!(
        world.date_result.as_ref().expect("No date result").is_err(),
        "Expected date parsing to fail"
    );
}

// ============================================================================
// Then Steps — Time
// ============================================================================

#[then(regex = r"^the hour should be (\d+)$")]
fn then_hour(world: &mut DateTimeWorld, expected: u32) {
    let (h, _, _, _) = world
        .time_result
        .as_ref()
        .expect("No time result")
        .as_ref()
        .expect("Time parse failed");
    assert_eq!(*h, expected);
}

#[then(regex = r"^the minute should be (\d+)$")]
fn then_minute(world: &mut DateTimeWorld, expected: u32) {
    let (_, m, _, _) = world
        .time_result
        .as_ref()
        .expect("No time result")
        .as_ref()
        .expect("Time parse failed");
    assert_eq!(*m, expected);
}

#[then(regex = r"^the second should be (\d+)$")]
fn then_second(world: &mut DateTimeWorld, expected: u32) {
    let (_, _, s, _) = world
        .time_result
        .as_ref()
        .expect("No time result")
        .as_ref()
        .expect("Time parse failed");
    assert_eq!(*s, expected);
}

#[then("the fractional seconds should be absent")]
fn then_fractional_absent(world: &mut DateTimeWorld) {
    let (_, _, _, f) = world
        .time_result
        .as_ref()
        .expect("No time result")
        .as_ref()
        .expect("Time parse failed");
    assert_eq!(*f, None);
}

#[then(regex = r"^the fractional seconds should be (\d+)$")]
fn then_fractional_value(world: &mut DateTimeWorld, expected: u32) {
    let (_, _, _, f) = world
        .time_result
        .as_ref()
        .expect("No time result")
        .as_ref()
        .expect("Time parse failed");
    assert_eq!(*f, Some(expected));
}

#[then("time parsing should fail")]
fn then_time_parsing_fails(world: &mut DateTimeWorld) {
    assert!(
        world.time_result.as_ref().expect("No time result").is_err(),
        "Expected time parsing to fail"
    );
}

// ============================================================================
// Then Steps — Timestamp
// ============================================================================

#[then(regex = r"^the parsed datetime year should be (\d+)$")]
fn then_ts_year(world: &mut DateTimeWorld, expected: i32) {
    use chrono::Datelike;
    let dt = world
        .ts_result
        .as_ref()
        .expect("No timestamp result")
        .as_ref()
        .expect("Timestamp parse failed");
    assert_eq!(dt.year(), expected);
}

#[then(regex = r"^the parsed datetime month should be (\d+)$")]
fn then_ts_month(world: &mut DateTimeWorld, expected: u32) {
    use chrono::Datelike;
    let dt = world
        .ts_result
        .as_ref()
        .expect("No timestamp result")
        .as_ref()
        .expect("Timestamp parse failed");
    assert_eq!(dt.month(), expected);
}

#[then(regex = r"^the parsed datetime day should be (\d+)$")]
fn then_ts_day(world: &mut DateTimeWorld, expected: u32) {
    use chrono::Datelike;
    let dt = world
        .ts_result
        .as_ref()
        .expect("No timestamp result")
        .as_ref()
        .expect("Timestamp parse failed");
    assert_eq!(dt.day(), expected);
}

#[then(regex = r"^the parsed datetime hour should be (\d+)$")]
fn then_ts_hour(world: &mut DateTimeWorld, expected: u32) {
    use chrono::Timelike;
    let dt = world
        .ts_result
        .as_ref()
        .expect("No timestamp result")
        .as_ref()
        .expect("Timestamp parse failed");
    assert_eq!(dt.hour(), expected);
}

#[then(regex = r"^the parsed datetime minute should be (\d+)$")]
fn then_ts_minute(world: &mut DateTimeWorld, expected: u32) {
    use chrono::Timelike;
    let dt = world
        .ts_result
        .as_ref()
        .expect("No timestamp result")
        .as_ref()
        .expect("Timestamp parse failed");
    assert_eq!(dt.minute(), expected);
}

#[then(regex = r"^the parsed datetime second should be (\d+)$")]
fn then_ts_second(world: &mut DateTimeWorld, expected: u32) {
    use chrono::Timelike;
    let dt = world
        .ts_result
        .as_ref()
        .expect("No timestamp result")
        .as_ref()
        .expect("Timestamp parse failed");
    assert_eq!(dt.second(), expected);
}

#[then("timestamp parsing should fail")]
fn then_timestamp_parsing_fails(world: &mut DateTimeWorld) {
    assert!(
        world
            .ts_result
            .as_ref()
            .expect("No timestamp result")
            .is_err(),
        "Expected timestamp parsing to fail"
    );
}

// ============================================================================
// Then Steps — Precision
// ============================================================================

#[then(regex = r#"^the precision should be "([^"]+)"$"#)]
fn then_precision(world: &mut DateTimeWorld, expected: String) {
    let ts = world.parsed_ts.as_ref().expect("No parsed timestamp");
    let actual = match ts.precision {
        TimestampPrecision::Year => "Year",
        TimestampPrecision::Month => "Month",
        TimestampPrecision::Day => "Day",
        TimestampPrecision::Hour => "Hour",
        TimestampPrecision::Minute => "Minute",
        TimestampPrecision::Second => "Second",
        TimestampPrecision::FractionalSecond => "FractionalSecond",
    };
    assert_eq!(actual, expected);
}

// ============================================================================
// Then Steps — Comparison
// ============================================================================

#[then("the first timestamp should be before the second")]
fn then_first_before_second(world: &mut DateTimeWorld) {
    let first = world.first_ts.as_ref().expect("No first timestamp");
    let second = world.second_ts.as_ref().expect("No second timestamp");
    assert!(
        first.is_before(second),
        "Expected first ({:?}) to be before second ({:?})",
        first,
        second
    );
}

#[then("the second timestamp should be after the first")]
fn then_second_after_first(world: &mut DateTimeWorld) {
    let first = world.first_ts.as_ref().expect("No first timestamp");
    let second = world.second_ts.as_ref().expect("No second timestamp");
    assert!(
        second.is_after(first),
        "Expected second ({:?}) to be after first ({:?})",
        second,
        first
    );
}

#[then("the two timestamps should be on the same day")]
fn then_same_day(world: &mut DateTimeWorld) {
    let first = world.first_ts.as_ref().expect("No first timestamp");
    let second = world.second_ts.as_ref().expect("No second timestamp");
    assert!(
        first.is_same_day(second),
        "Expected same day: {:?} vs {:?}",
        first,
        second
    );
}

#[then("the two timestamps should not be on the same day")]
fn then_not_same_day(world: &mut DateTimeWorld) {
    let first = world.first_ts.as_ref().expect("No first timestamp");
    let second = world.second_ts.as_ref().expect("No second timestamp");
    assert!(
        !first.is_same_day(second),
        "Expected different days: {:?} vs {:?}",
        first,
        second
    );
}

// ============================================================================
// Then Steps — to_hl7_string round-trip
// ============================================================================

#[then(regex = r#"^the HL7 string should be "([^"]*)"$"#)]
fn then_hl7_string(world: &mut DateTimeWorld, expected: String) {
    assert_eq!(world.hl7_string, expected);
}

// ============================================================================
// Then Steps — Validation helpers
// ============================================================================

#[then(regex = r"^the date validity should be (true|false)$")]
fn then_date_validity(world: &mut DateTimeWorld, expected: String) {
    let valid = is_valid_hl7_date(&world.date_string);
    let expected_bool = expected == "true";
    assert_eq!(
        valid, expected_bool,
        "is_valid_hl7_date({:?}) = {}, expected {}",
        world.date_string, valid, expected_bool
    );
}

#[then(regex = r"^the time validity should be (true|false)$")]
fn then_time_validity(world: &mut DateTimeWorld, expected: String) {
    let valid = is_valid_hl7_time(&world.time_string);
    let expected_bool = expected == "true";
    assert_eq!(
        valid, expected_bool,
        "is_valid_hl7_time({:?}) = {}, expected {}",
        world.time_string, valid, expected_bool
    );
}

#[then(regex = r"^the timestamp validity should be (true|false)$")]
fn then_timestamp_validity(world: &mut DateTimeWorld, expected: String) {
    let valid = is_valid_hl7_timestamp(&world.timestamp_string);
    let expected_bool = expected == "true";
    assert_eq!(
        valid, expected_bool,
        "is_valid_hl7_timestamp({:?}) = {}, expected {}",
        world.timestamp_string, valid, expected_bool
    );
}

// ============================================================================
// Then Steps — now_hl7 / today_hl7 helpers
// ============================================================================

#[then(regex = r"^the result should be (\d+) digits long$")]
fn then_result_length(world: &mut DateTimeWorld, expected: usize) {
    assert_eq!(
        world.helper_output.len(),
        expected,
        "Expected length {}, got {} for {:?}",
        expected,
        world.helper_output.len(),
        world.helper_output
    );
    assert!(
        world.helper_output.chars().all(|c| c.is_ascii_digit()),
        "Expected all digits, got {:?}",
        world.helper_output
    );
}

#[then("the result should be a valid timestamp")]
fn then_result_valid_timestamp(world: &mut DateTimeWorld) {
    assert!(
        is_valid_hl7_timestamp(&world.helper_output),
        "Expected valid timestamp, got {:?}",
        world.helper_output
    );
}

#[then("the result should be a valid date")]
fn then_result_valid_date(world: &mut DateTimeWorld) {
    assert!(
        is_valid_hl7_date(&world.helper_output),
        "Expected valid date, got {:?}",
        world.helper_output
    );
}

// ============================================================================
// Cucumber Main
// ============================================================================

#[tokio::main]
async fn main() {
    DateTimeWorld::run("features/datetime.feature").await;
}
