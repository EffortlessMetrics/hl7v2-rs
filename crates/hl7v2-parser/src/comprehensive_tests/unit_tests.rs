//! Unit tests for the hl7v2-parser crate.
//!
//! Tests cover:
//! - Basic message parsing
//! - Segment parsing
//! - Field/Component/Subcomponent parsing
//! - Escape sequence handling
//! - Error cases
//! - Edge cases

use crate::{get, get_presence, parse, parse_batch, parse_file_batch, parse_mllp};
use hl7v2_model::*;

// =============================================================================
// Basic Message Parsing Tests
// =============================================================================

#[test]
fn test_parse_simple_adt_a01() {
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r";
    let message = parse(hl7).unwrap();

    assert_eq!(message.delims.field, '|');
    assert_eq!(message.delims.comp, '^');
    assert_eq!(message.delims.rep, '~');
    assert_eq!(message.delims.esc, '\\');
    assert_eq!(message.delims.sub, '&');
    assert_eq!(message.segments.len(), 2);
    assert_eq!(&message.segments[0].id, b"MSH");
    assert_eq!(&message.segments[1].id, b"PID");
}

#[test]
fn test_parse_oru_r01() {
    let hl7 = b"MSH|^~\\&|LabSys|Lab|LIS|Hospital|20250128150000||ORU^R01|MSG003|P|2.5\rPID|1||MRN789^^^Lab^MR||Patient^Test||19850610|M\rOBR|1|ORD123|FIL456|CBC^Complete Blood Count|||20250128120000\rOBX|1|NM|WBC^White Blood Count||7.5|10^9/L|4.0-11.0|N|||F\r";
    let message = parse(hl7).unwrap();

    assert_eq!(message.segments.len(), 4);
    assert_eq!(&message.segments[0].id, b"MSH");
    assert_eq!(&message.segments[1].id, b"PID");
    assert_eq!(&message.segments[2].id, b"OBR");
    assert_eq!(&message.segments[3].id, b"OBX");
}

#[test]
fn test_parse_adt_a04() {
    let hl7 = b"MSH|^~\\&|RegSys|Hospital|ADT|Hospital|20250128140000||ADT^A04|MSG002|P|2.5\rPID|1||MRN456^^^Hospital^MR||Smith^Jane^M||19900215|F\r";
    let message = parse(hl7).unwrap();

    assert_eq!(message.segments.len(), 2);
    assert_eq!(get(&message, "MSH.9.1"), Some("ADT"));
    assert_eq!(get(&message, "MSH.9.2"), Some("A04"));
    assert_eq!(get(&message, "PID.5.1"), Some("Smith"));
    assert_eq!(get(&message, "PID.5.2"), Some("Jane"));
}

// =============================================================================
// Delimiter Tests
// =============================================================================

#[test]
fn test_parse_custom_delimiters() {
    // Message with custom delimiters: #$*@!
    let hl7 =
        b"MSH#$*@!App#Fac#Rec#RecFac#20250128120000##ADT$A01#1#P#2.5\rPID#1##123##Name$First\r";
    let message = parse(hl7).unwrap();

    assert_eq!(message.delims.field, '#');
    assert_eq!(message.delims.comp, '$');
    assert_eq!(message.delims.rep, '*');
    assert_eq!(message.delims.esc, '@');
    assert_eq!(message.delims.sub, '!');
    assert_eq!(message.segments.len(), 2);
}

#[test]
fn test_default_delimiters() {
    let delims = Delims::default();
    assert_eq!(delims.field, '|');
    assert_eq!(delims.comp, '^');
    assert_eq!(delims.rep, '~');
    assert_eq!(delims.esc, '\\');
    assert_eq!(delims.sub, '&');
}

// =============================================================================
// Field Access Tests
// =============================================================================

#[test]
fn test_get_simple_field() {
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r";
    let message = parse(hl7).unwrap();

    assert_eq!(get(&message, "PID.5.1"), Some("Doe"));
    assert_eq!(get(&message, "PID.5.2"), Some("John"));
    assert_eq!(get(&message, "PID.3.1"), Some("123456"));
}

#[test]
fn test_get_msh_fields() {
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\r";
    let message = parse(hl7).unwrap();

    assert_eq!(get(&message, "MSH.3"), Some("SendingApp"));
    assert_eq!(get(&message, "MSH.4"), Some("SendingFac"));
    assert_eq!(get(&message, "MSH.5"), Some("ReceivingApp"));
    assert_eq!(get(&message, "MSH.6"), Some("ReceivingFac"));
    assert_eq!(get(&message, "MSH.9.1"), Some("ADT"));
    assert_eq!(get(&message, "MSH.9.2"), Some("A01"));
    assert_eq!(get(&message, "MSH.10"), Some("ABC123"));
    assert_eq!(get(&message, "MSH.11"), Some("P"));
    assert_eq!(get(&message, "MSH.12"), Some("2.5.1"));
}

#[test]
fn test_get_with_repetitions() {
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac\rPID|1||123456^^^HOSP^MR||Doe^John~Smith^Jane~Brown^Bob\r";
    let message = parse(hl7).unwrap();

    // Test first repetition (default)
    assert_eq!(get(&message, "PID.5.1"), Some("Doe"));
    assert_eq!(get(&message, "PID.5.2"), Some("John"));

    // Test second repetition
    assert_eq!(get(&message, "PID.5[2].1"), Some("Smith"));
    assert_eq!(get(&message, "PID.5[2].2"), Some("Jane"));

    // Test third repetition
    assert_eq!(get(&message, "PID.5[3].1"), Some("Brown"));
    assert_eq!(get(&message, "PID.5[3].2"), Some("Bob"));
}

#[test]
fn test_get_missing_field() {
    let hl7 = b"MSH|^~\\&|App|Fac\rPID|1||123||Doe^John\r";
    let message = parse(hl7).unwrap();

    assert_eq!(get(&message, "PID.100"), None);
    assert_eq!(get(&message, "ZZZ.1"), None);
}

// =============================================================================
// Presence Semantics Tests
// =============================================================================

#[test]
fn test_presence_semantics() {
    let hl7 = b"MSH|^~\\&|App|Fac\rPID|1||123456^^^HOSP^MR||Doe^John|||\r";
    let message = parse(hl7).unwrap();

    // Test existing field with value
    match get_presence(&message, "PID.5.1") {
        Presence::Value(val) => assert_eq!(val, "Doe"),
        _ => panic!("Expected Value"),
    }

    // Test existing field with empty value
    match get_presence(&message, "PID.8.1") {
        Presence::Empty => {}
        _ => panic!("Expected Empty"),
    }

    // Test missing field
    match get_presence(&message, "PID.50.1") {
        Presence::Missing => {}
        _ => panic!("Expected Missing"),
    }
}

// =============================================================================
// Escape Sequence Tests
// =============================================================================

#[test]
fn test_escape_sequences() {
    // Test message with escape sequences
    let hl7 = b"MSH|^~\\&|App|Fac|||20250128120000||ADT^A01|1|P|2.5\rPID|1||123||Test\\F\\Field\r";
    let message = parse(hl7).unwrap();

    // \\F\\ should be unescaped to |
    assert_eq!(get(&message, "PID.5.1"), Some("Test|Field"));
}

#[test]
fn test_escape_special_chars() {
    // Test message with escape sequences
    // Note: The escape sequences are unescaped during parsing
    let hl7 = b"MSH|^~\\&|App|Fac|||20250128120000||ADT^A01|1|P|2.5\rPID|1||123||Test\\F\\Field\r";
    let message = parse(hl7).unwrap();

    // The escape sequence \\F\\ should be unescaped to |
    let value = get(&message, "PID.5.1").unwrap();
    assert!(
        value.contains('|'),
        "Should contain unescaped field separator"
    );
}

// =============================================================================
// MLLP Tests
// =============================================================================

#[test]
fn test_parse_mllp() {
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\r";
    let framed = hl7v2_mllp::wrap_mllp(hl7);
    let message = parse_mllp(&framed).unwrap();

    assert_eq!(message.segments.len(), 1);
    assert_eq!(&message.segments[0].id, b"MSH");
}

// =============================================================================
// Batch Parsing Tests
// =============================================================================

#[test]
fn test_parse_single_message_as_batch() {
    let hl7 = b"MSH|^~\\&|App|Fac|||20250128120000||ADT^A01|1|P|2.5\rPID|1||123||Test\r";
    let batch = parse_batch(hl7).unwrap();

    assert!(batch.header.is_none());
    assert!(batch.trailer.is_none());
    assert_eq!(batch.messages.len(), 1);
}

#[test]
fn test_parse_batch_with_header() {
    let hl7 = b"BHS|^~\\&|App|Fac|||20250128120000\rMSH|^~\\&|App|Fac|||20250128120000||ADT^A01|1|P|2.5\rPID|1||123||Test\rBTS|1\r";
    let batch = parse_batch(hl7).unwrap();

    assert!(batch.header.is_some());
    assert!(batch.trailer.is_some());
    assert_eq!(batch.messages.len(), 1);
}

#[test]
fn test_parse_file_batch() {
    let hl7 = b"FHS|^~\\&|App|Fac|||20250128120000\rMSH|^~\\&|App|Fac|||20250128120000||ADT^A01|1|P|2.5\rPID|1||123||Test\rFTS|1\r";
    let file_batch = parse_file_batch(hl7).unwrap();

    assert!(file_batch.header.is_some());
    assert!(file_batch.trailer.is_some());
    assert_eq!(file_batch.batches.len(), 1);
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[test]
fn test_error_empty_message() {
    let result = parse(b"");
    assert!(result.is_err());
}

#[test]
fn test_error_no_msh_segment() {
    let result = parse(b"PID|1||123||Doe^John\r");
    assert!(result.is_err());
}

#[test]
fn test_error_invalid_segment_id() {
    // Note: The parser accepts numeric segment IDs (like Z01)
    // This test verifies that lowercase letters are not accepted
    let result = parse(b"MSH|^~\\&|App|Fac\rabc|invalid\r");
    assert!(result.is_err());
}

#[test]
fn test_error_invalid_charset() {
    // Non-UTF8 bytes should fail
    let result = parse(&[0xFF, 0xFE, 0xFD]);
    assert!(result.is_err());
}

#[test]
fn test_error_truncated_message() {
    // MSH segment without proper delimiters
    let result = parse(b"MSH");
    assert!(result.is_err());
}

#[test]
fn test_error_duplicate_delimiters() {
    // All same delimiters - should fail
    let result = parse(b"MSH|||||App|Fac\r");
    assert!(result.is_err());
}

// =============================================================================
// Edge Case Tests
// =============================================================================

#[test]
fn test_empty_fields() {
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|||||ADT^A01|1|P|2.5\rPID|1|||||||||||\rPV1|1||||||||||||||\r";
    let message = parse(hl7).unwrap();

    assert_eq!(message.segments.len(), 3);

    // Check that empty fields are parsed correctly
    match get_presence(&message, "PID.3.1") {
        Presence::Empty => {}
        _ => panic!("Expected Empty for PID.3.1"),
    }
}

#[test]
fn test_very_long_field() {
    let long_value: String = "A".repeat(1000);
    let hl7 = format!(
        "MSH|^~\\&|App|Fac|||20250128120000||ADT^A01|1|P|2.5\rPID|1||{}||Test\r",
        long_value
    );
    let message = parse(hl7.as_bytes()).unwrap();

    assert_eq!(get(&message, "PID.3.1"), Some(long_value.as_str()));
}

#[test]
fn test_multiple_segments_same_type() {
    let hl7 = b"MSH|^~\\&|App|Fac|||20250128120000||ADT^A01|1|P|2.5\rOBX|1|ST|Test1||Value1\rOBX|2|ST|Test2||Value2\rOBX|3|ST|Test3||Value3\r";
    let message = parse(hl7).unwrap();

    assert_eq!(message.segments.len(), 4); // MSH + 3 OBX
}

#[test]
fn test_null_value() {
    let hl7 = b"MSH|^~\\&|App|Fac|||20250128120000||ADT^A01|1|P|2.5\rPID|1||\"\"||Test\r";
    let message = parse(hl7).unwrap();

    match get_presence(&message, "PID.3.1") {
        Presence::Null => {}
        _ => panic!("Expected Null for PID.3.1"),
    }
}

#[test]
fn test_subcomponent_parsing() {
    let hl7 = b"MSH|^~\\&|App|Fac|||20250128120000||ADT^A01|1|P|2.5\rPID|1||123||Test&Sub1&Sub2\r";
    let message = parse(hl7).unwrap();

    // The get function returns the first subcomponent by default
    assert_eq!(get(&message, "PID.5.1"), Some("Test"));
}

#[test]
fn test_component_parsing() {
    let hl7 = b"MSH|^~\\&|App|Fac|||20250128120000||ADT^A01|1|P|2.5\rPID|1||123^^^HOSP^MR||Test\r";
    let message = parse(hl7).unwrap();

    assert_eq!(get(&message, "PID.3.1"), Some("123"));
    assert_eq!(get(&message, "PID.3.4"), Some("HOSP"));
    assert_eq!(get(&message, "PID.3.5"), Some("MR"));
}

#[test]
fn test_segment_with_trailing_delimiters() {
    let hl7 = b"MSH|^~\\&|App|Fac|||20250128120000||ADT^A01|1|P|2.5\rPID|1||123||Test||||||\r";
    let message = parse(hl7).unwrap();

    assert_eq!(message.segments.len(), 2);
    assert_eq!(get(&message, "PID.5.1"), Some("Test"));
}

#[test]
fn test_msh_encoding_characters_field() {
    let hl7 = b"MSH|^~\\&|App|Fac|||20250128120000||ADT^A01|1|P|2.5\r";
    let message = parse(hl7).unwrap();

    // MSH-2 should contain the encoding characters as a single value
    assert_eq!(get(&message, "MSH.2"), Some("^~\\&"));
}

// =============================================================================
// Charset Extraction Tests
// =============================================================================

#[test]
fn test_charset_extraction() {
    let hl7 = b"MSH|^~\\&|App|Fac|||20250128120000||ADT^A01|1|P|2.5|||||||ASCII\r";
    let message = parse(hl7).unwrap();

    assert_eq!(message.charsets, vec!["ASCII"]);
}

#[test]
fn test_no_charset() {
    let hl7 = b"MSH|^~\\&|App|Fac|||20250128120000||ADT^A01|1|P|2.5\r";
    let message = parse(hl7).unwrap();

    assert!(message.charsets.is_empty());
}

// =============================================================================
// Line Ending Tests
// =============================================================================

#[test]
fn test_carriage_return_line_ending() {
    let hl7 = b"MSH|^~\\&|App|Fac\rPID|1||123||Test\r";
    let message = parse(hl7).unwrap();

    assert_eq!(message.segments.len(), 2);
}

#[test]
fn test_trailing_carriage_return() {
    let hl7 = b"MSH|^~\\&|App|Fac\rPID|1||123||Test\r";
    let message = parse(hl7).unwrap();

    assert_eq!(message.segments.len(), 2);
}

// =============================================================================
// Segment ID Validation Tests
// =============================================================================

#[test]
fn test_valid_segment_ids() {
    let hl7 = b"MSH|^~\\&|App|Fac|||20250128120000||ADT^A01|1|P|2.5\rPID|1\rPV1|1\rEVN|A01\r";
    let message = parse(hl7).unwrap();

    assert_eq!(message.segments.len(), 4);
    assert_eq!(&message.segments[0].id, b"MSH");
    assert_eq!(&message.segments[1].id, b"PID");
    assert_eq!(&message.segments[2].id, b"PV1");
    assert_eq!(&message.segments[3].id, b"EVN");
}

#[test]
fn test_numeric_segment_id() {
    // Some HL7 segments have numeric IDs like Z01
    let hl7 = b"MSH|^~\\&|App|Fac|||20250128120000||ADT^A01|1|P|2.5\rZ01|1\r";
    let message = parse(hl7).unwrap();

    assert_eq!(message.segments.len(), 2);
    assert_eq!(&message.segments[1].id, b"Z01");
}
