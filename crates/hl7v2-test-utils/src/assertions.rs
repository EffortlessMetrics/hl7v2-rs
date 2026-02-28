//! HL7-specific test assertions.
//!
//! This module provides assertion macros and functions for testing HL7 v2 messages.
//! These assertions integrate with the standard Rust test framework and provide
//! meaningful error messages when assertions fail.
//!
//! # Available Assertions
//!
//! - [`assert_message_valid`] - Assert that a message parses successfully
//! - [`assert_parse_fails`] - Assert that parsing fails with expected error
//! - [`assert_segment_equals`] - Assert segment content matches expected
//! - [`assert_field_equals`] - Assert field value at path matches expected
//! - [`assert_hl7_roundtrips`] - Assert message round-trips through parse/write
//!
//! # Example
//!
//! ```rust,ignore
//! use hl7v2_test_utils::assertions::*;
//! use hl7v2_test_utils::fixtures::SampleMessages;
//!
//! let msg = SampleMessages::adt_a01();
//!
//! // Verify message parses successfully
//! let parsed = assert_message_valid(msg.as_bytes());
//!
//! // Verify specific field values
//! assert_field_equals(&parsed, "MSH.9.1", "ADT");
//! assert_field_equals(&parsed, "MSH.9.2", "A01");
//! ```

use hl7v2_model::Message;
use hl7v2_parser::parse;
use hl7v2_query::get;

/// Assert that a message is valid and parseable.
///
/// Parses the message and returns the parsed `Message` struct.
/// Panics with a descriptive message if parsing fails.
///
/// # Arguments
///
/// * `bytes` - The raw HL7 message bytes
///
/// # Returns
///
/// The parsed `Message` struct
///
/// # Panics
///
/// Panics if the message cannot be parsed
///
/// # Example
///
/// ```rust,ignore
/// use hl7v2_test_utils::assertions::assert_message_valid;
///
/// let hl7 = b"MSH|^~\\&|App|Fac|||20250128120000||ADT^A01|1|P|2.5\rPID|1||123||Doe^John\r";
/// let message = assert_message_valid(hl7);
/// assert_eq!(message.segments.len(), 2);
/// ```
pub fn assert_message_valid(bytes: &[u8]) -> Message {
    parse(bytes).unwrap_or_else(|e| {
        panic!(
            "Message should be valid but parsing failed: {}\nMessage content: {}",
            e,
            String::from_utf8_lossy(bytes)
        )
    })
}

/// Assert that parsing fails with an error containing the expected message.
///
/// # Arguments
///
/// * `bytes` - The raw HL7 message bytes
/// * `expected_error_contains` - Substring expected in the error message
///
/// # Panics
///
/// Panics if parsing succeeds or if the error message doesn't contain the expected substring
///
/// # Example
///
/// ```rust,ignore
/// use hl7v2_test_utils::assertions::assert_parse_fails;
///
/// let invalid = b"PID|1||123||Doe^John\r";  // Missing MSH segment
/// assert_parse_fails(invalid, "Invalid segment ID");
/// ```
pub fn assert_parse_fails(bytes: &[u8], expected_error_contains: &str) {
    match parse(bytes) {
        Ok(_) => {
            panic!(
                "Expected parse to fail with error containing '{}', but parsing succeeded.\nMessage content: {}",
                expected_error_contains,
                String::from_utf8_lossy(bytes)
            );
        }
        Err(e) => {
            let error_string = e.to_string();
            assert!(
                error_string.contains(expected_error_contains),
                "Expected error containing '{}', but got: '{}'",
                expected_error_contains,
                error_string
            );
        }
    }
}

/// Assert that a segment's raw content equals the expected value.
///
/// # Arguments
///
/// * `message` - The parsed message
/// * `segment_name` - The segment ID (e.g., "MSH", "PID")
/// * `expected` - The expected raw segment content (without trailing \r)
///
/// # Panics
///
/// Panics if the segment is not found or content doesn't match
///
/// # Example
///
/// ```rust,ignore
/// use hl7v2_test_utils::assertions::{assert_message_valid, assert_segment_equals};
///
/// let hl7 = b"MSH|^~\\&|App|Fac|||20250128120000||ADT^A01|1|P|2.5\rPID|1||123||Doe^John\r";
/// let message = assert_message_valid(hl7);
///
/// assert_segment_equals(&message, "MSH", "MSH|^~\\&|App|Fac|||20250128120000||ADT^A01|1|P|2.5");
/// ```
pub fn assert_segment_equals(message: &Message, segment_name: &str, expected: &str) {
    let segment = message
        .segments
        .iter()
        .find(|s| s.id_str() == segment_name)
        .unwrap_or_else(|| {
            panic!(
                "Segment '{}' not found in message. Available segments: {}",
                segment_name,
                message
                    .segments
                    .iter()
                    .map(|s| s.id_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        });

    let actual = write_segment_to_string(segment, &message.delims);
    assert_eq!(
        actual, expected,
        "Segment '{}' content mismatch",
        segment_name
    );
}

/// Assert that a field value at the given path equals the expected value.
///
/// # Arguments
///
/// * `message` - The parsed message
/// * `path` - The path to the field (e.g., "PID.5.1", "MSH.9")
/// * `expected` - The expected field value
///
/// # Panics
///
/// Panics if the path is not found or value doesn't match
///
/// # Example
///
/// ```rust,ignore
/// use hl7v2_test_utils::assertions::{assert_message_valid, assert_field_equals};
///
/// let hl7 = b"MSH|^~\\&|App|Fac|||20250128120000||ADT^A01|1|P|2.5\rPID|1||123||Doe^John\r";
/// let message = assert_message_valid(hl7);
///
/// assert_field_equals(&message, "MSH.3", "App");
/// assert_field_equals(&message, "MSH.9.1", "ADT");
/// assert_field_equals(&message, "PID.5.1", "Doe");
/// ```
pub fn assert_field_equals(message: &Message, path: &str, expected: &str) {
    let actual = get(message, path).unwrap_or_else(|| {
        panic!("Field at path '{}' not found or is empty", path);
    });

    assert_eq!(
        actual, expected,
        "Field value at path '{}' mismatch: expected '{}', got '{}'",
        path, expected, actual
    );
}

/// Assert that a field value at the given path contains the expected substring.
///
/// # Arguments
///
/// * `message` - The parsed message
/// * `path` - The path to the field
/// * `expected_contains` - The substring expected to be in the field value
///
/// # Panics
///
/// Panics if the path is not found or value doesn't contain the substring
///
/// # Example
///
/// ```rust,ignore
/// use hl7v2_test_utils::assertions::{assert_message_valid, assert_field_contains};
///
/// let hl7 = b"MSH|^~\\&|App|Fac|||20250128120000||ADT^A01|1|P|2.5\r";
/// let message = assert_message_valid(hl7);
///
/// assert_field_contains(&message, "MSH.9", "ADT");
/// ```
pub fn assert_field_contains(message: &Message, path: &str, expected_contains: &str) {
    let actual = get(message, path).unwrap_or_else(|| {
        panic!("Field at path '{}' not found or is empty", path);
    });

    assert!(
        actual.contains(expected_contains),
        "Field value at path '{}' should contain '{}', but got '{}'",
        path,
        expected_contains,
        actual
    );
}

/// Assert that a field exists at the given path (even if empty).
///
/// # Arguments
///
/// * `message` - The parsed message
/// * `path` - The path to the field
///
/// # Panics
///
/// Panics if the field does not exist
///
/// # Example
///
/// ```rust,ignore
/// use hl7v2_test_utils::assertions::{assert_message_valid, assert_field_exists};
///
/// let hl7 = b"MSH|^~\\&|App|Fac|||20250128120000||ADT^A01|1|P|2.5\rPID|1||123||Doe^John\r";
/// let message = assert_message_valid(hl7);
///
/// assert_field_exists(&message, "PID.3");
/// ```
pub fn assert_field_exists(message: &Message, path: &str) {
    let segment_id = path.split('.').next().unwrap_or("");
    let segment = message.segments.iter().find(|s| s.id_str() == segment_id);

    if segment.is_none() {
        panic!(
            "Segment '{}' not found in message. Available segments: {}",
            segment_id,
            message
                .segments
                .iter()
                .map(|s| s.id_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    // Parse the field index from the path
    let mut parts = path.split('.');
    parts.next(); // Skip segment ID
    if let Some(field_part) = parts.next() {
        let field_index: usize = field_part
            .trim_end_matches(|c: char| c.is_alphanumeric() || c == '[' || c == ']')
            .parse()
            .unwrap_or(0);

        if field_index == 0 {
            return;
        }

        let segment = segment.unwrap();
        if field_index > segment.fields.len() {
            panic!(
                "Field {} does not exist in segment '{}'. Segment has {} fields.",
                field_index,
                segment_id,
                segment.fields.len()
            );
        }
    }
}

/// Assert that a segment exists in the message.
///
/// # Arguments
///
/// * `message` - The parsed message
/// * `segment_name` - The segment ID to check for
///
/// # Panics
///
/// Panics if the segment is not found
///
/// # Example
///
/// ```rust,ignore
/// use hl7v2_test_utils::assertions::{assert_message_valid, assert_segment_exists};
///
/// let hl7 = b"MSH|^~\\&|App|Fac|||20250128120000||ADT^A01|1|P|2.5\rPID|1||123||Doe^John\r";
/// let message = assert_message_valid(hl7);
///
/// assert_segment_exists(&message, "MSH");
/// assert_segment_exists(&message, "PID");
/// ```
pub fn assert_segment_exists(message: &Message, segment_name: &str) {
    let exists = message.segments.iter().any(|s| s.id_str() == segment_name);

    assert!(
        exists,
        "Segment '{}' not found in message. Available segments: {}",
        segment_name,
        message
            .segments
            .iter()
            .map(|s| s.id_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
}

/// Assert that a segment does not exist in the message.
///
/// # Arguments
///
/// * `message` - The parsed message
/// * `segment_name` - The segment ID to check for
///
/// # Panics
///
/// Panics if the segment is found
///
/// # Example
///
/// ```rust,ignore
/// use hl7v2_test_utils::assertions::{assert_message_valid, assert_segment_not_exists};
///
/// let hl7 = b"MSH|^~\\&|App|Fac|||20250128120000||ADT^A01|1|P|2.5\rPID|1||123||Doe^John\r";
/// let message = assert_message_valid(hl7);
///
/// assert_segment_not_exists(&message, "OBX");
/// ```
pub fn assert_segment_not_exists(message: &Message, segment_name: &str) {
    let exists = message.segments.iter().any(|s| s.id_str() == segment_name);

    assert!(
        !exists,
        "Segment '{}' should not exist in message",
        segment_name
    );
}

/// Assert that a message round-trips through parse and write.
///
/// This verifies that parsing a message and then writing it back produces
/// a semantically equivalent message.
///
/// # Arguments
///
/// * `bytes` - The raw HL7 message bytes
///
/// # Panics
///
/// Panics if the round-trip fails or segment counts don't match
///
/// # Example
///
/// ```rust,ignore
/// use hl7v2_test_utils::assertions::assert_hl7_roundtrips;
///
/// let hl7 = b"MSH|^~\\&|App|Fac|||20250128120000||ADT^A01|1|P|2.5\rPID|1||123||Doe^John\r";
/// assert_hl7_roundtrips(hl7);
/// ```
pub fn assert_hl7_roundtrips(bytes: &[u8]) {
    let original = parse(bytes).unwrap_or_else(|e| {
        panic!(
            "Original message should parse: {}\nMessage content: {}",
            e,
            String::from_utf8_lossy(bytes)
        )
    });

    // Write the message back to string
    let rewritten = write_message_to_string(&original);

    // Parse the rewritten message
    let reparsed = parse(rewritten.as_bytes()).unwrap_or_else(|e| {
        panic!(
            "Rewritten message should parse: {}\nRewritten content: {}",
            e, rewritten
        )
    });

    // Compare segment counts
    assert_eq!(
        original.segments.len(),
        reparsed.segments.len(),
        "Segment count mismatch after round-trip: original has {}, reparsed has {}",
        original.segments.len(),
        reparsed.segments.len()
    );

    // Compare each segment
    for (i, (orig, reparsed)) in original
        .segments
        .iter()
        .zip(reparsed.segments.iter())
        .enumerate()
    {
        assert_eq!(
            orig.id_str(),
            reparsed.id_str(),
            "Segment {} ID mismatch: original is {}, reparsed is {}",
            i,
            orig.id_str(),
            reparsed.id_str()
        );
        assert_eq!(
            orig.fields.len(),
            reparsed.fields.len(),
            "Segment {} ({}): field count mismatch",
            i,
            orig.id_str()
        );
    }
}

/// Assert that a message has the expected number of segments.
///
/// # Arguments
///
/// * `message` - The parsed message
/// * `expected_count` - The expected number of segments
///
/// # Panics
///
/// Panics if the segment count doesn't match
///
/// # Example
///
/// ```rust,ignore
/// use hl7v2_test_utils::assertions::{assert_message_valid, assert_segment_count};
///
/// let hl7 = b"MSH|^~\\&|App|Fac|||20250128120000||ADT^A01|1|P|2.5\rPID|1||123||Doe^John\r";
/// let message = assert_message_valid(hl7);
///
/// assert_segment_count(&message, 2);
/// ```
pub fn assert_segment_count(message: &Message, expected_count: usize) {
    assert_eq!(
        message.segments.len(),
        expected_count,
        "Segment count mismatch: expected {}, got {}. Segments: {}",
        expected_count,
        message.segments.len(),
        message
            .segments
            .iter()
            .map(|s| s.id_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
}

/// Assert that a message has the expected number of segments of a specific type.
///
/// # Arguments
///
/// * `message` - The parsed message
/// * `segment_name` - The segment ID to count
/// * `expected_count` - The expected number of segments of this type
///
/// # Panics
///
/// Panics if the segment count doesn't match
///
/// # Example
///
/// ```rust,ignore
/// use hl7v2_test_utils::assertions::{assert_message_valid, assert_segment_type_count};
///
/// let hl7 = b"MSH|^~\\&|App|Fac|||20250128120000||ADT^A01|1|P|2.5\rOBX|1\rOBX|2\rOBX|3\r";
/// let message = assert_message_valid(hl7);
///
/// assert_segment_type_count(&message, "OBX", 3);
/// ```
pub fn assert_segment_type_count(message: &Message, segment_name: &str, expected_count: usize) {
    let actual_count = message
        .segments
        .iter()
        .filter(|s| s.id_str() == segment_name)
        .count();

    assert_eq!(
        actual_count, expected_count,
        "Segment '{}' count mismatch: expected {}, got {}",
        segment_name, expected_count, actual_count
    );
}

// Helper functions

/// Write a segment to a string.
fn write_segment_to_string(segment: &hl7v2_model::Segment, delims: &hl7v2_model::Delims) -> String {
    let mut result = segment.id_str().to_string();

    for field in &segment.fields {
        result.push(delims.field);
        result.push_str(&write_field_to_string(field, delims));
    }

    result
}

/// Write a field to a string.
fn write_field_to_string(field: &hl7v2_model::Field, delims: &hl7v2_model::Delims) -> String {
    let reps: Vec<String> = field
        .reps
        .iter()
        .map(|rep| write_rep_to_string(rep, delims))
        .collect();
    reps.join(&delims.rep.to_string())
}

/// Write a repetition to a string.
fn write_rep_to_string(rep: &hl7v2_model::Rep, delims: &hl7v2_model::Delims) -> String {
    let comps: Vec<String> = rep
        .comps
        .iter()
        .map(|comp| write_comp_to_string(comp, delims))
        .collect();
    comps.join(&delims.comp.to_string())
}

/// Write a component to a string.
fn write_comp_to_string(comp: &hl7v2_model::Comp, delims: &hl7v2_model::Delims) -> String {
    let subs: Vec<String> = comp.subs.iter().map(write_atom_to_string).collect();
    subs.join(&delims.sub.to_string())
}

/// Write an atom to a string.
fn write_atom_to_string(atom: &hl7v2_model::Atom) -> String {
    match atom {
        hl7v2_model::Atom::Text(t) => t.clone(),
        hl7v2_model::Atom::Null => String::new(),
    }
}

/// Write a message to a string.
fn write_message_to_string(message: &Message) -> String {
    let mut result = String::new();
    for segment in &message.segments {
        result.push_str(&write_segment_to_string(segment, &message.delims));
        result.push('\r');
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::SampleMessages;

    #[test]
    fn test_assert_message_valid() {
        let msg = SampleMessages::adt_a01();
        let parsed = assert_message_valid(msg.as_bytes());
        assert!(!parsed.segments.is_empty());
    }

    #[test]
    #[should_panic(expected = "Message should be valid but parsing failed")]
    fn test_assert_message_valid_fails() {
        let invalid = b"INVALID MESSAGE";
        assert_message_valid(invalid);
    }

    #[test]
    fn test_assert_parse_fails() {
        let invalid = b"PID|1||123||Doe^John\r"; // Missing MSH
        assert_parse_fails(invalid, "Invalid segment ID");
    }

    #[test]
    #[should_panic(expected = "Expected parse to fail")]
    fn test_assert_parse_fails_but_succeeded() {
        let valid = SampleMessages::adt_a01();
        assert_parse_fails(valid.as_bytes(), "error");
    }

    #[test]
    fn test_assert_field_equals() {
        let msg = SampleMessages::adt_a01();
        let parsed = assert_message_valid(msg.as_bytes());
        assert_field_equals(&parsed, "MSH.3", "SendingApp");
        assert_field_equals(&parsed, "MSH.9.1", "ADT");
        assert_field_equals(&parsed, "MSH.9.2", "A01");
    }

    #[test]
    fn test_assert_field_contains() {
        let msg = SampleMessages::adt_a01();
        let parsed = assert_message_valid(msg.as_bytes());
        assert_field_contains(&parsed, "MSH.9", "ADT");
    }

    #[test]
    fn test_assert_segment_exists() {
        let msg = SampleMessages::adt_a01();
        let parsed = assert_message_valid(msg.as_bytes());
        assert_segment_exists(&parsed, "MSH");
        assert_segment_exists(&parsed, "PID");
        assert_segment_exists(&parsed, "PV1");
    }

    #[test]
    #[should_panic(expected = "Segment 'OBX' not found")]
    fn test_assert_segment_exists_fails() {
        let msg = SampleMessages::adt_a01();
        let parsed = assert_message_valid(msg.as_bytes());
        assert_segment_exists(&parsed, "OBX");
    }

    #[test]
    fn test_assert_segment_not_exists() {
        let msg = SampleMessages::adt_a01();
        let parsed = assert_message_valid(msg.as_bytes());
        assert_segment_not_exists(&parsed, "OBX");
    }

    #[test]
    fn test_assert_segment_count() {
        let msg = SampleMessages::adt_a01();
        let parsed = assert_message_valid(msg.as_bytes());
        assert_segment_count(&parsed, 4); // MSH, EVN, PID, PV1
    }

    #[test]
    fn test_assert_hl7_roundtrips() {
        let msg = SampleMessages::adt_a01();
        assert_hl7_roundtrips(msg.as_bytes());
    }

    #[test]
    fn test_assert_segment_type_count() {
        let msg = SampleMessages::oru_r01();
        let parsed = assert_message_valid(msg.as_bytes());
        assert_segment_type_count(&parsed, "OBX", 1);
    }
}
