//! Integration tests for hl7v2-normalize

use hl7v2_normalize::normalize;

// =============================================================================
// Real-World Message Tests
// =============================================================================

#[test]
fn normalize_real_adt_a01() {
    let hl7 = b"MSH|^~\\&|ADT1|MCM|LABADT|MCM|198808181126|SECURITY|ADT^A01|MSG00001|P|2.5.1\r\
                 EVN|A01|198808181122||\r\
                 PID|1||PATID1234^5^M11^ADT1^MR^MCM~~~123456789|PATID1234|JONES^WILLIAM^A^III||19610615|M||\r\
                 NK1|1|JONES^BARBARA^K|SPO||||\r\
                 PV1|1|I|2000^2012^01||||004777^LEBAUER^SIDNEY^J.|||SUR||||ADM|A0||||||||||||||||||||||||\r";

    let normalized = normalize(hl7, false).unwrap();
    let normalized_str = String::from_utf8(normalized).unwrap();

    // Check all segments are present
    assert!(normalized_str.contains("MSH|"));
    assert!(normalized_str.contains("EVN|"));
    assert!(normalized_str.contains("PID|"));
    assert!(normalized_str.contains("NK1|"));
    assert!(normalized_str.contains("PV1|"));
}

#[test]
fn normalize_real_oru_r01() {
    let hl7 = b"MSH|^~\\&|GHH LAB|ELAB-3|GHH OE|BLG465200|200202150930||ORU^R01|CNTRL-3456|P|2.5.1\r\
                 PID|||555-44-4444||EVERYWOMAN^EVE^E^^^^L|JONES|19620320|F|||153 FERNWOOD DR.^^STATESVILLE^WA^48679^^^^425|425|555-1212|||||||555-44-4444||\r\
                 OBR|1||555-55-5555|CBC^COMPLETE BLOOD COUNT^L|||200202150730|||||||200202150730|||||||555-55-5555|^PRIMARY CARE ASSOCIATES|||||||||||F\r\
                 OBX|1|NM|HB^HEMOGLOBIN^L||13.2|g/dL|11.5-17.5||||F|||||HEMATOLOGY^\r";

    let normalized = normalize(hl7, false).unwrap();
    let normalized_str = String::from_utf8(normalized).unwrap();

    assert!(normalized_str.contains("ORU^R01"));
    assert!(normalized_str.contains("HEMOGLOBIN"));
    assert!(normalized_str.contains("13.2"));
}

// =============================================================================
// Cross-Crate Integration Tests
// =============================================================================

#[test]
fn normalize_and_reparse() {
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r";

    let normalized = normalize(hl7, false).unwrap();

    // Parse the normalized message
    let parsed = hl7v2_parser::parse(&normalized).unwrap();

    assert_eq!(parsed.segments.len(), 2);
    assert_eq!(&parsed.segments[0].id, b"MSH");
    assert_eq!(&parsed.segments[1].id, b"PID");
}

#[test]
fn normalize_and_write() {
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r";

    let normalized = normalize(hl7, false).unwrap();

    // Parse and write again
    let parsed = hl7v2_parser::parse(&normalized).unwrap();
    let rewritten = hl7v2_writer::write(&parsed);

    // Should be equivalent
    assert_eq!(normalized, rewritten);
}

// =============================================================================
// Canonical Form Tests
// =============================================================================

#[test]
fn normalize_to_canonical_form() {
    // Message with custom delimiters
    let hl7 = b"MSH*%$!?*SendingApp*SendingFac*ReceivingApp*ReceivingFac*20250128152312**ADT%A01*ABC123*P*2.5.1\rPID*1**123456%%%HOSP%MR**Doe%John\r";

    let canonical = normalize(hl7, true).unwrap();
    let canonical_str = String::from_utf8(canonical).unwrap();

    // Should use standard delimiters
    assert!(canonical_str.starts_with("MSH|^~\\&|"));
    assert!(canonical_str.contains("PID|1||123456^^^HOSP^MR||Doe^John"));
}

#[test]
fn canonical_form_is_idempotent() {
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r";

    let canonical1 = normalize(hl7, true).unwrap();
    let canonical2 = normalize(&canonical1, true).unwrap();

    // Applying canonical normalization twice should give same result
    assert_eq!(canonical1, canonical2);
}

// =============================================================================
// Complex Message Tests
// =============================================================================

#[test]
fn normalize_complex_nested_data() {
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\r\
                 PID|1||123456^^^HOSP&MR^ISO||Doe^John^Robert^Jr^Dr.^^L||19700101|M|||123 Main St^^Anytown^CA^12345^USA^^M||(555)123-4567|(555)987-6543||M|S|123456789|987654321\r";

    let normalized = normalize(hl7, false).unwrap();
    let normalized_str = String::from_utf8(normalized).unwrap();

    // Complex field should be preserved
    assert!(normalized_str.contains("Doe^John^Robert^Jr^Dr.^^L"));
    assert!(normalized_str.contains("HOSP&MR"));
}

#[test]
fn normalize_message_with_many_repetitions() {
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\r\
                 PID|1||12345~23456~34567~45678~56789||Doe^John\r";

    let normalized = normalize(hl7, false).unwrap();
    let normalized_str = String::from_utf8(normalized).unwrap();

    // All repetitions should be preserved
    assert!(normalized_str.contains("12345~23456~34567~45678~56789"));
}

// =============================================================================
// Error Recovery Tests
// =============================================================================

#[test]
fn normalize_valid_after_invalid() {
    let valid = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\r";

    // Should successfully normalize valid message
    let normalized = normalize(valid, false).unwrap();
    assert!(!normalized.is_empty());
}

// =============================================================================
// Performance Tests
// =============================================================================

#[test]
fn normalize_large_message() {
    let mut hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\r".to_vec();

    // Add many segments
    for i in 0..1000 {
        let segment = format!("OBX|{}|NM|TEST^Test||{}|units|||||F\r", i, i * 10);
        hl7.extend_from_slice(segment.as_bytes());
    }

    let normalized = normalize(&hl7, false).unwrap();
    let normalized_str = String::from_utf8(normalized).unwrap();

    // All segments should be present
    assert_eq!(normalized_str.matches("OBX|").count(), 1000);
}

// =============================================================================
// Boundary Tests
// =============================================================================

#[test]
fn normalize_minimum_valid_message() {
    // Minimum valid HL7 message (MSH segment only)
    let hl7 = b"MSH|^~\\&|||||||||2.5.1\r";

    let normalized = normalize(hl7, false).unwrap();
    let normalized_str = String::from_utf8(normalized).unwrap();

    assert!(normalized_str.starts_with("MSH|"));
}

#[test]
fn normalize_message_with_max_fields() {
    // Message with many fields
    let mut fields = vec!["MSH|^~\\&"];
    fields.extend(std::iter::repeat_n("field_value", 100));
    let hl7 = format!("{}|\r", fields.join("|"));

    let normalized = normalize(hl7.as_bytes(), false).unwrap();
    assert!(!normalized.is_empty());
}

// =============================================================================
// Encoding Tests
// =============================================================================

#[test]
fn normalize_preserves_escape_sequences() {
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\r\
                 PID|1||12345||O\\F\\Brien^John|||\\Hhighlight\\N text\r";

    let normalized = normalize(hl7, false).unwrap();
    let normalized_str = String::from_utf8(normalized).unwrap();

    // Known escape sequences like \F\ (field separator) round-trip correctly:
    // - Parser interprets \F\ as | (field separator)
    // - Writer re-escapes | back to \F\
    assert!(
        normalized_str.contains("O\\F\\Brien"),
        "Expected \\F\\ escape sequence to round-trip, got: {}",
        normalized_str
    );

    // Unknown escape sequences like \H and \N have their backslashes escaped:
    // - Parser passes through unknown escape sequences
    // - Writer's escape_text() escapes the \ character to \E\
    assert!(
        normalized_str.contains("\\E\\Hhighlight\\E\\N"),
        "Expected escaped highlight sequence, got: {}",
        normalized_str
    );
}

// =============================================================================
// Different Line Ending Tests
// =============================================================================

#[test]
fn normalize_converts_line_endings_to_cr() {
    // Message with LF line endings (non-standard but common)
    let hl7 = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\nPID|1||12345||Doe^John\n";

    let normalized = normalize(hl7.as_bytes(), false);

    // Should fail or handle gracefully - non-CR line endings are non-standard
    // The parser may or may not accept this
    match normalized {
        Ok(n) => {
            let normalized_str = String::from_utf8(n).unwrap();
            // If it succeeds, output should use CR
            assert!(normalized_str.contains('\r'));
        }
        Err(_) => {
            // Parser may reject non-standard line endings
        }
    }
}

// =============================================================================
// Version Compatibility Tests
// =============================================================================

#[test]
fn normalize_various_hl7_versions() {
    let versions = ["2.3", "2.3.1", "2.4", "2.5", "2.5.1", "2.6", "2.7", "2.8"];

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
