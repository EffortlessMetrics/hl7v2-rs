//! Fuzz target for MLLP codec encoding.
//!
//! This target focuses specifically on the encoder, testing various
//! byte sequences as message content.

#![no_main]

use bytes::BytesMut;
use libfuzzer_sys::fuzz_target;
use tokio_util::codec::Encoder;
use hl7v2_network::MllpCodec;

fuzz_target!(|data: &[u8]| {
    let mut codec = MllpCodec::new();
    let mut buffer = BytesMut::new();
    
    // Try to encode - should never panic
    match codec.encode(BytesMut::from(data), &mut buffer) {
        Ok(()) => {
            // Successfully encoded
            // Verify frame structure
            if buffer.len() >= 3 {
                // Should start with 0x0B
                assert_eq!(buffer[0], 0x0B);
                // Should end with 0x1C 0x0D
                assert_eq!(buffer[buffer.len() - 2], 0x1C);
                assert_eq!(buffer[buffer.len() - 1], 0x0D);
            }
        }
        Err(_e) => {
            // Error (e.g., message too large), this is fine
        }
    }
});
