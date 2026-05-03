//! Property-based tests for hl7v2-mllp crate using proptest
//!
//! These tests verify MLLP framing properties hold for arbitrary inputs.

use hl7v2_mllp::{
    MLLP_END_1, MLLP_END_2, MLLP_START, MllpFrameIterator, find_complete_mllp_message,
    is_mllp_framed, unwrap_mllp, unwrap_mllp_owned, wrap_mllp,
};
use proptest::prelude::*;

/// Generate arbitrary byte content that doesn't contain the MLLP end sequence
/// Uses sampling from valid range instead of filtering for efficiency
fn arbitrary_bytes_no_end_sequence() -> impl Strategy<Value = Vec<u8>> {
    // Generate bytes 0-27 and 29-255 (skip 28 which is MLLP_END_1 = 0x1C)
    proptest::collection::vec(
        proptest::arbitrary::any::<u8>().prop_map(|b| {
            // Map 0-254 to valid bytes (skip MLLP_END_1)
            if b == MLLP_END_1 { b + 1 } else { b }
        }),
        0..1000,
    )
}

/// Generate HL7-like message content (printable ASCII, no end sequence)
/// Uses direct generation instead of filtering for efficiency
fn hl7_like_content() -> impl Strategy<Value = Vec<u8>> {
    // Generate printable ASCII: 0x20-0x7E = 95 characters
    // Map values 0-94 to 0x20-0x7E
    proptest::collection::vec((0u8..95u8).prop_map(|b| b + 0x20), 0..500)
}

proptest! {
    /// Test that wrap never panics for any input
    #[test]
    fn prop_wrap_never_panics(content in proptest::collection::vec(any::<u8>(), 0..1000)) {
        let _ = wrap_mllp(&content);
    }
}

proptest! {
    /// Test that unwrap of wrapped content always succeeds
    #[test]
    fn prop_unwrap_of_wrap_succeeds(content in arbitrary_bytes_no_end_sequence()) {
        let framed = wrap_mllp(&content);
        let unwrapped = unwrap_mllp(&framed);
        prop_assert!(unwrapped.is_ok());
    }
}

proptest! {
    /// Test that roundtrip preserves content (when content doesn't contain end sequence)
    #[test]
    fn prop_roundtrip_preserves_content(content in arbitrary_bytes_no_end_sequence()) {
        let framed = wrap_mllp(&content);
        let unwrapped = unwrap_mllp(&framed).unwrap();
        prop_assert_eq!(unwrapped, content.as_slice());
    }
}

proptest! {
    /// Test that wrapped content starts with start byte
    #[test]
    fn prop_wrapped_starts_with_start_byte(content in arbitrary_bytes_no_end_sequence()) {
        let framed = wrap_mllp(&content);
        prop_assert_eq!(framed[0], MLLP_START);
    }
}

proptest! {
    /// Test that wrapped content ends with end sequence
    #[test]
    fn prop_wrapped_ends_with_end_sequence(content in arbitrary_bytes_no_end_sequence()) {
        let framed = wrap_mllp(&content);
        let len = framed.len();
        prop_assert!(len >= 3);
        prop_assert_eq!(framed[len - 2], MLLP_END_1);
        prop_assert_eq!(framed[len - 1], MLLP_END_2);
    }
}

proptest! {
    /// Test that is_mllp_framed returns true for wrapped content
    #[test]
    fn prop_is_mllp_framed_true_for_wrapped(content in arbitrary_bytes_no_end_sequence()) {
        let framed = wrap_mllp(&content);
        prop_assert!(is_mllp_framed(&framed));
    }
}

proptest! {
    /// Test that find_complete finds the full message
    #[test]
    fn prop_find_complete_finds_full_message(content in arbitrary_bytes_no_end_sequence()) {
        let framed = wrap_mllp(&content);
        let len = find_complete_mllp_message(&framed);
        prop_assert_eq!(len, Some(framed.len()));
    }
}

proptest! {
    /// Test that unwrap_owned produces same result as unwrap
    #[test]
    fn prop_unwrap_owned_same_as_unwrap(content in arbitrary_bytes_no_end_sequence()) {
        let framed = wrap_mllp(&content);
        let borrowed = unwrap_mllp(&framed).unwrap();
        let owned = unwrap_mllp_owned(&framed).unwrap();
        prop_assert_eq!(borrowed, owned.as_slice());
    }
}

proptest! {
    /// Test frame iterator with single message
    #[test]
    fn prop_frame_iterator_single_message(content in hl7_like_content()) {
        let mut iter = MllpFrameIterator::new();
        let framed = wrap_mllp(&content);

        iter.extend(&framed);

        let extracted = iter.next_message();
        prop_assert!(extracted.is_some());
        let extracted = extracted.unwrap();
        prop_assert!(extracted.is_ok());
        let extracted = extracted.unwrap();
        prop_assert_eq!(extracted.as_slice(), content.as_slice());
    }
}

proptest! {
    /// Test frame iterator with two messages
    #[test]
    fn prop_frame_iterator_two_messages(
        content1 in hl7_like_content(),
        content2 in hl7_like_content()
    ) {
        let mut iter = MllpFrameIterator::new();

        let framed1 = wrap_mllp(&content1);
        let framed2 = wrap_mllp(&content2);

        iter.extend(&framed1);
        iter.extend(&framed2);

        let msg1 = iter.next_message().unwrap().unwrap();
        prop_assert_eq!(msg1.as_slice(), content1.as_slice());

        let msg2 = iter.next_message().unwrap().unwrap();
        prop_assert_eq!(msg2.as_slice(), content2.as_slice());

        prop_assert!(iter.next_message().is_none());
    }
}

proptest! {
    /// Test frame iterator with fragmented input
    #[test]
    fn prop_frame_iterator_fragmented(content in hl7_like_content()) {
        let mut iter = MllpFrameIterator::new();
        let framed = wrap_mllp(&content);

        // Split the framed message at various points
        if framed.len() > 2 {
            let split = framed.len() / 2;
            iter.extend(&framed[..split]);
            prop_assert!(iter.next_message().is_none());

            iter.extend(&framed[split..]);
            let msg = iter.next_message().unwrap().unwrap();
            prop_assert_eq!(msg.as_slice(), content.as_slice());
        }
    }
}

#[test]
fn test_empty_roundtrip() {
    let framed = wrap_mllp(b"");
    let unwrapped = unwrap_mllp(&framed).unwrap();
    assert_eq!(unwrapped, b"");
}

proptest! {
    /// Test with various message sizes
    #[test]
    fn prop_various_sizes(size in 0usize..10000) {
        // Generate content without end sequence
        let content: Vec<u8> = (0..size).map(|i| ((i % 255) + 1) as u8).collect();
        let framed = wrap_mllp(&content);
        let unwrapped = unwrap_mllp(&framed).unwrap();
        prop_assert_eq!(unwrapped, content.as_slice());
    }
}
