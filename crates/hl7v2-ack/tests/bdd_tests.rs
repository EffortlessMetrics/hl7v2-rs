//! BDD tests for hl7v2-ack using Cucumber
//!
//! Run with: cargo test --test bdd_tests

use cucumber::{World, given, then, when};
use hl7v2_ack::{AckCode, ack};
use hl7v2_core::{Message, parse};

/// Test world for ACK BDD tests
#[derive(Debug, World)]
#[world(init = Self::new)]
pub struct AckWorld {
    /// Original message being acknowledged
    original_message: Option<Message>,
    /// Generated ACK message
    ack_message: Option<Message>,
    /// Current ACK code
    ack_code: Option<AckCode>,
    /// Raw bytes for testing
    raw_bytes: Vec<u8>,
}

impl AckWorld {
    fn new() -> Self {
        Self {
            original_message: None,
            ack_message: None,
            ack_code: None,
            raw_bytes: Vec::new(),
        }
    }
}

// ============================================================================
// Given Steps
// ============================================================================

#[given(regex = r#"a valid ([A-Z]{3}\^[A-Z0-9]{2,3}) message with message ID "([^"]+)""#)]
fn given_valid_message_with_id(world: &mut AckWorld, message_type: String, message_id: String) {
    let hl7 = format!(
        "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||{}|{}|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r",
        message_type, message_id
    );
    world.raw_bytes = hl7.as_bytes().to_vec();
    world.original_message = Some(parse(&world.raw_bytes).expect("Parse failed"));
}

#[given("a valid ADT^A01 message")]
fn given_valid_adt_a01(world: &mut AckWorld) {
    given_valid_message_with_id(world, "ADT^A01".to_string(), "MSG001".to_string());
}

#[given("an HL7 message with custom delimiters \"#$*@!\"")]
fn given_custom_delimiters(world: &mut AckWorld) {
    // Using # as field separator, $ as component, * as repetition, @ as escape, ! as subcomponent
    let hl7 = b"MSH#$*@!SendingApp#SendingFac#ReceivingApp#ReceivingFac#20250128152312##ADT$A01#MSG001#P#2.5.1\rPID#1##123456$$$HOSP$MR##Doe$John\r";
    world.raw_bytes = hl7.to_vec();
    world.original_message = Some(parse(&world.raw_bytes).expect("Parse failed"));
}

#[given("a message from \"SendingApp\" to \"ReceivingApp\"")]
fn given_from_to_app(world: &mut AckWorld) {
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|MSG001|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r";
    world.raw_bytes = hl7.to_vec();
    world.original_message = Some(parse(&world.raw_bytes).expect("Parse failed"));
}

#[given("a message from \"SendingFac\" to \"ReceivingFac\"")]
fn given_from_to_fac(world: &mut AckWorld) {
    let hl7 = b"MSH|^~\\&|App1|SendingFac|App2|ReceivingFac|20250128152312||ADT^A01|MSG001|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r";
    world.raw_bytes = hl7.to_vec();
    world.original_message = Some(parse(&world.raw_bytes).expect("Parse failed"));
}

// ============================================================================
// When Steps
// ============================================================================

#[when(regex = r"I generate an ACK with code (AA|AE|AR|CA|CE|CR)")]
fn when_generate_ack(world: &mut AckWorld, code: String) {
    let ack_code = match code.as_str() {
        "AA" => AckCode::AA,
        "AE" => AckCode::AE,
        "AR" => AckCode::AR,
        "CA" => AckCode::CA,
        "CE" => AckCode::CE,
        "CR" => AckCode::CR,
        _ => panic!("Invalid ACK code: {}", code),
    };
    world.ack_code = Some(ack_code);

    let original = world
        .original_message
        .as_ref()
        .expect("No original message");

    // Note: The ack function only takes 2 arguments (original, code)
    // Error messages are not directly supported in the current API
    world.ack_message = Some(ack(original, ack_code).expect("ACK generation failed"));
}

// ============================================================================
// Then Steps
// ============================================================================

#[then("the ACK message should have 2 segments")]
fn then_ack_two_segments(world: &mut AckWorld) {
    let ack = world
        .ack_message
        .as_ref()
        .expect("No ACK message generated");
    assert_eq!(ack.segments.len(), 2);
}

#[then("the first segment should be MSH")]
fn then_first_segment_msh(world: &mut AckWorld) {
    let ack = world
        .ack_message
        .as_ref()
        .expect("No ACK message generated");
    assert_eq!(&ack.segments[0].id, b"MSH");
}

#[then("the second segment should be MSA")]
fn then_second_segment_msa(world: &mut AckWorld) {
    let ack = world
        .ack_message
        .as_ref()
        .expect("No ACK message generated");
    assert_eq!(&ack.segments[1].id, b"MSA");
}

#[then(regex = r#"MSH\.(\d+)\.(\d+) should be "([^"]+)""#)]
fn then_msh_component(world: &mut AckWorld, field: usize, component: usize, value: String) {
    let ack = world
        .ack_message
        .as_ref()
        .expect("No ACK message generated");
    let path = format!("MSH.{}.{}", field, component);
    assert_eq!(hl7v2_core::get(ack, &path), Some(value.as_str()));
}

#[then(regex = r#"MSA\.(\d+) should be "([^"]+)""#)]
fn then_msa_field(world: &mut AckWorld, field: usize, value: String) {
    let ack = world
        .ack_message
        .as_ref()
        .expect("No ACK message generated");
    let path = format!("MSA.{}", field);
    assert_eq!(hl7v2_core::get(ack, &path), Some(value.as_str()));
}

#[then("the ACK should use the same delimiters")]
fn then_same_delimiters(world: &mut AckWorld) {
    let original = world
        .original_message
        .as_ref()
        .expect("No original message");
    let ack = world
        .ack_message
        .as_ref()
        .expect("No ACK message generated");
    assert_eq!(original.delims, ack.delims);
}

#[then("the delimiters should be \"#$*@!\"")]
fn then_delimiters_custom(world: &mut AckWorld) {
    let ack = world
        .ack_message
        .as_ref()
        .expect("No ACK message generated");
    assert_eq!(ack.delims.field, '#');
    assert_eq!(ack.delims.comp, '$');
    assert_eq!(ack.delims.rep, '*');
    assert_eq!(ack.delims.esc, '@');
    assert_eq!(ack.delims.sub, '!');
}

#[then("MSA.3 should be \"Invalid data\"")]
fn then_msa3_invalid_data(world: &mut AckWorld) {
    then_msa_field(world, 3, "Invalid data".to_string());
}

#[then("MSA.3 should be \"Format error\"")]
fn then_msa3_format_error(world: &mut AckWorld) {
    then_msa_field(world, 3, "Format error".to_string());
}

#[then("MSH.3 should be \"ReceivingApp\"")]
fn then_msh3_receiving_app(world: &mut AckWorld) {
    let ack = world
        .ack_message
        .as_ref()
        .expect("No ACK message generated");
    assert_eq!(hl7v2_core::get(ack, "MSH.3"), Some("ReceivingApp"));
}

#[then("MSH.5 should be \"SendingApp\"")]
fn then_msh5_sending_app(world: &mut AckWorld) {
    let ack = world
        .ack_message
        .as_ref()
        .expect("No ACK message generated");
    assert_eq!(hl7v2_core::get(ack, "MSH.5"), Some("SendingApp"));
}

#[then("MSH.4 should be \"ReceivingFac\"")]
fn then_msh4_receiving_fac(world: &mut AckWorld) {
    let ack = world
        .ack_message
        .as_ref()
        .expect("No ACK message generated");
    assert_eq!(hl7v2_core::get(ack, "MSH.4"), Some("ReceivingFac"));
}

#[then("MSH.6 should be \"SendingFac\"")]
fn then_msh6_sending_fac(world: &mut AckWorld) {
    let ack = world
        .ack_message
        .as_ref()
        .expect("No ACK message generated");
    assert_eq!(hl7v2_core::get(ack, "MSH.6"), Some("SendingFac"));
}

#[then("MSA.2 should be \"MSG009\"")]
fn then_msa2_msg009(world: &mut AckWorld) {
    then_msa_field(world, 2, "MSG009".to_string());
}

#[then("MSA.3 should be \"Processing failed\"")]
fn then_msa3_processing_failed(world: &mut AckWorld) {
    then_msa_field(world, 3, "Processing failed".to_string());
}

// ============================================================================
// Cucumber Main
// ============================================================================

#[tokio::main]
async fn main() {
    AckWorld::run("features/acknowledgment.feature").await;
}
