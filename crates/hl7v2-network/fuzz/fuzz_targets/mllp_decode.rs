//! Fuzz target for MLLP codec decoding.
//!
//! This target focuses specifically on the decoder, testing various
//! malformed and edge-case MLLP frames.

#![no_main]

use bytes::BytesMut;
use libfuzzer_sys::fuzz_target;
use tokio_util::codec::Decoder;
use hl7v2_network::MllpCodec;

fuzz_target!(|data: &[u8]| {
    let mut codec = MllpCodec::new();
    let mut buffer = BytesMut::from(data);
    
    // Try to decode - should never panic
    match codec.decode(&mut buffer) {
        Ok(Some(_content)) => {
            // Successfully decoded a frame
        }
        Ok(None) => {
            // Incomplete frame, this is fine
        }
        Err(_e) => {
            // Error (e.g., frame too large), this is fine
        }
    }
    
    // Try to decode again with remaining data
    let _ = codec.decode(&mut buffer);
});
