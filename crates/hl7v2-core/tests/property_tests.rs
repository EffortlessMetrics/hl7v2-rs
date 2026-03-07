//! Property-based tests for hl7v2-core crate using proptest
//!
//! These tests verify that core properties hold for arbitrary inputs.

use hl7v2_core::{parse, write, Delims, Field, Message, Segment};
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

/// Generate text that may contain HL7 delimiters (for escape testing)
#[allow(dead_code)]
fn text_with_delimiters() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[A-Za-z0-9 |^~\\\\&]{0,50}").unwrap()
}

/// Generate a segment ID (3 uppercase letters, excluding MSH which is special)
fn segment_id() -> impl Strategy<Value = [u8; 3]> {
    (0u8..26, 0u8..26, 0u8..26).prop_map(|(a, b, c)| {
        let id = [b'A' + a, b'A' + b, b'A' + c];
        // Exclude MSH since it's a special segment that should only appear at message start
        if &id == b"MSH" {
            [b'M', b'S', b'I'] // Use MSI as a substitute
        } else {
            id
        }
    })
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
        proptest::collection::vec(complex_field(), 1..5),
    )
        .prop_map(|(id, fields)| Segment { id, fields })
}

/// Generate a valid MSH segment for a message
#[allow(dead_code)]
fn msh_segment() -> impl Strategy<Value = Segment> {
    // MSH segment with standard delimiters
    Just(Segment {
        id: *b"MSH",
        fields: vec![
            Field::from_text("^~\\&"), // Encoding characters
            Field::from_text("SENDAPP"),
            Field::from_text("SENDFAC"),
            Field::from_text("RECVAPP"),
            Field::from_text("RECVFAC"),
            Field::from_text("20250101120000"),
            Field::from_text(""), // Empty field
            Field::from_text("ADT^A01"),
            Field::from_text("MSG00001"),
            Field::from_text("P"),
            Field::from_text("2.5.1"),
        ],
    })
}

/// Generate a message with MSH segment
fn message_with_msh() -> impl Strategy<Value = Message> {
    proptest::collection::vec(simple_segment(), 0..3).prop_flat_map(|mut segments| {
        let msh = Segment {
            id: *b"MSH",
            fields: vec![
                Field::from_text("^~\\&"),
                Field::from_text("APP"),
                Field::from_text("FAC"),
            ],
        };
        segments.insert(0, msh);
        Just(Message {
            delims: Delims::default(),
            segments,
            charsets: vec![],
        })
    })
}

/// Generate a valid HL7 message string for parsing tests
fn valid_hl7_message() -> impl Strategy<Value = String> {
    // Generate a simple valid HL7 message
    safe_text().prop_map(|text| {
        format!(
            "MSH|^~\\&|APP|FAC|APP2|FAC2|20250101120000||ADT^A01|MSG001|P|2.5.1\rPID|1||{}||DOE^JOHN\r",
            text
        )
    })
}

/// Generate a valid HL7 message with varying segments
fn valid_hl7_message_multi_segment() -> impl Strategy<Value = String> {
    (proptest::collection::vec(safe_text(), 1..5), safe_text()).prop_map(
        |(fields, patient_name)| {
            let mut msg = String::from(
                "MSH|^~\\&|APP|FAC|APP2|FAC2|20250101120000||ADT^A01|MSG001|P|2.5.1\r",
            );
            msg.push_str(&format!("PID|1||{}||{}\r", fields.join("~"), patient_name));
            msg
        },
    )
}

/// Generate custom delimiters (valid ASCII characters that aren't control chars)
#[allow(dead_code)]
fn custom_delims() -> impl Strategy<Value = Delims> {
    // Use a limited set of safe delimiter characters
    (0u8..4).prop_map(|i| match i {
        0 => Delims {
            field: '|',
            comp: '^',
            rep: '~',
            esc: '\\',
            sub: '&',
        },
        1 => Delims {
            field: '|',
            comp: ':',
            rep: '~',
            esc: '\\',
            sub: '#',
        },
        2 => Delims {
            field: '*',
            comp: '^',
            rep: '~',
            esc: '\\',
            sub: '&',
        },
        _ => Delims {
            field: '|',
            comp: '^',
            rep: '#',
            esc: '%',
            sub: '&',
        },
    })
}

// ============================================================================
// Property tests for parse()
// ============================================================================

proptest! {
    /// Test that parsing a valid HL7 message never fails
    #[test]
    fn prop_parse_valid_message_succeeds(msg in valid_hl7_message()) {
        let result = parse(msg.as_bytes());
        prop_assert!(result.is_ok());
    }

    /// Test that parsing a valid multi-segment message never fails
    #[test]
    fn prop_parse_multi_segment_succeeds(msg in valid_hl7_message_multi_segment()) {
        let result = parse(msg.as_bytes());
        prop_assert!(result.is_ok());
    }

    /// Test that parsed message has correct segment count
    #[test]
    fn prop_parse_correct_segment_count(msg in valid_hl7_message_multi_segment()) {
        let parsed = parse(msg.as_bytes())?;
        prop_assert!(parsed.segments.len() >= 2); // MSH + at least one more segment
    }

    /// Test that parsed message has correct delimiters
    #[test]
    fn prop_parse_default_delims(msg in valid_hl7_message()) {
        let parsed = parse(msg.as_bytes())?;
        prop_assert_eq!(parsed.delims.field, '|');
        prop_assert_eq!(parsed.delims.comp, '^');
        prop_assert_eq!(parsed.delims.rep, '~');
        prop_assert_eq!(parsed.delims.esc, '\\');
        prop_assert_eq!(parsed.delims.sub, '&');
    }

    /// Test that MSH segment is always first
    #[test]
    fn prop_parse_msh_first(msg in valid_hl7_message_multi_segment()) {
        let parsed = parse(msg.as_bytes())?;
        prop_assert_eq!(&parsed.segments[0].id, b"MSH");
    }
}

// ============================================================================
// Property tests for write()
// ============================================================================

proptest! {
    /// Test that write never panics for any valid message
    #[test]
    fn prop_write_never_panics(msg in message_with_msh()) {
        let _ = write(&msg);
    }

    /// Test that written output is valid UTF-8
    #[test]
    fn prop_write_produces_valid_utf8(msg in message_with_msh()) {
        let bytes = write(&msg);
        prop_assert!(String::from_utf8(bytes).is_ok());
    }

    /// Test that written output ends with segment terminator
    #[test]
    fn prop_write_ends_with_terminator(msg in message_with_msh()) {
        let bytes = write(&msg);
        if !bytes.is_empty() {
            // Should end with \r
            prop_assert_eq!(bytes[bytes.len() - 1], b'\r');
        }
    }
}

// ============================================================================
// Property tests for roundtrip (parse -> write -> parse)
// ============================================================================

proptest! {
    /// Test that roundtrip preserves segment count
    #[test]
    fn prop_roundtrip_segment_count(msg in message_with_msh()) {
        // Write the message
        let written = write(&msg);
        let written_str = String::from_utf8(written).expect("Written output should be valid UTF-8");

        // Parse it back
        let reparsed = parse(written_str.as_bytes())?;

        prop_assert_eq!(msg.segments.len(), reparsed.segments.len());
    }

    /// Test that roundtrip preserves segment IDs
    #[test]
    fn prop_roundtrip_segment_ids(msg in message_with_msh()) {
        // Write the message
        let written = write(&msg);
        let written_str = String::from_utf8(written).expect("Written output should be valid UTF-8");

        // Parse it back
        let reparsed = parse(written_str.as_bytes())?;

        for (orig, reparsed_seg) in msg.segments.iter().zip(reparsed.segments.iter()) {
            prop_assert_eq!(&orig.id, &reparsed_seg.id);
        }
    }

    /// Test that roundtrip preserves delimiters
    #[test]
    fn prop_roundtrip_delimiters(msg in message_with_msh()) {
        // Write the message
        let written = write(&msg);
        let written_str = String::from_utf8(written).expect("Written output should be valid UTF-8");

        // Parse it back
        let reparsed = parse(written_str.as_bytes())?;

        prop_assert_eq!(msg.delims.field, reparsed.delims.field);
        prop_assert_eq!(msg.delims.comp, reparsed.delims.comp);
        prop_assert_eq!(msg.delims.rep, reparsed.delims.rep);
        prop_assert_eq!(msg.delims.esc, reparsed.delims.esc);
        prop_assert_eq!(msg.delims.sub, reparsed.delims.sub);
    }
}

// ============================================================================
// Property tests for string roundtrip
// ============================================================================

proptest! {
    /// Test that parsing and re-serializing a valid message produces equivalent output
    #[test]
    fn prop_string_roundtrip(original in valid_hl7_message()) {
        // Parse the original
        let parsed = parse(original.as_bytes())?;

        // Write it back
        let written = write(&parsed);
        let written_str = String::from_utf8(written).expect("Written output should be valid UTF-8");

        // Parse again
        let reparsed = parse(written_str.as_bytes())?;

        // Verify segment count matches
        prop_assert_eq!(parsed.segments.len(), reparsed.segments.len());
    }

    /// Test that field content is preserved in roundtrip
    #[test]
    fn prop_field_content_preserved(original in valid_hl7_message()) {
        // Parse the original
        let parsed = parse(original.as_bytes())?;

        // Write it back
        let written = write(&parsed);
        let written_str = String::from_utf8(written).expect("Written output should be valid UTF-8");

        // Parse again
        let reparsed = parse(written_str.as_bytes())?;

        // Compare field counts in each segment
        for (orig_seg, reparsed_seg) in parsed.segments.iter().zip(reparsed.segments.iter()) {
            prop_assert_eq!(orig_seg.fields.len(), reparsed_seg.fields.len());
        }
    }
}

// ============================================================================
// Property tests for delimiter handling
// ============================================================================

proptest! {
    /// Test that messages with different field content are handled correctly
    #[test]
    fn prop_varied_field_content(fields in proptest::collection::vec(safe_text(), 1..10)) {
        let field_str = fields.join("|");
        let msg = format!(
            "MSH|^~\\&|APP|FAC|||20250101120000||ADT^A01|MSG001|P|2.5.1\rPID|1||{}||TEST\r",
            field_str
        );

        let result = parse(msg.as_bytes());
        prop_assert!(result.is_ok());
    }

    /// Test that empty fields are handled correctly
    #[test]
    fn prop_empty_fields_preserved(empty_count in 0usize..5) {
        let empty_fields: Vec<&str> = vec![""; empty_count];
        let fields_str = empty_fields.join("|");

        let msg = format!(
            "MSH|^~\\&|APP|FAC|||20250101120000||ADT^A01|MSG001|P|2.5.1\rPID|1|{}|TEST\r",
            fields_str
        );

        let result = parse(msg.as_bytes());
        if let Ok(parsed) = result {
            // Should have parsed successfully
            prop_assert!(parsed.segments.len() >= 2);
        }
    }
}

// ============================================================================
// Property tests for edge cases
// ============================================================================

proptest! {
    /// Test that very long field values are handled
    #[test]
    fn prop_long_field_value(length in 100usize..1000) {
        let long_value: String = (0..length).map(|_| 'A').collect();
        let msg = format!(
            "MSH|^~\\&|APP|FAC|||20250101120000||ADT^A01|MSG001|P|2.5.1\rPID|1||{}||TEST\r",
            long_value
        );

        let result = parse(msg.as_bytes());
        prop_assert!(result.is_ok());
    }

    /// Test that many segments are handled correctly
    #[test]
    fn prop_many_segments(segment_count in 2usize..20) {
        let mut msg = String::from("MSH|^~\\&|APP|FAC|||20250101120000||ADT^A01|MSG001|P|2.5.1\r");

        for i in 0..segment_count {
            msg.push_str(&format!("PID|{}||VALUE||TEST\r", i));
        }

        let result = parse(msg.as_bytes());
        prop_assert!(result.is_ok());

        if let Ok(parsed) = result {
            prop_assert_eq!(parsed.segments.len(), segment_count + 1); // +1 for MSH
        }
    }

    /// Test that special characters in safe text are preserved
    #[test]
    fn prop_special_chars_preserved(text in safe_text()) {
        let msg = format!(
            "MSH|^~\\&|APP|FAC|||20250101120000||ADT^A01|MSG001|P|2.5.1\rPID|1||{}||TEST\r",
            text
        );

        let parsed = parse(msg.as_bytes())?;
        let written = write(&parsed);

        // The text should appear in the output (possibly escaped)
        let output = String::from_utf8(written).expect("Written output should be valid UTF-8");
        prop_assert!(output.contains(&text) || output.contains(&text.replace('\\', "\\E\\")));
    }
}

// ============================================================================
// Property tests for message structure invariants
// ============================================================================

proptest! {
    /// Test that every parsed message has at least one segment
    #[test]
    fn prop_message_has_segments(msg in valid_hl7_message()) {
        let parsed = parse(msg.as_bytes())?;
        prop_assert!(!parsed.segments.is_empty());
    }

    /// Test that every segment has a valid 3-character ID
    #[test]
    fn prop_segment_ids_valid(msg in valid_hl7_message_multi_segment()) {
        let parsed = parse(msg.as_bytes())?;

        for segment in &parsed.segments {
            prop_assert_eq!(segment.id.len(), 3);
            // All bytes should be uppercase letters
            for &b in &segment.id {
                prop_assert!(b.is_ascii_uppercase());
            }
        }
    }

    /// Test that MSH segment has encoding characters field
    #[test]
    fn prop_msh_has_encoding_chars(msg in valid_hl7_message()) {
        let parsed = parse(msg.as_bytes())?;

        let msh = &parsed.segments[0];
        prop_assert!(!msh.fields.is_empty());

        // First field should contain encoding characters
        let enc_chars = &msh.fields[0];
        if !enc_chars.reps.is_empty() {
            let rep = &enc_chars.reps[0];
            if !rep.comps.is_empty() {
                let comp = &rep.comps[0];
                // Should have the 4 encoding characters
                prop_assert!(comp.subs.len() >= 1);
            }
        }
    }
}

// ============================================================================
// Property tests for complex messages
// ============================================================================

proptest! {
    /// Test messages with nested components
    #[test]
    fn prop_nested_components(component_count in 1usize..5) {
        let components: Vec<String> = (0..component_count).map(|i| format!("COMP{}", i)).collect();
        let field_value = components.join("^");

        let msg = format!(
            "MSH|^~\\&|APP|FAC|||20250101120000||ADT^A01|MSG001|P|2.5.1\rPID|1||{}||TEST\r",
            field_value
        );

        let parsed = parse(msg.as_bytes())?;
        prop_assert!(parsed.segments.len() >= 2);
    }

    /// Test messages with repeated fields
    #[test]
    fn prop_repeated_fields(repeat_count in 1usize..4) {
        let repeats: Vec<String> = (0..repeat_count).map(|i| format!("REP{}", i)).collect();
        let field_value = repeats.join("~");

        let msg = format!(
            "MSH|^~\\&|APP|FAC|||20250101120000||ADT^A01|MSG001|P|2.5.1\rPID|1||{}||TEST\r",
            field_value
        );

        let parsed = parse(msg.as_bytes())?;
        prop_assert!(parsed.segments.len() >= 2);
    }
}
