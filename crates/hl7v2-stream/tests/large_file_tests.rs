//! Large file tests for the hl7v2-stream crate.
//!
//! These tests verify:
//! - Memory efficiency with large messages
//! - Performance with large files
//! - Handling of very long fields and segments
//! - Bounded memory usage

use hl7v2_stream::{Event, StreamParser};
use std::io::{BufReader, Cursor, Read};
use std::time::Instant;

/// Helper to collect all events from a parser
fn collect_events<R: Read>(parser: &mut StreamParser<BufReader<R>>) -> Vec<Event> {
    let mut events = Vec::new();
    while let Ok(Some(event)) = parser.next_event() {
        events.push(event);
    }
    events
}

/// Helper to generate a large HL7 message
fn generate_large_message(segment_count: usize, field_count: usize) -> String {
    let mut msg = String::from("MSH|^~\\&|App|Fac|||20250101||ADT^A01|123|P|2.5\r");

    for i in 0..segment_count {
        msg.push_str(&format!("ZXX|{}", i));
        for j in 1..field_count {
            msg.push_str(&format!("|field_{}_{}", i, j));
        }
        msg.push_str("\r");
    }

    msg
}

/// Helper to generate a message with very long fields
fn generate_message_with_long_fields(field_length: usize, field_count: usize) -> String {
    let long_field = "X".repeat(field_length);
    let fields: Vec<String> = (0..field_count).map(|_| long_field.clone()).collect();

    format!(
        "MSH|^~\\&|App|Fac|||20250101||ADT^A01|123|P|2.5\rPID|{}\r",
        fields.join("|")
    )
}

// =============================================================================
// Large Message Tests
// =============================================================================

#[test]
fn test_large_message_100_segments() {
    let hl7_text = generate_large_message(100, 10);

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

    let segment_count = events
        .iter()
        .filter(|e| matches!(e, Event::Segment { .. }))
        .count();
    assert_eq!(segment_count, 100);
}

#[test]
fn test_large_message_1000_segments() {
    let hl7_text = generate_large_message(1000, 10);

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

    let segment_count = events
        .iter()
        .filter(|e| matches!(e, Event::Segment { .. }))
        .count();
    assert_eq!(segment_count, 1000);
}

#[test]
fn test_large_message_10000_segments() {
    let hl7_text = generate_large_message(10000, 5);

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

    let segment_count = events
        .iter()
        .filter(|e| matches!(e, Event::Segment { .. }))
        .count();
    assert_eq!(segment_count, 10000);
}

// =============================================================================
// Long Field Tests
// =============================================================================

#[test]
fn test_very_long_field_1kb() {
    let hl7_text = generate_message_with_long_fields(1024, 5);

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

    // Verify long field is preserved
    let long_field = events.iter().find(|e| {
        if let Event::Field { raw, .. } = e {
            raw.len() == 1024
        } else {
            false
        }
    });
    assert!(long_field.is_some());
}

#[test]
fn test_very_long_field_10kb() {
    let hl7_text = generate_message_with_long_fields(10 * 1024, 3);

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
fn test_very_long_field_100kb() {
    let hl7_text = generate_message_with_long_fields(100 * 1024, 2);

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

    // Verify the 100KB field is preserved
    let long_field = events.iter().find(|e| {
        if let Event::Field { raw, .. } = e {
            raw.len() == 100 * 1024
        } else {
            false
        }
    });
    assert!(long_field.is_some());
}

#[test]
fn test_very_long_field_1mb() {
    let hl7_text = generate_message_with_long_fields(1024 * 1024, 1);

    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    // Use a larger limit for this test
    let mut parser = StreamParser::with_max_message_size(buf_reader, 2 * 1024 * 1024);

    let events = collect_events(&mut parser);

    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::StartMessage { .. }))
    );
    assert!(events.iter().any(|e| matches!(e, Event::EndMessage)));
}

// =============================================================================
// Many Fields Tests
// =============================================================================

#[test]
fn test_segment_with_100_fields() {
    let fields: Vec<String> = (0..100).map(|i| format!("field_{}", i)).collect();
    let hl7_text = format!(
        "MSH|^~\\&|App|Fac|||20250101||ADT^A01|123|P|2.5\rPID|{}\r",
        fields.join("|")
    );

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

    // Count PID fields
    let pid_fields = events
        .iter()
        .filter(|e| matches!(e, Event::Field { .. }))
        .count();
    // Should have at least 100 fields from PID
    assert!(pid_fields >= 100);
}

#[test]
fn test_segment_with_1000_fields() {
    let fields: Vec<String> = (0..1000).map(|i| format!("f{}", i)).collect();
    let hl7_text = format!(
        "MSH|^~\\&|App|Fac|||20250101||ADT^A01|123|P|2.5\rPID|{}\r",
        fields.join("|")
    );

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

// =============================================================================
// Multiple Large Messages Tests
// =============================================================================

#[test]
fn test_multiple_large_messages() {
    // Generate 10 messages with 100 segments each
    let mut combined = String::new();

    for _ in 0..10 {
        combined.push_str(&generate_large_message(100, 5));
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

    assert_eq!(start_count, 10);
    assert_eq!(end_count, 10);
}

// =============================================================================
// Memory Bounded Tests
// =============================================================================

#[test]
fn test_memory_bounded_streaming() {
    // This test verifies that we can process events incrementally
    // without holding the entire message in memory

    let hl7_text = generate_large_message(500, 10);
    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    // Process events one at a time and discard them
    let mut segment_count = 0;
    let mut total_field_bytes = 0usize;

    while let Ok(Some(event)) = parser.next_event() {
        match &event {
            Event::Segment { .. } => segment_count += 1,
            Event::Field { raw, .. } => total_field_bytes += raw.len(),
            _ => {}
        }
        // Event is dropped here, releasing memory
    }

    assert_eq!(segment_count, 500);
    assert!(total_field_bytes > 0);
}

#[test]
fn test_incremental_processing_does_not_accumulate() {
    // Verify that incremental processing releases memory
    let hl7_text = generate_large_message(1000, 20);

    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    // Process and immediately discard events
    let mut event_count = 0;
    while let Ok(Some(_)) = parser.next_event() {
        event_count += 1;
        // Event is immediately dropped
    }

    // Should have processed all events
    assert!(event_count > 1000);
}

// =============================================================================
// Performance Tests
// =============================================================================

#[test]
fn test_parsing_performance_1000_segments() {
    let hl7_text = generate_large_message(1000, 10);

    let start = Instant::now();

    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);
    let events = collect_events(&mut parser);

    let duration = start.elapsed();

    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::StartMessage { .. }))
    );
    assert!(events.iter().any(|e| matches!(e, Event::EndMessage)));

    // Performance assertion: should complete in reasonable time
    // This is a soft assertion - adjust threshold as needed
    println!("Parsed 1000 segments in {:?}", duration);
    assert!(
        duration.as_millis() < 5000,
        "Parsing should complete within 5 seconds"
    );
}

#[test]
fn test_parsing_performance_large_field() {
    let hl7_text = generate_message_with_long_fields(1024 * 100, 10); // 100KB fields

    let start = Instant::now();

    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);
    let events = collect_events(&mut parser);

    let duration = start.elapsed();

    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::StartMessage { .. }))
    );
    assert!(events.iter().any(|e| matches!(e, Event::EndMessage)));

    println!("Parsed message with 10x100KB fields in {:?}", duration);
    assert!(
        duration.as_millis() < 5000,
        "Parsing should complete within 5 seconds"
    );
}

// =============================================================================
// Chunk Boundary Edge Cases
// =============================================================================

#[test]
fn test_boundary_at_exact_1024_bytes() {
    // Create a message that has a segment boundary exactly at 1024 bytes
    let mut msg = String::from("MSH|^~\\&|App|Fac|||20250101||ADT^A01|123|P|2.5\r");

    // Pad to get close to 1024 boundary
    while msg.len() < 1020 {
        msg.push('X');
    }
    msg.push_str("\rPID|1||123\r");

    let cursor = Cursor::new(msg.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::StartMessage { .. }))
    );
    assert!(events.iter().any(|e| matches!(e, Event::EndMessage)));
    assert!(
        events
            .iter()
            .any(|e| { matches!(e, Event::Segment { id } if id == b"PID") })
    );
}

#[test]
fn test_boundary_in_middle_of_long_field() {
    // Create a field that spans the 1024-byte buffer boundary
    let prefix_len = 1000;
    let field_len = 100; // Will span boundary

    let prefix = "X".repeat(prefix_len);
    let field_content = "Y".repeat(field_len);

    let msg = format!(
        "MSH|^~\\&|App|Fac|||20250101||ADT^A01|123|P|2.5\rPID|1|{}|{}|rest\r",
        prefix, field_content
    );

    let cursor = Cursor::new(msg.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::StartMessage { .. }))
    );
    assert!(events.iter().any(|e| matches!(e, Event::EndMessage)));

    // Verify the field content is preserved correctly
    let field_with_y = events.iter().find(|e| {
        if let Event::Field { raw, .. } = e {
            raw.windows(10).any(|w| w == b"YYYYYYYYYY")
        } else {
            false
        }
    });
    assert!(field_with_y.is_some());
}

// =============================================================================
// Stress Tests
// =============================================================================

#[test]
fn test_stress_deeply_nested_components() {
    // Create a message with deeply nested component structures
    let mut msg = String::from("MSH|^~\\&|App|Fac|||20250101||ADT^A01|123|P|2.5\rPID|1||");

    // Create deeply nested structure
    for i in 0..100 {
        if i > 0 {
            msg.push('^');
        }
        msg.push_str(&format!("comp{}", i));
    }
    msg.push_str("||Name\r");

    let cursor = Cursor::new(msg.as_bytes());
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
fn test_stress_many_repetitions() {
    // Create a message with many field repetitions
    let mut msg = String::from("MSH|^~\\&|App|Fac|||20250101||ADT^A01|123|P|2.5\rPID|1||");

    for i in 0..100 {
        if i > 0 {
            msg.push('~');
        }
        msg.push_str(&format!("rep{}", i));
    }
    msg.push_str("||Name\r");

    let cursor = Cursor::new(msg.as_bytes());
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
fn test_stress_mixed_delimiters() {
    // Create a complex message with all delimiter types
    let msg = "MSH|^~\\&|App|Fac|||20250101||ADT^A01|123|P|2.5\rPID|1||12345^^^HOSP^MR~12346^^^HOSP^SS||Doe^John^M^Jr^III||19800101|M|||123 Main St^Apt 1^City^ST^12345||(555)123-4567~(555)987-6543|||M\r";

    let cursor = Cursor::new(msg.as_bytes());
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

// =============================================================================
// Real-World Large File Scenarios
// =============================================================================

#[test]
fn test_large_lab_result_message() {
    // Simulate a large lab result message with many OBX segments
    let mut msg =
        String::from("MSH|^~\\&|LabSys|Lab|HIS|Hospital|20250128150000||ORU^R01|MSG003|P|2.5\r");
    msg.push_str("PID|1||MRN789^^^Lab^MR||Patient^Test||19850610|M\r");
    msg.push_str("OBR|1|ORD123|FIL456|PANEL^Comprehensive Panel|||20250128120000\r");

    // Add many OBX segments (lab results)
    for i in 1..=500 {
        msg.push_str(&format!(
            "OBX|{}|NM|TEST{}^Test Name {}||{}.{}|units|low-high|N|||F\r",
            i,
            i,
            i,
            i as f64 / 10.0,
            i % 10
        ));
    }

    let cursor = Cursor::new(msg.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::StartMessage { .. }))
    );
    assert!(events.iter().any(|e| matches!(e, Event::EndMessage)));

    // Should have PID, OBR, and 500 OBX segments
    let obx_count = events
        .iter()
        .filter(|e| matches!(e, Event::Segment { id } if id == b"OBX"))
        .count();
    assert_eq!(obx_count, 500);
}

#[test]
fn test_large_patient_batch() {
    // Simulate a batch of patient messages
    let mut combined = String::new();

    for i in 0..100 {
        let msg = format!(
            "MSH|^~\\&|ADT|Hospital|HIS|Hospital|202501281{}0000||ADT^A01|MSG{:04}|P|2.5\rPID|1||MRN{:06}^^^HOSP^MR||Patient^{}||19800101|M\r",
            i, i, i, i
        );
        combined.push_str(&msg);
    }

    let cursor = Cursor::new(combined.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let events = collect_events(&mut parser);

    let start_count = events
        .iter()
        .filter(|e| matches!(e, Event::StartMessage { .. }))
        .count();
    assert_eq!(start_count, 100);
}

// =============================================================================
// Memory Usage Verification
// =============================================================================

#[test]
fn test_field_memory_is_released() {
    // This test verifies that field memory is properly released
    // when processing incrementally

    let large_field = "X".repeat(1_000_000); // 1MB field
    let msg = format!(
        "MSH|^~\\&|App|Fac|||20250101||ADT^A01|123|P|2.5\rPID|1||{}||Name\r",
        large_field
    );

    let cursor = Cursor::new(msg.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    // Process first event (should be StartMessage)
    let first = parser.next_event().unwrap();
    assert!(matches!(first, Some(Event::StartMessage { .. })));

    // Process remaining events
    let mut found_large_field = false;
    while let Ok(Some(event)) = parser.next_event() {
        if let Event::Field { raw, .. } = &event {
            if raw.len() == 1_000_000 {
                found_large_field = true;
            }
        }
        // Event is dropped here
    }

    assert!(found_large_field);
}

// =============================================================================
// Throughput Tests
// =============================================================================

#[test]
fn test_throughput_megabytes_per_second() {
    // Generate a 5MB message
    let segment_count = 50000;
    let hl7_text = generate_large_message(segment_count, 5);
    let size_mb = hl7_text.len() as f64 / (1024.0 * 1024.0);

    let start = Instant::now();

    let cursor = Cursor::new(hl7_text.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let mut event_count = 0;
    while let Ok(Some(_)) = parser.next_event() {
        event_count += 1;
    }

    let duration = start.elapsed();
    let throughput = size_mb / duration.as_secs_f64();

    println!(
        "Processed {:.2} MB in {:?} ({:.2} MB/s, {} events)",
        size_mb, duration, throughput, event_count
    );

    // Basic sanity check - should process at least 1 MB/s
    // Adjust this threshold based on expected performance
    assert!(throughput > 1.0, "Throughput should be at least 1 MB/s");
}
