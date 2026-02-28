//! Integration tests for hl7v2-json crate
//!
//! Tests cover real-world HL7 to JSON conversion scenarios

use hl7v2_json::*;
use hl7v2_model::*;

// ============================================================================
// Real-World HL7 Message Tests
// ============================================================================

#[test]
fn test_adt_a01_message_to_json() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![
            Segment {
                id: *b"MSH",
                fields: vec![
                    Field::from_text("^~\\&"),
                    Field::from_text("SendingApp"),
                    Field::from_text("SendingFac"),
                    Field::from_text("ReceivingApp"),
                    Field::from_text("ReceivingFac"),
                    Field::from_text("20250128152312"),
                    Field::new(),
                    Field::from_text("ADT^A01^ADT_A01"),
                    Field::from_text("MSG123"),
                    Field::from_text("P"),
                    Field::from_text("2.5.1"),
                ],
            },
            Segment {
                id: *b"EVN",
                fields: vec![Field::from_text("A01"), Field::from_text("20250128152312")],
            },
            Segment {
                id: *b"PID",
                fields: vec![
                    Field::from_text("1"),
                    Field::from_text("12345^^^HOSP^MR"),
                    Field::new(),
                    Field {
                        reps: vec![Rep {
                            comps: vec![
                                Comp::from_text("Doe"),
                                Comp::from_text("John"),
                                Comp::from_text("A"),
                            ],
                        }],
                    },
                    Field::from_text("19850615"),
                    Field::from_text("M"),
                ],
            },
        ],
        charsets: vec![],
    };

    let json = to_json(&message);

    // Verify structure
    assert!(json.get("meta").is_some());
    assert!(json.get("segments").is_some());

    let segments = json.get("segments").unwrap().as_array().unwrap();
    assert_eq!(segments.len(), 3);

    // Verify MSH segment
    let msh = &segments[0];
    assert_eq!(msh.get("id").unwrap(), "MSH");
}

#[test]
fn test_oru_r01_message_to_json() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![
            Segment {
                id: *b"MSH",
                fields: vec![
                    Field::from_text("^~\\&"),
                    Field::from_text("LabSys"),
                    Field::from_text("Lab"),
                ],
            },
            Segment {
                id: *b"PID",
                fields: vec![Field::from_text("1"), Field::from_text("MRN789^^^Lab^MR")],
            },
            Segment {
                id: *b"OBR",
                fields: vec![
                    Field::from_text("1"),
                    Field::from_text("ORD123"),
                    Field::from_text("FIL456"),
                    Field {
                        reps: vec![Rep {
                            comps: vec![
                                Comp::from_text("CBC"),
                                Comp::from_text("Complete Blood Count"),
                            ],
                        }],
                    },
                ],
            },
            Segment {
                id: *b"OBX",
                fields: vec![
                    Field::from_text("1"),
                    Field::from_text("NM"),
                    Field {
                        reps: vec![Rep {
                            comps: vec![
                                Comp::from_text("WBC"),
                                Comp::from_text("White Blood Count"),
                            ],
                        }],
                    },
                    Field::from_text("7.5"),
                    Field::from_text("10^9/L"),
                    Field::from_text("4.0-11.0"),
                    Field::from_text("N"),
                ],
            },
        ],
        charsets: vec![],
    };

    let json = to_json(&message);
    let segments = json.get("segments").unwrap().as_array().unwrap();
    assert_eq!(segments.len(), 4);
}

// ============================================================================
// Segment Structure Tests
// ============================================================================

#[test]
fn test_segment_with_empty_fields() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"PID",
            fields: vec![
                Field::from_text("1"),
                Field::new(), // Empty field
                Field::from_text("value"),
                Field::new(), // Empty field
            ],
        }],
        charsets: vec![],
    };

    let json = to_json(&message);
    assert!(json.is_object());
}

#[test]
fn test_segment_with_repeating_fields() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"PID",
            fields: vec![
                Field::from_text("1"),
                Field {
                    reps: vec![
                        Rep {
                            comps: vec![Comp::from_text("Doe^John")],
                        },
                        Rep {
                            comps: vec![Comp::from_text("Smith^Jane")],
                        },
                        Rep {
                            comps: vec![Comp::from_text("Johnson^Bob")],
                        },
                    ],
                },
            ],
        }],
        charsets: vec![],
    };

    let json = to_json(&message);
    assert!(json.is_object());
}

// ============================================================================
// Component Structure Tests
// ============================================================================

#[test]
fn test_components_with_subcomponents() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"PID",
            fields: vec![Field {
                reps: vec![Rep {
                    comps: vec![Comp {
                        subs: vec![Atom::text("Sub1"), Atom::text("Sub2"), Atom::text("Sub3")],
                    }],
                }],
            }],
        }],
        charsets: vec![],
    };

    let json = to_json(&message);
    assert!(json.is_object());
}

#[test]
fn test_mixed_components() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"PID",
            fields: vec![Field {
                reps: vec![Rep {
                    comps: vec![
                        Comp::from_text("Simple"),
                        Comp {
                            subs: vec![Atom::text("With"), Atom::text("Subcomponents")],
                        },
                        Comp::from_text("AnotherSimple"),
                    ],
                }],
            }],
        }],
        charsets: vec![],
    };

    let json = to_json(&message);
    assert!(json.is_object());
}

// ============================================================================
// Null Value Handling Tests
// ============================================================================

#[test]
fn test_null_in_field() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"PID",
            fields: vec![
                Field::from_text("1"),
                Field {
                    reps: vec![Rep {
                        comps: vec![Comp {
                            subs: vec![Atom::Null],
                        }],
                    }],
                },
            ],
        }],
        charsets: vec![],
    };

    let json_str = to_json_string(&message);
    assert!(json_str.contains("__NULL__"));
}

#[test]
fn test_mixed_null_and_values() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"PID",
            fields: vec![Field {
                reps: vec![Rep {
                    comps: vec![
                        Comp::from_text("Value1"),
                        Comp {
                            subs: vec![Atom::Null],
                        },
                        Comp::from_text("Value2"),
                    ],
                }],
            }],
        }],
        charsets: vec![],
    };

    let json_str = to_json_string(&message);
    assert!(json_str.contains("Value1"));
    assert!(json_str.contains("__NULL__"));
    assert!(json_str.contains("Value2"));
}

// ============================================================================
// Delimiter Handling Tests
// ============================================================================

#[test]
fn test_custom_delimiters() {
    let message = Message {
        delims: Delims {
            field: '*',
            comp: ':',
            rep: '+',
            esc: '\\',
            sub: '#',
        },
        segments: vec![Segment {
            id: *b"MSH",
            fields: vec![Field::from_text(":~\\&")],
        }],
        charsets: vec![],
    };

    let json = to_json(&message);
    let delims = json.get("meta").unwrap().get("delims").unwrap();

    assert_eq!(delims.get("field").unwrap(), "*");
    assert_eq!(delims.get("comp").unwrap(), ":");
    assert_eq!(delims.get("rep").unwrap(), "+");
}

// ============================================================================
// Charset Handling Tests
// ============================================================================

#[test]
fn test_single_charset() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![],
        charsets: vec!["ASCII".to_string()],
    };

    let json = to_json(&message);
    let charsets = json
        .get("meta")
        .unwrap()
        .get("charsets")
        .unwrap()
        .as_array()
        .unwrap();

    assert_eq!(charsets.len(), 1);
    assert_eq!(charsets[0], "ASCII");
}

#[test]
fn test_multiple_charsets() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![],
        charsets: vec![
            "ASCII".to_string(),
            "UNICODE".to_string(),
            "UTF-8".to_string(),
        ],
    };

    let json = to_json(&message);
    let charsets = json
        .get("meta")
        .unwrap()
        .get("charsets")
        .unwrap()
        .as_array()
        .unwrap();

    assert_eq!(charsets.len(), 3);
}

// ============================================================================
// Output Format Tests
// ============================================================================

#[test]
fn test_compact_vs_pretty() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"MSH",
            fields: vec![Field::from_text("^~\\&")],
        }],
        charsets: vec![],
    };

    let compact = to_json_string(&message);
    let pretty = to_json_string_pretty(&message);

    // Compact should not have newlines
    assert!(!compact.contains('\n'));

    // Pretty should have newlines
    assert!(pretty.contains('\n'));

    // Both should parse to the same JSON
    let compact_parsed: serde_json::Value = serde_json::from_str(&compact).unwrap();
    let pretty_parsed: serde_json::Value = serde_json::from_str(&pretty).unwrap();

    assert_eq!(compact_parsed, pretty_parsed);
}

#[test]
fn test_json_validity() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![
            Segment {
                id: *b"MSH",
                fields: vec![Field::from_text("^~\\&")],
            },
            Segment {
                id: *b"PID",
                fields: vec![Field::from_text("1")],
            },
        ],
        charsets: vec!["ASCII".to_string()],
    };

    let json_str = to_json_string(&message);

    // Should be valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert!(parsed.is_object());
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_empty_message() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![],
        charsets: vec![],
    };

    let json = to_json(&message);

    assert!(json.get("meta").is_some());
    let segments = json.get("segments").unwrap().as_array().unwrap();
    assert!(segments.is_empty());
}

#[test]
fn test_special_characters_in_values() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"PID",
            fields: vec![Field {
                reps: vec![Rep {
                    comps: vec![Comp::from_text("O'Brien^John\\Test")],
                }],
            }],
        }],
        charsets: vec![],
    };

    let json = to_json(&message);
    assert!(json.is_object());
}

#[test]
fn test_unicode_in_values() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"PID",
            fields: vec![Field {
                reps: vec![Rep {
                    comps: vec![Comp::from_text("日本語^中文")],
                }],
            }],
        }],
        charsets: vec!["UNICODE".to_string()],
    };

    let json = to_json(&message);
    assert!(json.is_object());
}

// ============================================================================
// Large Message Tests
// ============================================================================

#[test]
fn test_many_segments() {
    let mut segments = vec![Segment {
        id: *b"MSH",
        fields: vec![Field::from_text("^~\\&")],
    }];

    for i in 1..=100 {
        segments.push(Segment {
            id: *b"OBX",
            fields: vec![
                Field::from_text(i.to_string()),
                Field::from_text("NM"),
                Field::from_text("TEST"),
                Field::from_text("100"),
            ],
        });
    }

    let message = Message {
        delims: Delims::default(),
        segments,
        charsets: vec![],
    };

    let json = to_json(&message);
    let segments = json.get("segments").unwrap().as_array().unwrap();
    assert_eq!(segments.len(), 101);
}

#[test]
fn test_deeply_nested_structure() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"PID",
            fields: vec![Field {
                reps: vec![
                    Rep {
                        comps: vec![Comp {
                            subs: vec![
                                Atom::text("A"),
                                Atom::text("B"),
                                Atom::text("C"),
                                Atom::text("D"),
                                Atom::text("E"),
                            ],
                        }],
                    },
                    Rep {
                        comps: vec![Comp {
                            subs: vec![Atom::text("F"), Atom::text("G")],
                        }],
                    },
                ],
            }],
        }],
        charsets: vec![],
    };

    let json = to_json(&message);
    assert!(json.is_object());
}
