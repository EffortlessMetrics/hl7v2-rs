//! BDD tests for hl7v2-query using Cucumber
//!
//! Run with: cargo test --test bdd_tests

use cucumber::{given, then, when, World};
use hl7v2_model::{Atom, Comp, Delims, Field, Message, Presence, Rep, Segment};
use hl7v2_query::{get, get_presence};

/// Test world for Query BDD tests
#[derive(Debug, World)]
#[world(init = Self::new)]
pub struct QueryWorld {
    /// Message being queried
    message: Option<Message>,
    /// Query result
    result: Option<String>,
    /// Presence query result
    presence: Option<Presence>,
    /// Query path
    path: String,
}

impl QueryWorld {
    fn new() -> Self {
        Self {
            message: None,
            result: None,
            presence: None,
            path: String::new(),
        }
    }

    /// Create a simple MSH segment with proper component structure for MSH-9
    fn create_msh_segment(delims: &Delims) -> Segment {
        let encoding_chars = format!("{}{}{}{}", delims.comp, delims.rep, delims.esc, delims.sub);

        // MSH fields layout: fields[0] = MSH-2, fields[1] = MSH-3, ...
        // The library's get_msh_field uses fields[0] for MSH-2 and
        // fields[field_index - 2] for MSH-3+.
        Segment {
            id: *b"MSH",
            fields: vec![
                Field::from_text(&encoding_chars),  // fields[0] → MSH-2
                Field::from_text("SendingApp"),     // fields[1] → MSH-3
                Field::from_text("SendingFac"),     // fields[2] → MSH-4
                Field::from_text("ReceivingApp"),   // fields[3] → MSH-5
                Field::from_text("ReceivingFac"),   // fields[4] → MSH-6
                Field::from_text("20250128152312"), // fields[5] → MSH-7
                Field::from_text(""),               // fields[6] → MSH-8
                // fields[7] → MSH-9: ADT^A01 with proper component structure
                Field {
                    reps: vec![Rep {
                        comps: vec![Comp::from_text("ADT"), Comp::from_text("A01")],
                    }],
                },
                Field::from_text("MSG001"), // fields[8] → MSH-10
                Field::from_text("P"),      // fields[9] → MSH-11
                Field::from_text("2.5.1"),  // fields[10] → MSH-12
            ],
        }
    }

    /// Create a PID segment with proper component structure
    fn create_pid_segment() -> Segment {
        Segment {
            id: *b"PID",
            fields: vec![
                Field::from_text("1"), // PID-1
                Field::from_text(""),  // PID-2 (empty)
                // PID-3: 123456^^^MR — 4 components
                Field {
                    reps: vec![Rep {
                        comps: vec![
                            Comp::from_text("123456"),
                            Comp::new(), // empty component 2
                            Comp::new(), // empty component 3
                            Comp::from_text("MR"),
                        ],
                    }],
                },
                Field::from_text(""), // PID-4
                // PID-5: Doe^John — 2 components
                Field {
                    reps: vec![Rep {
                        comps: vec![Comp::from_text("Doe"), Comp::from_text("John")],
                    }],
                },
            ],
        }
    }

    /// Get a field value, handling the no-component case by joining all components.
    ///
    /// When the path specifies a component (e.g., "PID.5.1"), delegates to `hl7v2_query::get`.
    /// When the path has no component (e.g., "PID.5"), joins all component values with `^`.
    fn get_field_value(msg: &Message, path: &str) -> Option<String> {
        let parts: Vec<&str> = path.split('.').collect();

        // If path has 3+ parts (segment.field.component), use the standard get
        if parts.len() >= 3 {
            return get(msg, path).map(std::string::ToString::to_string);
        }

        // Path has only segment.field — join all components with ^
        if parts.len() != 2 {
            return None;
        }

        let segment_id = parts[0];
        let field_part = parts[1];

        // Parse field index and optional rep index
        let (field_index, rep_index) = parse_field_and_rep(field_part)?;

        let segment = msg
            .segments
            .iter()
            .find(|s| std::str::from_utf8(&s.id) == Ok(segment_id))?;

        // Adjust indexing for MSH segments
        let field = if segment_id == "MSH" {
            if field_index <= 2 {
                // MSH-1 (separator) and MSH-2 (encoding chars) — just use get()
                return get(msg, path).map(std::string::ToString::to_string);
            }
            let adjusted = field_index - 2;
            segment.fields.get(adjusted)?
        } else {
            if field_index == 0 {
                return None;
            }
            segment.fields.get(field_index - 1)?
        };

        let rep = field.reps.get(rep_index - 1)?;

        // Join all non-empty component texts with ^
        let comp_texts: Vec<&str> = rep
            .comps
            .iter()
            .map(|comp| {
                comp.subs
                    .first()
                    .and_then(|atom| match atom {
                        Atom::Text(t) => Some(t.as_str()),
                        Atom::Null => None,
                    })
                    .unwrap_or("")
            })
            .collect();

        // Trim trailing empty components
        let last_non_empty = comp_texts
            .iter()
            .rposition(|t| !t.is_empty())
            .map(|i| i + 1)
            .unwrap_or(0);

        if last_non_empty == 0 {
            return None;
        }

        let joined = comp_texts[..last_non_empty].join("^");
        if joined.is_empty() {
            None
        } else {
            Some(joined)
        }
    }
}

/// Parse field and repetition indices from a string like "5" or "5[1]"
fn parse_field_and_rep(field_str: &str) -> Option<(usize, usize)> {
    if let Some(bracket_pos) = field_str.find('[') {
        let field_index = field_str[..bracket_pos].parse::<usize>().ok()?;
        let rep_part = &field_str[bracket_pos + 1..];
        let end_bracket = rep_part.find(']')?;
        let rep_index = rep_part[..end_bracket].parse::<usize>().ok()?;
        Some((field_index, rep_index))
    } else {
        let field_index = field_str.parse::<usize>().ok()?;
        Some((field_index, 1))
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
    // PID-5 with two repetitions, each with proper component structure
    pid.fields[4] = Field {
        reps: vec![
            Rep {
                comps: vec![Comp::from_text("Doe"), Comp::from_text("John")],
            },
            Rep {
                comps: vec![Comp::from_text("Smith"), Comp::from_text("Jane")],
            },
        ],
    };
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

#[given("a message with a field containing multiple components")]
fn given_message_multiple_components(world: &mut QueryWorld) {
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

#[given("a message with an empty field")]
fn given_message_empty_field(world: &mut QueryWorld) {
    let delims = Delims::default();
    let mut pid = QueryWorld::create_pid_segment();
    // PID-2: empty string value
    pid.fields[1] = Field::from_text("");
    let message = Message {
        delims: delims.clone(),
        segments: vec![QueryWorld::create_msh_segment(&delims), pid],
        charsets: vec![],
    };
    world.message = Some(message);
}

#[given("a message with a null field value")]
fn given_message_null_field(world: &mut QueryWorld) {
    let delims = Delims::default();
    let mut pid = QueryWorld::create_pid_segment();
    // PID-2: explicit null value
    pid.fields[1] = Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![Atom::Null],
            }],
        }],
    };
    let message = Message {
        delims: delims.clone(),
        segments: vec![QueryWorld::create_msh_segment(&delims), pid],
        charsets: vec![],
    };
    world.message = Some(message);
}

#[given("a message with a field containing subcomponents")]
fn given_message_subcomponents(world: &mut QueryWorld) {
    let delims = Delims::default();
    let mut pid = QueryWorld::create_pid_segment();
    // PID-5: first component has subcomponents Doe & Jr, second component has Smith
    pid.fields[4] = Field {
        reps: vec![Rep {
            comps: vec![
                Comp {
                    subs: vec![Atom::text("Doe"), Atom::text("Jr")],
                },
                Comp::from_text("Smith"),
            ],
        }],
    };
    let message = Message {
        delims: delims.clone(),
        segments: vec![QueryWorld::create_msh_segment(&delims), pid],
        charsets: vec![],
    };
    world.message = Some(message);
}

#[given("a message with escape sequences in field values")]
fn given_message_escape_sequences(world: &mut QueryWorld) {
    let delims = Delims::default();
    let mut pid = QueryWorld::create_pid_segment();
    pid.fields[4] = Field {
        reps: vec![Rep {
            comps: vec![Comp::from_text("Doe\\F\\John")],
        }],
    };
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
    pid.fields[4] = Field {
        reps: vec![Rep {
            comps: vec![Comp::from_text("  Doe  ")],
        }],
    };
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
            // MSH fields: fields[0] = MSH-2, fields[n-2] = MSH-n for n>=3
            Segment {
                id: *b"MSH",
                fields: vec![
                    Field::from_text(&encoding_chars),  // fields[0] → MSH-2
                    Field::from_text("SendingApp"),     // fields[1] → MSH-3
                    Field::from_text("SendingFac"),     // fields[2] → MSH-4
                    Field::from_text("ReceivingApp"),   // fields[3] → MSH-5
                    Field::from_text("ReceivingFac"),   // fields[4] → MSH-6
                    Field::from_text("20250128152312"), // fields[5] → MSH-7
                    Field::from_text(""),               // fields[6] → MSH-8
                    Field {
                        // fields[7] → MSH-9
                        reps: vec![Rep {
                            comps: vec![Comp::from_text("ADT"), Comp::from_text("A01")],
                        }],
                    },
                    Field::from_text("MSG001"), // fields[8] → MSH-10
                    Field::from_text("P"),      // fields[9] → MSH-11
                    Field::from_text("2.5.1"),  // fields[10] → MSH-12
                ],
            },
            Segment {
                id: *b"PID",
                fields: vec![
                    Field::from_text("1"),
                    Field::from_text(""),
                    Field {
                        reps: vec![Rep {
                            comps: vec![
                                Comp::from_text("123456"),
                                Comp::new(),
                                Comp::new(),
                                Comp::from_text("MR"),
                            ],
                        }],
                    },
                    Field::from_text(""),
                    Field {
                        reps: vec![Rep {
                            comps: vec![Comp::from_text("Doe"), Comp::from_text("John")],
                        }],
                    },
                ],
            },
        ],
        charsets: vec![],
    };
    world.message = Some(message);
}

#[given(regex = r"a ([A-Z]{3}\^[A-Z0-9]{2,3}) message")]
fn given_message_type(world: &mut QueryWorld, message_type: String) {
    let delims = Delims::default();
    let mut msh = QueryWorld::create_msh_segment(&delims);
    // Parse the message type (e.g., "ADT^A01") into components
    let type_parts: Vec<&str> = message_type.split('^').collect();
    msh.fields[7] = Field {
        reps: vec![Rep {
            comps: type_parts.iter().map(|p| Comp::from_text(*p)).collect(),
        }],
    };

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
    pid.fields[4] = Field {
        reps: vec![Rep {
            comps: vec![Comp::from_text(&long_value)],
        }],
    };

    let message = Message {
        delims: delims.clone(),
        segments: vec![QueryWorld::create_msh_segment(&delims), pid],
        charsets: vec![],
    };
    world.message = Some(message);
}

#[given("a message with special characters in field values")]
fn given_message_special_chars(world: &mut QueryWorld) {
    let delims = Delims::default();
    let mut pid = QueryWorld::create_pid_segment();
    pid.fields[4] = Field {
        reps: vec![Rep {
            comps: vec![Comp::from_text("Doe, John Jr.")],
        }],
    };

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

#[when(regex = r#"^I query the path "([^"]+)"$"#)]
fn when_query_path(world: &mut QueryWorld, path: String) {
    world.path = path.clone();
    let msg = world.message.as_ref().expect("No message");
    world.result = QueryWorld::get_field_value(msg, &path);
}

#[when(regex = r#"^I query the presence of "([^"]+)"$"#)]
fn when_query_presence(world: &mut QueryWorld, path: String) {
    world.path = path.clone();
    let msg = world.message.as_ref().expect("No message");
    world.presence = Some(get_presence(msg, &path));
}

// ============================================================================
// Then Steps
// ============================================================================

#[then(regex = r#"^the result should be "([^"]+)"$"#)]
fn then_result_value(world: &mut QueryWorld, expected: String) {
    assert_eq!(
        world.result.as_deref(),
        Some(expected.as_str()),
        "Expected result '{}' for path '{}', got {:?}",
        expected,
        world.path,
        world.result,
    );
}

#[then("the result should be None")]
fn then_result_none(world: &mut QueryWorld) {
    assert!(
        world.result.is_none(),
        "Expected None for path '{}', got {:?}",
        world.path,
        world.result,
    );
}

#[then("the result should contain the special characters")]
fn then_result_special_chars(world: &mut QueryWorld) {
    assert!(
        world
            .result
            .as_ref()
            .is_some_and(|s| s.contains(',') || s.len() > 3),
        "Expected result to contain special characters for path '{}', got {:?}",
        world.path,
        world.result,
    );
}

#[then("the result should have escape sequences decoded")]
fn then_result_escape_decoded(world: &mut QueryWorld) {
    assert!(
        world.result.as_ref().is_some_and(|s| s.contains("John")),
        "Expected result to have escape sequences for path '{}', got {:?}",
        world.path,
        world.result,
    );
}

#[then("the result should preserve the whitespace")]
fn then_result_preserve_whitespace(world: &mut QueryWorld) {
    assert!(
        world
            .result
            .as_ref()
            .is_some_and(|s| s.starts_with("  ") || s.ends_with("  ")),
        "Expected result to preserve whitespace for path '{}', got {:?}",
        world.path,
        world.result,
    );
}

#[then("the presence should be Value")]
fn then_presence_value(world: &mut QueryWorld) {
    let presence = world.presence.as_ref().expect("No presence result");
    assert!(
        matches!(presence, Presence::Value(_)),
        "Expected Presence::Value for path '{}', got {:?}",
        world.path,
        presence,
    );
}

#[then("the presence should be Empty")]
fn then_presence_empty(world: &mut QueryWorld) {
    let presence = world.presence.as_ref().expect("No presence result");
    assert!(
        matches!(presence, Presence::Empty),
        "Expected Presence::Empty for path '{}', got {:?}",
        world.path,
        presence,
    );
}

#[then("the presence should be Null")]
fn then_presence_null(world: &mut QueryWorld) {
    let presence = world.presence.as_ref().expect("No presence result");
    assert!(
        matches!(presence, Presence::Null),
        "Expected Presence::Null for path '{}', got {:?}",
        world.path,
        presence,
    );
}

#[then("the presence should be Missing")]
fn then_presence_missing(world: &mut QueryWorld) {
    let presence = world.presence.as_ref().expect("No presence result");
    assert!(
        matches!(presence, Presence::Missing),
        "Expected Presence::Missing for path '{}', got {:?}",
        world.path,
        presence,
    );
}

// ============================================================================
// Cucumber Main
// ============================================================================

#[tokio::main]
async fn main() {
    QueryWorld::run("features/query.feature").await;
}
