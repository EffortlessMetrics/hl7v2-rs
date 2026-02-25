//! Fuzz target for parsing individual HL7 fields.
//!
//! This fuzz target tests field-level parsing with various
//! delimiter combinations and edge cases.

#![no_main]

use libfuzzer_sys::fuzz_target;
use hl7v2_parser::parse;
use hl7v2_model::Delims;

fuzz_target!(|data: &[u8]| {
    // Try to parse as a minimal message with the data as a field value
    // This tests field parsing with various edge cases

    // First, try to create a valid message with the data embedded
    // We need to be careful about control characters

    if let Ok(s) = std::str::from_utf8(data) {
        // Escape any special characters that might break the message structure
        let escaped = s
            .replace('|', "\\F\\")
            .replace('\r', "")
            .replace('\n', "");

        // Create a minimal message with the escaped data as a field
        let message = format!(
            "MSH|^~\\&|App|Fac|Recv|RecvFac|20250128120000||ADT^A01|MSG123|P|2.5\rPID|1||{}||Test\r",
            escaped
        );

        // The parser should never panic
        let _ = parse(message.as_bytes());
    }

    // Also try parsing the raw data directly
    let _ = parse(data);
});
