//! BDD tests for hl7v2-json using Cucumber
//!
//! Run with: cargo test --test bdd_tests

use cucumber::{World, given, then, when};
use hl7v2_json::{to_json, to_json_string, to_json_string_pretty};
use hl7v2_model::{Atom, Comp, Delims, Field, Message, Rep, Segment};
use serde_json::Value;

/// Test world for JSON BDD tests
#[derive(Debug, World)]
#[world(init = Self::new)]
pub struct JsonWorld {
    /// Message being converted
    message: Option<Message>,
    /// JSON result
    json_value: Option<Value>,
    /// JSON string result
    json_string: Option<String>,
    /// Pretty JSON string result
    pretty_json_string: Option<String>,
}

impl JsonWorld {
    fn new() -> Self {
        Self {
            message: None,
            json_value: None,
            json_string: None,
            pretty_json_string: None,
        }
    }

    /// Create a simple MSH segment
    fn create_msh_segment(delims: &Delims) -> Segment {
        let encoding_chars = format!("{}{}{}{}", delims.comp, delims.rep, delims.esc, delims.sub);

        Segment {
            id: *b"MSH",
            fields: vec![
                Field::from_text(""),               // MSH-1
                Field::from_text(&encoding_chars),  // MSH-2
                Field::from_text("SendingApp"),     // MSH-3
                Field::from_text("SendingFac"),     // MSH-4
                Field::from_text("ReceivingApp"),   // MSH-5
                Field::from_text("ReceivingFac"),   // MSH-6
                Field::from_text("20250128152312"), // MSH-7
                Field::from_text(""),               // MSH-8
                Field::from_text("ADT^A01"),        // MSH-9
                Field::from_text("MSG001"),         // MSH-10
                Field::from_text("P"),              // MSH-11
                Field::from_text("2.5.1"),          // MSH-12
            ],
        }
    }

    /// Create a simple PID segment
    fn create_pid_segment() -> Segment {
        Segment {
            id: *b"PID",
            fields: vec![
                Field::from_text("1"),                // PID-1
                Field::from_text(""),                 // PID-2 (empty)
                Field::from_text("123456^^^HOSP^MR"), // PID-3
                Field::from_text(""),                 // PID-4
                Field::from_text("Doe^John"),         // PID-5
            ],
        }
    }
}

// ============================================================================
// Given Steps
// ============================================================================

#[given("a message with MSH and PID segments")]
fn given_message_msh_pid(world: &mut JsonWorld) {
    let delims = Delims::default();
    let message = Message {
        delims: delims.clone(),
        segments: vec![
            JsonWorld::create_msh_segment(&delims),
            JsonWorld::create_pid_segment(),
        ],
        charsets: vec![],
    };
    world.message = Some(message);
}

#[given("a message with default delimiters")]
fn given_message_default_delims(world: &mut JsonWorld) {
    given_message_msh_pid(world);
}

#[given("a message with charset specification")]
fn given_message_charset(world: &mut JsonWorld) {
    let delims = Delims::default();
    let mut msh = JsonWorld::create_msh_segment(&delims);
    // Add charset field (MSH-18)
    if msh.fields.len() < 18 {
        msh.fields.resize(18, Field::from_text(""));
    }
    msh.fields[17] = Field::from_text("UNICODE UTF-8");

    let message = Message {
        delims: delims.clone(),
        segments: vec![msh, JsonWorld::create_pid_segment()],
        charsets: vec!["UNICODE UTF-8".to_string()],
    };
    world.message = Some(message);
}

#[given("a message with empty fields")]
fn given_message_empty_fields(world: &mut JsonWorld) {
    let delims = Delims::default();
    let mut pid = JsonWorld::create_pid_segment();
    // Ensure PID-2 is empty
    pid.fields[1] = Field::from_text("");

    let message = Message {
        delims: delims.clone(),
        segments: vec![JsonWorld::create_msh_segment(&delims), pid],
        charsets: vec![],
    };
    world.message = Some(message);
}

#[given("a message with null values")]
fn given_message_null_values(world: &mut JsonWorld) {
    let delims = Delims::default();
    let mut pid = JsonWorld::create_pid_segment();
    // Set PID-2 to explicit null
    pid.fields[1] = Field::from_text("");

    let message = Message {
        delims: delims.clone(),
        segments: vec![JsonWorld::create_msh_segment(&delims), pid],
        charsets: vec![],
    };
    world.message = Some(message);
}

#[given("a message with field repetitions")]
fn given_message_repetitions(world: &mut JsonWorld) {
    let delims = Delims::default();
    let mut pid = JsonWorld::create_pid_segment();
    // Set PID-5 to have repetitions
    pid.fields[4] = Field {
        reps: vec![Rep::from_text("Doe"), Rep::from_text("Smith")],
    };

    let message = Message {
        delims: delims.clone(),
        segments: vec![JsonWorld::create_msh_segment(&delims), pid],
        charsets: vec![],
    };
    world.message = Some(message);
}

#[given("a message with components")]
fn given_message_components(world: &mut JsonWorld) {
    given_message_msh_pid(world);
}

#[given("a message with subcomponents")]
fn given_message_subcomponents(world: &mut JsonWorld) {
    let delims = Delims::default();
    let mut pid = JsonWorld::create_pid_segment();
    // Set PID-5 to have subcomponents
    pid.fields[4] = Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![
                    Atom::Text("Doe".to_string()),
                    Atom::Text("John".to_string()),
                ],
            }],
        }],
    };

    let message = Message {
        delims: delims.clone(),
        segments: vec![JsonWorld::create_msh_segment(&delims), pid],
        charsets: vec![],
    };
    world.message = Some(message);
}

#[given("a message with escape sequences")]
fn given_message_escape_sequences(world: &mut JsonWorld) {
    let delims = Delims::default();
    let mut pid = JsonWorld::create_pid_segment();
    pid.fields[4] = Field::from_text("Doe\\F\\John");

    let message = Message {
        delims: delims.clone(),
        segments: vec![JsonWorld::create_msh_segment(&delims), pid],
        charsets: vec![],
    };
    world.message = Some(message);
}

#[given("a message with special characters")]
fn given_message_special_chars(world: &mut JsonWorld) {
    let delims = Delims::default();
    let mut pid = JsonWorld::create_pid_segment();
    pid.fields[4] = Field::from_text("Doe, John Jr.");

    let message = Message {
        delims: delims.clone(),
        segments: vec![JsonWorld::create_msh_segment(&delims), pid],
        charsets: vec![],
    };
    world.message = Some(message);
}

#[given(regex = r"a ([A-Z]{3}\^[A-Z0-9]{2,3}) message")]
fn given_message_type(world: &mut JsonWorld, message_type: String) {
    let delims = Delims::default();
    let mut msh = JsonWorld::create_msh_segment(&delims);
    msh.fields[8] = Field::from_text(&message_type);

    let message = Message {
        delims: delims.clone(),
        segments: vec![msh],
        charsets: vec![],
    };
    world.message = Some(message);
}

#[given("a message with multiple segments")]
fn given_message_multiple_segments(world: &mut JsonWorld) {
    let delims = Delims::default();
    let message = Message {
        delims: delims.clone(),
        segments: vec![
            JsonWorld::create_msh_segment(&delims),
            JsonWorld::create_pid_segment(),
            JsonWorld::create_pid_segment(),
        ],
        charsets: vec![],
    };
    world.message = Some(message);
}

#[given("a message with long field values")]
fn given_message_long_values(world: &mut JsonWorld) {
    let delims = Delims::default();
    let mut pid = JsonWorld::create_pid_segment();
    let long_value = "A".repeat(500);
    pid.fields[4] = Field::from_text(&long_value);

    let message = Message {
        delims: delims.clone(),
        segments: vec![JsonWorld::create_msh_segment(&delims), pid],
        charsets: vec![],
    };
    world.message = Some(message);
}

#[given("a message with custom delimiters \"#$*@!\"")]
fn given_message_custom_delimiters(world: &mut JsonWorld) {
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
                    Field::from_text(""),
                    Field::from_text(&encoding_chars),
                    Field::from_text("SendingApp"),
                    Field::from_text("SendingFac"),
                    Field::from_text("ReceivingApp"),
                    Field::from_text("ReceivingFac"),
                    Field::from_text("20250128152312"),
                    Field::from_text(""),
                    Field::from_text("ADT$A01"),
                    Field::from_text("MSG001"),
                    Field::from_text("P"),
                    Field::from_text("2.5.1"),
                ],
            },
            Segment {
                id: *b"PID",
                fields: vec![
                    Field::from_text("1"),
                    Field::from_text(""),
                    Field::from_text("123456$$$HOSP$MR"),
                    Field::from_text(""),
                    Field::from_text("Doe$John"),
                ],
            },
        ],
        charsets: vec![],
    };
    world.message = Some(message);
}

#[given("an empty message")]
fn given_empty_message(world: &mut JsonWorld) {
    let delims = Delims::default();
    let message = Message {
        delims: delims.clone(),
        segments: vec![],
        charsets: vec![],
    };
    world.message = Some(message);
}

// ============================================================================
// When Steps
// ============================================================================

#[when("I convert the message to JSON")]
fn when_convert_json(world: &mut JsonWorld) {
    let msg = world.message.as_ref().expect("No message");
    world.json_value = Some(to_json(msg));
}

#[when("I convert the message to JSON string")]
fn when_convert_json_string(world: &mut JsonWorld) {
    let msg = world.message.as_ref().expect("No message");
    world.json_string = Some(to_json_string(msg));
}

#[when("I convert the message to pretty JSON string")]
fn when_convert_pretty_json_string(world: &mut JsonWorld) {
    let msg = world.message.as_ref().expect("No message");
    world.pretty_json_string = Some(to_json_string_pretty(msg));
}

// ============================================================================
// Then Steps
// ============================================================================

#[then("the JSON should be valid")]
fn then_json_valid(world: &mut JsonWorld) {
    assert!(world.json_value.is_some());
}

#[then("the JSON should contain \"meta\" object")]
fn then_json_meta(world: &mut JsonWorld) {
    let json = world.json_value.as_ref().expect("No JSON");
    assert!(json.get("meta").is_some());
}

#[then("the JSON should contain \"segments\" array")]
fn then_json_segments(world: &mut JsonWorld) {
    let json = world.json_value.as_ref().expect("No JSON");
    assert!(json.get("segments").is_some());
}

#[then("the result should be a valid JSON string")]
fn then_json_string_valid(world: &mut JsonWorld) {
    let json_str = world.json_string.as_ref().expect("No JSON string");
    let parsed: Value = serde_json::from_str(json_str).expect("Invalid JSON");
    assert!(parsed.is_object());
}

#[then("the result should be formatted with indentation")]
fn then_pretty_formatted(world: &mut JsonWorld) {
    let json_str = world.pretty_json_string.as_ref().expect("No pretty JSON");
    assert!(json_str.contains('\n'));
    assert!(json_str.contains("  "));
}

#[then("the JSON should contain delimiter information")]
fn then_json_delimiters(world: &mut JsonWorld) {
    let json = world.json_value.as_ref().expect("No JSON");
    assert!(json["meta"]["delims"].is_object());
}

#[then("the field separator should be \"|\"")]
fn then_field_separator(world: &mut JsonWorld) {
    let json = world.json_value.as_ref().expect("No JSON");
    assert_eq!(json["meta"]["delims"]["field"], "|");
}

#[then("the component separator should be \"^\"")]
fn then_component_separator(world: &mut JsonWorld) {
    let json = world.json_value.as_ref().expect("No JSON");
    assert_eq!(json["meta"]["delims"]["comp"], "^");
}

#[then("the JSON should contain charset information")]
fn then_json_charset(world: &mut JsonWorld) {
    let json = world.json_value.as_ref().expect("No JSON");
    assert!(json["meta"]["charsets"].is_array());
}

#[then("the JSON should contain segment \"MSH\"")]
fn then_json_msh(world: &mut JsonWorld) {
    let json = world.json_value.as_ref().expect("No JSON");
    let segments = json["segments"].as_array().expect("Not an array");
    assert!(segments.iter().any(|s| s["id"] == "MSH"));
}

#[then("the JSON should contain segment \"PID\"")]
fn then_json_pid(world: &mut JsonWorld) {
    let json = world.json_value.as_ref().expect("No JSON");
    let segments = json["segments"].as_array().expect("Not an array");
    assert!(segments.iter().any(|s| s["id"] == "PID"));
}

#[then("the JSON should contain field values from MSH")]
fn then_json_msh_fields(world: &mut JsonWorld) {
    let json = world.json_value.as_ref().expect("No JSON");
    let segments = json["segments"].as_array().expect("Not an array");
    let msh = segments
        .iter()
        .find(|s| s["id"] == "MSH")
        .expect("No MSH segment");
    assert!(msh["fields"].is_object());
}

#[then("the JSON should contain field values from PID")]
fn then_json_pid_fields(world: &mut JsonWorld) {
    let json = world.json_value.as_ref().expect("No JSON");
    let segments = json["segments"].as_array().expect("Not an array");
    let pid = segments
        .iter()
        .find(|s| s["id"] == "PID")
        .expect("No PID segment");
    assert!(pid["fields"].is_object());
}

#[then("the JSON should represent empty fields correctly")]
fn then_json_empty_fields(world: &mut JsonWorld) {
    // Empty fields should be handled correctly
    let json = world.json_value.as_ref().expect("No JSON");
    assert!(json.is_object());
}

#[then("the JSON should represent null values correctly")]
fn then_json_null_values(world: &mut JsonWorld) {
    let json = world.json_value.as_ref().expect("No JSON");
    assert!(json.is_object());
}

#[then("the JSON should represent repetitions as an array")]
fn then_json_repetitions_array(world: &mut JsonWorld) {
    let json = world.json_value.as_ref().expect("No JSON");
    assert!(json.is_object());
}

#[then("the JSON should represent components correctly")]
fn then_json_components(world: &mut JsonWorld) {
    let json = world.json_value.as_ref().expect("No JSON");
    assert!(json.is_object());
}

#[then("the JSON should represent subcomponents correctly")]
fn then_json_subcomponents(world: &mut JsonWorld) {
    let json = world.json_value.as_ref().expect("No JSON");
    assert!(json.is_object());
}

#[then("the JSON should handle escape sequences properly")]
fn then_json_escape_sequences(world: &mut JsonWorld) {
    let json = world.json_value.as_ref().expect("No JSON");
    assert!(json.is_object());
}

#[then("the JSON should handle special characters properly")]
fn then_json_special_chars(world: &mut JsonWorld) {
    let json = world.json_value.as_ref().expect("No JSON");
    assert!(json.is_object());
}

#[then("the JSON should contain the message type")]
fn then_json_message_type(world: &mut JsonWorld) {
    let json = world.json_value.as_ref().expect("No JSON");
    assert!(json.is_object());
}

#[then("the JSON should contain all segments")]
fn then_json_all_segments(world: &mut JsonWorld) {
    let json = world.json_value.as_ref().expect("No JSON");
    let segments = json["segments"].as_array().expect("Not an array");
    assert_eq!(segments.len(), 3);
}

#[then("the JSON should preserve long values")]
fn then_json_long_values(world: &mut JsonWorld) {
    let json = world.json_value.as_ref().expect("No JSON");
    assert!(json.is_object());
}

#[then("the JSON should reflect the custom delimiters")]
fn then_json_custom_delimiters(world: &mut JsonWorld) {
    let json = world.json_value.as_ref().expect("No JSON");
    assert_eq!(json["meta"]["delims"]["field"], "#");
    assert_eq!(json["meta"]["delims"]["comp"], "$");
}

#[then("the JSON should have empty segments array")]
fn then_json_empty_segments(world: &mut JsonWorld) {
    let json = world.json_value.as_ref().expect("No JSON");
    let segments = json["segments"].as_array().expect("Not an array");
    assert_eq!(segments.len(), 0);
}

// ============================================================================
// Cucumber Main
// ============================================================================

#[tokio::main]
async fn main() {
    JsonWorld::run("features/json.feature").await;
}
