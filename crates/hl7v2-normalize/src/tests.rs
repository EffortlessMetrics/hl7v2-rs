//! Unit tests for hl7v2-normalize

use super::*;

// =============================================================================
// Basic Normalization Tests
// =============================================================================

#[test]
fn normalize_basic_message() {
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\r";

    let normalized = normalize(hl7, false).unwrap();

    assert!(normalized.starts_with(b"MSH|^~\\&|"));
}

#[test]
fn normalize_with_canonical_delimiters() {
    let hl7 = b"MSH*%$!?*SendingApp*SendingFac*ReceivingApp*ReceivingFac*20250128152312**ADT%A01*ABC123*P*2.5.1\r";

    let normalized = normalize(hl7, true).unwrap();
    let normalized_str = String::from_utf8(normalized).unwrap();

    assert!(normalized_str.starts_with("MSH|^~\\&|"));
}

#[test]
fn normalize_preserves_custom_delimiters_when_not_canonical() {
    let hl7 = b"MSH*%$!?*SendingApp*SendingFac*ReceivingApp*ReceivingFac*20250128152312**ADT%A01*ABC123*P*2.5.1\r";

    let normalized = normalize(hl7, false).unwrap();

    assert!(normalized.starts_with(b"MSH*%$!?*"));
}

// =============================================================================
// Multi-Segment Tests
// =============================================================================

#[test]
fn normalize_multi_segment_message() {
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r";

    let normalized = normalize(hl7, false).unwrap();
    let normalized_str = String::from_utf8(normalized).unwrap();

    assert!(normalized_str.contains("MSH|"));
    assert!(normalized_str.contains("PID|"));
    assert!(normalized_str.contains("Doe^John"));
}

#[test]
fn normalize_multi_segment_with_canonical() {
    let hl7 = b"MSH*%$!?*SendingApp*SendingFac*ReceivingApp*ReceivingFac*20250128152312**ADT%A01*ABC123*P*2.5.1\rPID*1**123456%%%HOSP%MR**Doe%John\r";

    let normalized = normalize(hl7, true).unwrap();
    let normalized_str = String::from_utf8(normalized).unwrap();

    assert!(normalized_str.starts_with("MSH|^~\\&|"));
    assert!(normalized_str.contains("PID|1||123456^^^HOSP^MR||Doe^John"));
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[test]
fn normalize_rejects_invalid_message() {
    let invalid = b"PID|1||12345\r"; // Missing MSH segment
    let err = normalize(invalid, true).unwrap_err();

    assert!(matches!(err, Error::InvalidSegmentId));
}

#[test]
fn normalize_rejects_empty_input() {
    let empty = b"";
    let result = normalize(empty, true);

    assert!(result.is_err());
}

#[test]
fn normalize_accepts_nonstandard_segment_id() {
    // The normalize function doesn't validate segment IDs - it just normalizes format
    let nonstandard = b"MSH|^~\\&|App|Fac\rXYZ1|1||123\r";
    let result = normalize(nonstandard, true);

    // Should succeed - normalize doesn't validate segment ID format
    assert!(result.is_ok());
}

// =============================================================================
// Roundtrip Tests
// =============================================================================

#[test]
fn normalize_roundtrips_valid_message() {
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r";

    let normalized = normalize(hl7, false).unwrap();
    let reparsed = hl7v2_parser::parse(&normalized).unwrap();

    assert_eq!(reparsed.segments.len(), 2);
    assert_eq!(&reparsed.segments[0].id, b"MSH");
    assert_eq!(&reparsed.segments[1].id, b"PID");
}

#[test]
fn normalize_roundtrips_with_canonical() {
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r";

    let normalized = normalize(hl7, true).unwrap();
    let reparsed = hl7v2_parser::parse(&normalized).unwrap();

    // Should still be valid
    assert_eq!(reparsed.segments.len(), 2);

    // Delimiters should be canonical
    assert_eq!(reparsed.delims.field, '|');
    assert_eq!(reparsed.delims.comp, '^');
    assert_eq!(reparsed.delims.rep, '~');
    assert_eq!(reparsed.delims.esc, '\\');
    assert_eq!(reparsed.delims.sub, '&');
}

// =============================================================================
// Delimiter Tests
// =============================================================================

#[test]
fn normalize_converts_all_delimiter_types() {
    // Custom delimiters: * (field), % (component), $ (repetition), ! (escape), ? (subcomponent)
    let hl7 = b"MSH*%$!?*App*Fac*App*Fac*20250128152312**ADT%A01*123*P*2.5\rPID*1**123%%%MR**Doe%John~Jane\r";

    let normalized = normalize(hl7, true).unwrap();
    let normalized_str = String::from_utf8(normalized).unwrap();

    // Check canonical delimiters are used
    assert!(normalized_str.contains("|"));
    assert!(normalized_str.contains("^"));
    assert!(normalized_str.contains("~"));
}

#[test]
fn normalize_preserves_field_separator() {
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\r";

    let normalized = normalize(hl7, false).unwrap();

    // Field separator should always be |
    assert!(normalized.windows(4).any(|w| w == b"MSH|"));
}

// =============================================================================
// Edge Cases Tests
// =============================================================================

#[test]
fn normalize_message_with_empty_fields() {
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||||||\r";

    let normalized = normalize(hl7, false).unwrap();
    let normalized_str = String::from_utf8(normalized).unwrap();

    assert!(normalized_str.contains("PID|1||||||"));
}

#[test]
fn normalize_message_with_special_characters() {
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||12345||O\\F\\Brien^John\r";

    let normalized = normalize(hl7, false).unwrap();
    let normalized_str = String::from_utf8(normalized).unwrap();

    // Escaped characters should be preserved
    assert!(normalized_str.contains("O\\F\\Brien"));
}

#[test]
fn normalize_message_with_repetitions() {
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||12345~67890||Doe^John\r";

    let normalized = normalize(hl7, false).unwrap();
    let normalized_str = String::from_utf8(normalized).unwrap();

    assert!(normalized_str.contains("12345~67890"));
}

#[test]
fn normalize_message_with_subcomponents() {
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||12345^^^HOSP&MR||Doe^John\r";

    let normalized = normalize(hl7, false).unwrap();
    let normalized_str = String::from_utf8(normalized).unwrap();

    assert!(normalized_str.contains("HOSP&MR"));
}

// =============================================================================
// Different Message Types Tests
// =============================================================================

#[test]
fn normalize_adt_a01() {
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||12345||Doe^John\rPV1|1|I|ICU^01^01\r";

    let normalized = normalize(hl7, false).unwrap();
    let normalized_str = String::from_utf8(normalized).unwrap();

    assert!(normalized_str.contains("ADT^A01"));
    assert!(normalized_str.contains("PID|"));
    assert!(normalized_str.contains("PV1|"));
}

#[test]
fn normalize_adt_a04() {
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A04|ABC123|P|2.5.1\rPID|1||12345||Doe^John\r";

    let normalized = normalize(hl7, false).unwrap();
    let normalized_str = String::from_utf8(normalized).unwrap();

    assert!(normalized_str.contains("ADT^A04"));
}

#[test]
fn normalize_oru_r01() {
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ORU^R01|ABC123|P|2.5.1\rPID|1||12345||Doe^John\rOBR|1|||1234^Test\rOBX|1|NM|1234^Result||120|mg/dL\r";

    let normalized = normalize(hl7, false).unwrap();
    let normalized_str = String::from_utf8(normalized).unwrap();

    assert!(normalized_str.contains("ORU^R01"));
    assert!(normalized_str.contains("OBR|"));
    assert!(normalized_str.contains("OBX|"));
}

#[test]
fn normalize_ack() {
    let hl7 = b"MSH|^~\\&|ReceivingApp|ReceivingFac|SendingApp|SendingFac|20250128152312||ACK|ABC123|P|2.5.1\rMSA|AA|ABC123|Message accepted\r";

    let normalized = normalize(hl7, false).unwrap();
    let normalized_str = String::from_utf8(normalized).unwrap();

    assert!(normalized_str.contains("ACK"));
    assert!(normalized_str.contains("MSA|AA"));
}

// =============================================================================
// Encoding Characters Tests
// =============================================================================

#[test]
fn normalize_preserves_encoding_characters() {
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\r";

    let normalized = normalize(hl7, false).unwrap();

    // MSH-2 should be preserved
    assert!(normalized.windows(5).any(|w| w == b"|^~\\&"));
}

#[test]
fn normalize_changes_encoding_characters_when_canonical() {
    let hl7 = b"MSH*%$!?*SendingApp*SendingFac*ReceivingApp*ReceivingFac*20250128152312**ADT%A01*ABC123*P*2.5.1\r";

    let normalized = normalize(hl7, true).unwrap();

    // MSH-2 should be canonical
    assert!(normalized.windows(5).any(|w| w == b"|^~\\&"));
}

// =============================================================================
// Segment Terminator Tests
// =============================================================================

#[test]
fn normalize_uses_cr_segment_terminator() {
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||12345||Doe^John\r";

    let normalized = normalize(hl7, false).unwrap();

    // Should end with CR
    assert_eq!(normalized.last(), Some(&b'\r'));
}

#[test]
fn normalize_each_segment_ends_with_cr() {
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||12345||Doe^John\rPV1|1|I\r";

    let normalized = normalize(hl7, false).unwrap();
    let normalized_str = String::from_utf8(normalized).unwrap();

    // Count segments and CRs
    let segments = normalized_str.split('\r').filter(|s| !s.is_empty()).count();
    let crs = normalized_str.matches('\r').count();

    assert_eq!(segments, crs);
}

// =============================================================================
// Consistency Tests
// =============================================================================

#[test]
fn normalize_produces_consistent_output() {
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||12345||Doe^John\r";

    let normalized1 = normalize(hl7, false).unwrap();
    let normalized2 = normalize(hl7, false).unwrap();

    assert_eq!(normalized1, normalized2);
}

#[test]
fn normalize_canonical_produces_consistent_output() {
    let hl7 = b"MSH*%$!?*SendingApp*SendingFac*ReceivingApp*ReceivingFac*20250128152312**ADT%A01*ABC123*P*2.5.1\r";

    let normalized1 = normalize(hl7, true).unwrap();
    let normalized2 = normalize(hl7, true).unwrap();

    assert_eq!(normalized1, normalized2);
}

// =============================================================================
// Long Message Tests
// =============================================================================

#[test]
fn normalize_long_message() {
    let mut hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\r".to_vec();

    // Add many segments
    for i in 0..100 {
        let segment = format!("NTE|{}|Some note text that is somewhat long\r", i);
        hl7.extend_from_slice(segment.as_bytes());
    }

    let normalized = normalize(&hl7, false).unwrap();
    let normalized_str = String::from_utf8(normalized).unwrap();

    // Should contain all segments
    assert_eq!(normalized_str.matches("NTE|").count(), 100);
}

// =============================================================================
// Unicode Tests
// =============================================================================

#[test]
fn normalize_message_with_unicode() {
    let hl7 = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||12345||Müller^Jöhn\r".as_bytes();

    let normalized = normalize(hl7, false).unwrap();
    let normalized_str = String::from_utf8(normalized).unwrap();

    assert!(normalized_str.contains("Müller"));
    assert!(normalized_str.contains("Jöhn"));
}

// =============================================================================
// Version Tests
// =============================================================================

#[test]
fn normalize_preserves_version() {
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\r";

    let normalized = normalize(hl7, false).unwrap();
    let normalized_str = String::from_utf8(normalized).unwrap();

    assert!(normalized_str.contains("2.5.1"));
}

#[test]
fn normalize_different_versions() {
    let versions = ["2.3", "2.3.1", "2.4", "2.5", "2.5.1", "2.6", "2.7"];

    for version in versions {
        let hl7 = format!(
            "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|{}\r",
            version
        );

        let normalized = normalize(hl7.as_bytes(), false).unwrap();
        let normalized_str = String::from_utf8(normalized).unwrap();

        assert!(
            normalized_str.contains(version),
            "Version {} not preserved",
            version
        );
    }
}
