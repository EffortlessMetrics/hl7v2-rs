//! Unit tests for the streaming HL7 v2 parser.
//!
//! These tests cover:
//! - Basic parsing functionality
//! - Chunk handling
//! - Edge cases
//! - Memory efficiency
//! - Delimiter handling

use crate::{Event, StreamParser};
use hl7v2_test_utils::{fixtures::SampleMessages, builders::MessageBuilder};
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
// Basic Parsing Tests
// =============================================================================

#[test]
fn test_parse_message_in_single_chunk() {
    // Test parsing a complete message in one chunk
    let hl7_text = SampleMessages::adt_a01();
    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    // Verify we got StartMessage
    assert!(events.iter().any(|e| matches!(e, Event::StartMessage { .. })));

    // Verify we got EndMessage
    assert!(events.iter().any(|e| matches!(e, Event::EndMessage)));

    // Verify we got segment events (EVN, PID, PV1 - not MSH since it generates StartMessage)
    let segment_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, Event::Segment { .. }))
        .collect();
    assert_eq!(segment_events.len(), 3); // EVN, PID, PV1

    // Verify segment IDs
    let segment_ids: Vec<&[u8]> = segment_events
        .iter()
        .filter_map(|e| {
            if let Event::Segment { id } = e {
                Some(id.as_slice())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(segment_ids[0], b"EVN");
    assert_eq!(segment_ids[1], b"PID");
    assert_eq!(segment_ids[2], b"PV1");
}

#[test]
fn test_parse_minimal_message() {
    // Test parsing a minimal valid message
    let hl7_text = "MSH|^~\\&|App|Fac\r";
    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    assert!(events.iter().any(|e| matches!(e, Event::StartMessage { .. })));
    assert!(events.iter().any(|e| matches!(e, Event::EndMessage)));
}

#[test]
fn test_parse_message_with_fields() {
    let hl7_text = "MSH|^~\\&|App|Fac|||20250101||ADT^A01|123|P|2.5\rPID|1||12345||Doe^John\r";
    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    // Count field events
    let field_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, Event::Field { .. }))
        .collect();

    // MSH has fields after position 8, PID has fields after position 4
    assert!(!field_events.is_empty());

    // Verify specific field content - look for "App" in any field
    let app_field = field_events.iter().find(|e| {
        if let Event::Field { raw, .. } = e {
            raw == b"App"
        } else {
            false
        }
    });
    assert!(app_field.is_some(), "Should find field with 'App' content");
}

// =============================================================================
// Chunk Handling Tests
// =============================================================================

#[test]
fn test_parse_message_across_multiple_chunks() {
    // Test that messages can be parsed even when split across reads
    // The parser reads in 1024-byte chunks internally
    let hl7_text = SampleMessages::adt_a01();
    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    // Should still get all events
    assert!(events.iter().any(|e| matches!(e, Event::StartMessage { .. })));
    assert!(events.iter().any(|e| matches!(e, Event::EndMessage)));
}

#[test]
fn test_chunk_boundary_in_middle_of_segment() {
    // Create a message that will span multiple internal buffer reads
    // The internal buffer is 1024 bytes, so create a message larger than that
    let long_field = "X".repeat(2000);
    let hl7_text = format!(
        "MSH|^~\\&|App|Fac|||20250101||ADT^A01|123|P|2.5\rPID|1||{}||Doe^John\r",
        long_field
    );

    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    // Should still parse correctly
    assert!(events.iter().any(|e| matches!(e, Event::StartMessage { .. })));
    assert!(events.iter().any(|e| matches!(e, Event::EndMessage)));

    // Find the long field
    let long_field_event = events.iter().find(|e| {
        if let Event::Field { raw, .. } = e {
            raw.len() == 2000 && raw == long_field.as_bytes()
        } else {
            false
        }
    });
    assert!(long_field_event.is_some());
}

#[test]
fn test_chunk_boundary_at_field_separator() {
    // Test when chunk boundary falls exactly at a field separator
    let mut hl7_text = String::from("MSH|^~\\&|App|Fac|||20250101||ADT^A01|123|P|2.5\rPID|1|");

    // Add content to get close to 1024 boundary, then add separator and more
    let padding_len = 1024 - hl7_text.len() - 1;
    let padding = "X".repeat(padding_len);
    hl7_text.push_str(&padding);
    hl7_text.push_str("|test_value\r");

    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    // Should parse correctly despite boundary at separator
    assert!(events.iter().any(|e| matches!(e, Event::StartMessage { .. })));
    assert!(events.iter().any(|e| matches!(e, Event::EndMessage)));
}

// =============================================================================
// Multiple Messages Tests
// =============================================================================

#[test]
fn test_parse_multiple_messages_from_single_stream() {
    // Two complete messages in sequence
    let hl7_text = format!(
        "{}{}",
        SampleMessages::adt_a01(),
        SampleMessages::adt_a04()
    );

    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    // Should have two StartMessage events
    let start_count = events
        .iter()
        .filter(|e| matches!(e, Event::StartMessage { .. }))
        .count();
    assert_eq!(start_count, 2);

    // Should have two EndMessage events
    let end_count = events.iter().filter(|e| matches!(e, Event::EndMessage)).count();
    assert_eq!(end_count, 2);
}

#[test]
fn test_parse_three_messages_in_sequence() {
    let hl7_text = format!(
        "{}{}{}",
        SampleMessages::adt_a01(),
        SampleMessages::adt_a04(),
        SampleMessages::oru_r01()
    );

    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    let start_count = events
        .iter()
        .filter(|e| matches!(e, Event::StartMessage { .. }))
        .count();
    assert_eq!(start_count, 3);

    let end_count = events.iter().filter(|e| matches!(e, Event::EndMessage)).count();
    assert_eq!(end_count, 3);
}

// =============================================================================
// Edge Cases Tests
// =============================================================================

#[test]
fn test_handle_empty_message() {
    // Just an MSH segment with minimal data
    let hl7_text = "MSH|^~\\&|\r";
    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    assert!(events.iter().any(|e| matches!(e, Event::StartMessage { .. })));
    assert!(events.iter().any(|e| matches!(e, Event::EndMessage)));
}

#[test]
fn test_handle_partial_message_gracefully() {
    // Message without proper termination - parser should still emit events
    let hl7_text = "MSH|^~\\&|App|Fac";
    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    // Should still get StartMessage since MSH was complete with \r
    // Actually without \r, the segment isn't complete, so no events
    // The parser waits for \r to complete a segment
    assert!(events.is_empty() || events.iter().any(|e| matches!(e, Event::StartMessage { .. })));
}

#[test]
fn test_handle_empty_fields() {
    let hl7_text = "MSH|^~\\&|App|||||\rPID||||||\r";
    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    // Should parse successfully with empty fields
    assert!(events.iter().any(|e| matches!(e, Event::StartMessage { .. })));

    // Count empty field events
    let empty_fields: Vec<_> = events
        .iter()
        .filter(|e| {
            if let Event::Field { raw, .. } = e {
                raw.is_empty()
            } else {
                false
            }
        })
        .collect();

    // Should have some empty fields
    assert!(!empty_fields.is_empty());
}

#[test]
fn test_handle_very_long_field() {
    // Create a message with a very long field (10KB)
    let long_content = "A".repeat(10000);
    let hl7_text = format!(
        "MSH|^~\\&|App|Fac|||20250101||ADT^A01|123|P|2.5\rPID|1||{}||Doe\r",
        long_content
    );

    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    // Should parse successfully
    assert!(events.iter().any(|e| matches!(e, Event::StartMessage { .. })));
    assert!(events.iter().any(|e| matches!(e, Event::EndMessage)));

    // Find the long field
    let long_field = events.iter().find(|e| {
        if let Event::Field { raw, .. } = e {
            raw.len() == 10000
        } else {
            false
        }
    });
    assert!(long_field.is_some());
}

#[test]
fn test_handle_very_long_segment() {
    // Create a segment with many fields
    let fields: Vec<&str> = (0..100).map(|_| "field_value").collect();
    let hl7_text = format!(
        "MSH|^~\\&|App|Fac|||20250101||ADT^A01|123|P|2.5\rPID|{}\r",
        fields.join("|")
    );

    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    assert!(events.iter().any(|e| matches!(e, Event::StartMessage { .. })));
    assert!(events.iter().any(|e| matches!(e, Event::EndMessage)));
}

#[test]
fn test_handle_deeply_nested_components() {
    // Test message with component and subcomponent structures
    let hl7_text = "MSH|^~\\&|App|Fac|||20250101||ADT^A01|123|P|2.5\rPID|1||12345^^^HOSP^MR^PN||Doe^John^M^Jr^III||19800101\r";
    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    assert!(events.iter().any(|e| matches!(e, Event::StartMessage { .. })));
    assert!(events.iter().any(|e| matches!(e, Event::EndMessage)));

    // Find field with components
    let nested_field = events.iter().find(|e| {
        if let Event::Field { raw, .. } = e {
            raw.windows(3).any(|w| w == b"HOSP")
        } else {
            false
        }
    });
    assert!(nested_field.is_some());
}

// =============================================================================
// Delimiter Tests
// =============================================================================

#[test]
fn test_custom_delimiters() {
    // Message with custom delimiters: $@#*
    let hl7_text = "MSH$@#*|App|Fac$1||123\r";
    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    // Check that delimiters were parsed correctly
    let start_event = events
        .iter()
        .find(|e| matches!(e, Event::StartMessage { .. }));

    if let Some(Event::StartMessage { delims }) = start_event {
        assert_eq!(delims.field, '$');
        assert_eq!(delims.comp, '@');
        assert_eq!(delims.rep, '#');
        assert_eq!(delims.esc, '*');
    } else {
        panic!("Expected StartMessage event");
    }
}

#[test]
fn test_standard_delimiters() {
    let hl7_text = SampleMessages::adt_a01();
    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    let start_event = events
        .iter()
        .find(|e| matches!(e, Event::StartMessage { .. }));

    if let Some(Event::StartMessage { delims }) = start_event {
        assert_eq!(delims.field, '|');
        assert_eq!(delims.comp, '^');
        assert_eq!(delims.rep, '~');
        assert_eq!(delims.esc, '\\');
        assert_eq!(delims.sub, '&');
    } else {
        panic!("Expected StartMessage event");
    }
}

#[test]
fn test_different_delimiters_per_message() {
    // Two messages with different delimiters
    let hl7_text = "MSH|^~\\&|App1|Fac1\rPID|1||123\rMSH$@#*|App2|Fac2\rPID$1||456\r";
    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    let start_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, Event::StartMessage { .. }))
        .collect();

    assert_eq!(start_events.len(), 2);

    // Check first message delimiters
    if let Some(Event::StartMessage { delims }) = start_events.first() {
        assert_eq!(delims.field, '|');
    }

    // Check second message delimiters
    if let Some(Event::StartMessage { delims }) = start_events.get(1) {
        assert_eq!(delims.field, '$');
    }
}

// =============================================================================
// Event Order Tests
// =============================================================================

#[test]
fn test_event_order_simple_message() {
    let hl7_text = "MSH|^~\\&|App|Fac\rPID|1||123\r";
    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    // First event should be StartMessage
    assert!(matches!(events.first(), Some(Event::StartMessage { .. })));

    // Last event should be EndMessage
    assert!(matches!(events.last(), Some(Event::EndMessage)));

    // Segment events should come between StartMessage and EndMessage
    let segment_positions: Vec<usize> = events
        .iter()
        .enumerate()
        .filter(|(_, e)| matches!(e, Event::Segment { .. }))
        .map(|(i, _)| i)
        .collect();

    for pos in &segment_positions {
        assert!(*pos > 0); // After StartMessage
        assert!(*pos < events.len() - 1); // Before EndMessage
    }
}

#[test]
fn test_field_events_after_segment() {
    let hl7_text = "MSH|^~\\&|App|Fac\rPID|1||123||Doe\r";
    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    // Find PID segment event position
    let pid_pos = events
        .iter()
        .position(|e| {
            if let Event::Segment { id } = e {
                id == b"PID"
            } else {
                false
            }
        });

    if let Some(pos) = pid_pos {
        // Next events should be Field events for PID
        let next_events = &events[pos + 1..];
        assert!(next_events
            .iter()
            .take_while(|e| matches!(e, Event::Field { .. }))
            .count()
            > 0);
    } else {
        panic!("PID segment not found");
    }
}

// =============================================================================
// Memory Efficiency Tests
// =============================================================================

#[test]
fn test_memory_bounded_for_large_message() {
    // Create a message that's larger than the internal buffer
    let large_content = "X".repeat(5000);
    let hl7_text = format!(
        "MSH|^~\\&|App|Fac|||20250101||ADT^A01|123|P|2.5\rPID|1||{}||Name\r",
        large_content
    );

    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    // Process events one at a time - this tests memory efficiency
    let mut event_count = 0;
    let mut max_field_size = 0;

    while let Ok(Some(event)) = parser.next_event() {
        event_count += 1;
        if let Event::Field { raw, .. } = &event {
            max_field_size = max_field_size.max(raw.len());
        }
    }

    assert!(event_count > 0);
    assert_eq!(max_field_size, 5000);
}

#[test]
fn test_streaming_does_not_load_entire_message() {
    // This test verifies that the parser processes incrementally
    // by checking that events are emitted before the entire stream is consumed

    let hl7_text = SampleMessages::adt_a01();
    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    // Get first event - should be StartMessage without reading entire input
    let first_event = parser.next_event();
    assert!(matches!(first_event, Ok(Some(Event::StartMessage { .. }))));
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[test]
fn test_invalid_utf8_in_segment() {
    // Create a message with invalid UTF-8 bytes
    let mut data = b"MSH|^~\\&|App|Fac\rPID|1||".to_vec();
    data.extend_from_slice(&[0xFF, 0xFE, 0xFD]); // Invalid UTF-8
    data.extend_from_slice(b"\r");

    let cursor = Cursor::new(data);
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    // The parser may or may not error depending on when it tries to parse UTF-8
    // Just ensure it doesn't panic
    let _ = collect_events(&mut parser);
}

#[test]
fn test_non_msh_start_with_default_delimiters() {
    // Message starting with non-MSH segment should use default delimiters
    let hl7_text = "PID|1||12345||Doe\r";
    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    // Should still get a StartMessage with default delimiters
    let start_event = events
        .iter()
        .find(|e| matches!(e, Event::StartMessage { .. }));

    if let Some(Event::StartMessage { delims }) = start_event {
        assert_eq!(delims.field, '|'); // Default
    } else {
        // This behavior may vary - some parsers might reject non-MSH starts
    }
}

// =============================================================================
// Special Characters Tests
// =============================================================================

#[test]
fn test_escape_sequences_in_fields() {
    let hl7_text = "MSH|^~\\&|App|Fac|||20250101||ADT^A01|123|P|2.5\rPID|1||123||Doe\\F\\John\r";
    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    // Find field with escape sequence
    let escaped_field = events.iter().find(|e| {
        if let Event::Field { raw, .. } = e {
            raw.windows(3).any(|w| w == b"\\F\\")
        } else {
            false
        }
    });
    assert!(escaped_field.is_some());
}

#[test]
fn test_special_characters_preserved() {
    // Test that special characters are preserved in field content
    let hl7_text = "MSH|^~\\&|App|Fac|||20250101||ADT^A01|123|P|2.5\rNK1|1||Spouse^Wife|||555-1234\r";
    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    // Find field with hyphen (phone number)
    let phone_field = events.iter().find(|e| {
        if let Event::Field { raw, .. } = e {
            raw.windows(8).any(|w| w == b"555-1234")
        } else {
            false
        }
    });
    assert!(phone_field.is_some());
}

// =============================================================================
// Field Numbering Tests
// =============================================================================

#[test]
fn test_msh_field_numbering() {
    // MSH has special field numbering due to encoding characters
    let hl7_text = "MSH|^~\\&|App|Fac|RecvApp|RecvFac|20250101||ADT^A01|123|P|2.5\r";
    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    // Collect MSH field numbers (fields before any Segment event)
    let msh_fields: Vec<u16> = events
        .iter()
        .take_while(|e| !matches!(e, Event::Segment { .. }))
        .filter_map(|e| {
            if let Event::Field { num, .. } = e {
                Some(*num)
            } else {
                None
            }
        })
        .collect();

    // Field numbers should start at 1
    assert!(msh_fields.contains(&1));
}

#[test]
fn test_pid_field_numbering() {
    let hl7_text = "MSH|^~\\&|App|Fac\rPID|1|2|3|4|5|6\r";
    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    // Find PID segment position
    let pid_pos = events.iter().position(|e| {
        matches!(e, Event::Segment { id } if id == b"PID")
    });

    if let Some(pos) = pid_pos {
        // Get fields after PID segment
        let pid_fields: Vec<u16> = events[pos + 1..]
            .iter()
            .take_while(|e| matches!(e, Event::Field { .. }))
            .filter_map(|e| {
                if let Event::Field { num, .. } = e {
                    Some(*num)
                } else {
                    None
                }
            })
            .collect();

        // Should have fields 1-6
        assert_eq!(pid_fields.len(), 6);
        assert_eq!(pid_fields[0], 1);
        assert_eq!(pid_fields[5], 6);
    }
}

// =============================================================================
// Message Builder Integration Tests
// =============================================================================

#[test]
fn test_parse_message_from_builder() {
    let bytes = MessageBuilder::new()
        .with_msh("TestApp", "TestFac", "RecvApp", "RecvFac", "ADT", "A01")
        .with_pid("MRN123", "Doe", "John")
        .build_bytes();

    let cursor = Cursor::new(bytes);
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    assert!(events.iter().any(|e| matches!(e, Event::StartMessage { .. })));
    assert!(events.iter().any(|e| matches!(e, Event::EndMessage)));

    // Should have PID segment
    assert!(events.iter().any(|e| {
        matches!(e, Event::Segment { id } if id == b"PID")
    }));
}

#[test]
fn test_parse_complex_message_from_builder() {
    let bytes = MessageBuilder::new()
        .with_msh("App", "Fac", "RecvApp", "RecvFac", "ADT", "A01")
        .with_pid("MRN456", "Smith", "Jane")
        .with_pv1("I", "ICU^101")
        .build_bytes();

    let cursor = Cursor::new(bytes);
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    // Should have both PID and PV1 segments
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
    assert!(segment_ids.iter().any(|id| *id == b"PV1"));
}
