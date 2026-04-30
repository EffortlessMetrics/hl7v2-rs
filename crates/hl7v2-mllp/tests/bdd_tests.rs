//! BDD tests for hl7v2-mllp using Cucumber
//!
//! Run with: cargo test --test bdd_tests -p hl7v2-mllp

use cucumber::{World, given, then, when};
use hl7v2_mllp::{
    MllpError, MllpFrameIterator, find_complete_mllp_message, is_mllp_framed, unwrap_mllp,
    unwrap_mllp_checked, wrap_mllp,
};

/// Test world for MLLP BDD tests
#[derive(Debug, World)]
#[world(init = Self::new)]
pub struct MllpWorld {
    /// Original HL7 message bytes
    original_message: Vec<u8>,
    /// MLLP-wrapped data
    wrapped_data: Vec<u8>,
    /// Unwrapped result (if successful)
    unwrapped_data: Option<Vec<u8>>,
    /// Boolean result from is_mllp_framed
    framed_check_result: Option<bool>,
    /// Error result from unwrap_mllp_checked
    checked_error: Option<MllpError>,
    /// Length found by find_complete_mllp_message
    found_length: Option<usize>,
    /// Frame iterator for streaming tests
    frame_iterator: MllpFrameIterator,
    /// Last extracted message from the iterator
    extracted_message: Option<Vec<u8>>,
    /// Raw bytes for direct manipulation
    raw_bytes: Vec<u8>,
}

impl MllpWorld {
    fn new() -> Self {
        Self {
            original_message: Vec::new(),
            wrapped_data: Vec::new(),
            unwrapped_data: None,
            framed_check_result: None,
            checked_error: None,
            found_length: None,
            frame_iterator: MllpFrameIterator::new(),
            extracted_message: None,
            raw_bytes: Vec::new(),
        }
    }
}

/// Interpret Gherkin string literals, converting `\r` escape to carriage return.
fn parse_hl7_string(s: &str) -> Vec<u8> {
    s.replace("\\r", "\r").into_bytes()
}

// ============================================================================
// Given Steps
// ============================================================================

#[given(regex = r#"^an HL7 message "([^"]*)"$"#)]
fn given_hl7_message(world: &mut MllpWorld, message: String) {
    world.original_message = parse_hl7_string(&message);
}

#[given("the message is wrapped with MLLP framing")]
fn given_message_is_wrapped(world: &mut MllpWorld) {
    world.wrapped_data = wrap_mllp(&world.original_message);
}

#[given(regex = r#"^raw bytes "([^"]*)"$"#)]
fn given_raw_bytes(world: &mut MllpWorld, data: String) {
    let bytes = parse_hl7_string(&data);
    world.raw_bytes = bytes.clone();
    world.wrapped_data = bytes;
}

#[given(regex = r#"^a byte sequence starting with 0x0B followed by "([^"]*)"$"#)]
fn given_byte_sequence_with_start(world: &mut MllpWorld, content: String) {
    let mut bytes = vec![0x0B];
    bytes.extend_from_slice(parse_hl7_string(&content).as_slice());
    world.raw_bytes = bytes.clone();
    world.wrapped_data = bytes;
}

#[given("an empty message")]
fn given_empty_message(world: &mut MllpWorld) {
    world.original_message = Vec::new();
}

#[given("an MLLP frame iterator")]
fn given_frame_iterator(world: &mut MllpWorld) {
    world.frame_iterator = MllpFrameIterator::new();
}

#[given(regex = r#"^an HL7 message "([^"]*)" wrapped with MLLP framing is added to the iterator$"#)]
fn given_message_added_to_iterator(world: &mut MllpWorld, message: String) {
    let msg_bytes = parse_hl7_string(&message);
    let framed = wrap_mllp(&msg_bytes);
    world.frame_iterator.extend(&framed);
}

// ============================================================================
// When Steps
// ============================================================================

#[when("I wrap it with MLLP framing")]
fn when_wrap_mllp(world: &mut MllpWorld) {
    world.wrapped_data = wrap_mllp(&world.original_message);
}

#[when("I unwrap the MLLP frame")]
fn when_unwrap_mllp(world: &mut MllpWorld) {
    let result = unwrap_mllp(&world.wrapped_data).expect("Unwrap failed");
    world.unwrapped_data = Some(result.to_vec());
}

#[when("I check if the data is MLLP framed")]
fn when_check_mllp_framed(world: &mut MllpWorld) {
    world.framed_check_result = Some(is_mllp_framed(&world.wrapped_data));
}

#[when("I try to unwrap the data with checked unwrap")]
fn when_try_unwrap_checked(world: &mut MllpWorld) {
    match unwrap_mllp_checked(&world.wrapped_data) {
        Ok(data) => world.unwrapped_data = Some(data.to_vec()),
        Err(e) => world.checked_error = Some(e),
    }
}

#[when("I search for a complete MLLP message")]
fn when_find_complete_message(world: &mut MllpWorld) {
    world.found_length = find_complete_mllp_message(&world.wrapped_data);
}

#[when("I extract the next message from the iterator")]
fn when_extract_next_message(world: &mut MllpWorld) {
    match world.frame_iterator.next_message() {
        Some(Ok(msg)) => world.extracted_message = Some(msg),
        Some(Err(e)) => panic!("Iterator returned error: {}", e),
        None => world.extracted_message = None,
    }
}

// ============================================================================
// Then Steps
// ============================================================================

#[then("the first byte should be 0x0B")]
fn then_first_byte_is_start(world: &mut MllpWorld) {
    assert_eq!(
        world.wrapped_data[0], 0x0B,
        "Expected first byte to be 0x0B (VT), got 0x{:02X}",
        world.wrapped_data[0]
    );
}

#[then("the second-to-last byte should be 0x1C")]
fn then_second_to_last_byte(world: &mut MllpWorld) {
    let len = world.wrapped_data.len();
    assert_eq!(
        world.wrapped_data[len - 2],
        0x1C,
        "Expected second-to-last byte to be 0x1C (FS), got 0x{:02X}",
        world.wrapped_data[len - 2]
    );
}

#[then("the last byte should be 0x0D")]
fn then_last_byte(world: &mut MllpWorld) {
    let len = world.wrapped_data.len();
    assert_eq!(
        world.wrapped_data[len - 1],
        0x0D,
        "Expected last byte to be 0x0D (CR), got 0x{:02X}",
        world.wrapped_data[len - 1]
    );
}

#[then("the wrapped length should be the message length plus 3")]
fn then_wrapped_length_plus_3(world: &mut MllpWorld) {
    assert_eq!(
        world.wrapped_data.len(),
        world.original_message.len() + 3,
        "Expected wrapped length {} but got {}",
        world.original_message.len() + 3,
        world.wrapped_data.len()
    );
}

#[then("the unwrapped content should equal the original message")]
fn then_unwrapped_equals_original(world: &mut MllpWorld) {
    let unwrapped = world
        .unwrapped_data
        .as_ref()
        .expect("No unwrapped data available");
    assert_eq!(
        unwrapped.as_slice(),
        world.original_message.as_slice(),
        "Unwrapped content does not match original message"
    );
}

#[then("the result should be true")]
fn then_result_true(world: &mut MllpWorld) {
    assert_eq!(
        world.framed_check_result,
        Some(true),
        "Expected is_mllp_framed to return true"
    );
}

#[then("the result should be false")]
fn then_result_false(world: &mut MllpWorld) {
    assert_eq!(
        world.framed_check_result,
        Some(false),
        "Expected is_mllp_framed to return false"
    );
}

#[then("the error should be MissingStartBlock")]
fn then_error_missing_start_block(world: &mut MllpWorld) {
    assert!(
        matches!(world.checked_error, Some(MllpError::MissingStartBlock)),
        "Expected MissingStartBlock error, got {:?}",
        world.checked_error
    );
}

#[then("the error should be MissingEndBlock")]
fn then_error_missing_end_block(world: &mut MllpWorld) {
    assert!(
        matches!(world.checked_error, Some(MllpError::MissingEndBlock)),
        "Expected MissingEndBlock error, got {:?}",
        world.checked_error
    );
}

#[then(regex = r"^the wrapped length should be (\d+)$")]
fn then_wrapped_length_exact(world: &mut MllpWorld, expected: usize) {
    assert_eq!(
        world.wrapped_data.len(),
        expected,
        "Expected wrapped length {} but got {}",
        expected,
        world.wrapped_data.len()
    );
}

#[then("the found length should equal the wrapped data length")]
fn then_found_length_equals_wrapped(world: &mut MllpWorld) {
    let found = world
        .found_length
        .expect("find_complete_mllp_message returned None");
    assert_eq!(
        found,
        world.wrapped_data.len(),
        "Expected found length {} but got {}",
        world.wrapped_data.len(),
        found
    );
}

#[then(regex = r#"^the extracted message should be "([^"]*)"$"#)]
fn then_extracted_message_equals(world: &mut MllpWorld, expected: String) {
    let expected_bytes = parse_hl7_string(&expected);
    let extracted = world
        .extracted_message
        .as_ref()
        .expect("No extracted message available");
    assert_eq!(
        extracted.as_slice(),
        expected_bytes.as_slice(),
        "Extracted message does not match expected"
    );
}

#[then("there should be no more messages in the iterator")]
fn then_no_more_messages(world: &mut MllpWorld) {
    assert!(
        world.frame_iterator.next_message().is_none(),
        "Expected no more messages but found one"
    );
}

// ============================================================================
// Cucumber Main
// ============================================================================

#[tokio::main]
async fn main() {
    MllpWorld::run("features/mllp.feature").await;
}
