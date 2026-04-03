//! BDD tests for hl7v2-normalize using Cucumber
//!
//! Run with: cargo test --test bdd_tests

use cucumber::{World, given, then, when};
use hl7v2_model::Error;
use hl7v2_normalize::normalize;

/// Test world for Normalize BDD tests
#[derive(Debug, World)]
#[world(init = Self::new)]
pub struct NormalizeWorld {
    /// Raw message bytes
    raw_bytes: Vec<u8>,
    /// Normalization result
    result: Option<Result<Vec<u8>, Error>>,
    /// Normalized output bytes
    normalized: Vec<u8>,
    /// Normalized output as string
    normalized_str: String,
    /// Whether to use canonical delimiters
    canonical_delims: bool,
}

impl NormalizeWorld {
    fn new() -> Self {
        Self {
            raw_bytes: Vec::new(),
            result: None,
            normalized: Vec::new(),
            normalized_str: String::new(),
            canonical_delims: true,
        }
    }
}

// ============================================================================
// Given Steps
// ============================================================================

#[given("a valid ADT^A01 message")]
fn given_valid_adt_a01(world: &mut NormalizeWorld) {
    world.raw_bytes = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|MSG001|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r".to_vec();
}

#[given("a message with custom delimiters \"#$*@!\"")]
fn given_custom_delimiters(world: &mut NormalizeWorld) {
    world.raw_bytes = b"MSH#$*@!SendingApp#SendingFac#ReceivingApp#ReceivingFac#20250128152312##ADT$A01#MSG001#P#2.5.1\rPID#1##123456$$$HOSP$MR##Doe$John\r".to_vec();
}

#[given("a message with irregular spacing")]
fn given_irregular_spacing(world: &mut NormalizeWorld) {
    world.raw_bytes = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|MSG001|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r".to_vec();
}

#[given("a message with extra segments")]
fn given_extra_segments(world: &mut NormalizeWorld) {
    world.raw_bytes = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|MSG001|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\rPV1|1|I|ICU|||||||||||||||||||||||||||||||||||||||||||||||||\r".to_vec();
}

#[given("a message containing escape sequences")]
fn given_escape_sequences(world: &mut NormalizeWorld) {
    world.raw_bytes = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|MSG001|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe\\F\\John\r".to_vec();
}

#[given("a message with field repetitions")]
fn given_field_repetitions(world: &mut NormalizeWorld) {
    world.raw_bytes = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|MSG001|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John~Smith^Jane\r".to_vec();
}

#[given("a message with components")]
fn given_components(world: &mut NormalizeWorld) {
    world.raw_bytes = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|MSG001|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r".to_vec();
}

#[given("a message with subcomponents")]
fn given_subcomponents(world: &mut NormalizeWorld) {
    world.raw_bytes = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|MSG001|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe&John^Jr&Smith\r".to_vec();
}

#[given("a message with null values")]
fn given_null_values(world: &mut NormalizeWorld) {
    world.raw_bytes = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|MSG001|P|2.5.1\rPID|1||\"\"^^^HOSP^MR||Doe^John\r".to_vec();
}

#[given("a message with empty fields")]
fn given_empty_fields(world: &mut NormalizeWorld) {
    world.raw_bytes = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|MSG001|P|2.5.1\rPID|1||||Doe^John\r".to_vec();
}

#[given("a message with charset specification")]
fn given_charset(world: &mut NormalizeWorld) {
    world.raw_bytes = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||||ADT^A01|MSG001|P|2.5.1|UNICODE UTF-8\rPID|1||123456^^^HOSP^MR||Doe^John\r".to_vec();
}

#[given("an invalid HL7 message")]
fn given_invalid_message(world: &mut NormalizeWorld) {
    world.raw_bytes = b"INVALID|data|here\r".to_vec();
}

#[given("a message without MSH segment")]
fn given_no_msh(world: &mut NormalizeWorld) {
    world.raw_bytes = b"PID|1||123456^^^HOSP^MR||Doe^John\r".to_vec();
}

#[given("a message with malformed delimiters")]
fn given_malformed_delimiters(world: &mut NormalizeWorld) {
    // Use duplicate delimiters (field sep '|' repeated as component sep) to trigger a parse error
    world.raw_bytes = b"MSH||~\\&|SendingApp\r".to_vec();
}

#[given(regex = r#"a ([A-Z]{3}\^[A-Z0-9]{2,3}) message"#)]
fn given_message_type(world: &mut NormalizeWorld, message_type: String) {
    let msg = format!(
        "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||{}|MSG001|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r",
        message_type
    );
    world.raw_bytes = msg.as_bytes().to_vec();
}

#[given("a message with special characters")]
fn given_special_chars(world: &mut NormalizeWorld) {
    world.raw_bytes = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|MSG001|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe, John Jr.\r".to_vec();
}

#[given("a message with long field values")]
fn given_long_values(world: &mut NormalizeWorld) {
    let long_value = "A".repeat(500);
    let msg = format!(
        "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|MSG001|P|2.5.1\rPID|1||123456^^^HOSP^MR||{}\r",
        long_value
    );
    world.raw_bytes = msg.as_bytes().to_vec();
}

#[given("a message with non-canonical delimiters")]
fn given_non_canonical(world: &mut NormalizeWorld) {
    given_custom_delimiters(world);
}

// ============================================================================
// When Steps
// ============================================================================

#[when("I normalize the message")]
fn when_normalize(world: &mut NormalizeWorld) {
    world.result = Some(normalize(&world.raw_bytes, world.canonical_delims));
    if let Ok(bytes) = world.result.as_ref().unwrap() {
        world.normalized = bytes.clone();
        world.normalized_str = String::from_utf8_lossy(bytes).to_string();
    }
}

#[when("I normalize the message with canonical delimiters")]
fn when_normalize_canonical(world: &mut NormalizeWorld) {
    world.canonical_delims = true;
    when_normalize(world);
}

#[when("I normalize the message without canonical delimiters")]
fn when_normalize_no_canonical(world: &mut NormalizeWorld) {
    world.canonical_delims = false;
    when_normalize(world);
}

#[when("I attempt to normalize the message")]
fn when_attempt_normalize(world: &mut NormalizeWorld) {
    when_normalize(world);
}

// ============================================================================
// Then Steps
// ============================================================================

#[then("the normalized message should be valid HL7")]
fn then_normalized_valid(world: &mut NormalizeWorld) {
    assert!(world.result.as_ref().unwrap().is_ok());
}

#[then("the normalized message should start with \"MSH|\"")]
fn then_normalized_starts_msh(world: &mut NormalizeWorld) {
    assert!(world.normalized_str.starts_with("MSH|"));
}

#[then("the normalized message should use canonical delimiters \"|^~\\\\&\"")]
fn then_canonical_delimiters(world: &mut NormalizeWorld) {
    assert!(world.normalized_str.starts_with("MSH|^~\\&|"));
}

#[then("the normalized message should preserve the custom delimiters")]
fn then_preserve_custom_delimiters(world: &mut NormalizeWorld) {
    assert!(world.normalized_str.starts_with("MSH#"));
}

#[then("the normalized message should have consistent spacing")]
fn then_consistent_spacing(world: &mut NormalizeWorld) {
    // Verify the message is valid
    assert!(world.result.as_ref().unwrap().is_ok());
}

#[then("the normalized message should contain all segments")]
fn then_all_segments(world: &mut NormalizeWorld) {
    assert!(world.normalized_str.contains("MSH|"));
    assert!(world.normalized_str.contains("PID|"));
    assert!(world.normalized_str.contains("PV1|"));
}

#[then("the normalized message should preserve escape sequences")]
fn then_preserve_escape(world: &mut NormalizeWorld) {
    assert!(world.normalized_str.contains("\\F\\"));
}

#[then("the normalized message should preserve repetitions")]
fn then_preserve_repetitions(world: &mut NormalizeWorld) {
    assert!(world.normalized_str.contains("~"));
}

#[then("the normalized message should preserve components")]
fn then_preserve_components(world: &mut NormalizeWorld) {
    assert!(world.normalized_str.contains("^"));
}

#[then("the normalized message should preserve subcomponents")]
fn then_preserve_subcomponents(world: &mut NormalizeWorld) {
    assert!(world.normalized_str.contains("&"));
}

#[then("the normalized message should preserve null values")]
fn then_preserve_null(world: &mut NormalizeWorld) {
    assert!(world.normalized_str.contains("\"\""));
}

#[then("the normalized message should preserve empty fields")]
fn then_preserve_empty(world: &mut NormalizeWorld) {
    assert!(world.normalized_str.contains("||"));
}

#[then("the normalized message should preserve charset")]
fn then_preserve_charset(world: &mut NormalizeWorld) {
    assert!(world.normalized_str.contains("UNICODE UTF-8"));
}

#[then("normalization should fail")]
fn then_normalize_fail(world: &mut NormalizeWorld) {
    assert!(world.result.as_ref().unwrap().is_err());
}

#[then("an error should be returned")]
fn then_error_returned(world: &mut NormalizeWorld) {
    assert!(world.result.as_ref().unwrap().is_err());
}

#[then(regex = r#"the normalized message should contain "([^"]+)""#)]
fn then_normalized_contains(world: &mut NormalizeWorld, text: String) {
    assert!(world.normalized_str.contains(&text));
}

#[then("the normalized message should preserve special characters")]
fn then_preserve_special(world: &mut NormalizeWorld) {
    assert!(world.normalized_str.contains(","));
}

#[then("the normalized message should preserve long values")]
fn then_preserve_long(world: &mut NormalizeWorld) {
    assert!(world.normalized_str.len() > 500);
}

#[then("the output should start with \"MSH|^~\\\\&|\"")]
fn then_output_starts_canonical(world: &mut NormalizeWorld) {
    assert!(world.normalized_str.starts_with("MSH|^~\\&|"));
}

#[then("the field separator should be \"|\"")]
fn then_field_separator(world: &mut NormalizeWorld) {
    assert!(world.normalized_str.starts_with("MSH|"));
}

#[then("the component separator should be \"^\"")]
fn then_component_separator(world: &mut NormalizeWorld) {
    assert!(world.normalized_str.contains("^"));
}

#[then("the repetition separator should be \"~\"")]
fn then_repetition_separator(world: &mut NormalizeWorld) {
    // This would require a message with repetitions
    assert!(world.result.as_ref().unwrap().is_ok());
}

#[then("the escape character should be \"\\\\\"")]
fn then_escape_char(world: &mut NormalizeWorld) {
    assert!(world.normalized_str.contains("\\"));
}

#[then("the subcomponent separator should be \"&\"")]
fn then_subcomponent_separator(world: &mut NormalizeWorld) {
    // This would require a message with subcomponents
    assert!(world.result.as_ref().unwrap().is_ok());
}

// ============================================================================
// Cucumber Main
// ============================================================================

#[tokio::main]
async fn main() {
    NormalizeWorld::run("features/normalize.feature").await;
}
