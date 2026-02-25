//! Integration tests for hl7v2-query

use hl7v2_query::{get, get_presence};
use hl7v2_model::{Atom, Comp, Delims, Field, Message, Rep, Segment, Presence};

/// Helper to create a segment from raw field data
fn create_segment(id: &str, fields: Vec<Field>) -> Segment {
    let id_bytes = id.as_bytes();
    let mut id_arr = [0u8; 3];
    id_arr.copy_from_slice(&id_bytes[..3]);
    Segment { id: id_arr, fields }
}

/// Helper to create a text field
fn text_field(value: &str) -> Field {
    Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![Atom::Text(value.to_string())],
            }],
        }],
    }
}

/// Helper to create a component field
fn component_field(components: Vec<&str>) -> Field {
    Field {
        reps: vec![Rep {
            comps: components
                .iter()
                .map(|c| Comp {
                    subs: vec![Atom::Text(c.to_string())],
                })
                .collect(),
        }],
    }
}

/// Helper to create a field with repetitions
fn repeating_field(repetitions: Vec<Vec<&str>>) -> Field {
    Field {
        reps: repetitions
            .iter()
            .map(|comps| Rep {
                comps: comps
                    .iter()
                    .map(|c| Comp {
                        subs: vec![Atom::Text(c.to_string())],
                    })
                    .collect(),
            })
            .collect(),
    }
}

// =============================================================================
// Real Message Query Tests
// =============================================================================

#[test]
fn test_query_adt_message() {
    // Create a typical ADT^A01 message structure
    let message = Message {
        delims: Delims::default(),
        segments: vec![
            create_segment("MSH", vec![
                text_field("^~\\&"),
                component_field(vec!["SendingApp"]),
                component_field(vec!["SendingFac"]),
                component_field(vec!["ReceivingApp"]),
                component_field(vec!["ReceivingFac"]),
                component_field(vec!["20250128152312"]),
                component_field(vec![]),
                component_field(vec!["ADT", "A01", "ADT_A01"]),
                component_field(vec!["MSG00001"]),
                component_field(vec!["P"]),
                component_field(vec!["2.5.1"]),
            ]),
            create_segment("PID", vec![
                text_field("1"),
                text_field(""),
                component_field(vec!["123456", "HOSP", "MR"]),
                text_field(""),
                component_field(vec!["Doe", "John", "R"]),
                text_field(""),
                component_field(vec!["19800101"]),
                component_field(vec!["M"]),
            ]),
            create_segment("PV1", vec![
                text_field("1"),
                component_field(vec!["I"]),
                component_field(vec!["ICU", "01", "01"]),
            ]),
        ],
        charsets: vec![],
    };

    // Query MSH fields
    assert_eq!(get(&message, "MSH.3"), Some("SendingApp"));
    assert_eq!(get(&message, "MSH.4"), Some("SendingFac"));
    assert_eq!(get(&message, "MSH.9.1"), Some("ADT"));
    assert_eq!(get(&message, "MSH.9.2"), Some("A01"));
    assert_eq!(get(&message, "MSH.10"), Some("MSG00001"));
    assert_eq!(get(&message, "MSH.12"), Some("2.5.1"));

    // Query PID fields
    assert_eq!(get(&message, "PID.1"), Some("1"));
    assert_eq!(get(&message, "PID.3.1"), Some("123456"));
    assert_eq!(get(&message, "PID.5.1"), Some("Doe"));
    assert_eq!(get(&message, "PID.5.2"), Some("John"));
    assert_eq!(get(&message, "PID.7"), Some("19800101"));
    assert_eq!(get(&message, "PID.8"), Some("M"));

    // Query PV1 fields
    assert_eq!(get(&message, "PV1.1"), Some("1"));
    assert_eq!(get(&message, "PV1.2"), Some("I"));
    assert_eq!(get(&message, "PV1.3.1"), Some("ICU"));
}

#[test]
fn test_query_oru_message() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![
            create_segment("MSH", vec![
                text_field("^~\\&"),
                component_field(vec!["LabSystem"]),
                component_field(vec!["Lab"]),
                component_field(vec!["HIS"]),
                component_field(vec!["Hospital"]),
                component_field(vec!["20250128152312"]),
                component_field(vec![]),
                component_field(vec!["ORU", "R01"]),
                component_field(vec!["LAB00001"]),
                component_field(vec!["P"]),
                component_field(vec!["2.5.1"]),
            ]),
            create_segment("PID", vec![
                text_field("1"),
                text_field(""),
                component_field(vec!["PATID123"]),
                text_field(""),
                component_field(vec!["Smith", "Jane"]),
            ]),
            create_segment("OBR", vec![
                text_field("1"),
                text_field(""),
                component_field(vec!["ORD001"]),
                component_field(vec!["CBC", "Complete Blood Count", "L"]),
            ]),
            create_segment("OBX", vec![
                text_field("1"),
                component_field(vec!["NM"]),
                component_field(vec!["HB", "Hemoglobin", "L"]),
                text_field(""),
                component_field(vec!["13.2"]),
                component_field(vec!["g/dL"]),
                component_field(vec!["11.5-17.5"]),
            ]),
        ],
        charsets: vec![],
    };

    // Query message type
    assert_eq!(get(&message, "MSH.9.1"), Some("ORU"));
    assert_eq!(get(&message, "MSH.9.2"), Some("R01"));

    // Query patient
    assert_eq!(get(&message, "PID.5.1"), Some("Smith"));
    assert_eq!(get(&message, "PID.5.2"), Some("Jane"));

    // Query observation
    assert_eq!(get(&message, "OBR.4.1"), Some("CBC"));
    assert_eq!(get(&message, "OBR.4.2"), Some("Complete Blood Count"));

    // Query result
    assert_eq!(get(&message, "OBX.5"), Some("13.2"));
    assert_eq!(get(&message, "OBX.6"), Some("g/dL"));
}

// =============================================================================
// Presence Tests
// =============================================================================

#[test]
fn test_presence_value() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![create_segment("PID", vec![text_field("12345")])],
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
        segments: vec![create_segment("PID", vec![text_field("")])],
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
        segments: vec![create_segment("PID", vec![])],
        charsets: vec![],
    };

    match get_presence(&message, "PID.50") {
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
        segments: vec![create_segment("PID", vec![Field {
            reps: vec![Rep {
                comps: vec![Comp {
                    subs: vec![Atom::Null],
                }],
            }],
        }])],
        charsets: vec![],
    };

    match get_presence(&message, "PID.1") {
        Presence::Null => {}
        _ => panic!("Expected Null"),
    }
}

// =============================================================================
// Repetition Tests
// =============================================================================

#[test]
fn test_query_with_repetitions() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![create_segment("PID", vec![
            repeating_field(vec![
                vec!["Doe", "John"],
                vec!["Smith", "Jane"],
                vec!["Brown", "Bob"],
            ]),
        ])],
        charsets: vec![],
    };

    // Default to first repetition
    assert_eq!(get(&message, "PID.1.1"), Some("Doe"));
    assert_eq!(get(&message, "PID.1.2"), Some("John"));

    // Explicit repetitions
    assert_eq!(get(&message, "PID.1[1].1"), Some("Doe"));
    assert_eq!(get(&message, "PID.1[2].1"), Some("Smith"));
    assert_eq!(get(&message, "PID.1[3].1"), Some("Brown"));

    // Invalid repetition
    assert_eq!(get(&message, "PID.1[4].1"), None);
}

// =============================================================================
// Cross-Crate Integration Tests
// =============================================================================

#[test]
fn test_query_parsed_message() {
    // Parse a real HL7 message
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|MSG00001|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\rPV1|1|I|ICU^01^01\r";
    
    let message = hl7v2_parser::parse(hl7).unwrap();

    // Query the parsed message
    assert_eq!(get(&message, "MSH.3"), Some("SendingApp"));
    assert_eq!(get(&message, "MSH.9.1"), Some("ADT"));
    assert_eq!(get(&message, "MSH.9.2"), Some("A01"));
    assert_eq!(get(&message, "PID.3.1"), Some("123456"));
    assert_eq!(get(&message, "PID.5.1"), Some("Doe"));
    assert_eq!(get(&message, "PID.5.2"), Some("John"));
    assert_eq!(get(&message, "PV1.2"), Some("I"));
}

// =============================================================================
// Edge Cases Tests
// =============================================================================

#[test]
fn test_query_empty_message() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![],
        charsets: vec![],
    };

    assert_eq!(get(&message, "PID.1"), None);
    assert_eq!(get(&message, "MSH.3"), None);
}

#[test]
fn test_query_multiple_same_segment() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![
            create_segment("NTE", vec![text_field("First note")]),
            create_segment("NTE", vec![text_field("Second note")]),
            create_segment("NTE", vec![text_field("Third note")]),
        ],
        charsets: vec![],
    };

    // Should return first matching segment
    assert_eq!(get(&message, "NTE.1"), Some("First note"));
}

#[test]
fn test_query_deep_component() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![create_segment("PID", vec![
            Field {
                reps: vec![Rep {
                    comps: vec![
                        Comp {
                            subs: vec![
                                Atom::Text("Doe".to_string()),
                                Atom::Text("John".to_string()),
                            ],
                        },
                    ],
                }],
            },
        ])],
        charsets: vec![],
    };

    // Get first subcomponent
    assert_eq!(get(&message, "PID.1.1"), Some("Doe"));
}

// =============================================================================
// MSH Special Cases Tests
// =============================================================================

#[test]
fn test_query_msh_field_1() {
    let message = Message {
        delims: Delims {
            field: '|',
            comp: '^',
            rep: '~',
            esc: '\\',
            sub: '&',
        },
        segments: vec![create_segment("MSH", vec![])],
        charsets: vec![],
    };

    // MSH-1 returns the field separator
    match get_presence(&message, "MSH.1") {
        Presence::Value(val) => assert_eq!(val, "|"),
        _ => panic!("Expected Value"),
    }
}

#[test]
fn test_query_msh_field_2() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![create_segment("MSH", vec![
            text_field("^~\\&"),
        ])],
        charsets: vec![],
    };

    assert_eq!(get(&message, "MSH.2"), Some("^~\\&"));
}

// =============================================================================
// Invalid Path Tests
// =============================================================================

#[test]
fn test_query_invalid_paths() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![create_segment("PID", vec![text_field("1")])],
        charsets: vec![],
    };

    // Empty path
    assert!(get(&message, "").is_none());

    // Segment only
    assert!(get(&message, "PID").is_none());

    // Invalid field index
    assert!(get(&message, "PID.abc").is_none());

    // Invalid repetition syntax
    assert!(get(&message, "PID.1[").is_none());
    assert!(get(&message, "PID.1[abc]").is_none());
}

// =============================================================================
// Complex Field Tests
// =============================================================================

#[test]
fn test_query_complex_pid_3() {
    // PID-3 is typically a complex field with components and subcomponents
    let message = Message {
        delims: Delims::default(),
        segments: vec![create_segment("PID", vec![
            text_field("1"),
            text_field(""),
            Field {
                reps: vec![Rep {
                    comps: vec![
                        Comp {
                            subs: vec![Atom::Text("123456".to_string())],
                        },
                        Comp {
                            subs: vec![Atom::Text("HOSP".to_string())],
                        },
                        Comp {
                            subs: vec![Atom::Text("MR".to_string())],
                        },
                    ],
                }],
            },
        ])],
        charsets: vec![],
    };

    assert_eq!(get(&message, "PID.3.1"), Some("123456"));
    assert_eq!(get(&message, "PID.3.2"), Some("HOSP"));
    assert_eq!(get(&message, "PID.3.3"), Some("MR"));
}

#[test]
fn test_query_complex_pid_5() {
    // PID-5 is typically a complex name field
    let message = Message {
        delims: Delims::default(),
        segments: vec![create_segment("PID", vec![
            text_field("1"),
            text_field(""),
            text_field(""),
            text_field(""),
            Field {
                reps: vec![Rep {
                    comps: vec![
                        Comp {
                            subs: vec![Atom::Text("Doe".to_string())],
                        },
                        Comp {
                            subs: vec![Atom::Text("John".to_string())],
                        },
                        Comp {
                            subs: vec![Atom::Text("R".to_string())],
                        },
                        Comp {
                            subs: vec![Atom::Text("Jr".to_string())],
                        },
                        Comp {
                            subs: vec![Atom::Text("Dr".to_string())],
                        },
                    ],
                }],
            },
        ])],
        charsets: vec![],
    };

    assert_eq!(get(&message, "PID.5.1"), Some("Doe"));    // Family name
    assert_eq!(get(&message, "PID.5.2"), Some("John"));    // Given name
    assert_eq!(get(&message, "PID.5.3"), Some("R"));       // Middle name
    assert_eq!(get(&message, "PID.5.4"), Some("Jr"));      // Suffix
    assert_eq!(get(&message, "PID.5.5"), Some("Dr"));      // Prefix
}
