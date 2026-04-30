//! BDD tests for hl7v2-datatype using Cucumber
//!
//! Run with: cargo test --test bdd_tests -p hl7v2-datatype

use cucumber::{World, given, then, when};
use hl7v2_datatype::{
    ChecksumAlgorithm, DataType, DataTypeError, DataTypeValidator, is_email, is_ssn,
    is_valid_age_range, is_valid_birth_date, is_within_range, matches_format, validate_datatype,
    validate_luhn_checksum,
};

/// Test world for datatype BDD tests
#[derive(Debug, World)]
#[world(init = Self::new)]
pub struct DatatypeWorld {
    /// The value under test
    value: String,
    /// The data type code
    datatype_code: String,
    /// The parsed data type result
    parsed_datatype: Option<DataType>,
    /// Whether the last validation succeeded
    validation_valid: bool,
    /// The validator builder
    validator: Option<DataTypeValidator>,
    /// Detailed validation error (if any)
    validation_error: Option<DataTypeError>,
    /// Birth date for age range tests
    birth_date: String,
    /// Reference date for age range tests
    reference_date: String,
    /// Numeric range min
    range_min: String,
    /// Numeric range max
    range_max: String,
}

impl DatatypeWorld {
    fn new() -> Self {
        Self {
            value: String::new(),
            datatype_code: String::new(),
            parsed_datatype: None,
            validation_valid: false,
            validator: None,
            validation_error: None,
            birth_date: String::new(),
            reference_date: String::new(),
            range_min: String::new(),
            range_max: String::new(),
        }
    }
}

// ============================================================================
// Given Steps
// ============================================================================

#[given(regex = r#"^a data type code "([^"]*)"$"#)]
fn given_datatype_code(world: &mut DatatypeWorld, code: String) {
    world.datatype_code = code;
}

#[given(regex = r#"^a value "([^"]*)"$"#)]
fn given_value(world: &mut DatatypeWorld, value: String) {
    world.value = value;
}

#[given(regex = r#"^a value with embedded newline "([^"]*)"$"#)]
fn given_value_with_newline(world: &mut DatatypeWorld, raw: String) {
    // The feature file uses literal \n in the string; convert it to an actual newline
    world.value = raw.replace("\\n", "\n");
}

#[given("a birth date in the past")]
fn given_birth_date_past(world: &mut DatatypeWorld) {
    world.value = "19900101".to_string();
}

#[given("a birth date in the future")]
fn given_birth_date_future(world: &mut DatatypeWorld) {
    world.value = "30000101".to_string();
}

#[given(regex = r#"^a birth date "([^"]*)" and reference date "([^"]*)"$"#)]
fn given_birth_and_reference(world: &mut DatatypeWorld, birth: String, reference: String) {
    world.birth_date = birth;
    world.reference_date = reference;
}

#[given(regex = r#"^a value "([^"]*)" with range "([^"]*)" to "([^"]*)"$"#)]
fn given_value_with_range(world: &mut DatatypeWorld, value: String, min: String, max: String) {
    world.value = value;
    world.range_min = min;
    world.range_max = max;
}

#[given(regex = r"^a validator with min length (\d+)$")]
fn given_validator_min_length(world: &mut DatatypeWorld, min: usize) {
    world.validator = Some(DataTypeValidator::new().with_min_length(min));
}

#[given(regex = r"^a validator with max length (\d+)$")]
fn given_validator_max_length(world: &mut DatatypeWorld, max: usize) {
    world.validator = Some(DataTypeValidator::new().with_max_length(max));
}

#[given(regex = r"^a validator with min length (\d+) and max length (\d+)$")]
fn given_validator_min_max(world: &mut DatatypeWorld, min: usize, max: usize) {
    world.validator = Some(
        DataTypeValidator::new()
            .with_min_length(min)
            .with_max_length(max),
    );
}

#[given(regex = r#"^a validator with pattern "([^"]*)"$"#)]
fn given_validator_pattern(world: &mut DatatypeWorld, pattern: String) {
    world.validator = Some(DataTypeValidator::new().with_pattern(&pattern));
}

#[given(regex = r#"^a validator with allowed values "([^"]*)"$"#)]
fn given_validator_allowed_values(world: &mut DatatypeWorld, values_str: String) {
    let values: Vec<String> = values_str.split(',').map(|s| s.to_string()).collect();
    world.validator = Some(DataTypeValidator::new().with_allowed_values(values));
}

#[given("a validator with Luhn checksum")]
fn given_validator_luhn(world: &mut DatatypeWorld) {
    world.validator = Some(DataTypeValidator::new().with_checksum(ChecksumAlgorithm::Luhn));
}

#[given(regex = r"^a validator with min length (\d+) and max length (\d+) and Luhn checksum$")]
fn given_validator_min_max_luhn(world: &mut DatatypeWorld, min: usize, max: usize) {
    world.validator = Some(
        DataTypeValidator::new()
            .with_min_length(min)
            .with_max_length(max)
            .with_checksum(ChecksumAlgorithm::Luhn),
    );
}

// ============================================================================
// When Steps
// ============================================================================

#[when("I parse the data type code")]
fn when_parse_datatype(world: &mut DatatypeWorld) {
    world.parsed_datatype = DataType::parse(&world.datatype_code);
}

#[when(regex = r#"^I validate it as data type "([^"]*)"$"#)]
fn when_validate_as_datatype(world: &mut DatatypeWorld, dtype: String) {
    world.validation_valid = validate_datatype(&world.value, &dtype);
}

#[when("I validate it as an email")]
fn when_validate_email(world: &mut DatatypeWorld) {
    world.validation_valid = is_email(&world.value);
}

#[when("I validate it as an SSN")]
fn when_validate_ssn(world: &mut DatatypeWorld) {
    world.validation_valid = is_ssn(&world.value);
}

#[when("I validate it with Luhn checksum")]
fn when_validate_luhn(world: &mut DatatypeWorld) {
    world.validation_valid = validate_luhn_checksum(&world.value);
}

#[when(regex = r#"^I validate value "([^"]*)" with the validator$"#)]
fn when_validate_with_validator(world: &mut DatatypeWorld, value: String) {
    let validator = world.validator.as_ref().expect("Validator not set");
    world.validation_valid = validator.validate(&value);
}

#[when(regex = r#"^I validate value "([^"]*)" with detailed errors$"#)]
fn when_validate_detailed(world: &mut DatatypeWorld, value: String) {
    let validator = world.validator.as_ref().expect("Validator not set");
    match validator.validate_detailed(&value) {
        Ok(()) => {
            world.validation_valid = true;
            world.validation_error = None;
        }
        Err(e) => {
            world.validation_valid = false;
            world.validation_error = Some(e);
        }
    }
}

#[when("I validate it as a birth date")]
fn when_validate_birth_date(world: &mut DatatypeWorld) {
    world.validation_valid = is_valid_birth_date(&world.value);
}

#[when("I validate the age range")]
fn when_validate_age_range(world: &mut DatatypeWorld) {
    world.validation_valid = is_valid_age_range(&world.birth_date, &world.reference_date);
}

#[when("I validate the numeric range")]
fn when_validate_numeric_range(world: &mut DatatypeWorld) {
    world.validation_valid = is_within_range(&world.value, &world.range_min, &world.range_max);
}

#[when(regex = r#"^I validate it matches format "([^"]*)" for data type "([^"]*)"$"#)]
fn when_validate_format(world: &mut DatatypeWorld, format: String, dtype: String) {
    world.validation_valid = matches_format(&world.value, &format, &dtype);
}

// ============================================================================
// Then Steps
// ============================================================================

#[then(regex = r"^the parsed data type should be (\w+)$")]
fn then_parsed_datatype(world: &mut DatatypeWorld, variant: String) {
    if variant == "None" {
        assert_eq!(
            world.parsed_datatype, None,
            "Expected None for code '{}', got {:?}",
            world.datatype_code, world.parsed_datatype
        );
    } else {
        let expected = DataType::parse(&variant);
        assert_eq!(
            world.parsed_datatype, expected,
            "Expected parsed data type {:?} for code '{}', got {:?}",
            expected, world.datatype_code, world.parsed_datatype
        );
    }
}

#[then("the validation result should be valid")]
fn then_valid(world: &mut DatatypeWorld) {
    assert!(
        world.validation_valid,
        "Expected validation to pass for value '{}'",
        world.value
    );
}

#[then("the validation result should be invalid")]
fn then_invalid(world: &mut DatatypeWorld) {
    assert!(
        !world.validation_valid,
        "Expected validation to fail for value '{}'",
        world.value
    );
}

#[then(regex = r"^the error should be TooShort with length (\d+) and min (\d+)$")]
fn then_error_too_short(world: &mut DatatypeWorld, length: usize, min: usize) {
    match &world.validation_error {
        Some(DataTypeError::TooShort { length: l, min: m }) => {
            assert_eq!(*l, length, "Expected length {}, got {}", length, l);
            assert_eq!(*m, min, "Expected min {}, got {}", min, m);
        }
        other => panic!("Expected TooShort error, got {:?}", other),
    }
}

#[then(regex = r"^the error should be TooLong with length (\d+) and max (\d+)$")]
fn then_error_too_long(world: &mut DatatypeWorld, length: usize, max: usize) {
    match &world.validation_error {
        Some(DataTypeError::TooLong { length: l, max: m }) => {
            assert_eq!(*l, length, "Expected length {}, got {}", length, l);
            assert_eq!(*m, max, "Expected max {}, got {}", max, m);
        }
        other => panic!("Expected TooLong error, got {:?}", other),
    }
}

#[then("the error should be PatternMismatch")]
fn then_error_pattern_mismatch(world: &mut DatatypeWorld) {
    match &world.validation_error {
        Some(DataTypeError::PatternMismatch { .. }) => {}
        other => panic!("Expected PatternMismatch error, got {:?}", other),
    }
}

#[then(regex = r#"^the error should be NotInAllowedSet with value "([^"]*)"$"#)]
fn then_error_not_in_allowed_set(world: &mut DatatypeWorld, expected_value: String) {
    match &world.validation_error {
        Some(DataTypeError::NotInAllowedSet { value }) => {
            assert_eq!(
                value, &expected_value,
                "Expected value '{}', got '{}'",
                expected_value, value
            );
        }
        other => panic!("Expected NotInAllowedSet error, got {:?}", other),
    }
}

#[then("the error should be ChecksumFailed")]
fn then_error_checksum_failed(world: &mut DatatypeWorld) {
    match &world.validation_error {
        Some(DataTypeError::ChecksumFailed) => {}
        other => panic!("Expected ChecksumFailed error, got {:?}", other),
    }
}

// ============================================================================
// Cucumber Main
// ============================================================================

#[tokio::main]
async fn main() {
    DatatypeWorld::run("tests/features/datatype.feature").await;
}
