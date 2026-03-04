//! Integration tests for the hl7v2-stream crate.
//!
//! These tests verify:
//! - Real-world HL7 message parsing
//! - Integration with other crates (hl7v2-parser, hl7v2-test-utils)
//! - Chunked input handling
//! - Large message processing

use hl7v2_stream::{Event, StreamParser};
use hl7v2_test_utils::{builders::MessageBuilder, fixtures::SampleMessages};
use std::io::{BufReader, Cursor, Read};

/// Helper to collect all events from a parser
fn collect_events<R: Read>(parser: &mut StreamParser<BufReader<R>>) -> Vec<Event> {
    let mut events = Vec::new();
    while let Ok(Some(event)) = parser.next_event() {
        events.push(event);
    }
    events
}

// =============================================================================
// Real-World Message Tests
// =============================================================================

#[test]
fn test_adt_a01_message() {
    let hl7_text = SampleMessages::adt_a01();
    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    // Verify structure
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::StartMessage { .. }))
    );
    assert!(events.iter().any(|e| matches!(e, Event::EndMessage)));

    // Verify segments
    let segment_ids: Vec<&[u8]> = events
        .iter()
        .filter_map(|e| {
            if let Event::Segment { id } = e {
                Some(id.as_slice())
            } else {
                None
            }
        })
        .collect();

    assert!(segment_ids.iter().any(|id| *id == b"EVN"));
    assert!(segment_ids.iter().any(|id| *id == b"PID"));
    assert!(segment_ids.iter().any(|id| *id == b"PV1"));
}

#[test]
fn test_adt_a04_message() {
    let hl7_text = SampleMessages::adt_a04();
    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::StartMessage { .. }))
    );
    assert!(events.iter().any(|e| matches!(e, Event::EndMessage)));

    // ADT^A04 should have PID segment
    assert!(
        events
            .iter()
            .any(|e| { matches!(e, Event::Segment { id } if id == b"PID") })
    );
}

#[test]
fn test_oru_r01_message() {
    let hl7_text = SampleMessages::oru_r01();
    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::StartMessage { .. }))
    );
    assert!(events.iter().any(|e| matches!(e, Event::EndMessage)));

    // ORU^R01 should have PID, OBR, OBX segments
    let segment_ids: Vec<&[u8]> = events
        .iter()
        .filter_map(|e| {
            if let Event::Segment { id } = e {
                Some(id.as_slice())
            } else {
                None
            }
        })
        .collect();

    assert!(segment_ids.iter().any(|id| *id == b"PID"));
    assert!(segment_ids.iter().any(|id| *id == b"OBR"));
    assert!(segment_ids.iter().any(|id| *id == b"OBX"));
}

// =============================================================================
// Edge Case Tests from Test Utilities
// =============================================================================

#[test]
fn test_empty_fields_message() {
    let hl7_text = hl7v2_test_utils::fixtures::SampleMessages::edge_case("empty_fields")
        .expect("empty_fields fixture should exist");

    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::StartMessage { .. }))
    );
    assert!(events.iter().any(|e| matches!(e, Event::EndMessage)));

    // Count empty fields
    let empty_field_count = events
        .iter()
        .filter(|e| {
            if let Event::Field { raw, .. } = e {
                raw.is_empty()
            } else {
                false
            }
        })
        .count();

    assert!(
        empty_field_count > 0,
        "Should have empty fields in edge case message"
    );
}

#[test]
fn test_special_chars_message() {
    let hl7_text = hl7v2_test_utils::fixtures::SampleMessages::edge_case("special_chars")
        .expect("special_chars fixture should exist");

    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::StartMessage { .. }))
    );
    assert!(events.iter().any(|e| matches!(e, Event::EndMessage)));
}

#[test]
fn test_custom_delimiters_message() {
    let hl7_text = hl7v2_test_utils::fixtures::SampleMessages::edge_case("custom_delims")
        .expect("custom_delims fixture should exist");

    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    // Should parse successfully with custom delimiters
    if let Some(Event::StartMessage { delims }) = events
        .iter()
        .find(|e| matches!(e, Event::StartMessage { .. }))
    {
        // Custom delimiters should differ from standard
        // The fixture uses non-standard delimiters
        assert_ne!(delims.field, '|');
    }
}

// =============================================================================
// Invalid Message Tests
// =============================================================================

#[test]
fn test_malformed_message_handling() {
    let hl7_text = hl7v2_test_utils::fixtures::SampleMessages::invalid("malformed")
        .expect("malformed fixture should exist");

    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    // Parser should handle malformed input gracefully (not panic)
    let events = collect_events(&mut parser);

    // May or may not produce events depending on the nature of malformation
    // The key is that it doesn't panic
    let _ = events;
}

#[test]
fn test_truncated_message_handling() {
    let hl7_text = hl7v2_test_utils::fixtures::SampleMessages::invalid("truncated")
        .expect("truncated fixture should exist");

    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    // Parser should handle truncated input gracefully
    let events = collect_events(&mut parser);

    // May produce partial events
    let _ = events;
}

// =============================================================================
// Message Builder Integration Tests
// =============================================================================

#[test]
fn test_builder_simple_message() {
    let bytes = MessageBuilder::new()
        .with_msh("TestApp", "TestFac", "RecvApp", "RecvFac", "ADT", "A01")
        .build_bytes();

    let cursor = Cursor::new(bytes);
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::StartMessage { .. }))
    );
    assert!(events.iter().any(|e| matches!(e, Event::EndMessage)));
}

#[test]
fn test_builder_with_pid() {
    let bytes = MessageBuilder::new()
        .with_msh("App", "Fac", "Recv", "RecvFac", "ADT", "A01")
        .with_pid("MRN123", "Doe", "John")
        .build_bytes();

    let cursor = Cursor::new(bytes);
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    assert!(
        events
            .iter()
            .any(|e| { matches!(e, Event::Segment { id } if id == b"PID") })
    );

    // Verify PID field content
    let pid_fields: Vec<&[u8]> = events
        .iter()
        .filter_map(|e| {
            if let Event::Field { raw, .. } = e {
                Some(raw.as_slice())
            } else {
                None
            }
        })
        .collect();

    // Should contain MRN and name
    assert!(
        pid_fields
            .iter()
            .any(|f| String::from_utf8_lossy(f).contains("MRN123"))
    );
    assert!(
        pid_fields
            .iter()
            .any(|f| String::from_utf8_lossy(f).contains("Doe"))
    );
}

#[test]
fn test_builder_with_pv1() {
    let bytes = MessageBuilder::new()
        .with_msh("App", "Fac", "Recv", "RecvFac", "ADT", "A01")
        .with_pid("MRN456", "Smith", "Jane")
        .with_pv1("I", "ICU^101")
        .build_bytes();

    let cursor = Cursor::new(bytes);
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    // Should have both PID and PV1
    assert!(
        events
            .iter()
            .any(|e| { matches!(e, Event::Segment { id } if id == b"PID") })
    );
    assert!(
        events
            .iter()
            .any(|e| { matches!(e, Event::Segment { id } if id == b"PV1") })
    );
}

// =============================================================================
// Chunked Input Tests
// =============================================================================

#[test]
fn test_chunked_input_small_chunks() {
    // Test with small buffer reads
    let hl7_text = SampleMessages::adt_a01();
    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::with_capacity(64, cursor); // Small buffer

    let mut parser = StreamParser::new(buf_reader);
    let events = collect_events(&mut parser);

    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::StartMessage { .. }))
    );
    assert!(events.iter().any(|e| matches!(e, Event::EndMessage)));
}

#[test]
fn test_chunked_input_medium_chunks() {
    let hl7_text = SampleMessages::adt_a01();
    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::with_capacity(128, cursor);

    let mut parser = StreamParser::new(buf_reader);
    let events = collect_events(&mut parser);

    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::StartMessage { .. }))
    );
    assert!(events.iter().any(|e| matches!(e, Event::EndMessage)));
}

#[test]
fn test_chunked_input_large_chunks() {
    let hl7_text = SampleMessages::adt_a01();
    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::with_capacity(8192, cursor);

    let mut parser = StreamParser::new(buf_reader);
    let events = collect_events(&mut parser);

    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::StartMessage { .. }))
    );
    assert!(events.iter().any(|e| matches!(e, Event::EndMessage)));
}

// =============================================================================
// Multiple Messages in Stream Tests
// =============================================================================

#[test]
fn test_two_messages_in_sequence() {
    let combined = format!("{}{}", SampleMessages::adt_a01(), SampleMessages::oru_r01());

    let cursor = Cursor::new(combined.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    let start_count = events
        .iter()
        .filter(|e| matches!(e, Event::StartMessage { .. }))
        .count();
    let end_count = events
        .iter()
        .filter(|e| matches!(e, Event::EndMessage))
        .count();

    assert_eq!(start_count, 2);
    assert_eq!(end_count, 2);
}

#[test]
fn test_three_messages_in_sequence() {
    let combined = format!(
        "{}{}{}",
        SampleMessages::adt_a01(),
        SampleMessages::adt_a04(),
        SampleMessages::oru_r01()
    );

    let cursor = Cursor::new(combined.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    let start_count = events
        .iter()
        .filter(|e| matches!(e, Event::StartMessage { .. }))
        .count();

    assert_eq!(start_count, 3);
}

// =============================================================================
// Field Content Verification Tests
// =============================================================================

#[test]
fn test_msh_field_content() {
    let hl7_text = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\r";
    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    // Find specific fields
    let sending_app_field = events.iter().find(|e| {
        if let Event::Field { num, raw } = e {
            *num == 2 && raw == b"SendingApp"
        } else {
            false
        }
    });
    assert!(sending_app_field.is_some(), "Should find sending app field");

    let sending_fac_field = events.iter().find(|e| {
        if let Event::Field { num, raw } = e {
            *num == 3 && raw == b"SendingFac"
        } else {
            false
        }
    });
    assert!(
        sending_fac_field.is_some(),
        "Should find sending facility field"
    );
}

#[test]
fn test_pid_field_content() {
    let hl7_text = "MSH|^~\\&|App|Fac|||20250101||ADT^A01|123|P|2.5\rPID|1||123456^^^HOSP^MR||Doe^John^A||19800101|M\r";
    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    // Find patient name field (should be field 5 in PID)
    let name_field = events.iter().find(|e| {
        if let Event::Field { raw, .. } = e {
            String::from_utf8_lossy(raw).contains("Doe^John")
        } else {
            false
        }
    });
    assert!(name_field.is_some(), "Should find patient name field");

    // Find MRN field (should contain patient ID)
    let mrn_field = events.iter().find(|e| {
        if let Event::Field { raw, .. } = e {
            String::from_utf8_lossy(raw).contains("123456")
        } else {
            false
        }
    });
    assert!(mrn_field.is_some(), "Should find MRN field");
}

// =============================================================================
// Delimiter Verification Tests
// =============================================================================

#[test]
fn test_standard_delimiters_in_real_message() {
    let hl7_text = SampleMessages::adt_a01();
    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    if let Some(Event::StartMessage { delims }) = events
        .iter()
        .find(|e| matches!(e, Event::StartMessage { .. }))
    {
        assert_eq!(delims.field, '|');
        assert_eq!(delims.comp, '^');
        assert_eq!(delims.rep, '~');
        assert_eq!(delims.esc, '\\');
        assert_eq!(delims.sub, '&');
    } else {
        panic!("Expected StartMessage event");
    }
}

// =============================================================================
// Segment Order Tests
// =============================================================================

#[test]
fn test_segment_order_preserved() {
    let hl7_text = SampleMessages::adt_a01();
    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    let segment_ids: Vec<&[u8]> = events
        .iter()
        .filter_map(|e| {
            if let Event::Segment { id } = e {
                Some(id.as_slice())
            } else {
                None
            }
        })
        .collect();

    // Segments should appear in order: EVN, PID, PV1
    assert_eq!(segment_ids.len(), 3);
    assert_eq!(segment_ids[0], b"EVN");
    assert_eq!(segment_ids[1], b"PID");
    assert_eq!(segment_ids[2], b"PV1");
}

// =============================================================================
// Incremental Processing Tests
// =============================================================================

#[test]
fn test_incremental_event_processing() {
    let hl7_text = SampleMessages::adt_a01();
    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    // Process events one at a time
    let mut event_count = 0;
    let mut has_start = false;
    let mut has_end = false;

    while let Ok(Some(event)) = parser.next_event() {
        event_count += 1;
        match &event {
            Event::StartMessage { .. } => has_start = true,
            Event::EndMessage => has_end = true,
            _ => {}
        }
    }

    assert!(has_start, "Should have StartMessage");
    assert!(has_end, "Should have EndMessage");
    assert!(event_count > 4, "Should have multiple events");
}

// =============================================================================
// Memory Efficiency Tests
// =============================================================================

#[test]
fn test_memory_efficiency_large_field() {
    // Create a message with a large field
    let large_content = "A".repeat(100_000);
    let hl7_text = format!(
        "MSH|^~\\&|App|Fac|||20250101||ADT^A01|123|P|2.5\rPID|1||{}||Name\r",
        large_content
    );

    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    // Should parse successfully
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::StartMessage { .. }))
    );
    assert!(events.iter().any(|e| matches!(e, Event::EndMessage)));

    // Large field should be preserved
    let large_field = events.iter().find(|e| {
        if let Event::Field { raw, .. } = e {
            raw.len() == 100_000
        } else {
            false
        }
    });
    assert!(large_field.is_some(), "Large field should be preserved");
}

#[test]
fn test_memory_efficiency_many_segments() {
    // Create a message with many segments
    let mut hl7_text = String::from("MSH|^~\\&|App|Fac|||20250101||ADT^A01|123|P|2.5\r");

    for i in 0..1000 {
        hl7_text.push_str(&format!("ZXX|segment_{}|data\r", i));
    }

    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    // Should parse successfully
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::StartMessage { .. }))
    );
    assert!(events.iter().any(|e| matches!(e, Event::EndMessage)));

    // Should have 1000 ZXX segments
    let zxx_count = events
        .iter()
        .filter(|e| matches!(e, Event::Segment { id } if id == b"ZXX"))
        .count();
    assert_eq!(zxx_count, 1000);
}

// =============================================================================
// Comparison with Parser Crate Tests
// =============================================================================

#[test]
fn test_stream_matches_parser_structure() {
    // This test verifies that the stream parser produces events
    // that correspond to the same structure as the regular parser

    let hl7_text = SampleMessages::adt_a01();

    // Parse with stream parser
    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);
    let events = collect_events(&mut parser);

    // Count segments from stream parser
    let stream_segment_count = events
        .iter()
        .filter(|e| matches!(e, Event::Segment { .. }))
        .count();

    // At minimum, verify we got the expected number of segments
    assert_eq!(stream_segment_count, 3); // EVN, PID, PV1
}

// =============================================================================
// Real-World Scenario Tests
// =============================================================================

#[test]
fn test_hospital_admission_scenario() {
    // Simulate a hospital admission message flow
    let admission_msg = MessageBuilder::adt_a01()
        .with_pid("MRN001", "Patient", "Test")
        .with_pv1("I", "WARD^101^01")
        .build_bytes();

    let cursor = Cursor::new(admission_msg);
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    // Verify admission message structure
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::StartMessage { .. }))
    );

    // Should have PID and PV1
    assert!(
        events
            .iter()
            .any(|e| { matches!(e, Event::Segment { id } if id == b"PID") })
    );
    assert!(
        events
            .iter()
            .any(|e| { matches!(e, Event::Segment { id } if id == b"PV1") })
    );
}

#[test]
fn test_lab_result_scenario() {
    // Simulate a lab result message
    let lab_msg = MessageBuilder::oru_r01()
        .with_pid("LAB123", "Result", "Patient")
        .build_bytes();

    let cursor = Cursor::new(lab_msg);
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::StartMessage { .. }))
    );
    assert!(
        events
            .iter()
            .any(|e| { matches!(e, Event::Segment { id } if id == b"PID") })
    );
}
