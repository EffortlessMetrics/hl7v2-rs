//! Fuzz target for parsing MLLP-framed HL7 messages.
//!
//! This fuzz target tests MLLP parsing with arbitrary byte sequences
//! to ensure it never panics and handles all inputs gracefully.

#![no_main]

use libfuzzer_sys::fuzz_target;
use hl7v2_parser::parse_mllp;

fuzz_target!(|data: &[u8]| {
    // The MLLP parser should never panic on any input
    // It should either parse successfully or return an error
    let _ = parse_mllp(data);
});
