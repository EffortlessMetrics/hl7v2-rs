//! Property-based tests for hl7v2-path crate using proptest
//!
//! These tests verify path parsing and generation properties hold for arbitrary inputs.

use hl7v2_path::{Path, PathError, parse_path};
use proptest::prelude::*;

// =============================================================================
// Strategies for generating valid path components
// =============================================================================

/// Generate valid 3-character segment IDs (uppercase alphanumeric)
fn segment_id() -> impl Strategy<Value = String> {
    // HL7 segment IDs are exactly 3 uppercase letters
    "[A-Z]{3}"
}

/// Generate valid field numbers (1-based, reasonable upper bound)
fn field_number() -> impl Strategy<Value = usize> {
    1..=999usize
}

/// Generate valid repetition indices (1-based, reasonable upper bound)
fn repetition_index() -> impl Strategy<Value = usize> {
    1..=100usize
}

/// Generate valid component numbers (1-based)
fn component_number() -> impl Strategy<Value = usize> {
    1..=50usize
}

/// Generate valid subcomponent numbers (1-based)
fn subcomponent_number() -> impl Strategy<Value = usize> {
    1..=20usize
}

/// Generate a complete valid Path with all optional fields
/// Note: subcomponent requires component to be set (HL7 path format limitation)
fn arbitrary_path() -> impl Strategy<Value = Path> {
    // First decide which "level" of path we're generating
    // Level 0: Just segment.field
    // Level 1: segment.field[rep]
    // Level 2: segment.field.component
    // Level 3: segment.field[rep].component
    // Level 4: segment.field.component.subcomponent
    // Level 5: segment.field[rep].component.subcomponent

    let simple = (segment_id(), field_number()).prop_map(|(seg, field)| Path::new(&seg, field));

    let with_rep = (segment_id(), field_number(), repetition_index())
        .prop_map(|(seg, field, rep)| Path::new(&seg, field).with_repetition(rep));

    let with_comp = (segment_id(), field_number(), component_number())
        .prop_map(|(seg, field, comp)| Path::new(&seg, field).with_component(comp));

    let with_rep_and_comp = (
        segment_id(),
        field_number(),
        repetition_index(),
        component_number(),
    )
        .prop_map(|(seg, field, rep, comp)| {
            Path::new(&seg, field)
                .with_repetition(rep)
                .with_component(comp)
        });

    let with_comp_and_sub = (
        segment_id(),
        field_number(),
        component_number(),
        subcomponent_number(),
    )
        .prop_map(|(seg, field, comp, sub)| {
            Path::new(&seg, field)
                .with_component(comp)
                .with_subcomponent(sub)
        });

    let full = (
        segment_id(),
        field_number(),
        repetition_index(),
        component_number(),
        subcomponent_number(),
    )
        .prop_map(|(seg, field, rep, comp, sub)| {
            Path::new(&seg, field)
                .with_repetition(rep)
                .with_component(comp)
                .with_subcomponent(sub)
        });

    proptest::prop_oneof![
        20 => simple,
        15 => with_rep,
        20 => with_comp,
        15 => with_rep_and_comp,
        10 => with_comp_and_sub,
        10 => full,
    ]
}

/// Generate a path string from components
fn path_string() -> impl Strategy<Value = String> {
    let simple_path =
        (segment_id(), field_number()).prop_map(|(seg, field)| format!("{}.{}", seg, field));

    let with_rep = (segment_id(), field_number(), repetition_index())
        .prop_map(|(seg, field, rep)| format!("{}.{}[{}]", seg, field, rep));

    let with_comp = (segment_id(), field_number(), component_number())
        .prop_map(|(seg, field, comp)| format!("{}.{}.{}", seg, field, comp));

    let with_rep_and_comp = (
        segment_id(),
        field_number(),
        repetition_index(),
        component_number(),
    )
        .prop_map(|(seg, field, rep, comp)| format!("{}.{}[{}].{}", seg, field, rep, comp));

    let full_path = (
        segment_id(),
        field_number(),
        repetition_index(),
        component_number(),
        subcomponent_number(),
    )
        .prop_map(|(seg, field, rep, comp, sub)| {
            format!("{}.{}[{}].{}.{}", seg, field, rep, comp, sub)
        });

    proptest::prop_oneof![
        20 => simple_path,
        15 => with_rep,
        20 => with_comp,
        15 => with_rep_and_comp,
        10 => full_path,
    ]
}

// =============================================================================
// Property Tests: Parsing never panics
// =============================================================================

proptest! {
    /// Test that parsing any arbitrary string never panics
    #[test]
    fn prop_parse_never_panics(s in ".*") {
        let _ = parse_path(&s);
    }
}

proptest! {
    /// Test that parsing any valid path string succeeds
    #[test]
    fn prop_parse_valid_path_succeeds(path_str in path_string()) {
        let result = parse_path(&path_str);
        prop_assert!(result.is_ok(), "Failed to parse valid path: {}", path_str);
    }
}

// =============================================================================
// Property Tests: Roundtrip invariants
// =============================================================================

proptest! {
    /// Test that Path -> to_path_string -> parse_path produces equivalent Path
    #[test]
    fn prop_roundtrip_path_to_string(path in arbitrary_path()) {
        let path_string = path.to_path_string();
        let parsed = parse_path(&path_string).expect("Should parse generated path string");

        prop_assert_eq!(parsed.segment, path.segment);
        prop_assert_eq!(parsed.field, path.field);
        prop_assert_eq!(parsed.repetition, path.repetition);
        prop_assert_eq!(parsed.component, path.component);
        prop_assert_eq!(parsed.subcomponent, path.subcomponent);
    }
}

proptest! {
    /// Test that parsing and re-stringifying is idempotent
    #[test]
    fn prop_parse_stringify_idempotent(path_str in path_string()) {
        let parsed1 = parse_path(&path_str).expect("Should parse valid path");
        let string1 = parsed1.to_path_string();

        let parsed2 = parse_path(&string1).expect("Should parse re-generated path");
        let string2 = parsed2.to_path_string();

        prop_assert_eq!(string1, string2);
        prop_assert_eq!(parsed1, parsed2);
    }
}

// =============================================================================
// Property Tests: Path component invariants
// =============================================================================

proptest! {
    /// Test that segment is always uppercase after parsing
    #[test]
    fn prop_segment_always_uppercase(segment in "[a-zA-Z]{3}", field in field_number()) {
        let path_str = format!("{}.{}", segment, field);
        let parsed = parse_path(&path_str).expect("Should parse path");
        prop_assert!(parsed.segment.chars().all(|c| c.is_ascii_uppercase()));
    }
}

proptest! {
    /// Test that field numbers are always >= 1
    #[test]
    fn prop_field_always_positive(path in arbitrary_path()) {
        prop_assert!(path.field >= 1);
    }
}

proptest! {
    /// Test that repetition is always Some(n) where n >= 1, or None
    #[test]
    fn prop_repetition_valid(path in arbitrary_path()) {
        if let Some(rep) = path.repetition {
            prop_assert!(rep >= 1);
        }
    }
}

proptest! {
    /// Test that component is always Some(n) where n >= 1, or None
    #[test]
    fn prop_component_valid(path in arbitrary_path()) {
        if let Some(comp) = path.component {
            prop_assert!(comp >= 1);
        }
    }
}

proptest! {
    /// Test that subcomponent is always Some(n) where n >= 1, or None
    #[test]
    fn prop_subcomponent_valid(path in arbitrary_path()) {
        if let Some(sub) = path.subcomponent {
            prop_assert!(sub >= 1);
        }
    }
}

// =============================================================================
// Property Tests: Path string format invariants
// =============================================================================

proptest! {
    /// Test that to_path_string always contains at least one dot
    #[test]
    fn prop_path_string_contains_dot(path in arbitrary_path()) {
        let s = path.to_path_string();
        prop_assert!(s.contains('.'), "Path string should contain at least one dot: {}", s);
    }
}

proptest! {
    /// Test that path string starts with segment ID
    #[test]
    fn prop_path_string_starts_with_segment(path in arbitrary_path()) {
        let s = path.to_path_string();
        prop_assert!(s.starts_with(&path.segment), "Path should start with segment: {} vs {}", s, path.segment);
    }
}

proptest! {
    /// Test that repetition brackets are properly formatted
    #[test]
    fn prop_repetition_brackets_valid(path in arbitrary_path()) {
        let s = path.to_path_string();
        if path.repetition.is_some() {
            prop_assert!(s.contains('[') && s.contains(']'));
            // Check that '[' comes before ']'
            let open = s.find('[');
            let close = s.find(']');
            if let (Some(o), Some(c)) = (open, close) {
                prop_assert!(o < c);
            }
        }
    }
}

// =============================================================================
// Property Tests: MSH special handling
// =============================================================================

proptest! {
    /// Test that is_msh returns true only for MSH segments
    #[test]
    fn prop_is_msh_detection(segment in segment_id(), field in field_number()) {
        let path = Path::new(&segment, field);
        let is_msh = segment == "MSH";
        prop_assert_eq!(path.is_msh(), is_msh);
    }
}

proptest! {
    /// Test MSH adjusted field calculation
    #[test]
    fn prop_msh_adjusted_field(field in 1usize..=100) {
        let path = Path::new("MSH", field);
        let adjusted = path.msh_adjusted_field();

        if field <= 2 {
            // MSH-1 -> 0, MSH-2 -> 1
            prop_assert_eq!(adjusted, field - 1);
        } else {
            // MSH-3+ -> field - 2
            prop_assert_eq!(adjusted, field - 2);
        }
    }
}

// =============================================================================
// Property Tests: Error handling
// =============================================================================

proptest! {
    /// Test that empty strings return InvalidFormat error
    #[test]
    fn prop_empty_path_error(s in "") {
        let result = parse_path(&s);
        prop_assert!(matches!(result, Err(PathError::InvalidFormat(_))));
    }
}

proptest! {
    /// Test that whitespace-only strings return error
    #[test]
    fn prop_whitespace_only_error(s in "[ \t]+") {
        let result = parse_path(&s);
        prop_assert!(result.is_err());
    }
}

proptest! {
    /// Test that segment IDs with wrong length are rejected
    #[test]
    fn prop_invalid_segment_length(s in "[A-Z]{1,2}|[A-Z]{4,10}", field in field_number()) {
        let path_str = format!("{}.{}", s, field);
        let result = parse_path(&path_str);
        prop_assert!(matches!(result, Err(PathError::InvalidSegmentId(_))));
    }
}

proptest! {
    /// Test that field number 0 is rejected
    #[test]
    fn prop_field_zero_rejected(segment in segment_id()) {
        let path_str = format!("{}.0", segment);
        let result = parse_path(&path_str);
        prop_assert!(matches!(result, Err(PathError::InvalidFieldNumber(_))));
    }
}

proptest! {
    /// Test that repetition 0 is rejected
    #[test]
    fn prop_repetition_zero_rejected(segment in segment_id(), field in field_number()) {
        let path_str = format!("{}.{}[0]", segment, field);
        let result = parse_path(&path_str);
        prop_assert!(matches!(result, Err(PathError::InvalidRepetitionIndex(_))));
    }
}

proptest! {
    /// Test that component 0 is rejected
    #[test]
    fn prop_component_zero_rejected(segment in segment_id(), field in field_number()) {
        let path_str = format!("{}.{}.0", segment, field);
        let result = parse_path(&path_str);
        prop_assert!(matches!(result, Err(PathError::InvalidComponentNumber(_))));
    }
}

proptest! {
    /// Test that non-alphanumeric segment IDs are rejected
    #[test]
    fn prop_non_alnum_segment_rejected(segment in "[A-Z]{2}[!@#$%^&*()]", field in field_number()) {
        let path_str = format!("{}.{}", segment, field);
        let result = parse_path(&path_str);
        prop_assert!(result.is_err());
    }
}

// =============================================================================
// Property Tests: Path equality and cloning
// =============================================================================

proptest! {
    /// Test that cloning produces equal paths
    #[test]
    fn prop_clone_equality(path in arbitrary_path()) {
        let cloned = path.clone();
        prop_assert_eq!(path, cloned);
    }
}

proptest! {
    /// Test that paths with same components are equal (via clone)
    #[test]
    fn prop_equal_components_equal_paths(path in arbitrary_path()) {
        // Clone produces an equal path
        let path2 = path.clone();
        prop_assert_eq!(path, path2);
    }
}

proptest! {
    /// Test that Display trait matches to_path_string
    #[test]
    fn prop_display_matches_to_path_string(path in arbitrary_path()) {
        let display_str = format!("{}", path);
        let method_str = path.to_path_string();
        prop_assert_eq!(display_str, method_str);
    }
}

// =============================================================================
// Property Tests: Whitespace handling
// =============================================================================

proptest! {
    /// Test that leading/trailing whitespace is trimmed
    #[test]
    fn prop_whitespace_trimmed(path_str in path_string(), ws in "[ \t]{0,10}") {
        let with_ws = format!("{}{}{}", ws, path_str, ws);
        let parsed = parse_path(&with_ws);
        prop_assert!(parsed.is_ok());

        let parsed_no_ws = parse_path(&path_str).expect("Should parse without whitespace");
        let parsed_with_ws = parsed.expect("Should parse with whitespace");
        prop_assert_eq!(parsed_no_ws, parsed_with_ws);
    }
}

// =============================================================================
// Property Tests: Case insensitivity for segment IDs
// =============================================================================

proptest! {
    /// Test that lowercase segment IDs are converted to uppercase
    #[test]
    fn prop_lowercase_segment_converted(segment in "[a-z]{3}", field in field_number()) {
        let path_str = format!("{}.{}", segment, field);
        let parsed = parse_path(&path_str).expect("Should parse lowercase segment");
        prop_assert_eq!(parsed.segment, segment.to_uppercase());
    }
}

proptest! {
    /// Test that mixed case segment IDs are normalized to uppercase
    #[test]
    fn prop_mixed_case_segment_normalized(segment in "[a-zA-Z]{3}", field in field_number()) {
        let path_str = format!("{}.{}", segment, field);
        let parsed = parse_path(&path_str).expect("Should parse mixed case segment");
        prop_assert_eq!(parsed.segment, segment.to_uppercase());
    }
}

// =============================================================================
// Property Tests: Builder pattern invariants
// =============================================================================

proptest! {
    /// Test that builder pattern produces consistent results
    #[test]
    fn prop_builder_consistency(path in arbitrary_path()) {
        // The arbitrary_path strategy already uses the builder pattern internally
        // Verify that to_path_string produces a parseable path
        let path_str = path.to_path_string();
        let parsed = parse_path(&path_str).expect("Generated path should be parseable");

        // Verify all components match after round-trip
        prop_assert_eq!(parsed.segment, path.segment);
        prop_assert_eq!(parsed.field, path.field);
        prop_assert_eq!(parsed.repetition, path.repetition);
        prop_assert_eq!(parsed.component, path.component);
        prop_assert_eq!(parsed.subcomponent, path.subcomponent);
    }
}

// =============================================================================
// Property Tests: Large value handling
// =============================================================================

proptest! {
    /// Test parsing paths with large field numbers
    #[test]
    fn prop_large_field_numbers(field in 1000usize..=9999) {
        let path_str = format!("PID.{}", field);
        let parsed = parse_path(&path_str);
        prop_assert!(parsed.is_ok());
        let path = parsed.unwrap();
        prop_assert_eq!(path.field, field);
    }
}

proptest! {
    /// Test parsing paths with large repetition indices
    #[test]
    fn prop_large_repetition_indices(rep in 1000usize..=9999) {
        let path_str = format!("PID.5[{}]", rep);
        let parsed = parse_path(&path_str);
        prop_assert!(parsed.is_ok());
        let path = parsed.unwrap();
        prop_assert_eq!(path.repetition, Some(rep));
    }
}

// =============================================================================
// Property Tests: Numeric segment IDs (valid per HL7 spec)
// =============================================================================

proptest! {
    /// Test that alphanumeric segment IDs with digits are accepted
    #[test]
    fn prop_alphanumeric_segment_with_digits(field in field_number()) {
        // Some HL7 segments use digits like ZP1, Z3A, etc.
        let segments = vec!["ZP1", "Z3A", "IN1", "IN2", "PR1", "DG1"];
        for seg in segments {
            let path_str = format!("{}.{}", seg, field);
            let parsed = parse_path(&path_str);
            prop_assert!(parsed.is_ok(), "Should parse segment: {}", seg);
        }
    }
}
