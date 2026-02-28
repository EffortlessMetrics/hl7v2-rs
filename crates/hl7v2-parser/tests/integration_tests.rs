//! Integration tests for the hl7v2-parser crate.
//!
//! These tests exercise the parser against realistic HL7 messages
//! and test integration with other crates in the workspace.

use hl7v2_parser::{get, parse, parse_batch, parse_file_batch, parse_mllp};
use hl7v2_test_utils::{MessageBuilder, SampleMessages};

// =============================================================================
// Standard Message Type Tests
// =============================================================================

#[test]
fn test_parse_adt_a01_from_fixtures() {
    let hl7 = SampleMessages::adt_a01();
    let message = parse(hl7.as_bytes()).unwrap();

    // Verify message structure
    assert!(message.segments.len() >= 2);
    assert_eq!(&message.segments[0].id, b"MSH");

    // Verify MSH fields
    assert_eq!(get(&message, "MSH.9.1"), Some("ADT"));
    assert_eq!(get(&message, "MSH.9.2"), Some("A01"));

    // Verify PID segment exists
    let pid_segment = message.segments.iter().find(|s| &s.id == b"PID");
    assert!(pid_segment.is_some());
}

#[test]
fn test_parse_adt_a04_from_fixtures() {
    let hl7 = SampleMessages::adt_a04();
    let message = parse(hl7.as_bytes()).unwrap();

    assert_eq!(get(&message, "MSH.9.1"), Some("ADT"));
    assert_eq!(get(&message, "MSH.9.2"), Some("A04"));
}

#[test]
fn test_parse_oru_r01_from_fixtures() {
    let hl7 = SampleMessages::oru_r01();
    let message = parse(hl7.as_bytes()).unwrap();

    assert_eq!(get(&message, "MSH.9.1"), Some("ORU"));
    assert_eq!(get(&message, "MSH.9.2"), Some("R01"));

    // Verify OBX segment exists
    let obx_segment = message.segments.iter().find(|s| &s.id == b"OBX");
    assert!(obx_segment.is_some());
}

// =============================================================================
// Edge Case Tests from Test Utilities
// =============================================================================

#[test]
fn test_parse_empty_fields_edge_case() {
    let hl7 = SampleMessages::edge_case("empty_fields").unwrap();
    let message = parse(hl7.as_bytes()).unwrap();

    // Message should parse successfully even with empty fields
    assert!(message.segments.len() >= 2);
}

#[test]
fn test_parse_max_lengths_edge_case() {
    let hl7 = SampleMessages::edge_case("max_lengths").unwrap();
    let message = parse(hl7.as_bytes()).unwrap();

    // Message with long field values should parse
    assert!(message.segments.len() >= 2);
}

#[test]
fn test_parse_special_chars_edge_case() {
    let hl7 = SampleMessages::edge_case("special_chars").unwrap();
    let message = parse(hl7.as_bytes()).unwrap();

    // Message with escape sequences should parse
    assert!(!message.segments.is_empty());
}

#[test]
fn test_parse_custom_delims_edge_case() {
    let hl7 = SampleMessages::edge_case("custom_delims").unwrap();
    let message = parse(hl7.as_bytes()).unwrap();

    // Verify custom delimiters were parsed
    assert_eq!(message.delims.field, '#');
    assert_eq!(message.delims.comp, '$');
    assert_eq!(message.delims.rep, '*');
}

#[test]
fn test_parse_repetitions_edge_case() {
    let hl7 = SampleMessages::edge_case("with_repetitions").unwrap();
    let message = parse(hl7.as_bytes()).unwrap();

    // Verify repetitions are accessible
    assert_eq!(get(&message, "PID.5[1].1"), Some("Doe"));
    assert_eq!(get(&message, "PID.5[2].1"), Some("Smith"));
    assert_eq!(get(&message, "PID.5[3].1"), Some("Brown"));
}

// =============================================================================
// Invalid Message Tests
// =============================================================================

#[test]
fn test_invalid_malformed_message() {
    let hl7 = SampleMessages::invalid("malformed").unwrap();
    // Malformed messages should either parse (lenient) or return an error
    let _ = parse(hl7.as_bytes());
}

#[test]
fn test_invalid_truncated_message() {
    let hl7 = SampleMessages::invalid("truncated").unwrap();
    // Truncated messages may still parse if they have a valid MSH segment start
    // The parser is lenient with incomplete messages
    let _ = parse(hl7.as_bytes());
}

#[test]
fn test_invalid_no_msh_message() {
    let hl7 = SampleMessages::invalid("no_msh").unwrap();
    // Messages without MSH should return an error
    let result = parse(hl7.as_bytes());
    assert!(result.is_err());
}

#[test]
fn test_invalid_bad_encoding_message() {
    let hl7 = SampleMessages::invalid("bad_encoding").unwrap();
    // Messages with bad encoding may or may not parse depending on content
    // The key is that they should not panic
    let _ = parse(hl7.as_bytes());
}

// =============================================================================
// MessageBuilder Integration Tests
// =============================================================================

#[test]
fn test_message_builder_creates_valid_message() {
    let bytes = MessageBuilder::new()
        .with_msh("TestApp", "TestFac", "RecvApp", "RecvFac", "ADT", "A01")
        .build_bytes();

    // Should parse successfully
    let message = parse(&bytes).unwrap();
    assert_eq!(get(&message, "MSH.3"), Some("TestApp"));
    assert_eq!(get(&message, "MSH.9.1"), Some("ADT"));
}

// =============================================================================
// MLLP Integration Tests
// =============================================================================

#[test]
fn test_mllp_roundtrip() {
    let original =
        b"MSH|^~\\&|App|Fac|Recv|RecvFac|20250128120000||ADT^A01|MSG123|P|2.5\rPID|1||123||Test\r";

    // Wrap in MLLP framing
    let framed = hl7v2_mllp::wrap_mllp(original);

    // Parse through MLLP
    let message = parse_mllp(&framed).unwrap();

    assert_eq!(message.segments.len(), 2);
    assert_eq!(get(&message, "PID.3.1"), Some("123"));
}

// =============================================================================
// Batch Integration Tests
// =============================================================================

#[test]
fn test_batch_parsing() {
    let batch = concat!(
        "BHS|^~\\&|App|Fac|||20250128120000\r",
        "MSH|^~\\&|App|Fac|Recv|RecvFac|20250128120000||ADT^A01|MSG1|P|2.5\r",
        "PID|1||123||Patient1\r",
        "MSH|^~\\&|App|Fac|Recv|RecvFac|20250128120001||ADT^A01|MSG2|P|2.5\r",
        "PID|1||456||Patient2\r",
        "BTS|2\r"
    );

    let parsed_batch = parse_batch(batch.as_bytes()).unwrap();

    assert!(parsed_batch.header.is_some());
    assert!(parsed_batch.trailer.is_some());
    assert_eq!(parsed_batch.messages.len(), 2);
}

#[test]
fn test_file_batch_parsing() {
    let file_batch = concat!(
        "FHS|^~\\&|App|Fac|||20250128120000\r",
        "BHS|^~\\&|App|Fac|||20250128120000\r",
        "MSH|^~\\&|App|Fac|Recv|RecvFac|20250128120000||ADT^A01|MSG1|P|2.5\r",
        "PID|1||123||Patient1\r",
        "BTS|1\r",
        "FTS|1\r"
    );

    let parsed = parse_file_batch(file_batch.as_bytes()).unwrap();

    assert!(parsed.header.is_some());
    assert!(parsed.trailer.is_some());
    assert_eq!(parsed.batches.len(), 1);
}

// =============================================================================
// Complex Message Tests
// =============================================================================

#[test]
fn test_complex_message_with_all_segment_types() {
    let hl7 = concat!(
        "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|",
        "20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1|||AL|NE|ASCII\r",
        "EVN|A01|20250128152312|||\r",
        "PID|1||123456^^^HOSP^MR||Doe^John^A||19800101|M|||C|\r",
        "PD1|||Practice^Clinic||||\r",
        "NK1|1|Doe^Jane|SPO|\r",
        "PV1|1|I|ICU^101^01||||DOC123^Smith^Jane||||||||V123456\r",
        "PV2||||||||||||||||||||||\r",
        "DB1|1||123456789||\r",
        "OBX|1|NM|WBC^White Blood Count||7.5|10^9/L|4.0-11.0|N|||F\r",
        "AL1|1||PEN^Penicillin||Rash\r"
    );

    let message = parse(hl7.as_bytes()).unwrap();

    // Verify all segment types are present
    let segment_ids: Vec<&str> = message.segments.iter().map(|s| s.id_str()).collect();

    assert!(segment_ids.contains(&"MSH"));
    assert!(segment_ids.contains(&"EVN"));
    assert!(segment_ids.contains(&"PID"));
    assert!(segment_ids.contains(&"PD1"));
    assert!(segment_ids.contains(&"NK1"));
    assert!(segment_ids.contains(&"PV1"));
    assert!(segment_ids.contains(&"PV2"));
    assert!(segment_ids.contains(&"DB1"));
    assert!(segment_ids.contains(&"OBX"));
    assert!(segment_ids.contains(&"AL1"));
}

#[test]
fn test_message_with_nested_components() {
    let hl7 = b"MSH|^~\\&|App|Fac|Recv|RecvFac|20250128120000||ADT^A01|MSG123|P|2.5\rPID|1||123456^^^HOSP^MR||Doe^John^Adam^III^Sr.||19800101|M\r";

    let message = parse(hl7.as_slice()).unwrap();

    // Test nested component access
    assert_eq!(get(&message, "PID.5.1"), Some("Doe")); // Family name
    assert_eq!(get(&message, "PID.5.2"), Some("John")); // Given name
}

#[test]
fn test_message_with_subcomponents() {
    let hl7 = b"MSH|^~\\&|App|Fac|Recv|RecvFac|20250128120000||ADT^A01|MSG123|P|2.5\rPID|1||123&MR&HOSP^^^HOSP^MR\r";

    let message = parse(hl7.as_slice()).unwrap();

    // The get function returns the first subcomponent by default
    assert_eq!(get(&message, "PID.3.1"), Some("123"));
}

// =============================================================================
// Character Set Tests
// =============================================================================

#[test]
fn test_charset_extraction() {
    let hl7 = b"MSH|^~\\&|App|Fac|Recv|RecvFac|20250128120000||ADT^A01|MSG123|P|2.5|||||||ASCII\r";
    let message = parse(hl7.as_slice()).unwrap();

    assert_eq!(message.charsets, vec!["ASCII"]);
}

#[test]
fn test_multiple_charsets() {
    let hl7 = b"MSH|^~\\&|App|Fac|Recv|RecvFac|20250128120000||ADT^A01|MSG123|P|2.5|||||||ASCII^UNICODE\r";
    let message = parse(hl7.as_slice()).unwrap();

    assert_eq!(message.charsets, vec!["ASCII", "UNICODE"]);
}

// =============================================================================
// Real-World Message Tests
// =============================================================================

#[test]
fn test_real_world_adt_a01() {
    // A realistic ADT^A01 message
    let hl7 = concat!(
        "MSH|^~\\&|ADT|GOOD_HEALTH_HOSPITAL|PACS|IMAGE_ARCHIVE|20250128152312-0500||ADT^A01|MSG00001|P|2.5.1|||||ASCII\r",
        "EVN|A01|20250128152312-0500|20250128160000||JOHNSON^MIKE^A^^DR^^MD^^&MD&&PHG^^^^PHYS\r",
        "PID|1||PATID5421^^^GOOD_HEALTH_HOSPITAL^MR||TEST^PATIENT^A||19550505|M||C|12345 MAIN ST^^NEW YORK^NY^10001^USA||(212)555-1212|(212)555-1234||E|S||123456789|987654^NC||\r",
        "PD1|||GOOD_HEALTH_HOSPITAL^^^^GH|||JOHNSON^MIKE^A^^DR^^MD^^&MD&&PHG^^^^PHYS\r",
        "NK1|1|TEST^SPOUSE^A|SPO|12345 MAIN ST^^NEW YORK^NY^10001^USA||(212)555-1212\r",
        "PV1|1|I|ICU^101^01^GOOD_HEALTH_HOSPITAL^^^^GH||||JOHNSON^MIKE^A^^DR^^MD^^&MD&&PHG^^^^PHYS||||||||ADMITTED|||||||||||||||||||||||||20250128152312-0500\r",
        "PV2||||||||||||||||||||||||20250128152312-0500\r"
    );

    let message = parse(hl7.as_bytes()).unwrap();

    // Verify key fields
    assert_eq!(get(&message, "MSH.3"), Some("ADT"));
    assert_eq!(get(&message, "MSH.4"), Some("GOOD_HEALTH_HOSPITAL"));
    assert_eq!(get(&message, "MSH.5"), Some("PACS"));
    assert_eq!(get(&message, "MSH.9.1"), Some("ADT"));
    assert_eq!(get(&message, "MSH.9.2"), Some("A01"));
    assert_eq!(get(&message, "PID.3.1"), Some("PATID5421"));
    assert_eq!(get(&message, "PID.5.1"), Some("TEST"));
    assert_eq!(get(&message, "PID.5.2"), Some("PATIENT"));
}

#[test]
fn test_real_world_oru_r01() {
    // A realistic ORU^R01 message (lab results)
    let hl7 = concat!(
        "MSH|^~\\&|LAB|GOOD_HEALTH_HOSPITAL|HIS|GOOD_HEALTH_HOSPITAL|20250128150000||ORU^R01|LAB00001|P|2.5.1|||||ASCII\r",
        "PID|1||PATID5421^^^GOOD_HEALTH_HOSPITAL^MR||TEST^PATIENT^A||19550505|M||C|12345 MAIN ST^^NEW YORK^NY^10001^USA||(212)555-1212\r",
        "ORC|RE|ORDER0001|RESULT0001||CM||||20250128120000\r",
        "OBR|1|ORDER0001|RESULT0001|CBC^COMPLETE BLOOD COUNT^L|||20250128120000|||||||||||||||F\r",
        "OBX|1|NM|WBC^WHITE BLOOD COUNT^L||7.5|10*9/L|4.0-11.0|N|||F|||20250128150000\r",
        "OBX|2|NM|RBC^RED BLOOD COUNT^L||4.5|10*12/L|4.0-5.5|N|||F|||20250128150000\r",
        "OBX|3|NM|HGB^HEMOGLOBIN^L||14.0|g/dL|12.0-16.0|N|||F|||20250128150000\r",
        "OBX|4|NM|HCT^HEMATOCRIT^L||42|%36.0-48.0|N|||F|||20250128150000\r",
        "OBX|5|NM|PLT^PLATELET COUNT^L||250|10*9/L|150-400|N|||F|||20250128150000\r"
    );

    let message = parse(hl7.as_bytes()).unwrap();

    // Verify message structure
    assert_eq!(get(&message, "MSH.9.1"), Some("ORU"));
    assert_eq!(get(&message, "MSH.9.2"), Some("R01"));

    // Count OBX segments
    let obx_count = message.segments.iter().filter(|s| &s.id == b"OBX").count();
    assert_eq!(obx_count, 5);
}

// =============================================================================
// Stress Tests
// =============================================================================

#[test]
fn test_large_message() {
    // Create a message with many segments
    let mut hl7 =
        String::from("MSH|^~\\&|App|Fac|Recv|RecvFac|20250128120000||ADT^A01|MSG123|P|2.5\r");

    for i in 0..100 {
        hl7.push_str(&format!("OBX|{}|ST|Test{}||Value{}\r", i, i, i));
    }

    let message = parse(hl7.as_bytes()).unwrap();

    // Verify all segments were parsed
    assert_eq!(message.segments.len(), 101); // MSH + 100 OBX
}

#[test]
fn test_many_repetitions() {
    // Create a message with many field repetitions
    let mut field_value = String::new();
    for i in 0..50 {
        if i > 0 {
            field_value.push('~');
        }
        field_value.push_str(&format!("Value{}", i));
    }

    let hl7 = format!(
        "MSH|^~\\&|App|Fac|Recv|RecvFac|20250128120000||ADT^A01|MSG123|P|2.5\rPID|1||123||{}\r",
        field_value
    );

    let message = parse(hl7.as_bytes()).unwrap();

    // Verify first and last repetitions
    assert_eq!(get(&message, "PID.5[1].1"), Some("Value0"));
    assert_eq!(get(&message, "PID.5[50].1"), Some("Value49"));
}
