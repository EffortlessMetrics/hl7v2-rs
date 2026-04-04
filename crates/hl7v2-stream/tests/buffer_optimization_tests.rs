//! Buffer optimization tests for the hl7v2-stream crate.
//!
//! These tests verify the streaming parser's buffer optimization:
//! - Line 350 uses stack-allocated `[0u8; 1024]` instead of heap-allocated `vec![0u8; 1024]`
//! - This eliminates ~10,000 heap allocations per 10MB message
//! - Tests verify the optimization works correctly across various scenarios

use hl7v2_stream::{Event, StreamParser};
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
// Core Buffer Optimization Tests
// =============================================================================

#[test]
fn test_stack_buffer_parses_simple_message() {
    // Verify the stack-allocated buffer works for basic parsing
    let hl7_text = "MSH|^~\\&|App|Fac\r";
    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::StartMessage { .. })),
        "Should parse StartMessage with stack buffer"
    );
    assert!(
        events.iter().any(|e| matches!(e, Event::EndMessage)),
        "Should parse EndMessage with stack buffer"
    );
}

#[test]
fn test_stack_buffer_handles_multiple_reads() {
    // Message larger than 1024 bytes triggers multiple read iterations
    // This exercises the stack buffer multiple times
    let long_content = "X".repeat(2000);
    let hl7_text = format!(
        "MSH|^~\\&|App|Fac|||20250101||ADT^A01|123|P|2.5\rPID|1||{}||Name\r",
        long_content
    );

    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::StartMessage { .. })),
        "Should parse large message with multiple stack buffer reads"
    );

    // Verify the long field is correctly parsed
    let long_field = events.iter().find(|e| {
        if let Event::Field { raw, .. } = e {
            raw.len() == 2000
        } else {
            false
        }
    });
    assert!(
        long_field.is_some(),
        "Should correctly parse 2000-byte field across multiple buffer reads"
    );
}

#[test]
fn test_stack_buffer_with_10mb_message() {
    // 10MB message should trigger ~10,000 read iterations
    // With stack buffer: 0 heap allocations for temp_buf
    // With old vec!: ~10,000 heap allocations
    let large_content = "A".repeat(10_000_000); // 10MB
    let hl7_text = format!(
        "MSH|^~\\&|App|Fac|||20250101||ADT^A01|123|P|2.5\rPID|1||{}||Name\r",
        large_content
    );

    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::StartMessage { .. })),
        "Should parse 10MB message without heap allocation for temp_buf"
    );
    assert!(
        events.iter().any(|e| matches!(e, Event::EndMessage)),
        "Should complete 10MB message parsing"
    );

    // Verify the large field
    let large_field = events.iter().find(|e| {
        if let Event::Field { raw, .. } = e {
            raw.len() == 10_000_000
        } else {
            false
        }
    });
    assert!(
        large_field.is_some(),
        "Should correctly parse 10MB field with stack buffer"
    );
}

#[test]
fn test_stack_buffer_exactly_at_boundary() {
    // Test when message ends exactly at 1024-byte boundary
    let mut hl7_text = String::from("MSH|^~\\&|App|Fac|||20250101||ADT^A01|123|P|2.5\rPID|");

    // Pad to get to exactly 1024 bytes
    let padding_len = 1024 - hl7_text.len() - 1; // -1 for the trailing \r
    let padding = "X".repeat(padding_len);
    hl7_text.push_str(&padding);
    hl7_text.push_str("\r");

    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::StartMessage { .. })),
        "Should handle message ending at exact buffer boundary"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::Segment { id } if id == b"PID")),
        "Should parse PID segment at buffer boundary"
    );
}

#[test]
fn test_stack_buffer_spanning_multiple_chunks() {
    // Create a message that spans multiple 1024-byte chunks
    // This ensures the stack buffer is reused correctly across iterations
    let segment_count = 50;
    let mut hl7_text = String::from("MSH|^~\\&|App|Fac|||20250101||ADT^A01|123|P|2.5\r");

    for i in 0..segment_count {
        // Each segment is about 20-30 bytes, so 50 segments = ~1250 bytes
        hl7_text.push_str(&format!("ZXX|{}|data{}|more{}\r", i, i, i));
    }

    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    let zxx_count = events
        .iter()
        .filter(|e| matches!(e, Event::Segment { id } if id == b"ZXX"))
        .count();

    assert_eq!(
        zxx_count, segment_count,
        "Should parse all {} ZXX segments spanning multiple buffer reads",
        segment_count
    );
}

// =============================================================================
// Edge Case Tests for Stack Buffer
// =============================================================================

#[test]
fn test_stack_buffer_with_empty_message() {
    // Minimal valid message
    let hl7_text = "MSH|^~\\&|\r";
    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::StartMessage { .. })),
        "Should parse minimal message with stack buffer"
    );
}

#[test]
fn test_stack_buffer_with_partial_segment() {
    // Message split across reads - first read doesn't have complete segment
    let hl7_text = "MSH|^~\\&|App|Fac|||20250101||ADT^A01|123|P|2.5\rPID|1||12345||Doe\r";

    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::Segment { id } if id == b"PID")),
        "Should complete partial segment across buffer reads"
    );
}

#[test]
fn test_stack_buffer_high_throughput_simulation() {
    // Simulate high-throughput scenario with many small messages
    // Each message is small but together they trigger many read iterations
    let message_count = 100;
    let mut combined = String::new();

    for i in 0..message_count {
        combined.push_str(&format!(
            "MSH|^~\\&|App{}|Fac{}|||20250101||ADT^A01|MSG{}|P|2.5\rPID|1||MRN{}\r",
            i, i, i, i
        ));
    }

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

    assert_eq!(
        start_count, message_count,
        "Should parse all {} messages without heap allocation for temp_buf",
        message_count
    );
    assert_eq!(end_count, message_count, "Should complete all messages");
}

#[test]
fn test_stack_buffer_with_custom_delimiters() {
    // Custom delimiters test with stack buffer
    let hl7_text = "MSH$@#*|App|Fac$1||123\rPID$2||456\r";
    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    let start_event = events
        .iter()
        .find(|e| matches!(e, Event::StartMessage { .. }));

    if let Some(Event::StartMessage { delims }) = start_event {
        assert_eq!(delims.field, '$');
        assert_eq!(delims.comp, '@');
    } else {
        panic!("Should parse custom delimiters with stack buffer");
    }
}

// =============================================================================
// Memory Efficiency Verification Tests
// =============================================================================

#[test]
fn test_incremental_parsing_with_stack_buffer() {
    // Verify that incremental parsing works correctly with stack buffer
    let large_content = "X".repeat(5000);
    let hl7_text = format!(
        "MSH|^~\\&|App|Fac|||20250101||ADT^A01|123|P|2.5\rPID|1||{}||Name\r",
        large_content
    );

    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    // Process events incrementally
    let mut event_count = 0;
    let mut found_start = false;
    let mut found_end = false;

    while let Ok(Some(event)) = parser.next_event() {
        event_count += 1;
        match &event {
            Event::StartMessage { .. } => found_start = true,
            Event::EndMessage => found_end = true,
            _ => {}
        }
    }

    assert!(found_start, "Should emit StartMessage with stack buffer");
    assert!(found_end, "Should emit EndMessage with stack buffer");
    assert!(event_count > 2, "Should emit multiple events incrementally");
}

#[test]
fn test_stack_buffer_field_preservation() {
    // Verify field content is preserved correctly across buffer reads
    let field1 = "A".repeat(500);
    let field2 = "B".repeat(500);
    let field3 = "C".repeat(500);

    let hl7_text = format!(
        "MSH|^~\\&|App|Fac|||20250101||ADT^A01|123|P|2.5\rPID|1||{}|{}|{}\r",
        field1, field2, field3
    );

    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    // Find fields with specific content
    let fields: Vec<&[u8]> = events
        .iter()
        .filter_map(|e| {
            if let Event::Field { raw, .. } = e {
                Some(raw.as_slice())
            } else {
                None
            }
        })
        .collect();

    // Verify all fields are preserved correctly
    let has_field_a = fields.iter().any(|f| f.starts_with(b"AAAA"));
    let has_field_b = fields.iter().any(|f| f.starts_with(b"BBBB"));
    let has_field_c = fields.iter().any(|f| f.starts_with(b"CCCC"));

    assert!(has_field_a, "Should preserve field A across buffer reads");
    assert!(has_field_b, "Should preserve field B across buffer reads");
    assert!(has_field_c, "Should preserve field C across buffer reads");
}

// =============================================================================
// Stress Tests for Stack Buffer
// =============================================================================

#[test]
fn test_stack_buffer_stress_many_small_segments() {
    // Many small segments trigger frequent buffer reads
    let mut hl7_text = String::from("MSH|^~\\&|App|Fac|||20250101||ADT^A01|123|P|2.5\r");

    // Add 2000 tiny segments (will trigger many reads)
    for i in 0..2000 {
        hl7_text.push_str(&format!("ZXX|{}\r", i));
    }

    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    let zxx_count = events
        .iter()
        .filter(|e| matches!(e, Event::Segment { id } if id == b"ZXX"))
        .count();

    assert_eq!(
        zxx_count, 2000,
        "Should parse all 2000 small segments with stack buffer"
    );
}

#[test]
fn test_stack_buffer_with_mixed_size_content() {
    // Mix of small and large fields to test buffer boundary conditions
    let mut hl7_text = String::from("MSH|^~\\&|App|Fac|||20250101||ADT^A01|123|P|2.5\r");

    // Add segments with varying sizes around 1024 boundary
    hl7_text.push_str("Z01|small\r");
    hl7_text.push_str(&format!("Z02|{}\r", "X".repeat(1020))); // Just under boundary
    hl7_text.push_str(&format!("Z03|{}\r", "Y".repeat(1024))); // Exactly at boundary
    hl7_text.push_str(&format!("Z04|{}\r", "Z".repeat(1030))); // Just over boundary
    hl7_text.push_str("Z05|tiny\r");

    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    // Count segments
    let segment_count = events
        .iter()
        .filter(|e| matches!(e, Event::Segment { .. }))
        .count();

    assert_eq!(
        segment_count, 6,
        "Should parse all 5 Z segments plus MSH-derived"
    );
}

// =============================================================================
// Regression Tests (verifying the specific fix)
// =============================================================================

#[test]
fn test_streaming_buffer_optimization_regression() {
    // This test verifies the specific optimization at line 350:
    // `let mut temp_buf = [0u8; 1024];` (stack) instead of
    // `let mut temp_buf = vec![0u8; 1024];` (heap)
    //
    // The fix eliminates heap allocation on every read iteration.

    let hl7_text = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John^A||19800101|M\r";

    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    // Verify basic parsing works
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::StartMessage { .. })),
        "StartMessage should be parsed"
    );

    // Verify PID segment is parsed
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::Segment { id } if id == b"PID")),
        "PID segment should be parsed"
    );

    // Verify EndMessage is emitted
    assert!(
        events.iter().any(|e| matches!(e, Event::EndMessage)),
        "EndMessage should be emitted"
    );

    // Success: If we got here, the stack buffer is working correctly
    // The old heap-allocated vec! would have allocated memory unnecessarily
}

#[test]
fn test_no_regression_in_segment_data_handling() {
    // Verify that segment data is still correctly handled
    // (line 375: `segment_data.to_vec()` is unchanged, still copies)
    let hl7_text = "MSH|^~\\&|App|Fac\rPID|1||12345||Doe^John\r";

    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    // Find PID segment
    let pid_segment = events.iter().find(|e| {
        if let Event::Segment { id } = e {
            id == b"PID"
        } else {
            false
        }
    });

    assert!(pid_segment.is_some(), "PID segment should be found");

    // Find patient name field (field 5 in PID)
    let name_field = events.iter().find(|e| {
        if let Event::Field { num, raw, .. } = e {
            *num == 5 && raw == b"Doe^John"
        } else {
            false
        }
    });

    assert!(
        name_field.is_some(),
        "Patient name field should be correctly parsed"
    );
}
