//! Property-based tests for hl7v2-query using proptest

use hl7v2_model::{Atom, Comp, Delims, Field, Message, Presence, Rep, Segment};
use hl7v2_query::{get, get_presence};
use proptest::prelude::*;

/// Helper to create a segment from raw field data
fn create_segment(id: &str, fields: Vec<Field>) -> Segment {
    let id_bytes = id.as_bytes();
    let mut id_arr = [0u8; 3];
    id_arr.copy_from_slice(&id_bytes[..3]);
    Segment { id: id_arr, fields }
}

/// Helper to create a text field
fn text_field(value: &str) -> Field {
    Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![Atom::Text(value.to_string())],
            }],
        }],
    }
}

// =============================================================================
// Query Property Tests
// =============================================================================

proptest! {
    #[test]
    fn test_get_missing_segment(segment_id in "[A-Z]{3}", field_num in 1usize..10) {
        let message = Message {
            delims: Delims::default(),
            segments: vec![],
            charsets: vec![],
        };

        let path = format!("{}.{}", segment_id, field_num);
        let result = get(&message, &path);
        prop_assert!(result.is_none());
    }

    #[test]
    fn test_get_missing_field(field_num in 10usize..100) {
        let message = Message {
            delims: Delims::default(),
            segments: vec![create_segment("PID", vec![text_field("1")])],
            charsets: vec![],
        };

        let path = format!("PID.{}", field_num);
        let result = get(&message, &path);
        prop_assert!(result.is_none());
    }
}

// =============================================================================
// Presence Property Tests
// =============================================================================

proptest! {
    #[test]
    fn test_presence_missing_segment(segment_id in "[A-Z]{3}") {
        let message = Message {
            delims: Delims::default(),
            segments: vec![],
            charsets: vec![],
        };

        let path = format!("{}.1", segment_id);
        let presence = get_presence(&message, &path);
        prop_assert!(matches!(presence, Presence::Missing));
    }

    #[test]
    fn test_presence_value_preserves_content(content in "[A-Za-z0-9]+") {
        let message = Message {
            delims: Delims::default(),
            segments: vec![create_segment("PID", vec![text_field(&content)])],
            charsets: vec![],
        };

        let presence = get_presence(&message, "PID.1");
        match presence {
            Presence::Value(val) => prop_assert_eq!(val, content),
            _ => prop_assert!(false, "Expected Value presence"),
        }
    }
}

// =============================================================================
// Repetition Property Tests
// =============================================================================

proptest! {
    #[test]
    fn test_get_with_repetitions(rep_count in 1usize..10) {
        let reps: Vec<Rep> = (1..=rep_count)
            .map(|i| Rep {
                comps: vec![Comp {
                    subs: vec![Atom::Text(format!("value{}", i))],
                }],
            })
            .collect();

        let message = Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"PID",
                fields: vec![Field { reps }],
            }],
            charsets: vec![],
        };

        // First repetition should always work
        let result = get(&message, "PID.1");
        prop_assert_eq!(result, Some("value1"));

        // Last repetition should work
        let path = format!("PID.1[{}]", rep_count);
        let result = get(&message, &path);
        let expected = format!("value{}", rep_count);
        prop_assert_eq!(result, Some(expected.as_str()));

        // Beyond last should return None
        let path = format!("PID.1[{}]", rep_count + 1);
        let result = get(&message, &path);
        prop_assert!(result.is_none());
    }
}

// =============================================================================
// Component Property Tests
// =============================================================================

proptest! {
    #[test]
    fn test_get_component(comp_count in 1usize..10) {
        let comps: Vec<Comp> = (1..=comp_count)
            .map(|i| Comp {
                subs: vec![Atom::Text(format!("comp{}", i))],
            })
            .collect();

        let message = Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"PID",
                fields: vec![Field {
                    reps: vec![Rep { comps }],
                }],
            }],
            charsets: vec![],
        };

        // First component should work
        let result = get(&message, "PID.1.1");
        prop_assert_eq!(result, Some("comp1"));

        // Last component should work
        let path = format!("PID.1.{}", comp_count);
        let result = get(&message, &path);
        let expected = format!("comp{}", comp_count);
        prop_assert_eq!(result, Some(expected.as_str()));

        // Beyond last should return None
        let path = format!("PID.1.{}", comp_count + 1);
        let result = get(&message, &path);
        prop_assert!(result.is_none());
    }
}

// =============================================================================
// Segment ID Property Tests
// =============================================================================

proptest! {
    #[test]
    fn test_get_different_segment_ids(segment_id in "[A-Z]{3}") {
        let mut id_arr = [0u8; 3];
        id_arr.copy_from_slice(segment_id.as_bytes());

        let message = Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: id_arr,
                fields: vec![text_field("value")],
            }],
            charsets: vec![],
        };

        let path = format!("{}.1", segment_id);
        let result = get(&message, &path);
        if segment_id == "MSH" {
            prop_assert_eq!(result, Some("|"));
        } else {
            prop_assert_eq!(result, Some("value"));
        }
    }
}

// =============================================================================
// Path Format Property Tests
// =============================================================================

proptest! {
    #[test]
    fn test_path_with_field_only(field_num in 1usize..100) {
        let message = Message {
            delims: Delims::default(),
            segments: vec![create_segment("PID", vec![text_field("test")])],
            charsets: vec![],
        };

        let path = format!("PID.{}", field_num);
        let result = get(&message, &path);

        // Only field 1 should return a value
        if field_num == 1 {
            prop_assert_eq!(result, Some("test"));
        } else {
            prop_assert!(result.is_none());
        }
    }

    #[test]
    fn test_path_with_field_and_component(field_num in 1usize..10, comp_num in 1usize..10) {
        let message = Message {
            delims: Delims::default(),
            segments: vec![create_segment("PID", vec![text_field("test")])],
            charsets: vec![],
        };

        let path = format!("PID.{}.{}", field_num, comp_num);
        let result = get(&message, &path);

        // Only field 1, component 1 should return a value
        if field_num == 1 && comp_num == 1 {
            prop_assert_eq!(result, Some("test"));
        } else {
            prop_assert!(result.is_none());
        }
    }
}

// =============================================================================
// Consistency Property Tests
// =============================================================================

proptest! {
    #[test]
    fn test_get_consistent_results(content in "[A-Za-z0-9]+") {
        let message = Message {
            delims: Delims::default(),
            segments: vec![create_segment("PID", vec![text_field(&content)])],
            charsets: vec![],
        };

        // Multiple calls with same path should return same result
        let result1 = get(&message, "PID.1");
        let result2 = get(&message, "PID.1");
        prop_assert_eq!(result1, result2);
        prop_assert_eq!(result1, Some(content.as_str()));
    }

    #[test]
    fn test_presence_consistent_results(content in "[A-Za-z0-9]+") {
        let message = Message {
            delims: Delims::default(),
            segments: vec![create_segment("PID", vec![text_field(&content)])],
            charsets: vec![],
        };

        // Multiple calls with same path should return same result
        let presence1 = get_presence(&message, "PID.1");
        let presence2 = get_presence(&message, "PID.1");

        match (&presence1, &presence2) {
            (Presence::Value(v1), Presence::Value(v2)) => prop_assert_eq!(v1, v2),
            (Presence::Empty, Presence::Empty) => {},
            (Presence::Null, Presence::Null) => {},
            (Presence::Missing, Presence::Missing) => {},
            _ => prop_assert!(false, "Inconsistent results"),
        }
    }
}

// =============================================================================
// MSH Segment Property Tests
// =============================================================================

proptest! {
    #[test]
    fn test_msh_field_1_returns_field_separator(field_sep in "[|]") {
        let message = Message {
            delims: Delims {
                field: field_sep.chars().next().unwrap(),
                comp: '^',
                rep: '~',
                esc: '\\',
                sub: '&',
            },
            segments: vec![create_segment("MSH", vec![])],
            charsets: vec![],
        };

        let presence = get_presence(&message, "MSH.1");
        match presence {
            Presence::Value(val) => prop_assert_eq!(val, field_sep),
            _ => prop_assert!(false, "Expected Value for MSH.1"),
        }
    }
}

// =============================================================================
// Empty/Null Value Property Tests
// =============================================================================

proptest! {
    #[test]
    fn test_empty_string_is_empty_presence(_content in "") {
        let message = Message {
            delims: Delims::default(),
            segments: vec![create_segment("PID", vec![text_field("")])],
            charsets: vec![],
        };

        let presence = get_presence(&message, "PID.1");
        prop_assert!(matches!(presence, Presence::Empty));
    }

    #[test]
    fn test_non_empty_string_is_value_presence(content in "[A-Za-z0-9]+") {
        let message = Message {
            delims: Delims::default(),
            segments: vec![create_segment("PID", vec![text_field(&content)])],
            charsets: vec![],
        };

        let presence = get_presence(&message, "PID.1");
        match presence {
            Presence::Value(val) => prop_assert_eq!(val, content),
            _ => prop_assert!(false, "Expected Value presence"),
        }
    }
}
