//! Property-based tests for hl7v2-normalize using proptest

use hl7v2_normalize::normalize;
use proptest::prelude::*;

// =============================================================================
// Basic Message Property Tests
// =============================================================================

/// Generate a valid MSH segment
fn msh_segment() -> impl Strategy<Value = String> {
    // MSH segment with standard delimiters
    Just("MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|MSG00001|P|2.5.1".to_string())
}

/// Generate a simple PID segment
fn pid_segment() -> impl Strategy<Value = String> {
    (
        Just("PID|1||".to_string()),
        "[0-9]{6}",
        Just("^^^HOSP^MR||".to_string()),
        "[A-Z][a-z]+",
        Just("^".to_string()),
        "[A-Z][a-z]+",
    )
        .prop_map(|(a, b, c, d, e, f)| format!("{}{}{}{}{}{}", a, b, c, d, e, f))
}

proptest! {
    #[test]
    fn test_normalize_preserves_valid_structure(
        msh in msh_segment(),
        pid in pid_segment()
    ) {
        let hl7 = format!("{}\r{}\r", msh, pid);
        let hl7_bytes = hl7.as_bytes();

        let result = normalize(hl7_bytes, false);

        prop_assert!(result.is_ok());

        let normalized = result.unwrap();
        let normalized_str = String::from_utf8(normalized).unwrap();

        // Should contain both segments
        prop_assert!(normalized_str.contains("MSH|"));
        prop_assert!(normalized_str.contains("PID|"));
    }
}

// =============================================================================
// Delimiter Property Tests
// =============================================================================

proptest! {
    #[test]
    fn test_canonical_normalization_produces_standard_delimiters(
        field_content in "[A-Za-z0-9]+"
    ) {
        let hl7 = format!(
            "MSH|^~\\&|{}|{}|{}|{}|20250128152312||ADT^A01|MSG00001|P|2.5.1\r",
            field_content, field_content, field_content, field_content
        );

        let normalized = normalize(hl7.as_bytes(), true).unwrap();
        let normalized_str = String::from_utf8(normalized).unwrap();

        // Should start with standard delimiters
        prop_assert!(normalized_str.starts_with("MSH|^~\\&|"));
    }
}

// =============================================================================
// Roundtrip Property Tests
// =============================================================================

proptest! {
    #[test]
    fn test_normalize_roundtrip(
        mrn in "[0-9]{6}",
        last_name in "[A-Z][a-z]{2,10}",
        first_name in "[A-Z][a-z]{2,10}"
    ) {
        let hl7 = format!(
            "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|MSG00001|P|2.5.1\rPID|1||{}^^^HOSP^MR||{}^{}\r",
            mrn, last_name, first_name
        );

        let normalized1 = normalize(hl7.as_bytes(), false).unwrap();
        let normalized2 = normalize(&normalized1, false).unwrap();

        // Normalizing twice should produce the same result
        prop_assert_eq!(normalized1, normalized2);
    }
}

// =============================================================================
// Idempotency Property Tests
// =============================================================================

proptest! {
    #[test]
    fn test_canonical_normalization_idempotent(
        mrn in "[0-9]{6}"
    ) {
        let hl7 = format!(
            "MSH|^~\\&|App|Fac|App|Fac|20250128152312||ADT^A01|MSG|P|2.5.1\rPID|1||{}^^^HOSP^MR\r",
            mrn
        );

        let canonical1 = normalize(hl7.as_bytes(), true).unwrap();
        let canonical2 = normalize(&canonical1, true).unwrap();

        // Canonical normalization should be idempotent
        prop_assert_eq!(canonical1, canonical2);
    }
}

// =============================================================================
// Content Preservation Property Tests
// =============================================================================

proptest! {
    #[test]
    fn test_normalize_preserves_field_content(
        content in "[A-Za-z0-9_]+"
    ) {
        let hl7 = format!(
            "MSH|^~\\&|{}|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|MSG00001|P|2.5.1\r",
            content
        );

        let normalized = normalize(hl7.as_bytes(), false).unwrap();
        let normalized_str = String::from_utf8(normalized).unwrap();

        // Content should be preserved
        prop_assert!(normalized_str.contains(&content));
    }

    #[test]
    fn test_normalize_preserves_segment_count(
        num_segments in 1usize..20
    ) {
        let mut hl7 = "MSH|^~\\&|App|Fac|App|Fac|20250128152312||ADT^A01|MSG|P|2.5.1\r".to_string();

        for i in 0..num_segments {
            hl7.push_str(&format!("NTE|{}|Note text\r", i));
        }

        let normalized = normalize(hl7.as_bytes(), false).unwrap();
        let normalized_str = String::from_utf8(normalized).unwrap();

        // Count segments (MSH + NTEs)
        let segment_count = normalized_str.matches('\r').count();
        prop_assert_eq!(segment_count, num_segments + 1);
    }
}

// =============================================================================
// Error Handling Property Tests
// =============================================================================

proptest! {
    #[test]
    fn test_normalize_rejects_missing_msh(
        content in ".*"
    ) {
        prop_assume!(!content.starts_with("MSH|"));

        let result = normalize(content.as_bytes(), true);

        // Should fail for messages without MSH segment
        prop_assert!(result.is_err());
    }
}

// =============================================================================
// Consistency Property Tests
// =============================================================================

proptest! {
    #[test]
    fn test_normalize_deterministic(
        mrn in "[0-9]{6}",
        name in "[A-Z][a-z]+"
    ) {
        let hl7 = format!(
            "MSH|^~\\&|App|Fac|App|Fac|20250128152312||ADT^A01|MSG|P|2.5.1\rPID|1||{}^^^HOSP^MR||{}^{}\r",
            mrn, name, name
        );

        let normalized1 = normalize(hl7.as_bytes(), false).unwrap();
        let normalized2 = normalize(hl7.as_bytes(), false).unwrap();

        // Same input should always produce same output
        prop_assert_eq!(normalized1, normalized2);
    }

    #[test]
    fn test_normalize_canonical_deterministic(
        mrn in "[0-9]{6}"
    ) {
        let hl7 = format!(
            "MSH|^~\\&|App|Fac|App|Fac|20250128152312||ADT^A01|MSG|P|2.5.1\rPID|1||{}^^^HOSP^MR\r",
            mrn
        );

        let canonical1 = normalize(hl7.as_bytes(), true).unwrap();
        let canonical2 = normalize(hl7.as_bytes(), true).unwrap();

        prop_assert_eq!(canonical1, canonical2);
    }
}

// =============================================================================
// Segment Structure Property Tests
// =============================================================================

proptest! {
    #[test]
    fn test_normalize_produces_valid_segments(
        mrn in "[0-9]{6}"
    ) {
        let hl7 = format!(
            "MSH|^~\\&|App|Fac|App|Fac|20250128152312||ADT^A01|MSG|P|2.5.1\rPID|1||{}^^^HOSP^MR\r",
            mrn
        );

        let normalized = normalize(hl7.as_bytes(), false).unwrap();

        // Should be parseable
        let parsed = hl7v2_parser::parse(&normalized);
        prop_assert!(parsed.is_ok());

        let message = parsed.unwrap();
        prop_assert_eq!(message.segments.len(), 2);
        prop_assert_eq!(&message.segments[0].id, b"MSH");
        prop_assert_eq!(&message.segments[1].id, b"PID");
    }
}

// =============================================================================
// Unicode Property Tests
// =============================================================================

proptest! {
    #[test]
    fn test_normalize_handles_unicode(
        name in "\\p{L}+"
    ) {
        let hl7 = format!(
            "MSH|^~\\&|App|Fac|App|Fac|20250128152312||ADT^A01|MSG|P|2.5.1\rPID|1||123456^^^HOSP^MR||{}^{}\r",
            name, name
        );

        let result = normalize(hl7.as_bytes(), false);

        // Should handle unicode
        prop_assert!(result.is_ok());

        let normalized = result.unwrap();
        let normalized_str = String::from_utf8(normalized).unwrap();
        prop_assert!(normalized_str.contains(&name));
    }
}

// =============================================================================
// Field Separator Property Tests
// =============================================================================

proptest! {
    #[test]
    fn test_field_separator_always_pipe(
        content in "[A-Za-z0-9]+"
    ) {
        let hl7 = format!(
            "MSH|^~\\&|{}|{}|{}|{}|20250128152312||ADT^A01|MSG|P|2.5.1\r",
            content, content, content, content
        );

        let normalized = normalize(hl7.as_bytes(), false).unwrap();

        // Parse to verify field separator
        let message = hl7v2_parser::parse(&normalized).unwrap();
        prop_assert_eq!(message.delims.field, '|');
    }
}
