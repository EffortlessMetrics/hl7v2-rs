//! Fuzz target for parsing HL7 batch messages.
//!
//! This fuzz target tests batch parsing with arbitrary byte sequences
//! to ensure it never panics and handles all inputs gracefully.

#![no_main]

use libfuzzer_sys::fuzz_target;
use hl7v2_parser::parse_batch;

fuzz_target!(|data: &[u8]| {
    // The batch parser should never panic on any input
    // It should either parse successfully or return an error
    let _ = parse_batch(data);
});
