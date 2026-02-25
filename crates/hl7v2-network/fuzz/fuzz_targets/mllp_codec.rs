//! Fuzz target for MLLP codec encoding and decoding.
//!
//! This target tests the MLLP codec with arbitrary byte sequences to find
//! panics, hangs, or unexpected behavior in the codec implementation.

#![no_main]

use bytes::BytesMut;
use libfuzzer_sys::fuzz_target;
use tokio_util::codec::{Decoder, Encoder};
use hl7v2_network::MllpCodec;

fuzz_target!(|data: &[u8]| {
    let mut codec = MllpCodec::new();
    
    // Test 1: Try to decode the input data
    let mut decode_buffer = BytesMut::from(data);
    let _ = codec.decode(&mut decode_buffer);
    
    // Test 2: Try to encode the input data
    let mut encode_buffer = BytesMut::new();
    let _ = codec.encode(BytesMut::from(data), &mut encode_buffer);
    
    // Test 3: Roundtrip - encode then decode
    if encode_buffer.len() > 0 {
        let _ = codec.decode(&mut encode_buffer);
    }
});
