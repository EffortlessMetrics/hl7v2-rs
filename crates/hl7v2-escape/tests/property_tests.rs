//! Property-based tests for hl7v2-escape crate using proptest
//!
//! These tests verify escape/unescape properties hold for arbitrary inputs.

use hl7v2_escape::{escape_text, needs_escaping, needs_unescaping, unescape_text};
use hl7v2_model::Delims;
use proptest::prelude::*;

/// Generate arbitrary text that may contain delimiters
fn text_with_delimiters() -> impl Strategy<Value = String> {
    // Generate text with possible delimiter characters
    "[A-Za-z0-9 |^~\\\\&]{0,100}"
}

/// Generate text without delimiters
fn text_without_delimiters() -> impl Strategy<Value = String> {
    "[A-Za-z0-9 ]{0,100}"
}

/// Generate custom delimiters
#[allow(dead_code)]
fn custom_delims() -> impl Strategy<Value = Delims> {
    (
        any::<char>(),
        any::<char>(),
        any::<char>(),
        any::<char>(),
        any::<char>(),
    )
        .prop_map(|(field, comp, rep, esc, sub)| Delims {
            field,
            comp,
            rep,
            esc,
            sub,
        })
}

proptest! {
    /// Test that escape never panics for any input
    #[test]
    fn prop_escape_never_panics(text in text_with_delimiters()) {
        let delims = Delims::default();
        let _ = escape_text(&text, &delims);
    }
}

proptest! {
    /// Test that unescape never panics for any input
    #[test]
    fn prop_unescape_never_panics(text in text_with_delimiters()) {
        let delims = Delims::default();
        let _ = unescape_text(&text, &delims);
    }
}

proptest! {
    /// Test that roundtrip preserves original text
    #[test]
    fn prop_roundtrip_preserves_text(text in text_with_delimiters()) {
        let delims = Delims::default();
        let escaped = escape_text(&text, &delims);
        let unescaped = unescape_text(&escaped, &delims).unwrap();
        prop_assert_eq!(unescaped, text);
    }
}

proptest! {
    /// Test that escaped text doesn't contain raw delimiters
    #[test]
    fn prop_escaped_has_no_raw_delimiters(text in text_with_delimiters()) {
        let delims = Delims::default();
        let escaped = escape_text(&text, &delims);

        // Escaped text should not contain raw delimiters (except in escape sequences)
        // Check that any delimiter char is part of an escape sequence
        let chars: Vec<char> = escaped.chars().collect();
        for i in 0..chars.len() {
            let c = chars[i];
            if c == delims.field || c == delims.comp || c == delims.rep || c == delims.sub {
                // This delimiter should be preceded by escape char
                prop_assert!(i > 0 && chars[i-1] == delims.esc);
            }
        }
    }
}

proptest! {
    /// Test that needs_escaping is accurate
    #[test]
    fn prop_needs_escaping_accurate(text in text_with_delimiters()) {
        let delims = Delims::default();
        let should_escape = text.contains(delims.field)
            || text.contains(delims.comp)
            || text.contains(delims.rep)
            || text.contains(delims.esc)
            || text.contains(delims.sub);

        prop_assert_eq!(needs_escaping(&text, &delims), should_escape);
    }
}

proptest! {
    /// Test that needs_unescaping is accurate for escaped text
    #[test]
    fn prop_needs_unescaping_accurate(text in text_with_delimiters()) {
        let delims = Delims::default();
        let escaped = escape_text(&text, &delims);

        // Escaped text should need unescaping if original had delimiters
        let should_unescape = needs_escaping(&text, &delims);
        prop_assert_eq!(needs_unescaping(&escaped, &delims), should_unescape || text.contains(delims.esc));
    }
}

proptest! {
    /// Test that text without delimiters is unchanged by escape
    #[test]
    fn prop_no_change_without_delimiters(text in text_without_delimiters()) {
        let delims = Delims::default();
        let escaped = escape_text(&text, &delims);
        prop_assert_eq!(escaped, text);
    }
}

proptest! {
    /// Test that text without delimiters is unchanged by unescape
    #[test]
    fn prop_no_change_without_escape_sequences(text in text_without_delimiters()) {
        let delims = Delims::default();
        let unescaped = unescape_text(&text, &delims).unwrap();
        prop_assert_eq!(unescaped, text);
    }
}

proptest! {
    /// Test escape with custom delimiters
    #[test]
    fn prop_custom_delimiters_roundtrip(
        text in "[A-Za-z0-9 ]{0,50}",
        field in any::<char>(),
        comp in any::<char>(),
        rep in any::<char>(),
        esc in any::<char>(),
        sub in any::<char>()
    ) {
        // Skip if delimiters conflict with text characters
        prop_assume!(!text.contains(field));
        prop_assume!(!text.contains(comp));
        prop_assume!(!text.contains(rep));
        prop_assume!(!text.contains(esc));
        prop_assume!(!text.contains(sub));

        let delims = Delims { field, comp, rep, esc, sub };
        let escaped = escape_text(&text, &delims);
        let unescaped = unescape_text(&escaped, &delims).unwrap();
        prop_assert_eq!(unescaped, text);
    }
}

proptest! {
    /// Test that escaping is idempotent when applied to already-escaped text
    /// (escaping escaped text should escape the escape characters again)
    #[test]
    fn prop_double_escape(text in text_with_delimiters()) {
        let delims = Delims::default();
        let escaped1 = escape_text(&text, &delims);
        let escaped2 = escape_text(&escaped1, &delims);

        // Double escaping should produce different result if original had escape chars
        if text.contains(delims.esc) {
            prop_assert_ne!(escaped1, escaped2);
        }
    }
}

#[test]
fn test_empty_string_roundtrip() {
    let delims = Delims::default();
    let escaped = escape_text("", &delims);
    let unescaped = unescape_text(&escaped, &delims).unwrap();
    assert_eq!(unescaped, "");
}

proptest! {
    /// Test with various unicode content
    #[test]
    fn prop_unicode_roundtrip(text in "[\\p{L}\\p{N}|^~\\\\&]{0,50}") {
        let delims = Delims::default();
        let escaped = escape_text(&text, &delims);
        let unescaped = unescape_text(&escaped, &delims).unwrap();
        prop_assert_eq!(unescaped, text);
    }
}

proptest! {
    /// Test that consecutive delimiters are all escaped
    #[test]
    fn prop_consecutive_delimiters(count in 1usize..20) {
        let delims = Delims::default();
        let text = "|".repeat(count);
        let escaped = escape_text(&text, &delims);

        // Should have count occurrences of \F\
        let escape_count = escaped.matches("\\F\\").count();
        prop_assert_eq!(escape_count, count);

        // Roundtrip should preserve
        let unescaped = unescape_text(&escaped, &delims).unwrap();
        prop_assert_eq!(unescaped, text);
    }
}

proptest! {
    /// Test mixed delimiter content
    #[test]
    fn prop_mixed_delimiters(a in "[A-Za-z]+", b in "[A-Za-z]+", c in "[A-Za-z]+") {
        let delims = Delims::default();
        let text = format!("{}|{}^{}~{}", a, b, c, a);
        let escaped = escape_text(&text, &delims);
        let unescaped = unescape_text(&escaped, &delims).unwrap();
        prop_assert_eq!(unescaped, text);
    }
}
