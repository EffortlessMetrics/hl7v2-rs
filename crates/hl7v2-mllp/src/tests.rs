//! Comprehensive unit tests for hl7v2-mllp crate
//!
//! This module contains unit tests for:
//! - MLLP framing (wrap/unwrap)
//! - MLLP constants
//! - Frame iterator
//! - Edge cases

use super::*;

// ============================================================================
// Constants Tests
// ============================================================================

#[test]
fn test_mllp_constants() {
    assert_eq!(MLLP_START, 0x0B);
    assert_eq!(MLLP_END_1, 0x1C);
    assert_eq!(MLLP_END_2, 0x0D);
}

// ============================================================================
// Wrap Tests
// ============================================================================

#[test]
fn test_wrap_simple() {
    let hl7 = b"MSH|^~\\&|TEST\r";
    let framed = wrap_mllp(hl7);
    
    assert_eq!(framed[0], MLLP_START);
    assert_eq!(framed[framed.len() - 2], MLLP_END_1);
    assert_eq!(framed[framed.len() - 1], MLLP_END_2);
}

#[test]
fn test_wrap_empty() {
    let framed = wrap_mllp(b"");
    
    assert_eq!(framed.len(), 3); // Start + End1 + End2
    assert_eq!(framed[0], MLLP_START);
    assert_eq!(framed[1], MLLP_END_1);
    assert_eq!(framed[2], MLLP_END_2);
}

#[test]
fn test_wrap_single_byte() {
    let framed = wrap_mllp(b"X");
    
    assert_eq!(framed.len(), 4);
    assert_eq!(framed[0], MLLP_START);
    assert_eq!(framed[1], b'X');
    assert_eq!(framed[2], MLLP_END_1);
    assert_eq!(framed[3], MLLP_END_2);
}

#[test]
fn test_wrap_preserves_content() {
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\r";
    let framed = wrap_mllp(hl7);
    
    // Content should be preserved
    assert_eq!(&framed[1..framed.len() - 2], hl7);
}

#[test]
fn test_wrap_with_special_bytes() {
    // Test with bytes that might be confused with MLLP markers
    let content = vec![0x0B, 0x1C, 0x0D, 0x00, 0xFF];
    let framed = wrap_mllp(&content);
    
    assert_eq!(framed[0], MLLP_START);
    assert_eq!(framed[framed.len() - 2], MLLP_END_1);
    assert_eq!(framed[framed.len() - 1], MLLP_END_2);
    
    // Content should be preserved including the special bytes
    assert_eq!(&framed[1..framed.len() - 2], content.as_slice());
}

#[test]
fn test_wrap_long_message() {
    let hl7: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();
    let framed = wrap_mllp(&hl7);
    
    assert_eq!(framed.len(), hl7.len() + 3);
    assert_eq!(framed[0], MLLP_START);
    assert_eq!(framed[framed.len() - 2], MLLP_END_1);
    assert_eq!(framed[framed.len() - 1], MLLP_END_2);
}

// ============================================================================
// Unwrap Tests
// ============================================================================

#[test]
fn test_unwrap_simple() {
    let hl7 = b"MSH|^~\\&|TEST\r";
    let framed = wrap_mllp(hl7);
    let unwrapped = unwrap_mllp(&framed).unwrap();
    
    assert_eq!(unwrapped, hl7);
}

#[test]
fn test_unwrap_empty_content() {
    let framed = wrap_mllp(b"");
    let unwrapped = unwrap_mllp(&framed).unwrap();
    
    assert_eq!(unwrapped, b"");
}

#[test]
fn test_unwrap_invalid_no_start() {
    let result = unwrap_mllp(b"MSH|TEST");
    assert!(result.is_err());
}

#[test]
fn test_unwrap_invalid_empty() {
    let result = unwrap_mllp(b"");
    assert!(result.is_err());
}

#[test]
fn test_unwrap_invalid_no_end() {
    // Start byte but no end sequence
    let data = vec![MLLP_START, b'M', b'S', b'H'];
    let result = unwrap_mllp(&data);
    assert!(result.is_err());
}

#[test]
fn test_unwrap_partial_end() {
    // Start byte and content but only partial end sequence
    let mut data = vec![MLLP_START];
    data.extend_from_slice(b"MSH|TEST");
    data.push(MLLP_END_1);
    // Missing MLLP_END_2
    
    let result = unwrap_mllp(&data);
    assert!(result.is_err());
}

#[test]
fn test_unwrap_owned() {
    let hl7 = b"MSH|^~\\&|TEST\r";
    let framed = wrap_mllp(hl7);
    let unwrapped = unwrap_mllp_owned(&framed).unwrap();
    
    assert_eq!(unwrapped.as_slice(), hl7);
}

// ============================================================================
// Is MLLP Framed Tests
// ============================================================================

#[test]
fn test_is_mllp_framed_valid() {
    let framed = wrap_mllp(b"MSH|TEST");
    assert!(is_mllp_framed(&framed));
}

#[test]
fn test_is_mllp_framed_invalid() {
    assert!(!is_mllp_framed(b"MSH|TEST"));
}

#[test]
fn test_is_mllp_framed_empty() {
    assert!(!is_mllp_framed(b""));
}

#[test]
fn test_is_mllp_framed_only_start() {
    let data = vec![MLLP_START];
    assert!(is_mllp_framed(&data));
}

// ============================================================================
// Find Complete Message Tests
// ============================================================================

#[test]
fn test_find_complete_message() {
    let hl7 = b"MSH|^~\\&|TEST\r";
    let framed = wrap_mllp(hl7);
    
    let len = find_complete_mllp_message(&framed).unwrap();
    assert_eq!(len, framed.len());
}

#[test]
fn test_find_complete_message_incomplete() {
    let framed = wrap_mllp(b"MSH|TEST");
    
    // Incomplete (missing end bytes)
    let incomplete = &framed[..framed.len() - 1];
    assert!(find_complete_mllp_message(incomplete).is_none());
}

#[test]
fn test_find_complete_message_no_start() {
    assert!(find_complete_mllp_message(b"MSH|TEST").is_none());
}

#[test]
fn test_find_complete_message_empty() {
    assert!(find_complete_mllp_message(b"").is_none());
}

#[test]
fn test_find_complete_message_with_embedded_end() {
    // Content that contains the end sequence in the middle
    // Note: MLLP will find the first end sequence, so the message will be truncated
    let mut content = b"MSH|TEST_".to_vec();
    content.push(MLLP_END_1);
    content.push(MLLP_END_2);
    content.extend_from_slice(b"_MORE");
    
    let framed = wrap_mllp(&content);
    let len = find_complete_mllp_message(&framed).unwrap();
    
    // Should find the first end sequence (at the embedded position)
    // Frame: SB + "MSH|TEST_" + EB1 + EB2 + EB1 + EB2
    // The first EB1+EB2 is the embedded one in content
    assert_eq!(len, 1 + 9 + 2); // SB + "MSH|TEST_" + EB1+EB2
}

// ============================================================================
// Frame Iterator Tests
// ============================================================================

#[test]
fn test_frame_iterator_new() {
    let iter = MllpFrameIterator::new();
    assert_eq!(iter.buffer_len(), 0);
}

#[test]
fn test_frame_iterator_default() {
    let iter = MllpFrameIterator::default();
    assert_eq!(iter.buffer_len(), 0);
}

#[test]
fn test_frame_iterator_single_message() {
    let mut iter = MllpFrameIterator::new();
    
    let hl7 = b"MSH|^~\\&|TEST\r";
    let framed = wrap_mllp(hl7);
    
    iter.extend(&framed);
    
    let msg = iter.next_message().unwrap().unwrap();
    assert_eq!(&msg, hl7);
    
    // No more messages
    assert!(iter.next_message().is_none());
}

#[test]
fn test_frame_iterator_multiple_messages() {
    let mut iter = MllpFrameIterator::new();
    
    let hl7_1 = b"MSH|^~\\&|TEST1\r";
    let hl7_2 = b"MSH|^~\\&|TEST2\r";
    let framed_1 = wrap_mllp(hl7_1);
    let framed_2 = wrap_mllp(hl7_2);
    
    // Add both messages
    iter.extend(&framed_1);
    iter.extend(&framed_2);
    
    // Extract first message
    let msg_1 = iter.next_message().unwrap().unwrap();
    assert_eq!(&msg_1, hl7_1);
    
    // Extract second message
    let msg_2 = iter.next_message().unwrap().unwrap();
    assert_eq!(&msg_2, hl7_2);
    
    // No more messages
    assert!(iter.next_message().is_none());
}

#[test]
fn test_frame_iterator_partial_message() {
    let mut iter = MllpFrameIterator::new();
    
    let hl7 = b"MSH|^~\\&|TEST\r";
    let framed = wrap_mllp(hl7);
    
    // Add partial message
    iter.extend(&framed[..5]);
    assert!(iter.next_message().is_none());
    
    // Add the rest
    iter.extend(&framed[5..]);
    
    // Now we can extract
    let msg = iter.next_message().unwrap().unwrap();
    assert_eq!(&msg, hl7);
}

#[test]
fn test_frame_iterator_byte_by_byte() {
    let mut iter = MllpFrameIterator::new();
    
    let hl7 = b"MSH|^~\\&|TEST\r";
    let framed = wrap_mllp(hl7);
    
    // Add byte by byte
    for byte in framed.iter() {
        iter.extend(&[*byte]);
        if iter.buffer_len() < framed.len() {
            assert!(iter.next_message().is_none());
        }
    }
    
    // Now we can extract
    let msg = iter.next_message().unwrap().unwrap();
    assert_eq!(&msg, hl7);
}

#[test]
fn test_frame_iterator_clear() {
    let mut iter = MllpFrameIterator::new();
    
    iter.extend(b"some data");
    assert!(iter.buffer_len() > 0);
    
    iter.clear();
    assert_eq!(iter.buffer_len(), 0);
}

#[test]
fn test_frame_iterator_buffer_len() {
    let mut iter = MllpFrameIterator::new();
    
    assert_eq!(iter.buffer_len(), 0);
    
    iter.extend(b"test");
    assert_eq!(iter.buffer_len(), 4);
    
    iter.extend(b"more");
    assert_eq!(iter.buffer_len(), 8);
}

#[test]
fn test_frame_iterator_next_frame() {
    let mut iter = MllpFrameIterator::new();
    
    let hl7 = b"MSH|^~\\&|TEST\r";
    let framed = wrap_mllp(hl7);
    
    iter.extend(&framed);
    
    let frame = iter.next_frame().unwrap();
    assert_eq!(frame, framed);
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_wrap_unwrap_roundtrip() {
    let all_bytes: Vec<u8> = (0..=255u8).collect();
    let test_cases: Vec<&[u8]> = vec![
        b"",
        b"X",
        b"MSH|^~\\&|TEST\r",
        b"MSH|^~\\&|App|Fac|Recv|RecvFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||12345\r",
    ];
    
    for original in test_cases {
        let framed = wrap_mllp(original);
        let unwrapped = unwrap_mllp(&framed).unwrap();
        assert_eq!(unwrapped, original);
    }
    
    // Test all bytes separately
    let framed = wrap_mllp(&all_bytes);
    let unwrapped = unwrap_mllp(&framed).unwrap();
    assert_eq!(unwrapped, all_bytes.as_slice());
}

#[test]
fn test_content_with_mllp_markers() {
    // Content that contains MLLP marker bytes (but not end sequence)
    // Note: If content contains the end sequence (0x1C 0x0D), unwrap will find it
    // and truncate the message. This is expected MLLP behavior.
    let mut content = b"MSH|TEST_".to_vec();
    content.push(MLLP_START); // This is fine - only marks start of frame
    content.extend_from_slice(b"_END");
    
    let framed = wrap_mllp(&content);
    let unwrapped = unwrap_mllp(&framed).unwrap();
    
    assert_eq!(unwrapped, content.as_slice());
}

#[test]
fn test_content_with_embedded_end_sequence() {
    // When content contains the end sequence, unwrap finds the first one
    // This is expected behavior - content should not contain end sequence
    let mut content = b"MSH|TEST_".to_vec();
    content.push(MLLP_END_1);
    content.push(MLLP_END_2);
    content.extend_from_slice(b"_MORE");
    
    let framed = wrap_mllp(&content);
    let unwrapped = unwrap_mllp(&framed).unwrap();
    
    // Unwrap finds the first end sequence, so content is truncated
    // Expected content is "MSH|TEST_"
    assert_eq!(unwrapped, b"MSH|TEST_");
}

#[test]
fn test_content_without_end_sequence() {
    // Content that contains start byte but not end sequence - should roundtrip correctly
    let mut content = b"MSH|TEST_".to_vec();
    content.push(MLLP_START);
    content.extend_from_slice(b"_MIDDLE_");
    content.push(MLLP_START);
    content.extend_from_slice(b"_END");
    
    let framed = wrap_mllp(&content);
    let unwrapped = unwrap_mllp(&framed).unwrap();
    
    // This should roundtrip correctly since no end sequence in content
    assert_eq!(unwrapped, content.as_slice());
}

#[test]
fn test_frame_iterator_concatenated_messages() {
    let mut iter = MllpFrameIterator::new();
    
    let hl7_1 = b"MSH|^~\\&|TEST1\r";
    let hl7_2 = b"MSH|^~\\&|TEST2\r";
    let framed_1 = wrap_mllp(hl7_1);
    let framed_2 = wrap_mllp(hl7_2);
    
    // Concatenate both frames
    let mut combined = framed_1.clone();
    combined.extend_from_slice(&framed_2);
    
    iter.extend(&combined);
    
    // Extract first message
    let msg_1 = iter.next_message().unwrap().unwrap();
    assert_eq!(&msg_1, hl7_1);
    
    // Extract second message
    let msg_2 = iter.next_message().unwrap().unwrap();
    assert_eq!(&msg_2, hl7_2);
    
    // No more messages
    assert!(iter.next_message().is_none());
}

#[test]
fn test_large_message() {
    // Large message
    let content: Vec<u8> = (0..100000).map(|i| (i % 256) as u8).collect();
    let framed = wrap_mllp(&content);
    let unwrapped = unwrap_mllp(&framed).unwrap();
    
    assert_eq!(unwrapped, content.as_slice());
}

#[test]
fn test_frame_iterator_large_message() {
    let mut iter = MllpFrameIterator::new();
    
    // Large message
    let content: Vec<u8> = (0..50000).map(|i| (i % 256) as u8).collect();
    let framed = wrap_mllp(&content);
    
    iter.extend(&framed);
    
    let msg = iter.next_message().unwrap().unwrap();
    assert_eq!(msg.as_slice(), content.as_slice());
}
