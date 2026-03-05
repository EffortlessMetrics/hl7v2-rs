//! Benchmarks for HL7 v2 validation performance
//!
//! This benchmark suite profiles validation performance across different scenarios:
//! - Field validation with different profile complexity
//! - Strict vs lenient validation comparison
//! - Data type validation performance

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use hl7v2_parser::parse;
use hl7v2_validation::{Issue, Validator, validate_data_type};
use std::hint::black_box;

/// Create a sample HL7 message for benchmarking
fn create_sample_message() -> String {
    "MSH|^~\\&|SendingApp|SendingFacility|ReceivingApp|ReceivingFacility|20250101000000||ADT^A01^ADT_A01|MSG00001|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John^A||19800101|M||C|123 Main St^^Anytown^ST^12345||(555)555-1212||E||S|||123456789|\rPV1|1|O|OP^PAREG^CHAREG|3|||DOE^JOHN^A^III^^^^MD|^DR.^JANE^B^^^^RN|||SURG||||ADM|||||20250101000000\r".to_string()
}

/// Create a complex message with many fields for validation
fn create_complex_message() -> String {
    "MSH|^~\\&|SendingApp|SendingFacility|ReceivingApp|ReceivingFacility|20250101120000||ADT^A01^ADT_A01|MSG00001|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John^A||19800101|M||C|123 Main St^^Anytown^ST^12345||(555)555-1212||E||S||123456789||||||||||||||||20250101\rPV1|1|I|ICU^101^01||||DOE^JOHN^A^III^^^^MD|||SUR||||||ADM|12345678|||||||||||||||||||||||||20250101120000\rOBX|1|NM|HEIGHT^Height^L||180|cm|||||F\rOBX|2|NM|WEIGHT^Weight^L||75|kg|||||F\rOBX|3|ST|BP^Blood Pressure^L||120/80|mmHg|||||F\rOBX|4|NM|HR^Heart Rate^L||72|bpm|||||F\rOBX|5|NM|TEMP^Temperature^L||37.0|C|||||F\rAL1|1|DA|PENICILLIN^Penicillin^L||RASH||20200101\rAL1|2|DA|ASPIRIN^Aspirin^L||ANAPHYLAXIS||20190101\rDG1|1|ICD10|J18.9^Pneumonia||20250101||A\rDG1|2|ICD10|E11.9^Type 2 Diabetes||20240101||A\r".to_string()
}

/// Create a message with intentional validation issues
fn create_message_with_issues() -> String {
    "MSH|^~\\&|SendingApp|SendingFacility|ReceivingApp|ReceivingFacility|INVALID_DATE||ADT^A01^ADT_A01|MSG00001|P|2.5.1\rPID|1||INVALID_MR||Doe^John^A||INVALID_DOB|M||C|123 Main St^^Anytown^ST^12345||(555)INVALID||E||S||INVALID_SSN|\r".to_string()
}

/// Basic validator for benchmarking
struct BasicValidator;

impl Validator for BasicValidator {
    fn validate(&self, msg: &hl7v2_core::Message) -> Vec<Issue> {
        let mut issues = Vec::new();

        // Check MSH segment
        for segment in &msg.segments {
            let seg_id = segment.id_str();
            if seg_id == "MSH" {
                // Check MSH.7 (Date/Time) - should be valid timestamp
                if segment.fields.len() > 6 {
                    if let Some(dt) = segment.fields[6].first_text() {
                        if !validate_data_type(dt, "TS") {
                            issues.push(Issue::error(
                                "INVALID_DATETIME",
                                Some("MSH.7".to_string()),
                                format!("Invalid datetime format: {}", dt),
                            ));
                        }
                    }
                }
                // Check MSH.9 (Message Type) - should be valid
                if segment.fields.len() > 8 {
                    if let Some(msg_type) = segment.fields[8].first_text() {
                        if !msg_type.contains('^') {
                            issues.push(Issue::warning(
                                "INVALID_MESSAGE_TYPE",
                                Some("MSH.9".to_string()),
                                "Message type should contain trigger event".to_string(),
                            ));
                        }
                    }
                }
            } else if seg_id == "PID" {
                // Check PID.3 (Patient ID) - should be valid
                if segment.fields.len() > 2 {
                    let patient_id = segment.fields[2].first_text();
                    if patient_id.is_none() || patient_id.map(|s| s.is_empty()).unwrap_or(true) {
                        issues.push(Issue::error(
                            "MISSING_PATIENT_ID",
                            Some("PID.3".to_string()),
                            "Patient identifier is required".to_string(),
                        ));
                    }
                }
                // Check PID.7 (Date of Birth) - should be valid date
                if segment.fields.len() > 6 {
                    if let Some(dob) = segment.fields[6].first_text() {
                        if !dob.is_empty() && !validate_data_type(dob, "DT") {
                            issues.push(Issue::error(
                                "INVALID_DOB",
                                Some("PID.7".to_string()),
                                format!("Invalid date of birth format: {}", dob),
                            ));
                        }
                    }
                }
                // Check PID.19 (SSN) - should be valid if present
                if segment.fields.len() > 18 {
                    if let Some(ssn) = segment.fields[18].first_text() {
                        if !ssn.is_empty()
                            && (!ssn.chars().all(|c| c.is_ascii_digit()) || ssn.len() != 9)
                        {
                            issues.push(Issue::warning(
                                "INVALID_SSN",
                                Some("PID.19".to_string()),
                                format!("Invalid SSN format: {}", ssn),
                            ));
                        }
                    }
                }
            }
        }

        issues
    }
}

/// Strict validator with more comprehensive checks
struct StrictValidator;

impl Validator for StrictValidator {
    fn validate(&self, msg: &hl7v2_core::Message) -> Vec<Issue> {
        let mut issues = Vec::new();

        for segment in &msg.segments {
            let seg_id = segment.id_str();
            match seg_id {
                "MSH" => {
                    // Validate all MSH fields strictly
                    if segment.fields.len() < 11 {
                        issues.push(Issue::error(
                            "MISSING_REQUIRED_FIELD",
                            Some("MSH".to_string()),
                            "MSH segment is missing required fields".to_string(),
                        ));
                    }

                    // MSH.3 - Sending Application (required)
                    if segment.fields.len() > 2 {
                        let sending_app = segment.fields[2].first_text();
                        if sending_app.is_none()
                            || sending_app.map(|s| s.is_empty()).unwrap_or(true)
                        {
                            issues.push(Issue::error(
                                "MISSING_SENDING_APP",
                                Some("MSH.3".to_string()),
                                "Sending application is required".to_string(),
                            ));
                        }
                    }

                    // MSH.7 - Date/Time (required, must be valid)
                    if segment.fields.len() > 6 {
                        let dt = segment.fields[6].first_text();
                        if dt.is_none() || dt.map(|s| s.is_empty()).unwrap_or(true) {
                            issues.push(Issue::error(
                                "MISSING_DATETIME",
                                Some("MSH.7".to_string()),
                                "Message datetime is required".to_string(),
                            ));
                        } else if let Some(dt_val) = dt {
                            if !validate_data_type(dt_val, "TS") {
                                issues.push(Issue::error(
                                    "INVALID_DATETIME",
                                    Some("MSH.7".to_string()),
                                    format!("Invalid datetime format: {}", dt_val),
                                ));
                            }
                        }
                    }

                    // MSH.9 - Message Type (required)
                    if segment.fields.len() > 8 {
                        let msg_type = segment.fields[8].first_text();
                        if msg_type.is_none() || msg_type.map(|s| s.is_empty()).unwrap_or(true) {
                            issues.push(Issue::error(
                                "MISSING_MESSAGE_TYPE",
                                Some("MSH.9".to_string()),
                                "Message type is required".to_string(),
                            ));
                        } else if let Some(mt) = msg_type {
                            let parts: Vec<&str> = mt.split('^').collect();
                            if parts.len() < 2 {
                                issues.push(Issue::error(
                                    "INVALID_MESSAGE_TYPE",
                                    Some("MSH.9".to_string()),
                                    "Message type must include trigger event".to_string(),
                                ));
                            }
                        }
                    }

                    // MSH.12 - Version ID (required)
                    if segment.fields.len() > 11 {
                        let version = segment.fields[11].first_text();
                        if version.is_none() || version.map(|s| s.is_empty()).unwrap_or(true) {
                            issues.push(Issue::error(
                                "MISSING_VERSION",
                                Some("MSH.12".to_string()),
                                "HL7 version is required".to_string(),
                            ));
                        }
                    }
                }
                "PID" => {
                    // PID.3 - Patient ID (required)
                    if segment.fields.len() <= 2 {
                        issues.push(Issue::error(
                            "MISSING_PATIENT_ID",
                            Some("PID.3".to_string()),
                            "Patient identifier is required".to_string(),
                        ));
                    } else {
                        let patient_id = segment.fields[2].first_text();
                        if patient_id.is_none() || patient_id.map(|s| s.is_empty()).unwrap_or(true)
                        {
                            issues.push(Issue::error(
                                "MISSING_PATIENT_ID",
                                Some("PID.3".to_string()),
                                "Patient identifier is required".to_string(),
                            ));
                        }
                    }

                    // PID.5 - Patient Name (required)
                    if segment.fields.len() <= 4 {
                        issues.push(Issue::error(
                            "MISSING_PATIENT_NAME",
                            Some("PID.5".to_string()),
                            "Patient name is required".to_string(),
                        ));
                    } else {
                        let name = segment.fields[4].first_text();
                        if name.is_none() || name.map(|s| s.is_empty()).unwrap_or(true) {
                            issues.push(Issue::error(
                                "MISSING_PATIENT_NAME",
                                Some("PID.5".to_string()),
                                "Patient name is required".to_string(),
                            ));
                        }
                    }

                    // PID.7 - DOB validation
                    if segment.fields.len() > 6 {
                        if let Some(dob) = segment.fields[6].first_text() {
                            if !dob.is_empty() && !validate_data_type(dob, "DT") {
                                issues.push(Issue::error(
                                    "INVALID_DOB",
                                    Some("PID.7".to_string()),
                                    format!("Invalid date of birth: {}", dob),
                                ));
                            }
                        }
                    }

                    // PID.8 - Administrative Sex (required)
                    if segment.fields.len() <= 7 {
                        issues.push(Issue::error(
                            "MISSING_SEX",
                            Some("PID.8".to_string()),
                            "Administrative sex is required".to_string(),
                        ));
                    } else {
                        let sex = segment.fields[7].first_text();
                        if sex.is_none() || sex.map(|s| s.is_empty()).unwrap_or(true) {
                            issues.push(Issue::error(
                                "MISSING_SEX",
                                Some("PID.8".to_string()),
                                "Administrative sex is required".to_string(),
                            ));
                        }
                    }
                }
                "PV1" => {
                    // PV1.2 - Patient Class (required)
                    if segment.fields.len() <= 1 {
                        issues.push(Issue::error(
                            "MISSING_PATIENT_CLASS",
                            Some("PV1.2".to_string()),
                            "Patient class is required".to_string(),
                        ));
                    } else {
                        let patient_class = segment.fields[1].first_text();
                        if patient_class.is_none()
                            || patient_class.map(|s| s.is_empty()).unwrap_or(true)
                        {
                            issues.push(Issue::error(
                                "MISSING_PATIENT_CLASS",
                                Some("PV1.2".to_string()),
                                "Patient class is required".to_string(),
                            ));
                        }
                    }
                }
                "OBX" => {
                    // OBX.2 - Value Type (required)
                    if segment.fields.len() <= 1 {
                        issues.push(Issue::error(
                            "MISSING_VALUE_TYPE",
                            Some("OBX.2".to_string()),
                            "Value type is required for observations".to_string(),
                        ));
                    } else {
                        let value_type = segment.fields[1].first_text();
                        if value_type.is_none() || value_type.map(|s| s.is_empty()).unwrap_or(true)
                        {
                            issues.push(Issue::error(
                                "MISSING_VALUE_TYPE",
                                Some("OBX.2".to_string()),
                                "Value type is required for observations".to_string(),
                            ));
                        }
                    }

                    // OBX.5 - Observation Value
                    if segment.fields.len() > 4 {
                        let obs_value = segment.fields[4].first_text();
                        if obs_value.is_none() || obs_value.map(|s| s.is_empty()).unwrap_or(true) {
                            issues.push(Issue::warning(
                                "MISSING_OBSERVATION_VALUE",
                                Some("OBX.5".to_string()),
                                "Observation value is empty".to_string(),
                            ));
                        }
                    }
                }
                _ => {}
            }
        }

        issues
    }
}

/// Lenient validator with minimal checks
struct LenientValidator;

impl Validator for LenientValidator {
    fn validate(&self, msg: &hl7v2_core::Message) -> Vec<Issue> {
        let mut issues = Vec::new();

        // Only check for critical errors
        if msg.segments.is_empty() {
            issues.push(Issue::error(
                "EMPTY_MESSAGE",
                None,
                "Message has no segments".to_string(),
            ));
            return issues;
        }

        // Check MSH is first segment
        if msg.segments[0].id_str() != "MSH" {
            issues.push(Issue::error(
                "MISSING_MSH",
                None,
                "Message must start with MSH segment".to_string(),
            ));
        }

        issues
    }
}

/// Benchmark basic validation
fn bench_basic_validation(c: &mut Criterion) {
    let message = create_sample_message();
    let bytes = message.as_bytes();
    let parsed = parse(bytes).expect("Failed to parse message");
    let validator = BasicValidator;

    c.bench_function("basic_validation", |b| {
        b.iter(|| {
            let result = validator.validate(black_box(&parsed));
            black_box(result)
        })
    });
}

/// Benchmark strict validation
fn bench_strict_validation(c: &mut Criterion) {
    let message = create_sample_message();
    let bytes = message.as_bytes();
    let parsed = parse(bytes).expect("Failed to parse message");
    let validator = StrictValidator;

    c.bench_function("strict_validation", |b| {
        b.iter(|| {
            let result = validator.validate(black_box(&parsed));
            black_box(result)
        })
    });
}

/// Benchmark lenient validation
fn bench_lenient_validation(c: &mut Criterion) {
    let message = create_sample_message();
    let bytes = message.as_bytes();
    let parsed = parse(bytes).expect("Failed to parse message");
    let validator = LenientValidator;

    c.bench_function("lenient_validation", |b| {
        b.iter(|| {
            let result = validator.validate(black_box(&parsed));
            black_box(result)
        })
    });
}

/// Benchmark validation with complex message
fn bench_complex_message_validation(c: &mut Criterion) {
    let message = create_complex_message();
    let bytes = message.as_bytes();
    let parsed = parse(bytes).expect("Failed to parse message");
    let validator = StrictValidator;

    c.bench_function("complex_message_validation", |b| {
        b.iter(|| {
            let result = validator.validate(black_box(&parsed));
            black_box(result)
        })
    });
}

/// Benchmark validation of messages with issues
fn bench_validation_with_issues(c: &mut Criterion) {
    let message = create_message_with_issues();
    let bytes = message.as_bytes();
    let parsed = parse(bytes).expect("Failed to parse message");
    let validator = StrictValidator;

    c.bench_function("validation_with_issues", |b| {
        b.iter(|| {
            let result = validator.validate(black_box(&parsed));
            black_box(result)
        })
    });
}

/// Benchmark data type validation
fn bench_data_type_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("data_type_validation");

    // Date validation
    let date_value = "20250101";
    group.bench_function("date", |b| {
        b.iter(|| {
            let result = validate_data_type(black_box(date_value), black_box("DT"));
            black_box(result)
        })
    });

    // Time validation
    let time_value = "120000";
    group.bench_function("time", |b| {
        b.iter(|| {
            let result = validate_data_type(black_box(time_value), black_box("TM"));
            black_box(result)
        })
    });

    // Timestamp validation
    let ts_value = "20250101120000";
    group.bench_function("timestamp", |b| {
        b.iter(|| {
            let result = validate_data_type(black_box(ts_value), black_box("TS"));
            black_box(result)
        })
    });

    // Numeric validation
    let nm_value = "123.45";
    group.bench_function("numeric", |b| {
        b.iter(|| {
            let result = validate_data_type(black_box(nm_value), black_box("NM"));
            black_box(result)
        })
    });

    // String validation
    let st_value = "Test String Value";
    group.bench_function("string", |b| {
        b.iter(|| {
            let result = validate_data_type(black_box(st_value), black_box("ST"));
            black_box(result)
        })
    });

    // Identifier validation
    let id_value = "ADT_A01";
    group.bench_function("identifier", |b| {
        b.iter(|| {
            let result = validate_data_type(black_box(id_value), black_box("ID"));
            black_box(result)
        })
    });

    group.finish();
}

/// Benchmark validation throughput with multiple messages
fn bench_validation_throughput(c: &mut Criterion) {
    let message = create_sample_message();
    let bytes = message.as_bytes();
    let parsed = parse(bytes).expect("Failed to parse message");
    let validator = StrictValidator;

    let mut group = c.benchmark_group("validation_throughput");

    for count in [1, 10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            b.iter(|| {
                for _ in 0..count {
                    let result = validator.validate(black_box(&parsed));
                    black_box(result);
                }
            })
        });
    }

    group.finish();
}

/// Compare strict vs lenient validation performance
fn bench_strict_vs_lenient_comparison(c: &mut Criterion) {
    let message = create_complex_message();
    let bytes = message.as_bytes();
    let parsed = parse(bytes).expect("Failed to parse message");
    let strict = StrictValidator;
    let lenient = LenientValidator;

    let mut group = c.benchmark_group("strict_vs_lenient");

    group.bench_function("strict", |b| {
        b.iter(|| {
            let result = strict.validate(black_box(&parsed));
            black_box(result)
        })
    });

    group.bench_function("lenient", |b| {
        b.iter(|| {
            let result = lenient.validate(black_box(&parsed));
            black_box(result)
        })
    });

    group.finish();
}

criterion_group!(
    validation_benches,
    bench_basic_validation,
    bench_strict_validation,
    bench_lenient_validation,
    bench_complex_message_validation,
    bench_validation_with_issues,
    bench_data_type_validation,
    bench_validation_throughput,
    bench_strict_vs_lenient_comparison,
);

criterion_main!(validation_benches);
