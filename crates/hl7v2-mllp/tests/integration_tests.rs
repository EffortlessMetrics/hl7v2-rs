//! Integration tests for hl7v2-mllp crate
//!
//! These tests verify MLLP framing works correctly with real-world
//! HL7 message scenarios.

use hl7v2_mllp::{
    wrap_mllp, unwrap_mllp, unwrap_mllp_owned, is_mllp_framed,
    find_complete_mllp_message, MllpFrameIterator, MLLP_START, MLLP_END_1, MLLP_END_2,
};

// ============================================================================
// Real-World HL7 Message Tests
// ============================================================================

#[test]
fn test_adt_a01_message() {
    let adt_a01 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John^Robert||19700101|M\rPV1|1|I|ICU^101\r";
    
    let framed = wrap_mllp(adt_a01);
    assert!(is_mllp_framed(&framed));
    
    let unwrapped = unwrap_mllp(&framed).unwrap();
    assert_eq!(unwrapped, adt_a01);
}

#[test]
fn test_oru_r01_message() {
    let oru_r01 = b"MSH|^~\\&|LAB|LABHOST|HIS|HISHOST|20250128154500||ORU^R01|MSG00003|P|2.5.1\rPID|1||12345^^^HOSP^MR||SMITH^JOHN\rOBX|1|NM|GLUCOSE^Blood Glucose||120|mg/dL|70-110|H|||F\r";
    
    let framed = wrap_mllp(oru_r01);
    let unwrapped = unwrap_mllp(&framed).unwrap();
    assert_eq!(unwrapped, oru_r01);
}

#[test]
fn test_ack_message() {
    let ack = b"MSH|^~\\&|ReceivingApp|ReceivingFac|SendingApp|SendingFac|20250128152315||ACK|ABC123|P|2.5.1\rMSA|AA|ABC123\r";
    
    let framed = wrap_mllp(ack);
    let unwrapped = unwrap_mllp(&framed).unwrap();
    assert_eq!(unwrapped, ack);
}

// ============================================================================
// Multiple Messages in Stream Tests
// ============================================================================

#[test]
fn test_multiple_messages_in_sequence() {
    let mut iter = MllpFrameIterator::new();
    
    let msg1 = b"MSH|^~\\&|APP1|FAC1|RECV1|RECVFAC1|20250128120000||ADT^A01|MSG001|P|2.5.1\r";
    let msg2 = b"MSH|^~\\&|APP2|FAC2|RECV2|RECVFAC2|20250128120100||ADT^A01|MSG002|P|2.5.1\r";
    let msg3 = b"MSH|^~\\&|APP3|FAC3|RECV3|RECVFAC3|20250128120200||ADT^A01|MSG003|P|2.5.1\r";
    
    // Frame all messages
    let framed1 = wrap_mllp(msg1);
    let framed2 = wrap_mllp(msg2);
    let framed3 = wrap_mllp(msg3);
    
    // Add all to iterator
    iter.extend(&framed1);
    iter.extend(&framed2);
    iter.extend(&framed3);
    
    // Extract all
    assert_eq!(&iter.next_message().unwrap().unwrap(), msg1);
    assert_eq!(&iter.next_message().unwrap().unwrap(), msg2);
    assert_eq!(&iter.next_message().unwrap().unwrap(), msg3);
    assert!(iter.next_message().is_none());
}

#[test]
fn test_concatenated_frames() {
    let msg1 = b"MSH|^~\\&|APP|FAC|RECV|RECVFAC|20250128120000||ADT^A01|MSG001|P|2.5.1\r";
    let msg2 = b"MSH|^~\\&|APP|FAC|RECV|RECVFAC|20250128120100||ADT^A01|MSG002|P|2.5.1\r";
    
    let framed1 = wrap_mllp(msg1);
    let framed2 = wrap_mllp(msg2);
    
    // Concatenate frames
    let mut combined = Vec::new();
    combined.extend_from_slice(&framed1);
    combined.extend_from_slice(&framed2);
    
    // Find first message
    let len1 = find_complete_mllp_message(&combined).unwrap();
    assert_eq!(len1, framed1.len());
    
    // Extract first and find second
    let remaining = &combined[len1..];
    let len2 = find_complete_mllp_message(remaining).unwrap();
    assert_eq!(len2, framed2.len());
}

// ============================================================================
// Chunked/Fragmented Data Tests
// ============================================================================

#[test]
fn test_fragmented_message_single_byte() {
    let mut iter = MllpFrameIterator::new();
    
    let msg = b"MSH|^~\\&|TEST\r";
    let framed = wrap_mllp(msg);
    
    // Add one byte at a time
    for byte in framed.iter() {
        iter.extend(&[*byte]);
    }
    
    // Should be able to extract the message
    let extracted = iter.next_message().unwrap().unwrap();
    assert_eq!(&extracted, msg);
}

#[test]
fn test_fragmented_message_small_chunks() {
    let mut iter = MllpFrameIterator::new();
    
    let msg = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r";
    let framed = wrap_mllp(msg);
    
    // Add in 5-byte chunks
    for chunk in framed.chunks(5) {
        iter.extend(chunk);
    }
    
    let extracted = iter.next_message().unwrap().unwrap();
    assert_eq!(&extracted, msg);
}

#[test]
fn test_fragmented_across_messages() {
    let mut iter = MllpFrameIterator::new();
    
    let msg1 = b"MSH|^~\\&|TEST1\r";
    let msg2 = b"MSH|^~\\&|TEST2\r";
    
    let framed1 = wrap_mllp(msg1);
    let framed2 = wrap_mllp(msg2);
    
    // Add first message and part of second
    let split_point = framed2.len() / 2;
    iter.extend(&framed1);
    iter.extend(&framed2[..split_point]);
    
    // Should get first message
    assert_eq!(&iter.next_message().unwrap().unwrap(), msg1);
    assert!(iter.next_message().is_none());
    
    // Add rest of second
    iter.extend(&framed2[split_point..]);
    assert_eq!(&iter.next_message().unwrap().unwrap(), msg2);
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_empty_message() {
    let framed = wrap_mllp(b"");
    assert_eq!(framed.len(), 3); // Start + End1 + End2
    
    let unwrapped = unwrap_mllp(&framed).unwrap();
    assert_eq!(unwrapped, b"");
}

#[test]
fn test_message_with_only_start_byte() {
    // Content that is just the start byte
    let content = vec![MLLP_START];
    let framed = wrap_mllp(&content);
    
    let unwrapped = unwrap_mllp(&framed).unwrap();
    assert_eq!(unwrapped, content.as_slice());
}

#[test]
fn test_message_containing_end_bytes() {
    // Message that contains the end sequence in content
    let mut content = b"MSH|^~\\&|TEST_".to_vec();
    content.push(MLLP_END_1);
    content.push(MLLP_END_2);
    content.extend_from_slice(b"_END\r");
    
    let framed = wrap_mllp(&content);
    let unwrapped = unwrap_mllp(&framed).unwrap();
    
    // Note: unwrap will find the first end sequence
    // This is expected behavior - content shouldn't contain end bytes
    assert_ne!(unwrapped.len(), content.len());
}

#[test]
fn test_large_message() {
    // Create a large message
    let segment = b"PID|1||123456^^^HOSP^MR||Doe^John^Robert||19700101|M|||123 Main St^^Anytown^CA^12345||5551234567\r";
    let mut msg = b"MSH|^~\\&|APP|FAC|RECV|RECVFAC|20250128120000||ADT^A01|MSG|P|2.5.1\r".to_vec();
    
    // Add many segments
    for _ in 0..1000 {
        msg.extend_from_slice(segment);
    }
    
    let framed = wrap_mllp(&msg);
    let unwrapped = unwrap_mllp(&framed).unwrap();
    assert_eq!(unwrapped, msg.as_slice());
}

// ============================================================================
// Frame Iterator Edge Cases
// ============================================================================

#[test]
fn test_iterator_clear() {
    let mut iter = MllpFrameIterator::new();
    
    // Add some data
    iter.extend(b"random data");
    assert!(iter.buffer_len() > 0);
    
    // Clear
    iter.clear();
    assert_eq!(iter.buffer_len(), 0);
    
    // Should work normally after clear
    let msg = b"MSH|^~\\&|TEST\r";
    let framed = wrap_mllp(msg);
    iter.extend(&framed);
    assert_eq!(&iter.next_message().unwrap().unwrap(), msg);
}

#[test]
fn test_iterator_buffer_management() {
    let mut iter = MllpFrameIterator::new();
    
    // Process many messages sequentially
    for i in 0..100 {
        let msg = format!("MSH|^~\\&|TEST{}\r", i);
        let framed = wrap_mllp(msg.as_bytes());
        iter.extend(&framed);
        
        let extracted = iter.next_message().unwrap().unwrap();
        assert_eq!(&extracted, msg.as_bytes());
    }
    
    // Buffer should be empty after processing all messages
    assert_eq!(iter.buffer_len(), 0);
}

// ============================================================================
// Error Handling Tests
// ============================================================================

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
fn test_unwrap_invalid_only_start() {
    let data = vec![MLLP_START];
    let result = unwrap_mllp(&data);
    assert!(result.is_err());
}

#[test]
fn test_unwrap_invalid_missing_end() {
    let mut data = vec![MLLP_START];
    data.extend_from_slice(b"MSH|TEST");
    let result = unwrap_mllp(&data);
    assert!(result.is_err());
}

// ============================================================================
// Roundtrip Tests
// ============================================================================

#[test]
fn test_roundtrip_various_messages() {
    let messages = vec![
        b"MSH|^~\\&|TEST\r".as_slice(),
        b"MSH|^~\\&|APP|FAC|RECV|RECVFAC|20250128120000||ADT^A01|MSG|P|2.5.1\r",
        b"MSH|^~\\&|A|B|C|D|E||F|G|H|I\rPID|1||123\rPV1|1|I\r",
        b"", // Empty message
    ];
    
    for original in messages {
        let framed = wrap_mllp(original);
        let unwrapped = unwrap_mllp(&framed).unwrap();
        assert_eq!(unwrapped, original);
    }
}

#[test]
fn test_owned_vs_borrowed() {
    let msg = b"MSH|^~\\&|TEST\r";
    let framed = wrap_mllp(msg);
    
    let borrowed = unwrap_mllp(&framed).unwrap();
    let owned = unwrap_mllp_owned(&framed).unwrap();
    
    assert_eq!(borrowed, msg);
    assert_eq!(owned.as_slice(), msg);
}

// ============================================================================
// Find Complete Message Tests
// ============================================================================

#[test]
fn test_find_complete_empty() {
    assert!(find_complete_mllp_message(b"").is_none());
}

#[test]
fn test_find_complete_no_start() {
    assert!(find_complete_mllp_message(b"MSH|TEST").is_none());
}

#[test]
fn test_find_complete_partial() {
    let msg = b"MSH|TEST\r";
    let framed = wrap_mllp(msg);
    
    // Partial message
    assert!(find_complete_mllp_message(&framed[..framed.len() - 1]).is_none());
}

#[test]
fn test_find_complete_full() {
    let msg = b"MSH|TEST\r";
    let framed = wrap_mllp(msg);
    
    let len = find_complete_mllp_message(&framed).unwrap();
    assert_eq!(len, framed.len());
}

// ============================================================================
// Is MLLP Framed Tests
// ============================================================================

#[test]
fn test_is_mllp_framed_true() {
    let framed = wrap_mllp(b"MSH|TEST");
    assert!(is_mllp_framed(&framed));
}

#[test]
fn test_is_mllp_framed_false() {
    assert!(!is_mllp_framed(b"MSH|TEST"));
    assert!(!is_mllp_framed(b""));
}

#[test]
fn test_is_mllp_framed_only_start() {
    let data = vec![MLLP_START];
    assert!(is_mllp_framed(&data));
}
