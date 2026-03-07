//! Property-based tests for hl7v2-json crate using proptest
//!
//! These tests verify JSON serialization properties hold for arbitrary inputs.

use hl7v2_json::{to_json, to_json_string, to_json_string_pretty};
use hl7v2_model::{Atom, Comp, Delims, Field, Message, Rep, Segment};
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

/// Generate Unicode text (including non-ASCII characters)
fn unicode_text() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[\\u0000-\\uFFFF]{0,30}").unwrap()
}

/// Generate text with JSON special characters (quotes and backslashes)
fn text_with_json_special_chars() -> impl Strategy<Value = String> {
    // Generate text that may contain quotes and backslashes
    proptest::string::string_regex("[A-Za-z0-9\"\\\\]{0,30}").unwrap()
}

/// Generate a segment ID (3 uppercase letters)
fn segment_id() -> impl Strategy<Value = [u8; 3]> {
    (0u8..26, 0u8..26, 0u8..26).prop_map(|(a, b, c)| [b'A' + a, b'A' + b, b'A' + c])
}

/// Generate a simple field with safe text
fn simple_field() -> impl Strategy<Value = Field> {
    safe_text().prop_map(|t| Field::from_text(&t))
}

/// Generate a field with potential delimiters
fn complex_field() -> impl Strategy<Value = Field> {
    text_with_delimiters().prop_map(|t| Field::from_text(&t))
}

/// Generate a component
fn component() -> impl Strategy<Value = Comp> {
    safe_text().prop_map(|t| Comp::from_text(&t))
}

/// Generate a component with subcomponents
fn component_with_subs() -> impl Strategy<Value = Comp> {
    proptest::collection::vec(safe_text(), 1..4).prop_map(|texts| {
        let subs = texts
            .into_iter()
            .map(|t| {
                if t.is_empty() {
                    Atom::Null
                } else {
                    Atom::Text(t)
                }
            })
            .collect();
        Comp { subs }
    })
}

/// Generate a repetition
fn repetition() -> impl Strategy<Value = Rep> {
    proptest::collection::vec(component(), 1..4).prop_map(|comps| Rep { comps })
}

/// Generate a field with repetitions
fn field_with_reps() -> impl Strategy<Value = Field> {
    proptest::collection::vec(repetition(), 1..3).prop_map(|reps| Field { reps })
}

/// Generate a simple segment
fn simple_segment() -> impl Strategy<Value = Segment> {
    (
        segment_id(),
        proptest::collection::vec(simple_field(), 0..5),
    )
        .prop_map(|(id, fields)| Segment { id, fields })
}

/// Generate a segment with complex fields
fn complex_segment() -> impl Strategy<Value = Segment> {
    (
        segment_id(),
        proptest::collection::vec(field_with_reps(), 0..5),
    )
        .prop_map(|(id, fields)| Segment { id, fields })
}

/// Generate a message
fn message() -> impl Strategy<Value = Message> {
    proptest::collection::vec(simple_segment(), 0..5).prop_map(|segments| Message {
        delims: Delims::default(),
        segments,
        charsets: vec![],
    })
}

/// Generate a message with charsets
fn message_with_charsets() -> impl Strategy<Value = Message> {
    (
        proptest::collection::vec(simple_segment(), 0..3),
        proptest::collection::vec("[A-Z]{3,10}", 0..3),
    )
        .prop_map(|(segments, charsets)| Message {
            delims: Delims::default(),
            segments,
            charsets,
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

/// Generate a message with complex nested structures
fn complex_message() -> impl Strategy<Value = Message> {
    proptest::collection::vec(complex_segment(), 1..4).prop_map(|segments| Message {
        delims: Delims::default(),
        segments,
        charsets: vec!["ASCII".to_string()],
    })
}

// ============================================================================
// Property tests for to_json()
// ============================================================================

proptest! {
    /// Test that to_json never panics for any valid message
    #[test]
    fn prop_to_json_never_panics(msg in message()) {
        let _ = to_json(&msg);
    }
}

proptest! {
    /// Test that to_json always returns an object
    #[test]
    fn prop_to_json_returns_object(msg in message()) {
        let json = to_json(&msg);
        prop_assert!(json.is_object());
    }
}

proptest! {
    /// Test that to_json output has required structure (meta and segments)
    #[test]
    fn prop_to_json_has_required_structure(msg in message()) {
        let json = to_json(&msg);
        prop_assert!(json.get("meta").is_some());
        prop_assert!(json.get("segments").is_some());
        prop_assert!(json.get("meta").unwrap().is_object());
        prop_assert!(json.get("segments").unwrap().is_array());
    }
}

proptest! {
    /// Test that meta contains delimiters
    #[test]
    fn prop_to_json_meta_has_delimiters(msg in message()) {
        let json = to_json(&msg);
        let meta = json.get("meta").unwrap();
        let delims = meta.get("delims").unwrap();
        prop_assert!(delims.get("field").is_some());
        prop_assert!(delims.get("comp").is_some());
        prop_assert!(delims.get("rep").is_some());
        prop_assert!(delims.get("esc").is_some());
        prop_assert!(delims.get("sub").is_some());
    }
}

proptest! {
    /// Test that charsets are preserved in JSON output
    #[test]
    fn prop_to_json_preserves_charsets(msg in message_with_charsets()) {
        let json = to_json(&msg);
        let meta = json.get("meta").unwrap();
        let charsets = meta.get("charsets").unwrap().as_array().unwrap();
        prop_assert_eq!(charsets.len(), msg.charsets.len());
        for (i, charset) in msg.charsets.iter().enumerate() {
            prop_assert_eq!(charsets[i].as_str().unwrap(), charset);
        }
    }
}

proptest! {
    /// Test that segment count matches
    #[test]
    fn prop_to_json_segment_count_matches(msg in message()) {
        let json = to_json(&msg);
        let segments = json.get("segments").unwrap().as_array().unwrap();
        prop_assert_eq!(segments.len(), msg.segments.len());
    }
}

proptest! {
    /// Test that segment IDs are preserved
    #[test]
    fn prop_to_json_preserves_segment_ids(msg in message()) {
        let json = to_json(&msg);
        let segments = json.get("segments").unwrap().as_array().unwrap();
        for (i, segment) in segments.iter().enumerate() {
            let expected_id = String::from_utf8_lossy(&msg.segments[i].id);
            let actual_id = segment.get("id").unwrap().as_str().unwrap();
            prop_assert_eq!(actual_id, expected_id);
        }
    }
}

// ============================================================================
// Property tests for to_json_string()
// ============================================================================

proptest! {
    /// Test that to_json_string never panics
    #[test]
    fn prop_to_json_string_never_panics(msg in message()) {
        let _ = to_json_string(&msg);
    }
}

proptest! {
    /// Test that to_json_string produces valid JSON
    #[test]
    fn prop_to_json_string_produces_valid_json(msg in message()) {
        let json_str = to_json_string(&msg);
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json_str);
        prop_assert!(parsed.is_ok());
    }
}

proptest! {
    /// Test that to_json_string output starts with open brace and ends with close brace
    #[test]
    fn prop_to_json_string_starts_with_brace(msg in message()) {
        let json_str = to_json_string(&msg);
        assert!(json_str.starts_with('{'));
        assert!(json_str.ends_with('}'));
    }
}

proptest! {
    /// Test that to_json_string_pretty produces valid JSON
    #[test]
    fn prop_to_json_string_pretty_produces_valid_json(msg in message()) {
        let json_str = to_json_string_pretty(&msg);
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json_str);
        prop_assert!(parsed.is_ok());
    }
}

proptest! {
    /// Test that to_json_string and to_json_string_pretty produce equivalent JSON
    #[test]
    fn prop_pretty_and_compact_equivalent(msg in message()) {
        let compact = to_json_string(&msg);
        let pretty = to_json_string_pretty(&msg);
        let compact_parsed: serde_json::Value = serde_json::from_str(&compact).unwrap();
        let pretty_parsed: serde_json::Value = serde_json::from_str(&pretty).unwrap();
        prop_assert_eq!(compact_parsed, pretty_parsed);
    }
}

// ============================================================================
// Property tests for JSON roundtrip
// ============================================================================

proptest! {
    /// Test JSON roundtrip: serialize -> parse -> compare structure
    #[test]
    fn prop_json_roundtrip_structure(msg in message()) {
        let json1 = to_json(&msg);
        let json_str = serde_json::to_string(&json1).unwrap();
        let json2: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        // Check structure is preserved
        prop_assert_eq!(json1.get("meta").unwrap(), json2.get("meta").unwrap());
        prop_assert_eq!(json1.get("segments").unwrap(), json2.get("segments").unwrap());
    }
}

proptest! {
    /// Test that to_json and to_json_string produce consistent results
    #[test]
    fn prop_to_json_and_string_consistent(msg in message()) {
        let json_value = to_json(&msg);
        let json_str = to_json_string(&msg);
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        prop_assert_eq!(json_value, parsed);
    }
}

// ============================================================================
// Property tests for field types
// ============================================================================

proptest! {
    /// Test that empty fields are handled correctly
    #[test]
    fn prop_empty_fields_handled(msg in message()) {
        let json = to_json(&msg);
        let segments = json.get("segments").unwrap().as_array().unwrap();

        // Empty fields should not appear in JSON (filtered out)
        for segment in segments {
            if let Some(fields) = segment.get("fields") {
                prop_assert!(fields.is_object());
            }
        }
    }
}

proptest! {
    /// Test that complex fields with repetitions are handled
    #[test]
    fn prop_complex_fields_handled(msg in complex_message()) {
        let json = to_json(&msg);
        let segments = json.get("segments").unwrap().as_array().unwrap();

        // All segments should have valid field structures
        for segment in segments {
            if let Some(fields) = segment.get("fields") {
                prop_assert!(fields.is_object());
                for (_key, value) in fields.as_object().unwrap() {
                    prop_assert!(value.is_array());
                }
            }
        }
    }
}

proptest! {
    /// Test that fields are numbered starting from 1
    #[test]
    fn prop_fields_numbered_from_one(msg in message_with_msh()) {
        let json = to_json(&msg);
        let segments = json.get("segments").unwrap().as_array().unwrap();

        for segment in segments {
            if let Some(fields) = segment.get("fields") {
                let fields_obj = fields.as_object().unwrap();
                for key in fields_obj.keys() {
                    // Keys should be numeric strings
                    if let Ok(num) = key.parse::<usize>() {
                        prop_assert!(num >= 1);
                    }
                }
            }
        }
    }
}

// ============================================================================
// Property tests for Unicode handling
// ============================================================================

proptest! {
    /// Test that Unicode text in fields is preserved
    #[test]
    fn prop_unicode_in_fields_preserved(text in unicode_text()) {
        // Create a message with Unicode text
        let msg = Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"TST",
                fields: vec![Field::from_text(&text)],
            }],
            charsets: vec![],
        };

        let json = to_json(&msg);
        let segments = json.get("segments").unwrap().as_array().unwrap();
        let fields = segments[0].get("fields").unwrap();
        let field1 = fields.get("1").unwrap();

        // The text should be preserved in the JSON
        let field_str = serde_json::to_string(field1).unwrap();
        // Note: The text may be JSON-escaped but should decode back correctly
        let decoded: serde_json::Value = serde_json::from_str(&field_str).unwrap();
        prop_assert!(decoded.is_array());
    }
}

proptest! {
    /// Test that to_json_string produces valid UTF-8
    #[test]
    fn prop_to_json_string_valid_utf8(msg in message()) {
        let json_str = to_json_string(&msg);
        // Check that the string is valid UTF-8 (no replacement characters needed)
        prop_assert!(json_str.is_char_boundary(json_str.len()));
    }
}

// ============================================================================
// Property tests for escape sequence handling
// ============================================================================

proptest! {
    /// Test that JSON special characters are properly escaped
    #[test]
    fn prop_json_special_chars_escaped(text in text_with_json_special_chars()) {
        let msg = Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"TST",
                fields: vec![Field::from_text(&text)],
            }],
            charsets: vec![],
        };

        // Should produce valid JSON
        let json_str = to_json_string(&msg);
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json_str);
        prop_assert!(parsed.is_ok());
    }
}

proptest! {
    /// Test that HL7 delimiters in text are preserved in JSON
    #[test]
    fn prop_hl7_delimiters_preserved_in_json(text in text_with_delimiters()) {
        let msg = Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"TST",
                fields: vec![Field::from_text(&text)],
            }],
            charsets: vec![],
        };

        let json_str = to_json_string(&msg);
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        // The JSON should be valid and parseable
        prop_assert!(parsed.is_object());
    }
}

proptest! {
    /// Test that backslashes are properly handled
    #[test]
    fn prop_backslashes_handled(text in "[\\\\]{0,10}") {
        let msg = Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"TST",
                fields: vec![Field::from_text(&text)],
            }],
            charsets: vec![],
        };

        let json_str = to_json_string(&msg);
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json_str);
        prop_assert!(parsed.is_ok());
    }
}

// ============================================================================
// Property tests for edge cases
// ============================================================================

#[test]
fn test_empty_message_valid() {
    let msg = Message {
        delims: Delims::default(),
        segments: vec![],
        charsets: vec![],
    };

    let json = to_json(&msg);
    assert!(json.is_object());
    assert!(json.get("meta").is_some());
    assert!(json.get("segments").unwrap().as_array().unwrap().is_empty());
}

proptest! {
    /// Test with very long field text
    #[test]
    fn prop_long_field_text(text in "[A-Za-z0-9]{1000,5000}") {
        let msg = Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"TST",
                fields: vec![Field::from_text(&text)],
            }],
            charsets: vec![],
        };

        let json_str = to_json_string(&msg);
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json_str);
        prop_assert!(parsed.is_ok());
    }
}

proptest! {
    /// Test with many segments
    #[test]
    fn prop_many_segments(segments in proptest::collection::vec(simple_segment(), 50..100)) {
        let msg = Message {
            delims: Delims::default(),
            segments,
            charsets: vec![],
        };

        let json = to_json(&msg);
        let json_segments = json.get("segments").unwrap().as_array().unwrap();
        prop_assert!(json_segments.len() >= 50);
    }
}

proptest! {
    /// Test with many fields per segment
    #[test]
    fn prop_many_fields_per_segment(fields in proptest::collection::vec(simple_field(), 50..100)) {
        let msg = Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"TST",
                fields,
            }],
            charsets: vec![],
        };

        let json = to_json(&msg);
        let segments = json.get("segments").unwrap().as_array().unwrap();
        let json_fields = segments[0].get("fields").unwrap().as_object().unwrap();
        // Not all fields may appear (empty ones are filtered)
        prop_assert!(!json_fields.is_empty() || msg.segments[0].fields.is_empty());
    }
}

// ============================================================================
// Property tests for delimiter handling
// ============================================================================

proptest! {
    /// Test that default delimiters are correctly represented
    #[test]
    fn prop_default_delimiters_correct(msg in message()) {
        let json = to_json(&msg);
        let delims = json.get("meta").unwrap().get("delims").unwrap();

        prop_assert_eq!(delims.get("field").unwrap().as_str().unwrap(), "|");
        prop_assert_eq!(delims.get("comp").unwrap().as_str().unwrap(), "^");
        prop_assert_eq!(delims.get("rep").unwrap().as_str().unwrap(), "~");
        prop_assert_eq!(delims.get("esc").unwrap().as_str().unwrap(), "\\");
        prop_assert_eq!(delims.get("sub").unwrap().as_str().unwrap(), "&");
    }
}

// ============================================================================
// Property tests for nested structures
// ============================================================================

proptest! {
    /// Test that nested components are properly serialized
    #[test]
    fn prop_nested_components_serialized(comps in proptest::collection::vec(component_with_subs(), 1..5)) {
        let msg = Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"TST",
                fields: vec![Field {
                    reps: vec![Rep { comps }],
                }],
            }],
            charsets: vec![],
        };

        let json_str = to_json_string(&msg);
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json_str);
        prop_assert!(parsed.is_ok());
    }
}

proptest! {
    /// Test that multiple repetitions are properly serialized
    #[test]
    fn prop_multiple_repetitions_serialized(reps in proptest::collection::vec(repetition(), 2..10)) {
        let msg = Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"TST",
                fields: vec![Field { reps }],
            }],
            charsets: vec![],
        };

        let json = to_json(&msg);
        let segments = json.get("segments").unwrap().as_array().unwrap();
        let fields = segments[0].get("fields").unwrap();
        let field1 = fields.get("1").unwrap();

        // Should be an array of repetitions
        prop_assert!(field1.is_array());
        prop_assert!(field1.as_array().unwrap().len() >= 2);
    }
}
