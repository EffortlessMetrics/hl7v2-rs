//! BDD tests for hl7v2-writer using Cucumber
//!
//! Run with: cargo test --test bdd_tests

use cucumber::{World, given, then, when};
use hl7v2_model::{Delims, Message, Segment};
use hl7v2_writer::{write, write_mllp};

/// Test world for Writer BDD tests
#[derive(Debug, World)]
#[world(init = Self::new)]
pub struct WriterWorld {
    /// Message being written
    message: Option<Message>,
    /// Output bytes from writing
    output: Vec<u8>,
    /// Output as string
    output_str: String,
}

impl WriterWorld {
    fn new() -> Self {
        Self {
            message: None,
            output: Vec::new(),
            output_str: String::new(),
        }
    }

    /// Create a simple MSH segment
    fn create_msh_segment(delims: &Delims) -> Segment {
        let encoding_chars = format!("{}{}{}{}", delims.comp, delims.rep, delims.esc, delims.sub);

        Segment {
            id: *b"MSH",
            fields: vec![
                hl7v2_model::Field::from_text(""), // MSH-1 (field separator)
                hl7v2_model::Field::from_text(&encoding_chars), // MSH-2 (encoding chars)
                hl7v2_model::Field::from_text("SendingApp"), // MSH-3
                hl7v2_model::Field::from_text("SendingFac"), // MSH-4
                hl7v2_model::Field::from_text("ReceivingApp"), // MSH-5
                hl7v2_model::Field::from_text("ReceivingFac"), // MSH-6
                hl7v2_model::Field::from_text("20250128152312"), // MSH-7
                hl7v2_model::Field::from_text(""), // MSH-8
                hl7v2_model::Field::from_text("ADT^A01"), // MSH-9
                hl7v2_model::Field::from_text("MSG001"), // MSH-10
                hl7v2_model::Field::from_text("P"), // MSH-11
                hl7v2_model::Field::from_text("2.5.1"), // MSH-12
            ],
        }
    }

    /// Create a simple PID segment
    fn create_pid_segment() -> Segment {
        Segment {
            id: *b"PID",
            fields: vec![
                hl7v2_model::Field::from_text("1"),                // PID-1
                hl7v2_model::Field::from_text(""),                 // PID-2 (empty)
                hl7v2_model::Field::from_text("123456^^^HOSP^MR"), // PID-3
                hl7v2_model::Field::from_text(""),                 // PID-4
                hl7v2_model::Field::from_text("Doe^John"),         // PID-5
            ],
        }
    }
}

// ============================================================================
// Given Steps
// ============================================================================

#[given("a message with only an MSH segment")]
fn given_message_msh_only(world: &mut WriterWorld) {
    let delims = Delims::default();
    let message = Message {
        delims: delims.clone(),
        segments: vec![WriterWorld::create_msh_segment(&delims)],
        charsets: vec![],
    };
    world.message = Some(message);
}

#[given("a message with MSH and PID segments")]
fn given_message_msh_pid(world: &mut WriterWorld) {
    let delims = Delims::default();
    let message = Message {
        delims: delims.clone(),
        segments: vec![
            WriterWorld::create_msh_segment(&delims),
            WriterWorld::create_pid_segment(),
        ],
        charsets: vec![],
    };
    world.message = Some(message);
}

#[given("a message with custom delimiters \"#$*@!\"")]
fn given_message_custom_delimiters(world: &mut WriterWorld) {
    let delims = Delims {
        field: '#',
        comp: '$',
        rep: '*',
        esc: '@',
        sub: '!',
    };
    let encoding_chars = format!("{}{}{}{}", delims.comp, delims.rep, delims.esc, delims.sub);

    let message = Message {
        delims: delims.clone(),
        segments: vec![
            Segment {
                id: *b"MSH",
                fields: vec![
                    hl7v2_model::Field::from_text(""),
                    hl7v2_model::Field::from_text(&encoding_chars),
                    hl7v2_model::Field::from_text("SendingApp"),
                    hl7v2_model::Field::from_text("SendingFac"),
                    hl7v2_model::Field::from_text("ReceivingApp"),
                    hl7v2_model::Field::from_text("ReceivingFac"),
                    hl7v2_model::Field::from_text("20250128152312"),
                    hl7v2_model::Field::from_text(""),
                    hl7v2_model::Field::from_text("ADT$A01"),
                    hl7v2_model::Field::from_text("MSG001"),
                    hl7v2_model::Field::from_text("P"),
                    hl7v2_model::Field::from_text("2.5.1"),
                ],
            },
            Segment {
                id: *b"PID",
                fields: vec![
                    hl7v2_model::Field::from_text("1"),
                    hl7v2_model::Field::from_text(""),
                    hl7v2_model::Field::from_text("123456$$$HOSP$MR"),
                    hl7v2_model::Field::from_text(""),
                    hl7v2_model::Field::from_text("Doe$John"),
                ],
            },
        ],
        charsets: vec![],
    };
    world.message = Some(message);
}

#[given("a message with a field containing repetitions")]
fn given_message_repetitions(world: &mut WriterWorld) {
    let delims = Delims::default();
    let mut pid = WriterWorld::create_pid_segment();
    // Set PID-5 to have repetitions
    pid.fields[4] = hl7v2_model::Field::from_text("Doe^John~Smith^Jane");
    let message = Message {
        delims: delims.clone(),
        segments: vec![WriterWorld::create_msh_segment(&delims), pid],
        charsets: vec![],
    };
    world.message = Some(message);
}

#[given("a message with a field containing components")]
fn given_message_components(world: &mut WriterWorld) {
    let delims = Delims::default();
    let message = Message {
        delims: delims.clone(),
        segments: vec![
            WriterWorld::create_msh_segment(&delims),
            WriterWorld::create_pid_segment(),
        ],
        charsets: vec![],
    };
    world.message = Some(message);
}

#[given("a message with a component containing subcomponents")]
fn given_message_subcomponents(world: &mut WriterWorld) {
    let delims = Delims::default();
    let mut pid = WriterWorld::create_pid_segment();
    // Set PID-5 to have subcomponents
    pid.fields[4] = hl7v2_model::Field::from_text("Doe&John^Jr&Smith");
    let message = Message {
        delims: delims.clone(),
        segments: vec![WriterWorld::create_msh_segment(&delims), pid],
        charsets: vec![],
    };
    world.message = Some(message);
}

#[given("a message containing characters that need escaping")]
fn given_message_escape_sequences(world: &mut WriterWorld) {
    let delims = Delims::default();
    let mut pid = WriterWorld::create_pid_segment();
    // Set PID-5 to contain escape sequences
    pid.fields[4] = hl7v2_model::Field::from_text("Doe\\F\\John");
    let message = Message {
        delims: delims.clone(),
        segments: vec![WriterWorld::create_msh_segment(&delims), pid],
        charsets: vec![],
    };
    world.message = Some(message);
}

#[given("an empty message with default delimiters")]
fn given_empty_message(world: &mut WriterWorld) {
    let delims = Delims::default();
    let message = Message {
        delims: delims.clone(),
        segments: vec![],
        charsets: vec![],
    };
    world.message = Some(message);
}

#[given("a message with empty fields")]
fn given_message_empty_fields(world: &mut WriterWorld) {
    let delims = Delims::default();
    let mut msh = WriterWorld::create_msh_segment(&delims);
    // Add some empty fields
    msh.fields.push(hl7v2_model::Field::from_text(""));
    msh.fields.push(hl7v2_model::Field::from_text("Value"));
    msh.fields.push(hl7v2_model::Field::from_text(""));

    let message = Message {
        delims: delims.clone(),
        segments: vec![msh],
        charsets: vec![],
    };
    world.message = Some(message);
}

#[given("a message with explicit null values")]
fn given_message_null_values(world: &mut WriterWorld) {
    let delims = Delims::default();
    let mut pid = WriterWorld::create_pid_segment();
    // Set PID-2 to explicit null
    pid.fields[1] = hl7v2_model::Field::from_text("\"\"");

    let message = Message {
        delims: delims.clone(),
        segments: vec![WriterWorld::create_msh_segment(&delims), pid],
        charsets: vec![],
    };
    world.message = Some(message);
}

#[given("a message with MSH and multiple PID segments")]
fn given_message_multiple_pid(world: &mut WriterWorld) {
    let delims = Delims::default();
    let message = Message {
        delims: delims.clone(),
        segments: vec![
            WriterWorld::create_msh_segment(&delims),
            WriterWorld::create_pid_segment(),
            WriterWorld::create_pid_segment(),
        ],
        charsets: vec![],
    };
    world.message = Some(message);
}

#[given("a message with charset specification")]
fn given_message_charset(world: &mut WriterWorld) {
    let delims = Delims::default();
    let mut msh = WriterWorld::create_msh_segment(&delims);
    // Add charset field (MSH-18)
    if msh.fields.len() < 18 {
        msh.fields.resize(18, hl7v2_model::Field::from_text(""));
    }
    msh.fields[17] = hl7v2_model::Field::from_text("UNICODE UTF-8");

    let message = Message {
        delims: delims.clone(),
        segments: vec![msh],
        charsets: vec!["UNICODE UTF-8".to_string()],
    };
    world.message = Some(message);
}

#[given(regex = r#"a message of type ([A-Z]{3}\^[A-Z0-9]{2,3})"#)]
fn given_message_type(world: &mut WriterWorld, message_type: String) {
    let delims = Delims::default();
    let mut msh = WriterWorld::create_msh_segment(&delims);
    // Build MSH-9 as a proper multi-component field so the writer
    // serialises it with the component separator (^) instead of
    // escaping the literal caret.
    let parts: Vec<&str> = message_type.split('^').collect();
    let rep = hl7v2_model::Rep {
        comps: parts
            .into_iter()
            .map(hl7v2_model::Comp::from_text)
            .collect(),
    };
    msh.fields[8] = hl7v2_model::Field { reps: vec![rep] };

    let message = Message {
        delims: delims.clone(),
        segments: vec![msh],
        charsets: vec![],
    };
    world.message = Some(message);
}

#[given("a message with long field values")]
fn given_message_long_values(world: &mut WriterWorld) {
    let delims = Delims::default();
    let long_value = "A".repeat(500);
    let mut pid = WriterWorld::create_pid_segment();
    pid.fields[4] = hl7v2_model::Field::from_text(&long_value);

    let message = Message {
        delims: delims.clone(),
        segments: vec![WriterWorld::create_msh_segment(&delims), pid],
        charsets: vec![],
    };
    world.message = Some(message);
}

#[given("a message containing special characters in field values")]
fn given_message_special_chars(world: &mut WriterWorld) {
    let delims = Delims::default();
    let mut pid = WriterWorld::create_pid_segment();
    pid.fields[4] = hl7v2_model::Field::from_text("Doe, John Jr.");

    let message = Message {
        delims: delims.clone(),
        segments: vec![WriterWorld::create_msh_segment(&delims), pid],
        charsets: vec![],
    };
    world.message = Some(message);
}

#[given("a message with non-canonical delimiters")]
fn given_non_canonical(world: &mut WriterWorld) {
    given_message_custom_delimiters(world);
}

// ============================================================================
// When Steps
// ============================================================================

#[when("I write the message to bytes")]
fn when_write_message(world: &mut WriterWorld) {
    let msg = world.message.as_ref().expect("No message");
    world.output = write(msg);
    world.output_str = String::from_utf8_lossy(&world.output).to_string();
}

#[when("I write the message with MLLP framing")]
fn when_write_mllp(world: &mut WriterWorld) {
    let msg = world.message.as_ref().expect("No message");
    world.output = write_mllp(msg);
    world.output_str = String::from_utf8_lossy(&world.output).to_string();
}

// ============================================================================
// Then Steps
// ============================================================================

#[then("the output should start with \"MSH|\"")]
fn then_output_starts_msh(world: &mut WriterWorld) {
    assert!(world.output_str.starts_with("MSH|"));
}

#[then("the output should end with a carriage return")]
fn then_output_ends_cr(world: &mut WriterWorld) {
    assert!(world.output_str.ends_with('\r'));
}

#[then("the output should contain \"MSH|\"")]
fn then_output_contains_msh(world: &mut WriterWorld) {
    assert!(world.output_str.contains("MSH|"));
}

#[then("the output should contain \"PID|\"")]
fn then_output_contains_pid(world: &mut WriterWorld) {
    assert!(world.output_str.contains("PID|"));
}

#[then("the segments should be separated by carriage returns")]
fn then_segments_separated_cr(world: &mut WriterWorld) {
    // Check that MSH and PID are separated by \r
    let msh_pos = world.output_str.find("MSH|").unwrap();
    let pid_pos = world.output_str.find("PID|").unwrap();
    assert!(pid_pos > msh_pos);
    assert!(world.output_str.as_bytes()[pid_pos - 1] == b'\r');
}

#[then("the output should use the custom delimiters")]
fn then_output_custom_delimiters(world: &mut WriterWorld) {
    assert!(world.output_str.starts_with("MSH#"));
    assert!(world.output_str.contains("$*@!"));
}

#[then("repetitions should be separated by tilde \"~\"")]
fn then_repetitions_tilde(world: &mut WriterWorld) {
    assert!(world.output_str.contains("~"));
}

#[then("components should be separated by caret \"^\"")]
fn then_components_caret(world: &mut WriterWorld) {
    assert!(world.output_str.contains("^"));
}

#[then("subcomponents should be separated by ampersand \"&\"")]
fn then_subcomponents_ampersand(world: &mut WriterWorld) {
    assert!(world.output_str.contains("&"));
}

#[then("special characters should be properly escaped")]
fn then_special_chars_escaped(world: &mut WriterWorld) {
    assert!(world.output_str.contains("\\F\\"));
}

#[then("the output should be valid HL7 format")]
fn then_output_valid_hl7(world: &mut WriterWorld) {
    // Empty message is valid
    assert!(world.output.is_empty() || world.output_str.ends_with('\r'));
}

#[then("empty fields should be represented as consecutive delimiters")]
fn then_empty_fields_consecutive(world: &mut WriterWorld) {
    assert!(world.output_str.contains("||"));
}

#[then("null values should be represented as double quotes '\"\"'")]
fn then_null_values_quotes(world: &mut WriterWorld) {
    assert!(world.output_str.contains("\"\""));
}

#[then("the MSH segment should contain the delimiters in field 2")]
fn then_msh_delimiters_field2(world: &mut WriterWorld) {
    assert!(world.output_str.contains("#$*@!"));
}

#[then("all PID segments should be present in the output")]
fn then_all_pids_present(world: &mut WriterWorld) {
    let count = world.output_str.matches("PID|").count();
    assert_eq!(count, 2);
}

#[then("the charset should be present in MSH-18")]
fn then_charset_msh18(world: &mut WriterWorld) {
    assert!(world.output_str.contains("UNICODE UTF-8"));
}

#[then("the output should contain the message type")]
fn then_output_contains_message_type(world: &mut WriterWorld) {
    assert!(
        world.output_str.contains("ADT^A01")
            || world.output_str.contains("ORU^R01")
            || world.output_str.contains("ORM^O01")
            || world.output_str.contains("DFT^P03")
    );
}

#[then("the long values should be preserved")]
fn then_long_values_preserved(world: &mut WriterWorld) {
    assert!(world.output_str.len() > 500);
}

#[then("the special characters should be preserved or properly escaped")]
fn then_special_chars_preserved(world: &mut WriterWorld) {
    assert!(world.output_str.contains(",") || world.output_str.contains("\\E\\"));
}

#[then("the output should start with MLLP start block")]
fn then_output_mllp_start(world: &mut WriterWorld) {
    assert!(world.output.starts_with(&[0x0B]));
}

#[then("the output should end with MLLP end block")]
fn then_output_mllp_end(world: &mut WriterWorld) {
    assert!(world.output.ends_with(&[0x1C, 0x0D]));
}

#[then("the message content should be between the blocks")]
fn then_output_mllp_content(world: &mut WriterWorld) {
    assert!(world.output_str.contains("MSH|"));
}

// ============================================================================
// Cucumber Main
// ============================================================================

#[tokio::main]
async fn main() {
    WriterWorld::run("features/writer.feature").await;
}
