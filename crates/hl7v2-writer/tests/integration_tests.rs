//! Integration tests for hl7v2-writer crate
//!
//! These tests verify the writer works correctly with real-world HL7 messages
//! and integrates properly with the parser for roundtrip testing.

use hl7v2_model::{Atom, Batch, Comp, Delims, Field, FileBatch, Message, Rep, Segment};
use hl7v2_parser::parse;
use hl7v2_writer::{
    to_json, to_json_string, to_json_string_pretty, write, write_batch, write_file_batch,
    write_mllp,
};

// ============================================================================
// Real-world message tests
// ============================================================================

#[test]
fn test_adt_a01_message() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![
            Segment {
                id: *b"MSH",
                fields: vec![
                    Field::from_text("^~\\&"),
                    Field::from_text("ADT"),
                    Field::from_text("HOSPITAL"),
                    Field::from_text("ADT"),
                    Field::from_text("CLINIC"),
                    Field::from_text("20231225120000"),
                    Field::from_text(""),
                    Field::from_text("ADT^A01"),
                    Field::from_text("MSG00001"),
                    Field::from_text("P"),
                    Field::from_text("2.5"),
                ],
            },
            Segment {
                id: *b"EVN",
                fields: vec![Field::from_text("A01"), Field::from_text("20231225120000")],
            },
            Segment {
                id: *b"PID",
                fields: vec![
                    Field::from_text("1"),
                    Field::from_text("12345"),
                    Field::from_text("DOE^JOHN^A"),
                    Field::from_text(""),
                    Field::from_text("19800101"),
                    Field::from_text("M"),
                ],
            },
            Segment {
                id: *b"PV1",
                fields: vec![
                    Field::from_text("1"),
                    Field::from_text("I"),
                    Field::from_text("WARD^ROOM^BED"),
                ],
            },
        ],
        charsets: vec![],
    };

    let bytes = write(&message);
    let result = String::from_utf8(bytes).unwrap();

    // Verify structure
    assert!(result.starts_with("MSH|^~\\&"));
    assert!(result.contains("EVN|"));
    assert!(result.contains("PID|"));
    assert!(result.contains("PV1|"));

    // Verify segment count
    assert_eq!(result.matches("\r").count(), 4);
}

#[test]
fn test_oru_r01_message() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![
            Segment {
                id: *b"MSH",
                fields: vec![
                    Field::from_text("^~\\&"),
                    Field::from_text("LAB"),
                    Field::from_text("HOSPITAL"),
                    Field::from_text("LIS"),
                    Field::from_text("LAB"),
                    Field::from_text("20231225120000"),
                    Field::from_text(""),
                    Field::from_text("ORU^R01"),
                    Field::from_text("MSG00002"),
                    Field::from_text("P"),
                    Field::from_text("2.5"),
                ],
            },
            Segment {
                id: *b"PID",
                fields: vec![
                    Field::from_text("1"),
                    Field::from_text("12345"),
                    Field::from_text("DOE^JOHN"),
                ],
            },
            Segment {
                id: *b"ORC",
                fields: vec![Field::from_text("RE"), Field::from_text("ORDER123")],
            },
            Segment {
                id: *b"OBR",
                fields: vec![
                    Field::from_text("1"),
                    Field::from_text("ORDER123"),
                    Field::from_text(""),
                    Field::from_text("CBC^COMPLETE BLOOD COUNT"),
                ],
            },
            Segment {
                id: *b"OBX",
                fields: vec![
                    Field::from_text("1"),
                    Field::from_text("NM"),
                    Field::from_text("WBC^WHITE BLOOD COUNT"),
                    Field::from_text(""),
                    Field::from_text("7.5"),
                    Field::from_text("10*3/uL"),
                    Field::from_text("4.0-11.0"),
                    Field::from_text("N"),
                ],
            },
        ],
        charsets: vec![],
    };

    let bytes = write(&message);
    let result = String::from_utf8(bytes).unwrap();

    assert!(result.contains("ORU\\S\\R01")); // ^ escaped
    assert!(result.contains("OBX|"));
    assert!(result.contains("WBC"));
}

#[test]
fn test_ack_message() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![
            Segment {
                id: *b"MSH",
                fields: vec![
                    Field::from_text("^~\\&"),
                    Field::from_text("RECV"),
                    Field::from_text("RECVFAC"),
                    Field::from_text("SEND"),
                    Field::from_text("SENDFAC"),
                    Field::from_text("20231225120000"),
                    Field::from_text(""),
                    Field::from_text("ACK^A01"),
                    Field::from_text("ACK001"),
                    Field::from_text("P"),
                    Field::from_text("2.5"),
                ],
            },
            Segment {
                id: *b"MSA",
                fields: vec![
                    Field::from_text("AA"),
                    Field::from_text("MSG00001"),
                    Field::from_text("Message accepted"),
                ],
            },
        ],
        charsets: vec![],
    };

    let bytes = write(&message);
    let result = String::from_utf8(bytes).unwrap();

    assert!(result.contains("ACK\\S\\A01"));
    assert!(result.contains("MSA|AA"));
}

// ============================================================================
// Roundtrip tests
// ============================================================================

#[test]
fn test_roundtrip_simple_message() {
    // Use \r as segment terminator (HL7 standard)
    let original = "MSH|^~\\&|SEND|SENDFAC|RECV|RECVFAC|20231225120000||ADT^A01|MSG001|P|2.5\rPID|1||12345^^^HOSPITAL^MR||DOE^JOHN^A||19800101|M\rPV1|1|I|WARD^ROOM^BED";

    // Parse and re-write
    let parsed = parse(original.as_bytes()).unwrap();
    let rewritten = write(&parsed);
    let result = String::from_utf8(rewritten).unwrap();

    // Parse again and compare structure
    let reparsed = parse(result.as_bytes()).unwrap();

    assert_eq!(parsed.segments.len(), reparsed.segments.len());
    assert_eq!(parsed.segments[0].id, reparsed.segments[0].id);
    assert_eq!(parsed.segments[1].id, reparsed.segments[1].id);
    assert_eq!(parsed.segments[2].id, reparsed.segments[2].id);
}

#[test]
fn test_roundtrip_complex_message() {
    // Use \r as segment terminator (HL7 standard)
    let original = "MSH|^~\\&|LAB|HOSPITAL|LIS|LAB|20231225120000||ORU^R01|MSG002|P|2.5\rPID|1||12345||DOE^JOHN||19800101|M\rOBX|1|NM|WBC||7.5|10*3/uL|4.0-11.0|N|||F\rOBX|2|NM|RBC||4.5|10*6/uL|4.0-5.5|N|||F";

    let parsed = parse(original.as_bytes()).unwrap();
    let rewritten = write(&parsed);
    let reparsed = parse(&rewritten).unwrap();

    assert_eq!(parsed.segments.len(), reparsed.segments.len());
}

#[test]
fn test_roundtrip_with_escaping() {
    // Use \r as segment terminator (HL7 standard)
    let original = "MSH|^~\\&|APP|FAC|||20231225120000||ADT^A01|MSG003|P|2.5\rPID|1||12345||DOE^JOHN||19800101|M\rNTE|1||This is a \\F\\pipe\\F\\ character";

    let parsed = parse(original.as_bytes()).unwrap();
    let rewritten = write(&parsed);
    let result = String::from_utf8(rewritten.clone()).unwrap();

    // The escaped pipe should be preserved
    assert!(result.contains("\\F\\"));

    let reparsed = parse(&rewritten).unwrap();
    assert_eq!(parsed.segments.len(), reparsed.segments.len());
}

// ============================================================================
// MLLP integration tests
// ============================================================================

#[test]
fn test_mllp_with_real_message() {
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

    let framed = write_mllp(&message);

    // Verify MLLP structure
    assert_eq!(framed[0], 0x0B); // SB
    assert_eq!(framed[framed.len() - 2], 0x1C); // EB1
    assert_eq!(framed[framed.len() - 1], 0x0D); // EB2

    // Extract and verify content
    let content = &framed[1..framed.len() - 2];
    assert!(content.starts_with(b"MSH|"));
}

#[test]
fn test_mllp_unwrap_roundtrip() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![
            Segment {
                id: *b"MSH",
                fields: vec![
                    Field::from_text("^~\\&"),
                    Field::from_text("APP"),
                    Field::from_text("FAC"),
                    Field::from_text("RECV"),
                    Field::from_text("RECVFAC"),
                ],
            },
            Segment {
                id: *b"PID",
                fields: vec![Field::from_text("1"), Field::from_text("12345")],
            },
        ],
        charsets: vec![],
    };

    let framed = write_mllp(&message);

    // Unwrap and verify
    let content = hl7v2_mllp::unwrap_mllp(&framed).unwrap();
    let parsed = parse(content).unwrap();

    assert_eq!(parsed.segments.len(), 2);
}

// ============================================================================
// Batch integration tests
// ============================================================================

#[test]
fn test_batch_with_multiple_messages() {
    let mut batch = Batch::default();

    for i in 0..3 {
        batch.messages.push(Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"MSH",
                fields: vec![
                    Field::from_text("^~\\&"),
                    Field::from_text(format!("APP{}", i)),
                    Field::from_text("FAC"),
                    Field::from_text(""),
                    Field::from_text(""),
                    Field::from_text("20231225120000"),
                    Field::from_text(""),
                    Field::from_text("ADT^A01"),
                    Field::from_text(format!("MSG{:03}", i)),
                    Field::from_text("P"),
                    Field::from_text("2.5"),
                ],
            }],
            charsets: vec![],
        });
    }

    let bytes = write_batch(&batch);
    let result = String::from_utf8(bytes).unwrap();

    // Should have 3 messages
    assert_eq!(result.matches("MSH|").count(), 3);
    assert!(result.contains("MSG000"));
    assert!(result.contains("MSG001"));
    assert!(result.contains("MSG002"));
}

#[test]
#[allow(clippy::field_reassign_with_default)]
fn test_file_batch_structure() {
    let mut file_batch = FileBatch::default();

    file_batch.header = Some(Segment {
        id: *b"FHS",
        fields: vec![
            Field::from_text("^~\\&"),
            Field::from_text("FILEAPP"),
            Field::from_text("FILEFAC"),
        ],
    });

    #[allow(clippy::field_reassign_with_default)]
    let mut batch = Batch::default();
    batch.header = Some(Segment {
        id: *b"BHS",
        fields: vec![
            Field::from_text("^~\\&"),
            Field::from_text("BATCHAPP"),
            Field::from_text("BATCHFAC"),
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

    assert!(result.starts_with("FHS|"));
    assert!(result.contains("BHS|"));
    assert!(result.contains("MSH|"));
    assert!(result.contains("BTS|"));
    assert!(result.contains("FTS|"));
}

// ============================================================================
// JSON integration tests
// ============================================================================

#[test]
fn test_json_output_structure() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![
            Segment {
                id: *b"MSH",
                fields: vec![Field::from_text("^~\\&"), Field::from_text("APP")],
            },
            Segment {
                id: *b"PID",
                fields: vec![Field::from_text("1"), Field::from_text("12345")],
            },
        ],
        charsets: vec![],
    };

    let json = to_json(&message);

    assert!(json.is_object());

    let segments = json.get("segments").unwrap().as_array().unwrap();
    assert_eq!(segments.len(), 2);

    // Check MSH segment
    let msh = &segments[0];
    assert_eq!(msh.get("id").unwrap().as_str().unwrap(), "MSH");

    // Check PID segment
    let pid = &segments[1];
    assert_eq!(pid.get("id").unwrap().as_str().unwrap(), "PID");
}

#[test]
fn test_json_string_compact() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"MSH",
            fields: vec![Field::from_text("^~\\&")],
        }],
        charsets: vec![],
    };

    let json = to_json_string(&message);

    // Should be valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed.is_object());

    // Should be compact (single line)
    assert!(!json.contains('\n'));
}

#[test]
fn test_json_string_pretty() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"MSH",
            fields: vec![Field::from_text("^~\\&")],
        }],
        charsets: vec![],
    };

    let json = to_json_string_pretty(&message);

    // Should be valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed.is_object());

    // Should be pretty (multiple lines)
    assert!(json.contains('\n'));
}

// ============================================================================
// Edge case integration tests
// ============================================================================

#[test]
fn test_empty_message_handling() {
    let message = Message::new();
    let _bytes = write(&message);

    // Empty message should produce some output
    // (exact behavior depends on implementation)
}

#[test]
fn test_message_with_empty_fields() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"PID",
            fields: vec![
                Field::from_text("1"),
                Field::from_text(""),
                Field::from_text(""),
                Field::from_text("VALUE"),
            ],
        }],
        charsets: vec![],
    };

    let bytes = write(&message);
    let result = String::from_utf8(bytes).unwrap();

    // Should preserve field positions with separators
    assert!(result.contains("PID|1|||VALUE"));
}

#[test]
fn test_message_with_unicode() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![
            Segment {
                id: *b"PID",
                fields: vec![
                    Field::from_text("1"),
                    Field::from_text("患者名前"), // Japanese
                ],
            },
            Segment {
                id: *b"NTE",
                fields: vec![
                    Field::from_text("1"),
                    Field::from_text("注释"), // Chinese
                ],
            },
        ],
        charsets: vec![],
    };

    let bytes = write(&message);
    let result = String::from_utf8(bytes).unwrap();

    assert!(result.contains("患者名前"));
    assert!(result.contains("注释"));
}

#[test]
fn test_very_long_message() {
    let mut segments = Vec::new();

    // Add MSH segment first
    segments.push(Segment {
        id: *b"MSH",
        fields: vec![
            Field::from_text("^~\\&"),
            Field::from_text("APP"),
            Field::from_text("FAC"),
        ],
    });

    // Create a message with many segments
    for i in 0..50 {
        segments.push(Segment {
            id: *b"OBX",
            fields: vec![
                Field::from_text(format!("{}", i + 1)),
                Field::from_text("ST"),
                Field::from_text(format!("OBS_{:04}", i)),
                Field::from_text(format!(
                    "Observation value {} with some additional text to make it longer",
                    i
                )),
            ],
        });
    }

    let message = Message {
        delims: Delims::default(),
        segments,
        charsets: vec![],
    };

    let bytes = write(&message);
    let result = String::from_utf8(bytes.clone()).unwrap();

    // Verify all segments present
    assert_eq!(result.matches("OBX|").count(), 50);

    // Parse it back
    let parsed = parse(&bytes).unwrap();
    assert_eq!(parsed.segments.len(), 51); // MSH + 50 OBX
}

// ============================================================================
// Parser compatibility tests
// ============================================================================

#[test]
fn test_parser_compatibility_simple() {
    // Create a message that should be parseable
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
    let parsed = parse(&bytes).unwrap();

    assert_eq!(parsed.segments.len(), 1);
    assert_eq!(parsed.segments[0].id, *b"MSH");
}

#[test]
fn test_parser_compatibility_complex() {
    // Create a complex message with all field types
    let message = Message {
        delims: Delims::default(),
        segments: vec![
            Segment {
                id: *b"MSH",
                fields: vec![Field::from_text("^~\\&"), Field::from_text("APP")],
            },
            Segment {
                id: *b"PID",
                fields: vec![
                    Field::from_text("1"),
                    Field {
                        reps: vec![
                            Rep {
                                comps: vec![Comp {
                                    subs: vec![
                                        Atom::text("DOE".to_string()),
                                        Atom::text("JOHN".to_string()),
                                    ],
                                }],
                            },
                            Rep {
                                comps: vec![Comp {
                                    subs: vec![Atom::text("SMITH".to_string())],
                                }],
                            },
                        ],
                    },
                ],
            },
        ],
        charsets: vec![],
    };

    let bytes = write(&message);
    let parsed = parse(&bytes).unwrap();

    assert_eq!(parsed.segments.len(), 2);
}

// ============================================================================
// Performance tests
// ============================================================================

#[test]
fn test_large_batch_performance() {
    let mut batch = Batch::default();

    // Create 100 messages
    for i in 0..100 {
        batch.messages.push(Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"MSH",
                fields: vec![
                    Field::from_text("^~\\&"),
                    Field::from_text(format!("APP{}", i)),
                    Field::from_text("FAC"),
                    Field::from_text(""),
                    Field::from_text(""),
                    Field::from_text("20231225120000"),
                    Field::from_text(""),
                    Field::from_text("ADT^A01"),
                    Field::from_text(format!("MSG{:04}", i)),
                ],
            }],
            charsets: vec![],
        });
    }

    // Should complete in reasonable time
    let bytes = write_batch(&batch);
    // 100 messages * ~90 bytes each = ~9000 bytes minimum
    assert!(bytes.len() > 5000); // Should have substantial content
}
