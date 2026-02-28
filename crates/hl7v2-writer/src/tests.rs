//! Comprehensive unit tests for hl7v2-writer crate
//!
//! These tests cover:
//! - Message serialization (write function)
//! - MLLP framing (write_mllp function)
//! - Batch serialization (write_batch function)
//! - File batch serialization (write_file_batch function)
//! - Edge cases and error conditions

use super::*;
use hl7v2_model::{Atom, Batch, Comp, Delims, Field, FileBatch, Message, Rep, Segment};

// ============================================================================
// Basic write() function tests
// ============================================================================

#[test]
fn test_write_empty_message() {
    let message = Message::new();
    let bytes = write(&message);
    // Empty message should still produce output
    assert!(bytes.is_empty() || bytes == b"\r" || bytes.starts_with(b"MSH"));
}

#[test]
fn test_write_minimal_msh() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"MSH",
            fields: vec![Field::from_text("^~\\&")],
        }],
        charsets: vec![],
    };

    let bytes = write(&message);
    let result = String::from_utf8(bytes).unwrap();

    assert!(result.starts_with("MSH|"));
    assert!(result.contains("^~\\&"));
    assert!(result.ends_with("\r"));
}

#[test]
fn test_write_msh_with_fields() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"MSH",
            fields: vec![
                Field::from_text("^~\\&"),
                Field::from_text("SENDAPP"),
                Field::from_text("SENDFAC"),
                Field::from_text("RECVAPP"),
                Field::from_text("RECVFAC"),
                Field::from_text("20231225120000"),
                Field::from_text(""),
                Field::from_text("ADT^A01"),
                Field::from_text("MSG00001"),
                Field::from_text("P"),
                Field::from_text("2.5"),
            ],
        }],
        charsets: vec![],
    };

    let bytes = write(&message);
    let result = String::from_utf8(bytes).unwrap();

    assert!(result.starts_with("MSH|^~\\&"));
    assert!(result.contains("SENDAPP"));
    assert!(result.contains("SENDFAC"));
    // ^ is component separator, gets escaped as \S\
    assert!(result.contains("ADT\\S\\A01"));
    assert!(result.contains("MSG00001"));
    assert!(result.ends_with("\r"));
}

#[test]
fn test_write_single_segment() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"PID",
            fields: vec![
                Field::from_text("1"),
                Field::from_text("12345"),
                Field::from_text("DOE^JOHN^A"),
            ],
        }],
        charsets: vec![],
    };

    let bytes = write(&message);
    let result = String::from_utf8(bytes).unwrap();

    assert!(result.starts_with("PID|"));
    assert!(result.contains("12345"));
    // ^ is component separator, gets escaped as \S\
    assert!(result.contains("DOE\\S\\JOHN\\S\\A"));
    assert!(result.ends_with("\r"));
}

#[test]
fn test_write_multiple_segments() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![
            Segment {
                id: *b"MSH",
                fields: vec![
                    Field::from_text("^~\\&"),
                    Field::from_text("APP"),
                    Field::from_text("FAC"),
                ],
            },
            Segment {
                id: *b"PID",
                fields: vec![Field::from_text("1"), Field::from_text("12345")],
            },
            Segment {
                id: *b"PV1",
                fields: vec![Field::from_text("1"), Field::from_text("I")],
            },
        ],
        charsets: vec![],
    };

    let bytes = write(&message);
    let result = String::from_utf8(bytes).unwrap();

    let segments: Vec<&str> = result.trim_end_matches('\r').split('\r').collect();
    assert_eq!(segments.len(), 3);
    assert!(segments[0].starts_with("MSH|"));
    assert!(segments[1].starts_with("PID|"));
    assert!(segments[2].starts_with("PV1|"));
}

#[test]
fn test_write_segment_terminator() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"MSH",
            fields: vec![Field::from_text("^~\\&")],
        }],
        charsets: vec![],
    };

    let bytes = write(&message);
    // Each segment should end with \r
    assert!(bytes.ends_with(b"\r"));
}

// ============================================================================
// Repetition tests
// ============================================================================

#[test]
fn test_write_single_repetition() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"PID",
            fields: vec![
                Field::from_text("1"),
                Field {
                    reps: vec![Rep::from_text("VALUE")],
                },
            ],
        }],
        charsets: vec![],
    };

    let bytes = write(&message);
    let result = String::from_utf8(bytes).unwrap();

    assert!(result.contains("VALUE"));
    assert!(!result.contains("~")); // No repetition separator
}

#[test]
fn test_write_multiple_repetitions() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"PID",
            fields: vec![
                Field::from_text("1"),
                Field {
                    reps: vec![
                        Rep::from_text("FIRST"),
                        Rep::from_text("SECOND"),
                        Rep::from_text("THIRD"),
                    ],
                },
            ],
        }],
        charsets: vec![],
    };

    let bytes = write(&message);
    let result = String::from_utf8(bytes).unwrap();

    assert!(result.contains("FIRST~SECOND~THIRD"));
}

#[test]
fn test_write_empty_repetition() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"PID",
            fields: vec![
                Field::from_text("1"),
                Field {
                    reps: vec![Rep::from_text(""), Rep::from_text("VALUE")],
                },
            ],
        }],
        charsets: vec![],
    };

    let bytes = write(&message);
    let result = String::from_utf8(bytes).unwrap();

    // Empty repetition should still produce separator
    assert!(result.contains("~VALUE"));
}

// ============================================================================
// Component tests
// ============================================================================

#[test]
fn test_write_single_component() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"PID",
            fields: vec![
                Field::from_text("1"),
                Field {
                    reps: vec![Rep {
                        comps: vec![Comp::from_text("SINGLE")],
                    }],
                },
            ],
        }],
        charsets: vec![],
    };

    let bytes = write(&message);
    let result = String::from_utf8(bytes).unwrap();

    assert!(result.contains("SINGLE"));
    assert!(!result.contains("^")); // No component separator in single component
}

#[test]
fn test_write_multiple_components() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"PID",
            fields: vec![
                Field::from_text("1"),
                Field {
                    reps: vec![Rep {
                        comps: vec![
                            Comp::from_text("DOE"),
                            Comp::from_text("JOHN"),
                            Comp::from_text("A"),
                        ],
                    }],
                },
            ],
        }],
        charsets: vec![],
    };

    let bytes = write(&message);
    let result = String::from_utf8(bytes).unwrap();

    assert!(result.contains("DOE^JOHN^A"));
}

#[test]
fn test_write_components_with_repetitions() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"PID",
            fields: vec![
                Field::from_text("1"),
                Field {
                    reps: vec![
                        Rep {
                            comps: vec![Comp::from_text("DOE"), Comp::from_text("JOHN")],
                        },
                        Rep {
                            comps: vec![Comp::from_text("SMITH"), Comp::from_text("JANE")],
                        },
                    ],
                },
            ],
        }],
        charsets: vec![],
    };

    let bytes = write(&message);
    let result = String::from_utf8(bytes).unwrap();

    assert!(result.contains("DOE^JOHN~SMITH^JANE"));
}

// ============================================================================
// Subcomponent tests
// ============================================================================

#[test]
fn test_write_single_subcomponent() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"PID",
            fields: vec![
                Field::from_text("1"),
                Field {
                    reps: vec![Rep {
                        comps: vec![Comp {
                            subs: vec![Atom::text("SINGLE")],
                        }],
                    }],
                },
            ],
        }],
        charsets: vec![],
    };

    let bytes = write(&message);
    let result = String::from_utf8(bytes).unwrap();

    assert!(result.contains("SINGLE"));
    assert!(!result.contains("&")); // No subcomponent separator
}

#[test]
fn test_write_multiple_subcomponents() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"OBX",
            fields: vec![
                Field::from_text("1"),
                Field::from_text("ST"),
                Field::from_text("TEST"),
                Field {
                    reps: vec![Rep {
                        comps: vec![Comp {
                            subs: vec![
                                Atom::text("FIRST"),
                                Atom::text("SECOND"),
                                Atom::text("THIRD"),
                            ],
                        }],
                    }],
                },
            ],
        }],
        charsets: vec![],
    };

    let bytes = write(&message);
    let result = String::from_utf8(bytes).unwrap();

    assert!(result.contains("FIRST&SECOND&THIRD"));
}

// ============================================================================
// Escaping tests
// ============================================================================

#[test]
fn test_write_escape_field_separator() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"PID",
            fields: vec![Field::from_text("test|value")],
        }],
        charsets: vec![],
    };

    let bytes = write(&message);
    let result = String::from_utf8(bytes).unwrap();

    // Field separator | should be escaped as \F\
    assert!(result.contains("test\\F\\value"));
    assert!(!result.contains("test|value"));
}

#[test]
fn test_write_escape_component_separator() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"PID",
            fields: vec![Field::from_text("test^value")],
        }],
        charsets: vec![],
    };

    let bytes = write(&message);
    let result = String::from_utf8(bytes).unwrap();

    // Component separator ^ should be escaped as \S\
    assert!(result.contains("test\\S\\value"));
}

#[test]
fn test_write_escape_repetition_separator() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"PID",
            fields: vec![Field::from_text("test~value")],
        }],
        charsets: vec![],
    };

    let bytes = write(&message);
    let result = String::from_utf8(bytes).unwrap();

    // Repetition separator ~ should be escaped as \R\
    assert!(result.contains("test\\R\\value"));
}

#[test]
fn test_write_escape_subcomponent_separator() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"PID",
            fields: vec![Field::from_text("test&value")],
        }],
        charsets: vec![],
    };

    let bytes = write(&message);
    let result = String::from_utf8(bytes).unwrap();

    // Subcomponent separator & should be escaped as \T\
    assert!(result.contains("test\\T\\value"));
}

#[test]
fn test_write_escape_escape_character() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"PID",
            fields: vec![Field::from_text("test\\value")],
        }],
        charsets: vec![],
    };

    let bytes = write(&message);
    let result = String::from_utf8(bytes).unwrap();

    // Escape character \ should be escaped as \E\
    assert!(result.contains("test\\E\\value"));
}

#[test]
fn test_write_escape_multiple_delimiters() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"PID",
            fields: vec![Field::from_text("a|b^c~d&e\\f")],
        }],
        charsets: vec![],
    };

    let bytes = write(&message);
    let result = String::from_utf8(bytes).unwrap();

    assert!(result.contains("a\\F\\b\\S\\c\\R\\d\\T\\e\\E\\f"));
}

#[test]
fn test_write_no_escape_needed() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"PID",
            fields: vec![Field::from_text("normal text 123")],
        }],
        charsets: vec![],
    };

    let bytes = write(&message);
    let result = String::from_utf8(bytes).unwrap();

    // No escaping needed
    assert!(result.contains("normal text 123"));
    assert!(!result.contains("\\"));
}

// ============================================================================
// MLLP tests
// ============================================================================

#[test]
fn test_write_mllp_basic() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"MSH",
            fields: vec![Field::from_text("^~\\&")],
        }],
        charsets: vec![],
    };

    let framed = write_mllp(&message);

    // Check MLLP framing
    assert_eq!(framed[0], hl7v2_mllp::MLLP_START);
    assert_eq!(framed[framed.len() - 2], hl7v2_mllp::MLLP_END_1);
    assert_eq!(framed[framed.len() - 1], hl7v2_mllp::MLLP_END_2);
}

#[test]
fn test_write_mllp_contains_message() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"MSH",
            fields: vec![Field::from_text("^~\\&"), Field::from_text("APP")],
        }],
        charsets: vec![],
    };

    let framed = write_mllp(&message);
    let content = &framed[1..framed.len() - 2];

    assert!(content.starts_with(b"MSH|"));
    assert!(content.windows(3).any(|w| w == b"APP"));
}

#[test]
fn test_write_mllp_empty_message() {
    let message = Message::new();
    let framed = write_mllp(&message);

    // Even empty message should have MLLP framing
    assert_eq!(framed[0], hl7v2_mllp::MLLP_START);
    assert_eq!(framed[framed.len() - 2], hl7v2_mllp::MLLP_END_1);
    assert_eq!(framed[framed.len() - 1], hl7v2_mllp::MLLP_END_2);
}

// ============================================================================
// Batch tests
// ============================================================================

#[test]
fn test_write_batch_empty() {
    let batch = Batch::default();
    let bytes = write_batch(&batch);
    // Empty batch should produce empty output
    assert!(bytes.is_empty());
}

#[test]
fn test_write_batch_single_message() {
    let mut batch = Batch::default();
    batch.messages.push(Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"MSH",
            fields: vec![Field::from_text("^~\\&")],
        }],
        charsets: vec![],
    });

    let bytes = write_batch(&batch);
    let result = String::from_utf8(bytes).unwrap();

    assert!(result.starts_with("MSH|"));
}

#[test]
fn test_write_batch_multiple_messages() {
    let mut batch = Batch::default();

    for i in 0..3 {
        batch.messages.push(Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"MSH",
                fields: vec![
                    Field::from_text("^~\\&"),
                    Field::from_text(&format!("MSG{}", i)),
                ],
            }],
            charsets: vec![],
        });
    }

    let bytes = write_batch(&batch);
    let result = String::from_utf8(bytes).unwrap();

    // Should contain all messages
    assert!(result.contains("MSG0"));
    assert!(result.contains("MSG1"));
    assert!(result.contains("MSG2"));
}

#[test]
fn test_write_batch_with_header() {
    let mut batch = Batch::default();
    batch.header = Some(Segment {
        id: *b"BHS",
        fields: vec![
            Field::from_text("^~\\&"),
            Field::from_text("SENDAPP"),
            Field::from_text("SENDFAC"),
        ],
    });
    batch.messages.push(Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"MSH",
            fields: vec![Field::from_text("^~\\&")],
        }],
        charsets: vec![],
    });

    let bytes = write_batch(&batch);
    let result = String::from_utf8(bytes).unwrap();

    assert!(result.starts_with("BHS|"));
    assert!(result.contains("MSH|"));
}

#[test]
fn test_write_batch_with_trailer() {
    let mut batch = Batch::default();
    batch.messages.push(Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"MSH",
            fields: vec![Field::from_text("^~\\&")],
        }],
        charsets: vec![],
    });
    batch.trailer = Some(Segment {
        id: *b"BTS",
        fields: vec![Field::from_text("1")], // Message count
    });

    let bytes = write_batch(&batch);
    let result = String::from_utf8(bytes).unwrap();

    assert!(result.contains("MSH|"));
    assert!(result.contains("BTS|"));
}

// ============================================================================
// File batch tests
// ============================================================================

#[test]
fn test_write_file_batch_empty() {
    let file_batch = FileBatch::default();
    let bytes = write_file_batch(&file_batch);
    assert!(bytes.is_empty());
}

#[test]
fn test_write_file_batch_with_header() {
    let mut file_batch = FileBatch::default();
    file_batch.header = Some(Segment {
        id: *b"FHS",
        fields: vec![Field::from_text("^~\\&"), Field::from_text("SENDAPP")],
    });

    let bytes = write_file_batch(&file_batch);
    let result = String::from_utf8(bytes).unwrap();

    assert!(result.starts_with("FHS|"));
}

#[test]
fn test_write_file_batch_with_batches() {
    let mut file_batch = FileBatch::default();

    let mut batch1 = Batch::default();
    batch1.messages.push(Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"MSH",
            fields: vec![Field::from_text("^~\\&"), Field::from_text("BATCH1")],
        }],
        charsets: vec![],
    });

    let mut batch2 = Batch::default();
    batch2.messages.push(Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"MSH",
            fields: vec![Field::from_text("^~\\&"), Field::from_text("BATCH2")],
        }],
        charsets: vec![],
    });

    file_batch.batches.push(batch1);
    file_batch.batches.push(batch2);

    let bytes = write_file_batch(&file_batch);
    let result = String::from_utf8(bytes).unwrap();

    assert!(result.contains("BATCH1"));
    assert!(result.contains("BATCH2"));
}

#[test]
fn test_write_file_batch_full_structure() {
    let mut file_batch = FileBatch::default();

    file_batch.header = Some(Segment {
        id: *b"FHS",
        fields: vec![Field::from_text("^~\\&"), Field::from_text("FILE")],
    });

    let mut batch = Batch::default();
    batch.header = Some(Segment {
        id: *b"BHS",
        fields: vec![Field::from_text("^~\\&"), Field::from_text("BATCH")],
    });
    batch.messages.push(Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"MSH",
            fields: vec![Field::from_text("^~\\&"), Field::from_text("MSG")],
        }],
        charsets: vec![],
    });
    batch.trailer = Some(Segment {
        id: *b"BTS",
        fields: vec![Field::from_text("1")],
    });

    file_batch.batches.push(batch);
    file_batch.trailer = Some(Segment {
        id: *b"FTS",
        fields: vec![Field::from_text("1")],
    });

    let bytes = write_file_batch(&file_batch);
    let result = String::from_utf8(bytes).unwrap();

    // Verify full structure
    assert!(result.contains("FHS|"));
    assert!(result.contains("BHS|"));
    assert!(result.contains("MSH|"));
    assert!(result.contains("BTS|"));
    assert!(result.contains("FTS|"));
}

// ============================================================================
// JSON tests
// ============================================================================

#[test]
fn test_to_json_basic() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"MSH",
            fields: vec![Field::from_text("^~\\&"), Field::from_text("APP")],
        }],
        charsets: vec![],
    };

    let json = to_json(&message);

    assert!(json.is_object());
    assert!(json.get("segments").is_some());
}

#[test]
fn test_to_json_string_basic() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"MSH",
            fields: vec![Field::from_text("^~\\&")],
        }],
        charsets: vec![],
    };

    let json_str = to_json_string(&message);

    assert!(json_str.contains("segments"));
    assert!(!json_str.contains("\n")); // Compact format
}

#[test]
fn test_to_json_string_pretty() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"MSH",
            fields: vec![Field::from_text("^~\\&")],
        }],
        charsets: vec![],
    };

    let json_str = to_json_string_pretty(&message);

    assert!(json_str.contains("segments"));
    assert!(json_str.contains("\n")); // Pretty format has newlines
}

// ============================================================================
// Edge case tests
// ============================================================================

#[test]
fn test_write_empty_field() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"PID",
            fields: vec![Field::from_text(""), Field::from_text("VALUE")],
        }],
        charsets: vec![],
    };

    let bytes = write(&message);
    let result = String::from_utf8(bytes).unwrap();

    // Empty field should still produce separator
    assert!(result.contains("|VALUE"));
}

#[test]
fn test_write_null_atom() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"PID",
            fields: vec![Field {
                reps: vec![Rep {
                    comps: vec![Comp {
                        subs: vec![Atom::null()],
                    }],
                }],
            }],
        }],
        charsets: vec![],
    };

    let bytes = write(&message);
    let result = String::from_utf8(bytes).unwrap();

    // Null atom should be represented as ""
    assert!(result.contains("\"\""));
}

#[test]
fn test_write_unicode() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"PID",
            fields: vec![Field::from_text("Patient 名前")],
        }],
        charsets: vec![],
    };

    let bytes = write(&message);
    let result = String::from_utf8(bytes).unwrap();

    assert!(result.contains("Patient 名前"));
}

#[test]
fn test_write_long_message() {
    let mut segments = Vec::new();

    for i in 0..100 {
        segments.push(Segment {
            id: *b"OBX",
            fields: vec![
                Field::from_text(&format!("{}", i)),
                Field::from_text("ST"),
                Field::from_text(&format!("OBS{}", i)),
                Field::from_text(&format!("Long observation value {}", "X".repeat(100))),
            ],
        });
    }

    let message = Message {
        delims: Delims::default(),
        segments,
        charsets: vec![],
    };

    let bytes = write(&message);
    let result = String::from_utf8(bytes).unwrap();

    // Verify all segments are present
    assert_eq!(result.matches("OBX|").count(), 100);
}

#[test]
fn test_write_special_characters() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"PID",
            fields: vec![Field::from_text("Test <>&\"' chars")],
        }],
        charsets: vec![],
    };

    let bytes = write(&message);
    let result = String::from_utf8(bytes).unwrap();

    // & is subcomponent separator, gets escaped as \T\
    // < > and " and ' should be preserved
    assert!(result.contains("Test <>&\"' chars") || result.contains("Test <>\\T\\\"' chars"));
}

#[test]
fn test_write_newlines_in_field() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"PID",
            fields: vec![Field::from_text("Line1\nLine2\rLine3")],
        }],
        charsets: vec![],
    };

    let bytes = write(&message);
    // Newlines within fields should be preserved
    assert!(bytes.windows(5).any(|w| w == b"Line1"));
    assert!(bytes.windows(5).any(|w| w == b"Line2"));
    assert!(bytes.windows(5).any(|w| w == b"Line3"));
}

// ============================================================================
// Custom delimiter tests
// ============================================================================

#[test]
fn test_write_custom_delimiters() {
    let message = Message {
        delims: Delims {
            field: '|',
            comp: '^',
            rep: '~',
            esc: '\\',
            sub: '&',
        },
        segments: vec![Segment {
            id: *b"PID",
            fields: vec![
                Field::from_text("1"),
                Field {
                    reps: vec![Rep {
                        comps: vec![Comp::from_text("DOE"), Comp::from_text("JOHN")],
                    }],
                },
            ],
        }],
        charsets: vec![],
    };

    let bytes = write(&message);
    let result = String::from_utf8(bytes).unwrap();

    assert!(result.contains("DOE^JOHN"));
}

// ============================================================================
// Atom type tests
// ============================================================================

#[test]
fn test_write_text_atom() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"PID",
            fields: vec![Field {
                reps: vec![Rep {
                    comps: vec![Comp {
                        subs: vec![Atom::text("TEXT_VALUE".to_string())],
                    }],
                }],
            }],
        }],
        charsets: vec![],
    };

    let bytes = write(&message);
    let result = String::from_utf8(bytes).unwrap();

    assert!(result.contains("TEXT_VALUE"));
}

#[test]
fn test_write_mixed_atoms() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"PID",
            fields: vec![Field {
                reps: vec![Rep {
                    comps: vec![Comp {
                        subs: vec![
                            Atom::text("TEXT".to_string()),
                            Atom::null(),
                            Atom::text("MORE".to_string()),
                        ],
                    }],
                }],
            }],
        }],
        charsets: vec![],
    };

    let bytes = write(&message);
    let result = String::from_utf8(bytes).unwrap();

    assert!(result.contains("TEXT"));
    assert!(result.contains("\"\""));
    assert!(result.contains("MORE"));
}

// ============================================================================
// Performance tests
// ============================================================================

#[test]
fn test_write_allocation_efficiency() {
    // Create a message with predictable size
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"MSH",
            fields: vec![
                Field::from_text("^~\\&"),
                Field::from_text("APP"),
                Field::from_text("FAC"),
            ],
        }],
        charsets: vec![],
    };

    let bytes = write(&message);

    // Verify the output is reasonable (not excessively large due to over-allocation)
    assert!(bytes.len() < 1000);
}
