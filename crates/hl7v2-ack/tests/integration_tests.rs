//! Integration tests for hl7v2-ack crate
//!
//! These tests verify ACK generation works correctly with real-world
//! HL7 message scenarios and integration with other crates.

use hl7v2_ack::{ack, ack_with_error, AckCode};
use hl7v2_core::parse;
use hl7v2_writer::write;

// ============================================================================
// Real-World Message Tests
// ============================================================================

#[test]
fn test_ack_for_adt_a01_admission() {
    let adt_a01 = parse(
        b"MSH|^~\\&|ADT|HOSPITAL|LAB|LABHOST|20250128152312||ADT^A01^ADT_A01|MSG00001|P|2.5.1\r\
          PID|1||12345^^^HOSP^MR||SMITH^JOHN^Q||19650101|M|||123 MAIN ST^^ANYTOWN^CA^12345||5551234567\r\
          PV1|1|I|ICU^101^^HOSP||||123456^DOCTOR^JANE^A^^MD||||||||ADM|A0|||||||||||||||||||HOSP||20250128100000\r"
    ).unwrap();
    
    let ack_msg = ack(&adt_a01, AckCode::AA).unwrap();
    
    // Verify ACK structure
    assert_eq!(ack_msg.segments.len(), 2);
    
    // Verify swapped applications
    let ack_msh = &ack_msg.segments[0];
    let sending_app = get_field_value_from_segment(ack_msh, 2).unwrap();
    assert_eq!(sending_app, "LAB");
    
    let receiving_app = get_field_value_from_segment(ack_msh, 4).unwrap();
    assert_eq!(receiving_app, "ADT");
}

#[test]
fn test_ack_for_adt_a04_register() {
    let adt_a04 = parse(
        b"MSH|^~\\&|REG|HOSPITAL|ADT|ADTHOST|20250128153000||ADT^A04^ADT_A04|MSG00002|P|2.5.1\r\
          PID|1||98765^^^HOSP^MR||DOE^JANE||19700101|F\r"
    ).unwrap();
    
    let ack_msg = ack(&adt_a04, AckCode::AA).unwrap();
    
    // Verify control ID is preserved
    let msa = &ack_msg.segments[1];
    let control_id = get_field_value_from_segment(msa, 2).unwrap();
    assert_eq!(control_id, "MSG00002");
}

#[test]
fn test_ack_for_oru_r01_result() {
    let oru_r01 = parse(
        b"MSH|^~\\&|LAB|LABHOST|HIS|HISHOST|20250128154500||ORU^R01|MSG00003|P|2.5.1\r\
          PID|1||12345^^^HOSP^MR||SMITH^JOHN\r\
          ORC|RE|1234|1234|||CM|||20250128154500\r\
          OBR|1|1234|1234|GLUCOSE^Blood Glucose|||20250128154500|||||||20250128154500|||F\r\
          OBX|1|NM|GLUCOSE^Blood Glucose||120|mg/dL|70-110|H|||F\r"
    ).unwrap();
    
    let ack_msg = ack(&oru_r01, AckCode::AA).unwrap();
    
    assert_eq!(ack_msg.segments.len(), 2);
}

// ============================================================================
// Error Scenario Tests
// ============================================================================

#[test]
fn test_ack_application_error_with_details() {
    let original = parse(
        b"MSH|^~\\&|APP|FAC|RECV|RECVFAC|20250128150000||ADT^A01|MSG12345|P|2.5.1\r\
          PID|1||INVALID||||||\r"
    ).unwrap();
    
    let ack_msg = ack_with_error(
        &original,
        AckCode::AE,
        Some("PID segment missing required fields")
    ).unwrap();
    
    assert_eq!(ack_msg.segments.len(), 3);
    
    let msa = &ack_msg.segments[1];
    let ack_code = get_field_value_from_segment(msa, 1).unwrap();
    assert_eq!(ack_code, "AE");
    
    let err = &ack_msg.segments[2];
    let error_msg = get_field_value_from_segment(err, 3).unwrap();
    assert_eq!(error_msg, "PID segment missing required fields");
}

#[test]
fn test_ack_reject_invalid_message_type() {
    let original = parse(
        b"MSH|^~\\&|APP|FAC|RECV|RECVFAC|20250128150000||UNKNOWN^TYPE|MSG12345|P|2.5.1\r"
    ).unwrap();
    
    let ack_msg = ack_with_error(
        &original,
        AckCode::AR,
        Some("Unknown message type")
    ).unwrap();
    
    let msa = &ack_msg.segments[1];
    let ack_code = get_field_value_from_segment(msa, 1).unwrap();
    assert_eq!(ack_code, "AR");
}

// ============================================================================
// Roundtrip Tests
// ============================================================================

#[test]
fn test_ack_roundtrip_with_writer() {
    let original = parse(
        b"MSH|^~\\&|APP|FAC|RECV|RECVFAC|20250128150000||ADT^A01|MSG12345|P|2.5.1\r"
    ).unwrap();
    
    let ack_msg = ack(&original, AckCode::AA).unwrap();
    
    // Serialize ACK using writer
    let ack_bytes = write(&ack_msg);
    
    // Verify it's valid HL7
    assert!(ack_bytes.starts_with(b"MSH|"));
    assert!(ack_bytes.ends_with(b"\r"));
    
    // Parse it back
    let parsed_ack = parse(&ack_bytes).unwrap();
    
    // Verify structure is preserved
    assert_eq!(parsed_ack.segments.len(), 2);
    assert_eq!(std::str::from_utf8(&parsed_ack.segments[0].id).unwrap(), "MSH");
    assert_eq!(std::str::from_utf8(&parsed_ack.segments[1].id).unwrap(), "MSA");
}

#[test]
fn test_ack_with_error_roundtrip() {
    let original = parse(
        b"MSH|^~\\&|APP|FAC|RECV|RECVFAC|20250128150000||ADT^A01|MSG12345|P|2.5.1\r"
    ).unwrap();
    
    let ack_msg = ack_with_error(&original, AckCode::AE, Some("Test error")).unwrap();
    
    // Serialize and parse back
    let ack_bytes = write(&ack_msg);
    let parsed_ack = parse(&ack_bytes).unwrap();
    
    assert_eq!(parsed_ack.segments.len(), 3);
}

// ============================================================================
// Different HL7 Versions Tests
// ============================================================================

#[test]
fn test_ack_for_hl7_23() {
    let original = parse(
        b"MSH|^~\\&|APP|FAC|RECV|RECVFAC|20250128150000||ADT^A01|MSG123|P|2.3\r"
    ).unwrap();
    
    let ack_msg = ack(&original, AckCode::AA).unwrap();
    
    let ack_msh = &ack_msg.segments[0];
    let version = get_field_value_from_segment(ack_msh, 11).unwrap();
    assert_eq!(version, "2.3");
}

#[test]
fn test_ack_for_hl7_251() {
    let original = parse(
        b"MSH|^~\\&|APP|FAC|RECV|RECVFAC|20250128150000||ADT^A01|MSG123|P|2.5.1\r"
    ).unwrap();
    
    let ack_msg = ack(&original, AckCode::AA).unwrap();
    
    let ack_msh = &ack_msg.segments[0];
    let version = get_field_value_from_segment(ack_msh, 11).unwrap();
    assert_eq!(version, "2.5.1");
}

#[test]
fn test_ack_for_hl7_27() {
    let original = parse(
        b"MSH|^~\\&|APP|FAC|RECV|RECVFAC|20250128150000||ADT^A01|MSG123|P|2.7\r"
    ).unwrap();
    
    let ack_msg = ack(&original, AckCode::AA).unwrap();
    
    let ack_msh = &ack_msg.segments[0];
    let version = get_field_value_from_segment(ack_msh, 11).unwrap();
    assert_eq!(version, "2.7");
}

// ============================================================================
// Processing Mode Tests
// ============================================================================

#[test]
fn test_ack_preserves_production_mode() {
    let original = parse(
        b"MSH|^~\\&|APP|FAC|RECV|RECVFAC|20250128150000||ADT^A01|MSG123|P|2.5.1\r"
    ).unwrap();
    
    let ack_msg = ack(&original, AckCode::AA).unwrap();
    
    let ack_msh = &ack_msg.segments[0];
    let processing_id = get_field_value_from_segment(ack_msh, 10).unwrap();
    assert_eq!(processing_id, "P");
}

#[test]
fn test_ack_preserves_training_mode() {
    let original = parse(
        b"MSH|^~\\&|APP|FAC|RECV|RECVFAC|20250128150000||ADT^A01|MSG123|T|2.5.1\r"
    ).unwrap();
    
    let ack_msg = ack(&original, AckCode::AA).unwrap();
    
    let ack_msh = &ack_msg.segments[0];
    let processing_id = get_field_value_from_segment(ack_msh, 10).unwrap();
    assert_eq!(processing_id, "T");
}

#[test]
fn test_ack_preserves_debugging_mode() {
    let original = parse(
        b"MSH|^~\\&|APP|FAC|RECV|RECVFAC|20250128150000||ADT^A01|MSG123|D|2.5.1\r"
    ).unwrap();
    
    let ack_msg = ack(&original, AckCode::AA).unwrap();
    
    let ack_msh = &ack_msg.segments[0];
    let processing_id = get_field_value_from_segment(ack_msh, 10).unwrap();
    assert_eq!(processing_id, "D");
}

// ============================================================================
// Complex Message Tests
// ============================================================================

#[test]
fn test_ack_for_message_with_many_segments() {
    let complex_message = parse(
        b"MSH|^~\\&|HIS|HOSPITAL|LIS|LABHOST|20250128160000||ORM^O01|MSG99999|P|2.5.1\r\
          PID|1||12345^^^HOSP^MR||SMITH^JOHN^Q||19650101|M|||123 MAIN ST^^ANYTOWN^CA^12345\r\
          PD1|||HOSP||||\r\
          PV1|1|I|ICU^101^^HOSP||||123456^DOCTOR^JANE^A^^MD||||||||ADM|A0|||||||||||||||||||HOSP\r\
          PV2||||||||||||||||||||||||\r\
          IN1|1|PLAN001|INSURANCE CO|INSURANCE CO|||123456789||||||||||\r\
          IN2||123456789||||||\r\
          GT1|1||SMITH^JOHN^Q||123 MAIN ST^^ANYTOWN^CA^12345||5551234567||||||\r\
          ORC|NW|ORD001|ORD001|||SC|||20250128160000\r\
          OBR|1|ORD001|ORD001|CBC^Complete Blood Count|||20250128160000|||||||20250128160000|||F\r"
    ).unwrap();
    
    let ack_msg = ack(&complex_message, AckCode::AA).unwrap();
    
    // ACK should only have MSH and MSA
    assert_eq!(ack_msg.segments.len(), 2);
}

#[test]
fn test_ack_for_message_with_repeating_fields() {
    let message_with_reps = parse(
        b"MSH|^~\\&|HIS|HOSPITAL|LIS|LABHOST|20250128160000||ORM^O01|MSG88888|P|2.5.1\r\
          PID|1||12345^^^HOSP^MR||SMITH^JOHN^Q~SMITH^JACK^Q||19650101|M|||123 MAIN ST^^ANYTOWN^CA^12345||5551234567~5559876543\r"
    ).unwrap();
    
    let ack_msg = ack(&message_with_reps, AckCode::AA).unwrap();
    
    assert_eq!(ack_msg.segments.len(), 2);
}

// ============================================================================
// Enhanced Mode (Commit Acknowledgment) Tests
// ============================================================================

#[test]
fn test_commit_accept_for_two_phase_commit() {
    let original = parse(
        b"MSH|^~\\&|APP|FAC|RECV|RECVFAC|20250128150000||ADT^A01|MSG12345|P|2.5.1\r"
    ).unwrap();
    
    let ack_msg = ack(&original, AckCode::CA).unwrap();
    
    let msa = &ack_msg.segments[1];
    let ack_code = get_field_value_from_segment(msa, 1).unwrap();
    assert_eq!(ack_code, "CA");
}

#[test]
fn test_commit_error_with_reason() {
    let original = parse(
        b"MSH|^~\\&|APP|FAC|RECV|RECVFAC|20250128150000||ADT^A01|MSG12345|P|2.5.1\r"
    ).unwrap();
    
    let ack_msg = ack_with_error(
        &original,
        AckCode::CE,
        Some("Commit failed: database unavailable")
    ).unwrap();
    
    let msa = &ack_msg.segments[1];
    let ack_code = get_field_value_from_segment(msa, 1).unwrap();
    assert_eq!(ack_code, "CE");
    
    assert_eq!(ack_msg.segments.len(), 3);
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_ack_for_minimal_message() {
    let minimal = parse(
        b"MSH|^~\\&|||||||ACK|||2.5.1\r"
    ).unwrap();
    
    let ack_msg = ack(&minimal, AckCode::AA).unwrap();
    
    // Should still produce valid ACK
    assert_eq!(ack_msg.segments.len(), 2);
}

#[test]
fn test_ack_for_message_with_special_characters() {
    let special = parse(
        b"MSH|^~\\&|APP\\F\\TEST|FAC^NAME|RECV~APP|FAC\\E\\ESC|20250128150000||ADT^A01|MSG123|P|2.5.1\r"
    ).unwrap();
    
    let ack_msg = ack(&special, AckCode::AA).unwrap();
    
    assert_eq!(ack_msg.segments.len(), 2);
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Helper function to extract field value from a segment
fn get_field_value_from_segment(segment: &hl7v2_core::Segment, field_index: usize) -> Option<String> {
    if field_index > segment.fields.len() {
        return None;
    }
    
    let field = &segment.fields[field_index - 1];
    if field.reps.is_empty() {
        return None;
    }
    
    let rep = &field.reps[0];
    if rep.comps.is_empty() {
        return None;
    }
    
    let comp = &rep.comps[0];
    if comp.subs.is_empty() {
        return None;
    }
    
    match &comp.subs[0] {
        hl7v2_core::Atom::Text(text) => Some(text.clone()),
        hl7v2_core::Atom::Null => None,
    }
}
