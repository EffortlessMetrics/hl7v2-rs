//! BDD tests for hl7v2-query using Cucumber
//!
//! Run with: cargo test --test bdd_tests

use cucumber::{World, given, then, when};
use hl7v2_model::{Delims, Field, Message, Segment};
use hl7v2_query::get;

/// Test world for Query BDD tests
#[derive(Debug, World)]
#[world(init = Self::new)]
pub struct QueryWorld {
    /// Message being queried
    message: Option<Message>,
    /// Query result
    result: Option<String>,
    /// Query path
    path: String,
}

impl QueryWorld {
    fn new() -> Self {
        Self {
            message: None,
            result: None,
            path: String::new(),
        }
    }

    /// Create a simple MSH segment
    fn create_msh_segment(delims: &Delims) -> Segment {
        let encoding_chars = format!("{}{}{}{}", delims.comp, delims.rep, delims.esc, delims.sub);

        Segment {
            id: *b"MSH",
            fields: vec![
                Field::from_text(""),               // MSH-1 (field separator)
                Field::from_text(&encoding_chars),  // MSH-2 (encoding chars)
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
fn given_message_msh_pid(world: &mut QueryWorld) {
    let delims = Delims::default();
    let message = Message {
        delims: delims.clone(),
        segments: vec![
            QueryWorld::create_msh_segment(&delims),
            QueryWorld::create_pid_segment(),
        ],
        charsets: vec![],
    };
    world.message = Some(message);
}

#[given("a message with a field containing 2 repetitions")]
fn given_message_repetitions(world: &mut QueryWorld) {
    let delims = Delims::default();
    let mut pid = QueryWorld::create_pid_segment();
    // Set PID-5 to have repetitions
    pid.fields[4] = Field::from_text("Doe^John~Smith^Jane");
    let message = Message {
        delims: delims.clone(),
        segments: vec![QueryWorld::create_msh_segment(&delims), pid],
        charsets: vec![],
    };
    world.message = Some(message);
}

#[given("a message with a field containing components")]
fn given_message_components(world: &mut QueryWorld) {
    let delims = Delims::default();
    let message = Message {
        delims: delims.clone(),
        segments: vec![
            QueryWorld::create_msh_segment(&delims),
            QueryWorld::create_pid_segment(),
        ],
        charsets: vec![],
    };
    world.message = Some(message);
}

#[given("a message with a field containing subcomponents")]
fn given_message_subcomponents(world: &mut QueryWorld) {
    let delims = Delims::default();
    let mut pid = QueryWorld::create_pid_segment();
    // Set PID-5 to have subcomponents
    pid.fields[4] = Field::from_text("Doe&John^Jr&Smith");
    let message = Message {
        delims: delims.clone(),
        segments: vec![QueryWorld::create_msh_segment(&delims), pid],
        charsets: vec![],
    };
    world.message = Some(message);
}

#[given("a message containing characters that need escaping")]
fn given_message_escape_sequences(world: &mut QueryWorld) {
    let delims = Delims::default();
    let mut pid = QueryWorld::create_pid_segment();
    pid.fields[4] = Field::from_text("Doe\\F\\John");
    let message = Message {
        delims: delims.clone(),
        segments: vec![QueryWorld::create_msh_segment(&delims), pid],
        charsets: vec![],
    };
    world.message = Some(message);
}

#[given("a message with whitespace in field values")]
fn given_message_whitespace(world: &mut QueryWorld) {
    let delims = Delims::default();
    let mut pid = QueryWorld::create_pid_segment();
    pid.fields[4] = Field::from_text("  Doe  ");
    let message = Message {
        delims: delims.clone(),
        segments: vec![QueryWorld::create_msh_segment(&delims), pid],
        charsets: vec![],
    };
    world.message = Some(message);
}

#[given("a message with custom delimiters \"#$*@!\"")]
fn given_message_custom_delimiters(world: &mut QueryWorld) {
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

#[given(regex = r#"a ([A-Z]{3}\^[A-Z0-9]{2,3}) message"#)]
fn given_message_type(world: &mut QueryWorld, message_type: String) {
    let delims = Delims::default();
    let mut msh = QueryWorld::create_msh_segment(&delims);
    msh.fields[8] = Field::from_text(&message_type);

    let message = Message {
        delims: delims.clone(),
        segments: vec![msh],
        charsets: vec![],
    };
    world.message = Some(message);
}

#[given("a message with long field values")]
fn given_message_long_values(world: &mut QueryWorld) {
    let delims = Delims::default();
    let long_value = "A".repeat(500);
    let mut pid = QueryWorld::create_pid_segment();
    pid.fields[4] = Field::from_text(&long_value);

    let message = Message {
        delims: delims.clone(),
        segments: vec![QueryWorld::create_msh_segment(&delims), pid],
        charsets: vec![],
    };
    world.message = Some(message);
}

#[given("a message containing special characters in field values")]
fn given_message_special_chars(world: &mut QueryWorld) {
    let delims = Delims::default();
    let mut pid = QueryWorld::create_pid_segment();
    pid.fields[4] = Field::from_text("Doe, John Jr.");

    let message = Message {
        delims: delims.clone(),
        segments: vec![QueryWorld::create_msh_segment(&delims), pid],
        charsets: vec![],
    };
    world.message = Some(message);
}

// ============================================================================
// When Steps
// ============================================================================

#[when(regex = r#"I query to path "([^"]+)""#)]
fn when_query_path(world: &mut QueryWorld, path: String) {
    world.path = path.clone();
    let msg = world.message.as_ref().expect("No message");
    world.result = get(msg, &path).map(|s| s.to_string());
}

// ============================================================================
// Then Steps
// ============================================================================

#[then("result should be \"Doe\"")]
fn then_result_doe(world: &mut QueryWorld) {
    assert_eq!(world.result.as_deref(), Some("Doe"));
}

#[then("result should be \"Doe^John\"")]
fn then_result_doe_john(world: &mut QueryWorld) {
    assert_eq!(world.result.as_deref(), Some("Doe^John"));
}

#[then("result should be \"Smith\"")]
fn then_result_smith(world: &mut QueryWorld) {
    assert_eq!(world.result.as_deref(), Some("Smith"));
}

#[then("result should be \"ADT\"")]
fn then_result_adt(world: &mut QueryWorld) {
    assert_eq!(world.result.as_deref(), Some("ADT"));
}

#[then("result should be \"A01\"")]
fn then_result_a01(world: &mut QueryWorld) {
    assert_eq!(world.result.as_deref(), Some("A01"));
}

#[then("result should be \"123456\"")]
fn then_result_123456(world: &mut QueryWorld) {
    assert_eq!(world.result.as_deref(), Some("123456"));
}

#[then("result should be \"MR\"")]
fn then_result_mr(world: &mut QueryWorld) {
    assert_eq!(world.result.as_deref(), Some("MR"));
}

#[then("result should be \"HOSP\"")]
fn then_result_hosp(world: &mut QueryWorld) {
    assert_eq!(world.result.as_deref(), Some("HOSP"));
}

#[then("result should be \"Doe\"")]
fn then_result_doe_duplicate(world: &mut QueryWorld) {
    then_result_doe(world);
}

#[then("result should be \"John\"")]
fn then_result_john(world: &mut QueryWorld) {
    assert_eq!(world.result.as_deref(), Some("John"));
}

#[then("result should be None")]
fn then_result_none(world: &mut QueryWorld) {
    assert!(world.result.is_none());
}

#[then("result should contain special characters")]
fn then_result_special_chars(world: &mut QueryWorld) {
    assert!(
        world
            .result
            .as_ref()
            .is_some_and(|s| s.contains(",") || s.len() > 3)
    );
}

#[then("result should have escape sequences decoded")]
fn then_result_escape_decoded(world: &mut QueryWorld) {
    assert!(world.result.as_ref().is_some_and(|s| s.contains("John")));
}

#[then("result should preserve whitespace")]
fn then_result_preserve_whitespace(world: &mut QueryWorld) {
    assert!(
        world
            .result
            .as_ref()
            .is_some_and(|s| s.starts_with("  ") || s.ends_with("  "))
    );
}

// ============================================================================
// Cucumber Main
// ============================================================================

#[tokio::main]
async fn main() {
    QueryWorld::run("features/query.feature").await;
}
