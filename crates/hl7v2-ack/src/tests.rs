//! Comprehensive unit tests for hl7v2-ack crate
//!
//! This module contains unit tests for:
//! - ACK code handling (AA, AE, AR, CA, CE, CR)
//! - ACK message generation
//! - ACK with error segments
//! - Edge cases and error handling

use super::*;
use hl7v2_core::parse;

// ============================================================================
// AckCode Tests
// ============================================================================

#[test]
fn test_ack_code_as_str_all_variants() {
    assert_eq!(AckCode::AA.as_str(), "AA");
    assert_eq!(AckCode::AE.as_str(), "AE");
    assert_eq!(AckCode::AR.as_str(), "AR");
    assert_eq!(AckCode::CA.as_str(), "CA");
    assert_eq!(AckCode::CE.as_str(), "CE");
    assert_eq!(AckCode::CR.as_str(), "CR");
}

#[test]
fn test_ack_code_display_all_variants() {
    assert_eq!(format!("{}", AckCode::AA), "AA");
    assert_eq!(format!("{}", AckCode::AE), "AE");
    assert_eq!(format!("{}", AckCode::AR), "AR");
    assert_eq!(format!("{}", AckCode::CA), "CA");
    assert_eq!(format!("{}", AckCode::CE), "CE");
    assert_eq!(format!("{}", AckCode::CR), "CR");
}

#[test]
fn test_ack_code_debug() {
    assert!(format!("{:?}", AckCode::AA).contains("AA"));
    assert!(format!("{:?}", AckCode::AE).contains("AE"));
}

#[test]
fn test_ack_code_clone() {
    let code = AckCode::AA;
    let cloned = code.clone();
    assert_eq!(code, cloned);
}

#[test]
fn test_ack_code_copy() {
    let code = AckCode::AA;
    let copied: AckCode = code;
    assert_eq!(code, copied);
}

#[test]
fn test_ack_code_partial_eq() {
    assert_eq!(AckCode::AA, AckCode::AA);
    assert_ne!(AckCode::AA, AckCode::AE);
    assert_ne!(AckCode::AR, AckCode::CR);
}

// ============================================================================
// ACK Generation Tests
// ============================================================================

#[test]
fn test_ack_generation_basic() {
    let original_message = parse(
        b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r"
    ).unwrap();
    
    let ack_message = ack(&original_message, AckCode::AA).unwrap();
    
    // Verify structure
    assert_eq!(ack_message.segments.len(), 2);
    assert_eq!(std::str::from_utf8(&ack_message.segments[0].id).unwrap(), "MSH");
    assert_eq!(std::str::from_utf8(&ack_message.segments[1].id).unwrap(), "MSA");
}

#[test]
fn test_ack_generation_with_all_ack_codes() {
    let original_message = parse(
        b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\r"
    ).unwrap();
    
    // Test all ACK codes
    for code in [AckCode::AA, AckCode::AE, AckCode::AR, AckCode::CA, AckCode::CE, AckCode::CR] {
        let ack_message = ack(&original_message, code).unwrap();
        assert_eq!(ack_message.segments.len(), 2);
        
        // Verify MSA segment contains correct code
        let msa = &ack_message.segments[1];
        let ack_code_value = get_field_value(msa, 1).unwrap();
        assert_eq!(ack_code_value, code.as_str());
    }
}

#[test]
fn test_ack_swaps_sending_receiving_applications() {
    let original_message = parse(
        b"MSH|^~\\&|AppA|FacA|AppB|FacB|20250128152312||ADT^A01|MSG123|P|2.5.1\r"
    ).unwrap();
    
    let ack_message = ack(&original_message, AckCode::AA).unwrap();
    
    // MSH-3 (sending app) in ACK should be AppB (original receiving)
    let ack_msh = &ack_message.segments[0];
    let sending_app = get_field_value(ack_msh, 2).unwrap();
    assert_eq!(sending_app, "AppB");
    
    // MSH-5 (receiving app) in ACK should be AppA (original sending)
    let receiving_app = get_field_value(ack_msh, 4).unwrap();
    assert_eq!(receiving_app, "AppA");
}

#[test]
fn test_ack_swaps_sending_receiving_facilities() {
    let original_message = parse(
        b"MSH|^~\\&|AppA|FacA|AppB|FacB|20250128152312||ADT^A01|MSG123|P|2.5.1\r"
    ).unwrap();
    
    let ack_message = ack(&original_message, AckCode::AA).unwrap();
    
    // MSH-4 (sending fac) in ACK should be FacB (original receiving)
    let ack_msh = &ack_message.segments[0];
    let sending_fac = get_field_value(ack_msh, 3).unwrap();
    assert_eq!(sending_fac, "FacB");
    
    // MSH-6 (receiving fac) in ACK should be FacA (original sending)
    let receiving_fac = get_field_value(ack_msh, 5).unwrap();
    assert_eq!(receiving_fac, "FacA");
}

#[test]
fn test_ack_preserves_control_id() {
    let original_message = parse(
        b"MSH|^~\\&|AppA|FacA|AppB|FacB|20250128152312||ADT^A01|MSG123|P|2.5.1\r"
    ).unwrap();
    
    let ack_message = ack(&original_message, AckCode::AA).unwrap();
    
    // MSA-2 should contain the original control ID
    let msa = &ack_message.segments[1];
    let control_id = get_field_value(msa, 2).unwrap();
    assert_eq!(control_id, "MSG123");
}

#[test]
fn test_ack_preserves_processing_id() {
    let original_message = parse(
        b"MSH|^~\\&|AppA|FacA|AppB|FacB|20250128152312||ADT^A01|MSG123|T|2.5.1\r"
    ).unwrap();
    
    let ack_message = ack(&original_message, AckCode::AA).unwrap();
    
    // MSH-11 should contain the original processing ID
    let ack_msh = &ack_message.segments[0];
    let processing_id = get_field_value(ack_msh, 10).unwrap();
    assert_eq!(processing_id, "T");
}

#[test]
fn test_ack_preserves_version() {
    let original_message = parse(
        b"MSH|^~\\&|AppA|FacA|AppB|FacB|20250128152312||ADT^A01|MSG123|P|2.5.1\r"
    ).unwrap();
    
    let ack_message = ack(&original_message, AckCode::AA).unwrap();
    
    // MSH-12 should contain the original version
    let ack_msh = &ack_message.segments[0];
    let version = get_field_value(ack_msh, 11).unwrap();
    assert_eq!(version, "2.5.1");
}

#[test]
fn test_ack_preserves_delimiters() {
    let original_message = parse(
        b"MSH|^~\\&|AppA|FacA|AppB|FacB|20250128152312||ADT^A01|MSG123|P|2.5.1\r"
    ).unwrap();
    
    let ack_message = ack(&original_message, AckCode::AA).unwrap();
    
    // Delimiters should be the same
    assert_eq!(ack_message.delims.field, original_message.delims.field);
    assert_eq!(ack_message.delims.comp, original_message.delims.comp);
    assert_eq!(ack_message.delims.rep, original_message.delims.rep);
    assert_eq!(ack_message.delims.esc, original_message.delims.esc);
    assert_eq!(ack_message.delims.sub, original_message.delims.sub);
}

#[test]
fn test_ack_with_custom_delimiters() {
    // Use custom delimiters: # (field), : (comp), @ (rep), * (esc), % (sub)
    // MSH-2 encoding characters are in order: comp, rep, esc, sub
    let original_message = parse(
        b"MSH#:@*%AppA#FacA#AppB#FacB#20250128152312##ADT:A01#MSG123#P#2.5.1\r"
    ).unwrap();
    
    let ack_message = ack(&original_message, AckCode::AA).unwrap();
    
    // Verify custom delimiters are preserved
    // MSH-2 format: comp + rep + esc + sub = :@*%
    assert_eq!(ack_message.delims.field, '#');
    assert_eq!(ack_message.delims.comp, ':');
    assert_eq!(ack_message.delims.rep, '@');
    assert_eq!(ack_message.delims.esc, '*');
    assert_eq!(ack_message.delims.sub, '%');
}

// ============================================================================
// ACK with Error Tests
// ============================================================================

#[test]
fn test_ack_with_error_segment() {
    let original_message = parse(
        b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\r"
    ).unwrap();
    
    let ack_message = ack_with_error(
        &original_message, 
        AckCode::AE, 
        Some("Processing error occurred")
    ).unwrap();
    
    assert_eq!(ack_message.segments.len(), 3);
    assert_eq!(std::str::from_utf8(&ack_message.segments[0].id).unwrap(), "MSH");
    assert_eq!(std::str::from_utf8(&ack_message.segments[1].id).unwrap(), "MSA");
    assert_eq!(std::str::from_utf8(&ack_message.segments[2].id).unwrap(), "ERR");
}

#[test]
fn test_ack_with_error_none() {
    let original_message = parse(
        b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\r"
    ).unwrap();
    
    let ack_message = ack_with_error(
        &original_message, 
        AckCode::AE, 
        None
    ).unwrap();
    
    // Should be same as basic ack when error is None
    assert_eq!(ack_message.segments.len(), 2);
}

#[test]
fn test_ack_with_ar_code_and_error() {
    let original_message = parse(
        b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\r"
    ).unwrap();
    
    let ack_message = ack_with_error(
        &original_message, 
        AckCode::AR, 
        Some("Invalid message structure")
    ).unwrap();
    
    assert_eq!(ack_message.segments.len(), 3);
    
    // Verify MSA contains AR code
    let msa = &ack_message.segments[1];
    let ack_code = get_field_value(msa, 1).unwrap();
    assert_eq!(ack_code, "AR");
}

#[test]
fn test_err_segment_contains_error_message() {
    let original_message = parse(
        b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\r"
    ).unwrap();
    
    let error_msg = "Segment sequence error";
    let ack_message = ack_with_error(
        &original_message, 
        AckCode::AE, 
        Some(error_msg)
    ).unwrap();
    
    // ERR-3 should contain the error message
    let err = &ack_message.segments[2];
    let err_message = get_field_value(err, 3).unwrap();
    assert_eq!(err_message, error_msg);
}

// ============================================================================
// Edge Cases and Error Handling Tests
// ============================================================================

#[test]
fn test_ack_with_minimal_message() {
    let original_message = parse(
        b"MSH|^~\\&|||||||ACK|||2.5.1\r"
    ).unwrap();
    
    let ack_message = ack(&original_message, AckCode::AA).unwrap();
    
    assert_eq!(ack_message.segments.len(), 2);
}

#[test]
fn test_ack_with_empty_fields() {
    let original_message = parse(
        b"MSH|^~\\&|AppA||AppB|||20250128152312||ADT^A01|MSG123|P|2.5.1\r"
    ).unwrap();
    
    let ack_message = ack(&original_message, AckCode::AA).unwrap();
    
    // Should still generate valid ACK
    assert_eq!(ack_message.segments.len(), 2);
}

#[test]
fn test_ack_with_missing_optional_fields() {
    let original_message = parse(
        b"MSH|^~\\&|AppA|FacA|AppB|FacB|20250128152312||ADT^A01|MSG123\r"
    ).unwrap();
    
    let ack_message = ack(&original_message, AckCode::AA).unwrap();
    
    // Should still generate valid ACK with defaults
    assert_eq!(ack_message.segments.len(), 2);
}

#[test]
fn test_ack_with_special_characters_in_fields() {
    let original_message = parse(
        b"MSH|^~\\&|App\\F\\Special|Fac^Test|Recv~App|Fac\\E\\Esc|20250128152312||ADT^A01|MSG123|P|2.5.1\r"
    ).unwrap();
    
    let ack_message = ack(&original_message, AckCode::AA).unwrap();
    
    // Should handle special characters correctly
    assert_eq!(ack_message.segments.len(), 2);
}

#[test]
fn test_ack_with_long_control_id() {
    let long_id = "THIS_IS_A_VERY_LONG_MESSAGE_CONTROL_ID_1234567890";
    let original_message = parse(
        format!("MSH|^~\\&|AppA|FacA|AppB|FacB|20250128152312||ADT^A01|{}|P|2.5.1\r", long_id).as_bytes()
    ).unwrap();
    
    let ack_message = ack(&original_message, AckCode::AA).unwrap();
    
    let msa = &ack_message.segments[1];
    let control_id = get_field_value(msa, 2).unwrap();
    assert_eq!(control_id, long_id);
}

#[test]
fn test_ack_with_unicode_in_fields() {
    let original_message = parse(
        "MSH|^~\\&|AppÄ|Facö|Recvß|Facü|20250128152312||ADT^A01|MSG123|P|2.5.1\r".as_bytes()
    ).unwrap();
    
    let ack_message = ack(&original_message, AckCode::AA).unwrap();
    
    // Should handle Unicode correctly
    assert_eq!(ack_message.segments.len(), 2);
}

// ============================================================================
// MSA Segment Tests
// ============================================================================

#[test]
fn test_msa_segment_structure() {
    let original_message = parse(
        b"MSH|^~\\&|AppA|FacA|AppB|FacB|20250128152312||ADT^A01|MSG123|P|2.5.1\r"
    ).unwrap();
    
    let ack_message = ack(&original_message, AckCode::AA).unwrap();
    let msa = &ack_message.segments[1];
    
    // MSA-1: Acknowledgment Code
    let ack_code = get_field_value(msa, 1).unwrap();
    assert_eq!(ack_code, "AA");
    
    // MSA-2: Message Control ID
    let control_id = get_field_value(msa, 2).unwrap();
    assert_eq!(control_id, "MSG123");
}

// ============================================================================
// ERR Segment Tests
// ============================================================================

#[test]
fn test_err_segment_structure() {
    let original_message = parse(
        b"MSH|^~\\&|AppA|FacA|AppB|FacB|20250128152312||ADT^A01|MSG123|P|2.5.1\r"
    ).unwrap();
    
    let ack_message = ack_with_error(
        &original_message, 
        AckCode::AE, 
        Some("Test error message")
    ).unwrap();
    
    let err = &ack_message.segments[2];
    
    // ERR-1: Error Code and Location (empty in our implementation)
    assert!(get_field_value(err, 1).is_some() || err.fields.len() >= 1);
    
    // ERR-3: HL7 Error Code
    let error_message = get_field_value(err, 3).unwrap();
    assert_eq!(error_message, "Test error message");
}

// ============================================================================
// Helper Function Tests
// ============================================================================

#[test]
fn test_get_field_value_valid() {
    let original_message = parse(
        b"MSH|^~\\&|AppA|FacA|AppB|FacB|20250128152312||ADT^A01|MSG123|P|2.5.1\r"
    ).unwrap();
    
    let msh = &original_message.segments[0];
    
    // MSH-3 (field index 2 due to MSH special handling)
    let sending_app = get_field_value(msh, 2).unwrap();
    assert_eq!(sending_app, "AppA");
}

#[test]
fn test_get_field_value_out_of_bounds() {
    let original_message = parse(
        b"MSH|^~\\&|AppA|FacA|AppB|FacB|20250128152312||ADT^A01|MSG123|P|2.5.1\r"
    ).unwrap();
    
    let msh = &original_message.segments[0];
    
    // Request field beyond available
    let result = get_field_value(msh, 100);
    assert!(result.is_none());
}

#[test]
fn test_get_field_value_empty_field() {
    let original_message = parse(
        b"MSH|^~\\&|AppA||AppB|FacB|20250128152312||ADT^A01|MSG123|P|2.5.1\r"
    ).unwrap();
    
    let msh = &original_message.segments[0];
    
    // MSH-4 should be empty
    let sending_fac = get_field_value(msh, 3);
    // Empty fields may return None or empty string depending on implementation
    assert!(sending_fac.is_none() || sending_fac.unwrap().is_empty());
}

// ============================================================================
// Multiple Segment Message Tests
// ============================================================================

#[test]
fn test_ack_for_message_with_multiple_segments() {
    let original_message = parse(
        b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||12345^^^HOSP^MR||Doe^John^Robert||19700101|M\rPV1|1|I|ICU^101\r"
    ).unwrap();
    
    let ack_message = ack(&original_message, AckCode::AA).unwrap();
    
    // ACK should only have MSH and MSA (not the original segments)
    assert_eq!(ack_message.segments.len(), 2);
}

#[test]
fn test_ack_for_message_with_many_repetitions() {
    let original_message = parse(
        b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||12345^^^HOSP^MR||Doe^John~Smith^Jane~Brown^Bob\r"
    ).unwrap();
    
    let ack_message = ack(&original_message, AckCode::AA).unwrap();
    
    assert_eq!(ack_message.segments.len(), 2);
}

// ============================================================================
// Timestamp Tests
// ============================================================================

#[test]
fn test_ack_timestamp_format() {
    let original_message = parse(
        b"MSH|^~\\&|AppA|FacA|AppB|FacB|20250128152312||ADT^A01|MSG123|P|2.5.1\r"
    ).unwrap();
    
    let ack_message = ack(&original_message, AckCode::AA).unwrap();
    
    // MSH-7 should contain a timestamp in format YYYYMMDDHHmmss
    let ack_msh = &ack_message.segments[0];
    let timestamp = get_field_value(ack_msh, 6).unwrap();
    
    // Verify format: 14 digits
    assert_eq!(timestamp.len(), 14);
    assert!(timestamp.chars().all(|c| c.is_ascii_digit()));
}

// ============================================================================
// Commit Acknowledgment Tests (Enhanced Mode)
// ============================================================================

#[test]
fn test_commit_accept_ack() {
    let original_message = parse(
        b"MSH|^~\\&|AppA|FacA|AppB|FacB|20250128152312||ADT^A01|MSG123|P|2.5.1\r"
    ).unwrap();
    
    let ack_message = ack(&original_message, AckCode::CA).unwrap();
    
    let msa = &ack_message.segments[1];
    let ack_code = get_field_value(msa, 1).unwrap();
    assert_eq!(ack_code, "CA");
}

#[test]
fn test_commit_error_ack() {
    let original_message = parse(
        b"MSH|^~\\&|AppA|FacA|AppB|FacB|20250128152312||ADT^A01|MSG123|P|2.5.1\r"
    ).unwrap();
    
    let ack_message = ack(&original_message, AckCode::CE).unwrap();
    
    let msa = &ack_message.segments[1];
    let ack_code = get_field_value(msa, 1).unwrap();
    assert_eq!(ack_code, "CE");
}

#[test]
fn test_commit_reject_ack() {
    let original_message = parse(
        b"MSH|^~\\&|AppA|FacA|AppB|FacB|20250128152312||ADT^A01|MSG123|P|2.5.1\r"
    ).unwrap();
    
    let ack_message = ack(&original_message, AckCode::CR).unwrap();
    
    let msa = &ack_message.segments[1];
    let ack_code = get_field_value(msa, 1).unwrap();
    assert_eq!(ack_code, "CR");
}
