//! Fuzz target for parsing complete HL7 messages.
//!
//! This fuzz target tests the parser with arbitrary byte sequences
//! to ensure it never panics and handles all inputs gracefully.

#![no_main]

use libfuzzer_sys::fuzz_target;
use hl7v2_parser::parse;

fuzz_target!(|data: &[u8]| {
    // The parser should never panic on any input
    // It should either parse successfully or return an error
    let _ = parse(data);
});
