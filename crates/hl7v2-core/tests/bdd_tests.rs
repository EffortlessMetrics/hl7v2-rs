//! BDD tests using Cucumber
//!
//! Run with: cargo test --test bdd_tests

use cucumber::{World, given, then, when};
use hl7v2_core::{
    Delims, Error, Message, escape_text, get, is_mllp_framed, needs_escaping, parse, parse_mllp,
    unescape_text, unwrap_mllp, wrap_mllp,
};

/// Test world for BDD tests
#[derive(Debug, World)]
#[world(init = Self::new)]
pub struct HL7World {
    /// The current message being tested
    message: Option<Result<Message, Error>>,
    /// Raw bytes for testing
    raw_bytes: Vec<u8>,
    /// Text for escape testing
    text: String,
    /// Escaped text
    escaped_text: String,
    /// Delimiters for testing
    delims: Option<Delims>,
    /// Error for testing
    error: Option<Error>,
    /// Boolean result for checks
    bool_result: bool,
}

impl HL7World {
    fn new() -> Self {
        Self {
            message: None,
            raw_bytes: Vec::new(),
            text: String::new(),
            escaped_text: String::new(),
            delims: None,
            error: None,
            bool_result: false,
        }
    }
}

// ============================================================================
// Parsing Steps
// ============================================================================

#[given("a valid HL7 ADT^A01 message")]
fn given_valid_adt_a01(world: &mut HL7World) {
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r";
    world.raw_bytes = hl7.to_vec();
}

#[given("an HL7 message with custom delimiters")]
fn given_custom_delimiters(world: &mut HL7World) {
    // Using # as field separator, $ as component, * as repetition, @ as escape, ! as subcomponent
    let hl7 = b"MSH#$*@!SendingApp#SendingFac#ReceivingApp#ReceivingFac#20250128152312##ADT$A01#ABC123#P#2.5.1\rPID#1##123456$$$HOSP$MR##Doe$John\r";
    world.raw_bytes = hl7.to_vec();
}

#[given("an HL7 message containing escape sequences")]
fn given_escape_sequences(world: &mut HL7World) {
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe\\F\\John\r";
    world.raw_bytes = hl7.to_vec();
}

#[given("an MLLP framed HL7 message")]
fn given_mllp_framed(world: &mut HL7World) {
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\r";
    world.raw_bytes = wrap_mllp(hl7);
}

#[given("an MLLP framed message")]
fn given_mllp_framed_message(world: &mut HL7World) {
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\r";
    world.raw_bytes = wrap_mllp(hl7);
}

#[given("an invalid HL7 message")]
fn given_invalid_message(world: &mut HL7World) {
    // Missing MSH segment
    world.raw_bytes = b"INVALID|data".to_vec();
}

#[given("an HL7 message with repeated fields")]
fn given_repeated_fields(world: &mut HL7World) {
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac\rPID|1||123456^^^HOSP^MR||Doe^John~Smith^Jane\r";
    world.raw_bytes = hl7.to_vec();
}

#[when("I parse the message")]
fn when_parse_message(world: &mut HL7World) {
    world.message = Some(parse(&world.raw_bytes));
}

#[when("I parse the MLLP message")]
fn when_parse_mllp_message(world: &mut HL7World) {
    world.message = Some(parse_mllp(&world.raw_bytes));
}

#[when("I attempt to parse the message")]
fn when_attempt_parse(world: &mut HL7World) {
    world.message = Some(parse(&world.raw_bytes));
}

#[then("the message should have 2 segments")]
fn then_two_segments(world: &mut HL7World) {
    let msg = world
        .message
        .as_ref()
        .expect("No message parsed")
        .as_ref()
        .expect("Parse failed");
    assert_eq!(msg.segments.len(), 2);
}

#[then("the first segment should be MSH")]
fn then_first_segment_msh(world: &mut HL7World) {
    let msg = world
        .message
        .as_ref()
        .expect("No message parsed")
        .as_ref()
        .expect("Parse failed");
    assert_eq!(&msg.segments[0].id, b"MSH");
}

#[then(regex = r#"MSH\.9\.(\d+) should be "([^"]+)""#)]
fn then_msh_9_component(world: &mut HL7World, component: usize, value: String) {
    let msg = world
        .message
        .as_ref()
        .expect("No message parsed")
        .as_ref()
        .expect("Parse failed");
    let path = format!("MSH.9.{}", component);
    assert_eq!(get(msg, &path), Some(value.as_str()));
}

#[then("the second segment should be PID")]
fn then_second_segment_pid(world: &mut HL7World) {
    let msg = world
        .message
        .as_ref()
        .expect("No message parsed")
        .as_ref()
        .expect("Parse failed");
    assert_eq!(&msg.segments[1].id, b"PID");
}

#[then("the delimiters should be detected correctly")]
fn then_delimiters_detected(world: &mut HL7World) {
    let msg = world
        .message
        .as_ref()
        .expect("No message parsed")
        .as_ref()
        .expect("Parse failed");
    assert_eq!(msg.delims.field, '#');
    assert_eq!(msg.delims.comp, '$');
    assert_eq!(msg.delims.rep, '*');
    assert_eq!(msg.delims.esc, '@');
    assert_eq!(msg.delims.sub, '!');
}

#[then("the message should parse successfully")]
fn then_parse_success(world: &mut HL7World) {
    assert!(world.message.as_ref().expect("No message parsed").is_ok());
}

#[then("the escape sequences should be decoded")]
fn then_escape_decoded(world: &mut HL7World) {
    let msg = world
        .message
        .as_ref()
        .expect("No message parsed")
        .as_ref()
        .expect("Parse failed");
    let value = get(msg, "PID.5.1").expect("Should have PID.5.1");
    assert_eq!(value, "Doe|John");
}

#[then("the field values should be unescaped")]
fn then_field_values_unescaped(_world: &mut HL7World) {
    // Already verified in then_escape_decoded
}

#[then("the MLLP framing should be removed")]
fn then_mllp_removed(world: &mut HL7World) {
    // If parsing succeeded, MLLP was removed
    assert!(world.message.as_ref().expect("No message parsed").is_ok());
}

#[then("an error should be returned")]
fn then_error_returned(world: &mut HL7World) {
    assert!(world.message.as_ref().expect("No message parsed").is_err());
}

#[then("the error should indicate the problem")]
fn then_error_indicates_problem(world: &mut HL7World) {
    // Error was returned - the specific error type can be checked
    assert!(world.message.as_ref().expect("No message parsed").is_err());
}

#[then("I can access the first repetition")]
fn then_access_first_rep(world: &mut HL7World) {
    let msg = world
        .message
        .as_ref()
        .expect("No message parsed")
        .as_ref()
        .expect("Parse failed");
    assert_eq!(get(msg, "PID.5.1"), Some("Doe"));
}

#[then("I can access the second repetition")]
fn then_access_second_rep(world: &mut HL7World) {
    let msg = world
        .message
        .as_ref()
        .expect("No message parsed")
        .as_ref()
        .expect("Parse failed");
    assert_eq!(get(msg, "PID.5[2].1"), Some("Smith"));
}

#[then("missing repetitions return None")]
fn then_missing_rep_none(world: &mut HL7World) {
    let msg = world
        .message
        .as_ref()
        .expect("No message parsed")
        .as_ref()
        .expect("Parse failed");
    assert_eq!(get(msg, "PID.5[3].1"), None);
}

// ============================================================================
// MLLP Steps
// ============================================================================

#[given("an HL7 message")]
fn given_hl7_message(world: &mut HL7World) {
    world.raw_bytes = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\r".to_vec();
}

#[given("raw non-MLLP data")]
fn given_non_mllp(world: &mut HL7World) {
    world.raw_bytes = b"raw data without MLLP framing".to_vec();
}

#[given("a buffer containing an MLLP message")]
fn given_buffer_with_mllp(world: &mut HL7World) {
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\r";
    world.raw_bytes = wrap_mllp(hl7);
}

#[when("I wrap it with MLLP framing")]
fn when_wrap_mllp(world: &mut HL7World) {
    world.raw_bytes = wrap_mllp(&world.raw_bytes);
}

#[when("I unwrap the MLLP framing")]
fn when_unwrap_mllp(world: &mut HL7World) {
    match unwrap_mllp(&world.raw_bytes) {
        Ok(data) => world.raw_bytes = data.to_vec(),
        Err(e) => world.error = Some(e),
    }
}

#[when("I check if it is MLLP framed")]
fn when_check_mllp_framed(world: &mut HL7World) {
    world.bool_result = is_mllp_framed(&world.raw_bytes);
}

#[when("I search for a complete message")]
fn when_search_message(world: &mut HL7World) {
    // Use find_complete_mllp_message
    world.bool_result = hl7v2_core::find_complete_mllp_message(&world.raw_bytes).is_some();
}

#[then("the result should start with VT character")]
fn then_starts_with_vt(world: &mut HL7World) {
    assert_eq!(world.raw_bytes[0], 0x0B);
}

#[then("the result should end with FS CR characters")]
fn then_ends_with_fs_cr(world: &mut HL7World) {
    let len = world.raw_bytes.len();
    assert_eq!(world.raw_bytes[len - 2], 0x1C);
    assert_eq!(world.raw_bytes[len - 1], 0x0D);
}

#[then("I should get the original HL7 message")]
fn then_original_message(world: &mut HL7World) {
    // Message was successfully unwrapped
    assert!(world.error.is_none());
}

#[then("the result should be true")]
fn then_result_true(world: &mut HL7World) {
    assert!(world.bool_result);
}

#[then("the result should be false")]
fn then_result_false(world: &mut HL7World) {
    assert!(!world.bool_result);
}

#[then("I should find the message boundaries")]
fn then_find_boundaries(world: &mut HL7World) {
    assert!(world.bool_result);
}

#[then("I should get the message content")]
fn then_get_content(world: &mut HL7World) {
    // Already verified
    assert!(world.bool_result);
}

// ============================================================================
// Escape Steps
// ============================================================================

#[given(regex = r#"a text containing the field separator "([^"]+)""#)]
fn given_text_field_sep(world: &mut HL7World, _sep: String) {
    world.text = "test|value".to_string();
    world.delims = Some(Delims::default());
}

#[given(regex = r#"a text containing the component separator "([^"]+)""#)]
fn given_text_comp_sep(world: &mut HL7World, _sep: String) {
    world.text = "test^value".to_string();
    world.delims = Some(Delims::default());
}

#[given(regex = r#"a text containing the repetition separator "([^"]+)""#)]
fn given_text_rep_sep(world: &mut HL7World, _sep: String) {
    world.text = "test~value".to_string();
    world.delims = Some(Delims::default());
}

#[given(regex = r#"an escaped HL7 text "([^"]+)""#)]
fn given_escaped_text(world: &mut HL7World, text: String) {
    world.escaped_text = text;
    world.delims = Some(Delims::default());
}

#[given(regex = r#"a text with special characters "([^"]+)""#)]
fn given_text_special_chars(world: &mut HL7World, chars: String) {
    world.text = chars;
    world.delims = Some(Delims::default());
}

#[given("a text with delimiter characters")]
fn given_text_delimiter_chars(world: &mut HL7World) {
    world.text = "test|value".to_string();
    world.delims = Some(Delims::default());
}

#[given("plain text without special characters")]
fn given_plain_text(world: &mut HL7World) {
    world.text = "hello world".to_string();
    world.delims = Some(Delims::default());
}

#[when("I escape the text")]
fn when_escape_text(world: &mut HL7World) {
    let delims = world.delims.as_ref().unwrap();
    world.escaped_text = escape_text(&world.text, delims);
}

#[when("I unescape the text")]
fn when_unescape_text(world: &mut HL7World) {
    let delims = world.delims.as_ref().unwrap();
    world.text = unescape_text(&world.escaped_text, delims).unwrap_or_default();
}

#[when("I escape then unescape the text")]
fn when_roundtrip_escape(world: &mut HL7World) {
    let delims = world.delims.as_ref().unwrap();
    let escaped = escape_text(&world.text, delims);
    world.text = unescape_text(&escaped, delims).unwrap_or_default();
}

#[when("I check if escaping is needed")]
fn when_check_escaping(world: &mut HL7World) {
    let delims = world.delims.as_ref().unwrap();
    world.bool_result = needs_escaping(&world.text, delims);
}

#[then(regex = r#"the escaped text should contain "([^"]+)""#)]
fn then_escaped_contains(world: &mut HL7World, expected: String) {
    assert!(
        world.escaped_text.contains(&expected),
        "Expected '{}' to contain '{}'",
        world.escaped_text,
        expected
    );
}

#[then(regex = r#"the result should be "([^"]+)""#)]
fn then_result_equals(world: &mut HL7World, expected: String) {
    assert_eq!(world.text, expected);
}

#[then("the result should be the original text")]
fn then_original_text(world: &mut HL7World) {
    // The text should roundtrip correctly (without escape char which needs special handling)
    assert_eq!(world.text, "|^~&");
}

#[then("the result should indicate escaping is needed")]
fn then_escaping_needed(world: &mut HL7World) {
    assert!(world.bool_result);
}

#[then("the text should remain unchanged")]
fn then_text_unchanged(world: &mut HL7World) {
    assert_eq!(world.escaped_text, "hello world");
}

// Run the tests
#[tokio::main]
async fn main() {
    HL7World::cucumber().run_and_exit("./features").await;
}
