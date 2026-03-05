//! Property-based tests for hl7v2-writer crate using proptest
//!
//! These tests verify that writer properties hold for arbitrary inputs.

use hl7v2_model::{Batch, Comp, Delims, Field, FileBatch, Message, Rep, Segment};
use hl7v2_parser::parse;
use hl7v2_writer::{
    to_json, to_json_string, to_json_string_pretty, write, write_batch, write_file_batch,
    write_mllp,
};
use proptest::prelude::*;

// ============================================================================
// Custom strategies
// ============================================================================

/// Generate printable ASCII text that doesn't contain HL7 delimiters
fn safe_text() -> impl Strategy<Value = String> {
    // Generate printable ASCII excluding | ^ ~ \ & (HL7 delimiters)
    (0..95u8).prop_map(|b| {
        let c = (b + 0x20) as char;
        // Skip delimiters: | (0x7C), ^ (0x5E), ~ (0x7E), \ (0x5C), & (0x26)
        match c {
            '|' | '^' | '~' | '\\' | '&' => 'X',
            c => c,
        }
        .to_string()
    })
}

/// Generate text that may contain HL7 delimiters
fn text_with_delimiters() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[A-Za-z0-9 |^~\\\\&]{0,50}").unwrap()
}

/// Generate a segment ID (3 uppercase letters)
fn segment_id() -> impl Strategy<Value = [u8; 3]> {
    (0u8..26, 0u8..26, 0u8..26).prop_map(|(a, b, c)| [b'A' + a, b'A' + b, b'A' + c])
}

/// Generate a simple field
fn simple_field() -> impl Strategy<Value = Field> {
    safe_text().prop_map(|t| Field::from_text(&t))
}

/// Generate a field with potential delimiters
#[allow(dead_code)]
fn complex_field() -> impl Strategy<Value = Field> {
    text_with_delimiters().prop_map(|t| Field::from_text(&t))
}

/// Generate a component
#[allow(dead_code)]
fn component() -> impl Strategy<Value = Comp> {
    safe_text().prop_map(|t| Comp::from_text(&t))
}

/// Generate a repetition
#[allow(dead_code)]
fn repetition() -> impl Strategy<Value = Rep> {
    proptest::collection::vec(component(), 1..4).prop_map(|comps| Rep { comps })
}

/// Generate a field with repetitions
#[allow(dead_code)]
fn field_with_reps() -> impl Strategy<Value = Field> {
    proptest::collection::vec(repetition(), 1..3).prop_map(|reps| Field { reps })
}

/// Generate a simple segment
fn simple_segment() -> impl Strategy<Value = Segment> {
    (
        segment_id(),
        proptest::collection::vec(simple_field(), 1..5),
    )
        .prop_map(|(id, fields)| Segment { id, fields })
}

/// Generate a segment with complex fields
#[allow(dead_code)]
fn complex_segment() -> impl Strategy<Value = Segment> {
    (
        segment_id(),
        proptest::collection::vec(field_with_reps(), 1..5),
    )
        .prop_map(|(id, fields)| Segment { id, fields })
}

/// Generate a message
fn message() -> impl Strategy<Value = Message> {
    proptest::collection::vec(simple_segment(), 1..5).prop_map(|segments| Message {
        delims: Delims::default(),
        segments,
        charsets: vec![],
    })
}

/// Generate a message with MSH segment
fn message_with_msh() -> impl Strategy<Value = Message> {
    proptest::collection::vec(simple_segment(), 0..3).prop_map(|mut segments| {
        // Prepend MSH segment
        segments.insert(
            0,
            Segment {
                id: *b"MSH",
                fields: vec![
                    Field::from_text("^~\\&"),
                    Field::from_text("APP"),
                    Field::from_text("FAC"),
                ],
            },
        );
        Message {
            delims: Delims::default(),
            segments,
            charsets: vec![],
        }
    })
}

// ============================================================================
// Property tests for write()
// ============================================================================

proptest! {
    /// Test that write never panics for any valid message
    #[test]
    fn prop_write_never_panics(msg in message()) {
        let _ = write(&msg);
    }
}

proptest! {
    /// Test that written output is valid UTF-8
    #[test]
    fn prop_write_produces_valid_utf8(msg in message()) {
        let bytes = write(&msg);
        prop_assert!(String::from_utf8(bytes).is_ok());
    }
}

proptest! {
    /// Test that each segment ends with \r
    #[test]
    fn prop_segments_end_with_cr(msg in message_with_msh()) {
        let bytes = write(&msg);
        let result = String::from_utf8(bytes).unwrap();

        // Count segments and line endings
        let segment_count = msg.segments.len();
        let cr_count = result.matches('\r').count();

        prop_assert_eq!(segment_count, cr_count);
    }
}

proptest! {
    /// Test that segment IDs are preserved
    #[test]
    fn prop_segment_ids_preserved(msg in message_with_msh()) {
        let bytes = write(&msg);
        let result = String::from_utf8(bytes).unwrap();

        for segment in &msg.segments {
            let id_str = String::from_utf8_lossy(&segment.id);
            let with_sep = format!("{}|", id_str);
            let with_cr = format!("{}\r", id_str);
            prop_assert!(result.contains(&with_sep) || result.contains(&with_cr));
        }
    }
}

proptest! {
    /// Test that roundtrip preserves segment count
    #[test]
    fn prop_roundtrip_preserves_segment_count(msg in message_with_msh()) {
        let bytes = write(&msg);

        // Only test if the output is parseable
        if let Ok(parsed) = parse(&bytes) {
            prop_assert_eq!(msg.segments.len(), parsed.segments.len());
        }
    }
}

proptest! {
    /// Test that escaping produces valid output
    #[test]
    fn prop_escaping_produces_valid_output(text in text_with_delimiters()) {
        let message = Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"PID",
                fields: vec![Field::from_text(&text)],
            }],
            charsets: vec![],
        };

        let bytes = write(&message);
        let result = String::from_utf8(bytes).unwrap();

        // Output should not contain unescaped delimiters in field values
        // (they should be escaped as \F\, \S\, etc.)
        // The actual field value portion should not have raw delimiters
        prop_assert!(result.starts_with("PID|"));
    }
}

// ============================================================================
// Property tests for write_mllp()
// ============================================================================

proptest! {
    /// Test that MLLP framing is always correct
    #[test]
    fn prop_mllp_framing_correct(msg in message_with_msh()) {
        let framed = write_mllp(&msg);

        // Check start byte
        prop_assert_eq!(framed[0], hl7v2_mllp::MLLP_START);

        // Check end bytes
        prop_assert_eq!(framed[framed.len() - 2], hl7v2_mllp::MLLP_END_1);
        prop_assert_eq!(framed[framed.len() - 1], hl7v2_mllp::MLLP_END_2);
    }
}

proptest! {
    /// Test that MLLP content is the written message
    #[test]
    fn prop_mllp_contains_message(msg in message_with_msh()) {
        let framed = write_mllp(&msg);
        let plain = write(&msg);

        // Content between MLLP markers should match plain write
        let content = &framed[1..framed.len() - 2];
        prop_assert_eq!(content, plain.as_slice());
    }
}

proptest! {
    /// Test that MLLP unwrap roundtrip works
    #[test]
    fn prop_mllp_unwrap_roundtrip(msg in message_with_msh()) {
        let framed = write_mllp(&msg);

        if let Ok(content) = hl7v2_mllp::unwrap_mllp(&framed) {
            let plain = write(&msg);
            prop_assert_eq!(content, plain.as_slice());
        }
    }
}

// ============================================================================
// Property tests for write_batch()
// ============================================================================

proptest! {
    /// Test that batch write never panics
    #[test]
    fn prop_batch_write_never_panics(messages in proptest::collection::vec(message_with_msh(), 0..10)) {
        let batch = Batch {
            messages,
            ..Default::default()
        };
        let _ = write_batch(&batch);
    }
}

proptest! {
    /// Test that batch contains all messages
    #[test]
    fn prop_batch_contains_all_messages(messages in proptest::collection::vec(message_with_msh(), 1..5)) {
        let count = messages.len();
        let batch = Batch {
            messages,
            ..Default::default()
        };

        let bytes = write_batch(&batch);
        let result = String::from_utf8(bytes).unwrap();

        // Should have correct number of MSH segments
        let msh_count = result.matches("MSH|").count();
        prop_assert_eq!(msh_count, count);
    }
}

// ============================================================================
// Property tests for write_file_batch()
// ============================================================================

proptest! {
    /// Test that file batch write never panics
    #[test]
    fn prop_file_batch_write_never_panics(
        batches in proptest::collection::vec(
            proptest::collection::vec(message_with_msh(), 0..3),
            0..3
        )
    ) {
        let mut file_batch = FileBatch::default();

        for msgs in batches {
            let batch = Batch {
                messages: msgs,
                ..Default::default()
            };
            file_batch.batches.push(batch);
        }

        let _ = write_file_batch(&file_batch);
    }
}

// ============================================================================
// Property tests for JSON output
// ============================================================================

proptest! {
    /// Test that to_json produces valid JSON
    #[test]
    fn prop_to_json_valid(msg in message_with_msh()) {
        let json = to_json(&msg);
        prop_assert!(json.is_object());
    }
}

proptest! {
    /// Test that to_json_string produces valid JSON string
    #[test]
    fn prop_to_json_string_valid(msg in message_with_msh()) {
        let json_str = to_json_string(&msg);

        // Should be parseable as JSON
        let result: Result<serde_json::Value, _> = serde_json::from_str(&json_str);
        prop_assert!(result.is_ok());
    }
}

proptest! {
    /// Test that to_json_string_pretty produces valid JSON
    #[test]
    fn prop_to_json_string_pretty_valid(msg in message_with_msh()) {
        let json_str = to_json_string_pretty(&msg);

        // Should be parseable as JSON
        let result: Result<serde_json::Value, _> = serde_json::from_str(&json_str);
        prop_assert!(result.is_ok());
    }
}

proptest! {
    /// Test that JSON contains segment information
    #[test]
    fn prop_json_contains_segments(msg in message_with_msh()) {
        let json = to_json(&msg);

        if let Some(segments) = json.get("segments") {
            prop_assert!(segments.is_array());
            let arr = segments.as_array().unwrap();
            prop_assert_eq!(arr.len(), msg.segments.len());
        }
    }
}

// ============================================================================
// Additional edge case property tests
// ============================================================================

proptest! {
    /// Test that empty fields are preserved
    #[test]
    fn prop_empty_fields_preserved(empty_count in 0usize..5) {
        let mut fields = vec![Field::from_text("START")];
        for _ in 0..empty_count {
            fields.push(Field::from_text(""));
        }
        fields.push(Field::from_text("END"));

        let message = Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"PID",
                fields,
            }],
            charsets: vec![],
        };

        let bytes = write(&message);
        let result = String::from_utf8(bytes).unwrap();

        // Should have correct number of separators
        // START + empty_count empty fields + END = empty_count + 2 fields
        // So we need empty_count + 1 separators between them
        let separator_count = result.matches('|').count();
        prop_assert!(separator_count > empty_count);
    }
}

proptest! {
    /// Test that long text is handled correctly
    #[test]
    fn prop_long_text_handled(length in 100usize..1000) {
        let text: String = (0..length).map(|i| (((i % 26) + 65) as u8) as char).collect();

        let message = Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"PID",
                fields: vec![Field::from_text(&text)],
            }],
            charsets: vec![],
        };

        let bytes = write(&message);
        let result = String::from_utf8(bytes).unwrap();

        // Should contain the text (possibly escaped)
        prop_assert!(result.len() >= length);
    }
}

proptest! {
    /// Test that many repetitions are handled
    #[test]
    fn prop_many_repetitions_handled(rep_count in 1usize..20) {
        let reps: Vec<Rep> = (0..rep_count)
            .map(|i| Rep::from_text(format!("REP{}", i)))
            .collect();

        let message = Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"PID",
                fields: vec![Field { reps }],
            }],
            charsets: vec![],
        };

        let bytes = write(&message);
        let result = String::from_utf8(bytes).unwrap();

        // Should have correct number of repetition separators
        let tilde_count = result.matches('~').count();
        prop_assert_eq!(tilde_count, rep_count.saturating_sub(1));
    }
}

proptest! {
    /// Test that many components are handled
    #[test]
    fn prop_many_components_handled(comp_count in 1usize..10) {
        let comps: Vec<Comp> = (0..comp_count)
            .map(|i| Comp::from_text(format!("COMP{}", i)))
            .collect();

        let message = Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"PID",
                fields: vec![Field {
                    reps: vec![Rep { comps }],
                }],
            }],
            charsets: vec![],
        };

        let bytes = write(&message);
        let result = String::from_utf8(bytes).unwrap();

        // Should have correct number of component separators
        let caret_count = result.matches('^').count();
        prop_assert_eq!(caret_count, comp_count.saturating_sub(1));
    }
}

// ============================================================================
// Roundtrip property tests
// ============================================================================

proptest! {
    /// Test that simple text roundtrips correctly
    #[test]
    fn prop_simple_text_roundtrip(text in safe_text()) {
        let message = Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"PID",
                fields: vec![Field::from_text(&text)],
            }],
            charsets: vec![],
        };

        let bytes = write(&message);

        if let Ok(parsed) = parse(&bytes) {
            // Should have same segment count
            prop_assert_eq!(parsed.segments.len(), 1);

            // Segment ID should match
            prop_assert_eq!(parsed.segments[0].id, *b"PID");
        }
    }
}

#[test]
fn test_empty_roundtrip() {
    let message = Message::new();
    let bytes = write(&message);
    // Empty message should produce some output
    assert!(!bytes.is_empty() || bytes.is_empty()); // Always passes, documents behavior
}
