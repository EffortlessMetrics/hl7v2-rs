//! Unit tests for hl7v2-query

use super::*;
use hl7v2_model::{Comp, Delims, Field, Rep};

/// Helper to create a simple segment for testing
fn create_test_segment(id: &str, fields: Vec<Field>) -> Segment {
    let id_bytes = id.as_bytes();
    let mut id_arr = [0u8; 3];
    id_arr.copy_from_slice(&id_bytes[..3]);
    Segment { id: id_arr, fields }
}

/// Helper to create a text field with repetitions
fn create_text_field(texts: Vec<&str>) -> Field {
    Field {
        reps: texts
            .iter()
            .map(|t| Rep {
                comps: vec![Comp {
                    subs: vec![Atom::Text(t.to_string())],
                }],
            })
            .collect(),
    }
}

/// Helper to create a field with components
fn create_component_field(components: Vec<Vec<&str>>) -> Field {
    Field {
        reps: vec![Rep {
            comps: components
                .iter()
                .map(|subs| Comp {
                    subs: subs
                        .iter()
                        .map(|s| {
                            if *s == "\"\"" {
                                Atom::Null
                            } else {
                                Atom::Text(s.to_string())
                            }
                        })
                        .collect(),
                })
                .collect(),
        }],
    }
}

// =============================================================================
// Path Parsing Tests
// =============================================================================

#[test]
fn test_parse_field_and_rep_simple() {
    assert_eq!(parse_field_and_rep("5"), Some((5, 1)));
    assert_eq!(parse_field_and_rep("1"), Some((1, 1)));
    assert_eq!(parse_field_and_rep("10"), Some((10, 1)));
}

#[test]
fn test_parse_field_and_rep_with_repetition() {
    assert_eq!(parse_field_and_rep("5[1]"), Some((5, 1)));
    assert_eq!(parse_field_and_rep("5[2]"), Some((5, 2)));
    assert_eq!(parse_field_and_rep("10[5]"), Some((10, 5)));
}

#[test]
fn test_parse_field_and_rep_invalid() {
    assert_eq!(parse_field_and_rep("abc"), None);
    assert_eq!(parse_field_and_rep("5["), None);
    assert_eq!(parse_field_and_rep("5[abc]"), None);
    assert_eq!(parse_field_and_rep(""), None);
}

// =============================================================================
// Basic Get Tests
// =============================================================================

#[test]
fn test_get_simple_field() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![create_test_segment(
            "PID",
            vec![
                create_text_field(vec!["1"]),
                create_text_field(vec![""]),
                create_text_field(vec!["12345"]),
            ],
        )],
        charsets: vec![],
    };

    assert_eq!(get(&message, "PID.1"), Some("1"));
    assert_eq!(get(&message, "PID.3"), Some("12345"));
}

#[test]
fn test_get_component_field() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![create_test_segment(
            "PID",
            vec![
                create_text_field(vec!["1"]),
                create_text_field(vec![""]),
                create_text_field(vec![""]),
                create_text_field(vec![""]),
                create_component_field(vec![vec!["Doe"], vec!["John"]]),
            ],
        )],
        charsets: vec![],
    };

    assert_eq!(get(&message, "PID.5.1"), Some("Doe"));
    assert_eq!(get(&message, "PID.5.2"), Some("John"));
}

#[test]
fn test_get_missing_segment() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![create_test_segment("PID", vec![])],
        charsets: vec![],
    };

    assert_eq!(get(&message, "EVN.1"), None);
}

#[test]
fn test_get_missing_field() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![create_test_segment(
            "PID",
            vec![create_text_field(vec!["1"])],
        )],
        charsets: vec![],
    };

    assert_eq!(get(&message, "PID.10"), None);
}

#[test]
fn test_get_missing_component() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![create_test_segment(
            "PID",
            vec![create_component_field(vec![vec!["Doe"], vec!["John"]])],
        )],
        charsets: vec![],
    };

    assert_eq!(get(&message, "PID.1.10"), None);
}

// =============================================================================
// Repetition Tests
// =============================================================================

#[test]
fn test_get_with_repetitions() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![create_test_segment(
            "PID",
            vec![Field {
                reps: vec![
                    Rep {
                        comps: vec![Comp {
                            subs: vec![Atom::Text("First".to_string())],
                        }],
                    },
                    Rep {
                        comps: vec![Comp {
                            subs: vec![Atom::Text("Second".to_string())],
                        }],
                    },
                    Rep {
                        comps: vec![Comp {
                            subs: vec![Atom::Text("Third".to_string())],
                        }],
                    },
                ],
            }],
        )],
        charsets: vec![],
    };

    // Default to first repetition
    assert_eq!(get(&message, "PID.1"), Some("First"));

    // Explicit repetitions
    assert_eq!(get(&message, "PID.1[1]"), Some("First"));
    assert_eq!(get(&message, "PID.1[2]"), Some("Second"));
    assert_eq!(get(&message, "PID.1[3]"), Some("Third"));

    // Invalid repetition
    assert_eq!(get(&message, "PID.1[4]"), None);
    assert_eq!(get(&message, "PID.1[0]"), None);
}

// =============================================================================
// MSH Segment Tests
// =============================================================================

#[test]
fn test_get_msh_field_2() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![create_test_segment(
            "MSH",
            vec![create_text_field(vec!["^~\\&"])],
        )],
        charsets: vec![],
    };

    assert_eq!(get(&message, "MSH.2"), Some("^~\\&"));
}

#[test]
fn test_get_msh_field_3_and_beyond() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![create_test_segment(
            "MSH",
            vec![
                create_text_field(vec!["^~\\&"]),
                create_component_field(vec![vec!["SendingApp"]]),
                create_component_field(vec![vec!["SendingFac"]]),
                create_component_field(vec![vec!["ADT"], vec!["A01"]]),
            ],
        )],
        charsets: vec![],
    };

    assert_eq!(get(&message, "MSH.3"), Some("SendingApp"));
    assert_eq!(get(&message, "MSH.4"), Some("SendingFac"));
    assert_eq!(get(&message, "MSH.9.1"), Some("ADT"));
    assert_eq!(get(&message, "MSH.9.2"), Some("A01"));
}

#[test]
fn test_get_msh_missing_field() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![create_test_segment("MSH", vec![])],
        charsets: vec![],
    };

    assert_eq!(get(&message, "MSH.3"), None);
    assert_eq!(get(&message, "MSH.10"), None);
}

// =============================================================================
// Presence Tests
// =============================================================================

#[test]
fn test_presence_value() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![create_test_segment(
            "PID",
            vec![create_text_field(vec!["12345"])],
        )],
        charsets: vec![],
    };

    match get_presence(&message, "PID.1") {
        Presence::Value(val) => assert_eq!(val, "12345"),
        _ => panic!("Expected Value"),
    }
}

#[test]
fn test_presence_empty() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![create_test_segment(
            "PID",
            vec![create_text_field(vec![""])],
        )],
        charsets: vec![],
    };

    match get_presence(&message, "PID.1") {
        Presence::Empty => {}
        _ => panic!("Expected Empty"),
    }
}

#[test]
fn test_presence_missing_field() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![create_test_segment("PID", vec![])],
        charsets: vec![],
    };

    match get_presence(&message, "PID.10") {
        Presence::Missing => {}
        _ => panic!("Expected Missing"),
    }
}

#[test]
fn test_presence_missing_segment() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![],
        charsets: vec![],
    };

    match get_presence(&message, "EVN.1") {
        Presence::Missing => {}
        _ => panic!("Expected Missing"),
    }
}

#[test]
fn test_presence_null() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![create_test_segment(
            "PID",
            vec![Field {
                reps: vec![Rep {
                    comps: vec![Comp {
                        subs: vec![Atom::Null],
                    }],
                }],
            }],
        )],
        charsets: vec![],
    };

    match get_presence(&message, "PID.1") {
        Presence::Null => {}
        _ => panic!("Expected Null"),
    }
}

#[test]
fn test_presence_msh_field_1() {
    let message = Message {
        delims: Delims {
            field: '|',
            comp: '^',
            rep: '~',
            esc: '\\',
            sub: '&',
        },
        segments: vec![create_test_segment("MSH", vec![])],
        charsets: vec![],
    };

    // MSH-1 returns the field separator
    match get_presence(&message, "MSH.1") {
        Presence::Value(val) => assert_eq!(val, "|"),
        _ => panic!("Expected Value"),
    }
}

// =============================================================================
// Invalid Path Tests
// =============================================================================

#[test]
fn test_get_empty_path() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![],
        charsets: vec![],
    };

    assert!(get(&message, "").is_none());
}

#[test]
fn test_get_segment_only() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![create_test_segment("PID", vec![])],
        charsets: vec![],
    };

    assert!(get(&message, "PID").is_none());
}

#[test]
fn test_get_invalid_field_index() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![create_test_segment("PID", vec![])],
        charsets: vec![],
    };

    assert!(get(&message, "PID.abc").is_none());
}

#[test]
fn test_get_invalid_component_index() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![create_test_segment(
            "PID",
            vec![create_component_field(vec![vec!["test"]])],
        )],
        charsets: vec![],
    };

    assert!(get(&message, "PID.1.abc").is_none());
}

#[test]
fn test_get_zero_field_index() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![create_test_segment(
            "PID",
            vec![create_text_field(vec!["test"])],
        )],
        charsets: vec![],
    };

    // Field index 0 is invalid
    assert!(get(&message, "PID.0").is_none());
}

#[test]
fn test_get_zero_component_index() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![create_test_segment(
            "PID",
            vec![create_component_field(vec![vec!["test"]])],
        )],
        charsets: vec![],
    };

    // Component index 0 is invalid
    assert!(get(&message, "PID.1.0").is_none());
}

// =============================================================================
// Edge Cases Tests
// =============================================================================

#[test]
fn test_get_null_atom() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![create_test_segment(
            "PID",
            vec![Field {
                reps: vec![Rep {
                    comps: vec![Comp {
                        subs: vec![Atom::Null],
                    }],
                }],
            }],
        )],
        charsets: vec![],
    };

    // Null atom should return None for get
    assert!(get(&message, "PID.1").is_none());
}

#[test]
fn test_get_empty_subcomponents() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![create_test_segment(
            "PID",
            vec![Field {
                reps: vec![Rep {
                    comps: vec![Comp { subs: vec![] }],
                }],
            }],
        )],
        charsets: vec![],
    };

    assert!(get(&message, "PID.1").is_none());
}

#[test]
fn test_get_multiple_segments() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![
            create_test_segment("MSH", vec![create_text_field(vec!["^~\\&"])]),
            create_test_segment("PID", vec![create_text_field(vec!["1"])]),
            create_test_segment("PV1", vec![create_text_field(vec!["I"])]),
        ],
        charsets: vec![],
    };

    assert_eq!(get(&message, "MSH.2"), Some("^~\\&"));
    assert_eq!(get(&message, "PID.1"), Some("1"));
    assert_eq!(get(&message, "PV1.1"), Some("I"));
}

#[test]
fn test_get_first_segment_when_multiple_same_id() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![
            create_test_segment("NTE", vec![create_text_field(vec!["First note"])]),
            create_test_segment("NTE", vec![create_text_field(vec!["Second note"])]),
        ],
        charsets: vec![],
    };

    // Should return the first matching segment
    assert_eq!(get(&message, "NTE.1"), Some("First note"));
}

// =============================================================================
// Presence with Repetitions Tests
// =============================================================================

#[test]
fn test_presence_with_repetitions() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![create_test_segment(
            "PID",
            vec![Field {
                reps: vec![
                    Rep {
                        comps: vec![Comp {
                            subs: vec![Atom::Text("First".to_string())],
                        }],
                    },
                    Rep {
                        comps: vec![Comp {
                            subs: vec![Atom::Text("Second".to_string())],
                        }],
                    },
                ],
            }],
        )],
        charsets: vec![],
    };

    match get_presence(&message, "PID.1[1]") {
        Presence::Value(val) => assert_eq!(val, "First"),
        _ => panic!("Expected Value"),
    }

    match get_presence(&message, "PID.1[2]") {
        Presence::Value(val) => assert_eq!(val, "Second"),
        _ => panic!("Expected Value"),
    }

    match get_presence(&message, "PID.1[3]") {
        Presence::Missing => {}
        _ => panic!("Expected Missing"),
    }
}

// =============================================================================
// MSH Presence Tests
// =============================================================================

#[test]
fn test_presence_msh_field_2() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![create_test_segment(
            "MSH",
            vec![create_text_field(vec!["^~\\&"])],
        )],
        charsets: vec![],
    };

    match get_presence(&message, "MSH.2") {
        Presence::Value(val) => assert_eq!(val, "^~\\&"),
        _ => panic!("Expected Value"),
    }
}

#[test]
fn test_presence_msh_field_3() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![create_test_segment(
            "MSH",
            vec![
                create_text_field(vec!["^~\\&"]),
                create_component_field(vec![vec!["SendingApp"]]),
            ],
        )],
        charsets: vec![],
    };

    match get_presence(&message, "MSH.3") {
        Presence::Value(val) => assert_eq!(val, "SendingApp"),
        _ => panic!("Expected Value"),
    }
}

#[test]
fn test_presence_msh_empty_field() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![create_test_segment(
            "MSH",
            vec![
                create_text_field(vec!["^~\\&"]),
                create_component_field(vec![vec![""]]),
            ],
        )],
        charsets: vec![],
    };

    match get_presence(&message, "MSH.3") {
        Presence::Empty => {}
        _ => panic!("Expected Empty"),
    }
}
