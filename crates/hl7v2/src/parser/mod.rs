//! HL7 v2 message parser.
//!
//! This module provides parsing functionality for HL7 v2 messages,
//! including:
//! - Message parsing from raw bytes
//! - Batch message handling (FHS/BHS/BTS/FTS)
//! - MLLP-framed message parsing
//! - Path-based field access (re-exported from `hl7v2::query`)
//!
//! # Memory Efficiency
//!
//! This parser uses a "zero-allocation where possible" approach rather than true zero-copy.
//! Parsed messages own their data via `Vec<u8>`, which provides:
//!
//! - Safe lifetime management without complex borrow checker patterns
//! - Ergonomic API that doesn't require managing input lifetimes
//! - Ability to modify and re-serialize messages
//!
//! For memory-constrained environments or very large messages, consider using
//! [`crate::stream`] which provides an event-based
//! streaming parser with bounded memory usage.
//!
//! # Example
//!
//! ```
//! use hl7v2::parser::parse;
//!
//! let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r";
//! let message = parse(hl7).unwrap();
//!
//! assert_eq!(message.segments.len(), 2);
//! ```

mod batch;
mod charset;
mod message;
mod segment;

pub use batch::{parse_batch, parse_file_batch};
pub use message::{parse, parse_mllp};

// Re-export query functionality for backward compatibility.
pub use crate::query::{get, get_presence};

#[cfg(test)]
mod tests {
    #![expect(
        clippy::panic,
        clippy::unwrap_used,
        reason = "pre-existing parser inline test panic-family debt moved into hl7v2; cleanup is split from topology collapse"
    )]

    use super::*;
    use crate::model::Presence;

    #[test]
    fn test_parse_simple_message() {
        let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r";
        let message = parse(hl7).unwrap();

        assert_eq!(message.delims.field, '|');
        assert_eq!(message.delims.comp, '^');
        assert_eq!(message.delims.rep, '~');
        assert_eq!(message.delims.esc, '\\');
        assert_eq!(message.delims.sub, '&');

        assert_eq!(message.segments.len(), 2);
        assert_eq!(&message.segments[0].id, b"MSH");
        assert_eq!(&message.segments[1].id, b"PID");
    }

    #[test]
    fn test_get_simple_field() {
        let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r";
        let message = parse(hl7).unwrap();

        // Get patient's last name (PID.5.1)
        assert_eq!(get(&message, "PID.5.1"), Some("Doe"));

        // Get patient's first name (PID.5.2)
        assert_eq!(get(&message, "PID.5.2"), Some("John"));
    }

    #[test]
    fn test_get_msh_fields() {
        let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\r";
        let message = parse(hl7).unwrap();

        // Get sending application (MSH.3)
        assert_eq!(get(&message, "MSH.3"), Some("SendingApp"));

        // Get message type (MSH.9)
        assert_eq!(get(&message, "MSH.9.1"), Some("ADT"));
        assert_eq!(get(&message, "MSH.9.2"), Some("A01"));
    }

    #[test]
    fn test_get_with_repetitions() {
        let hl7 =
            b"MSH|^~\\&|SendingApp|SendingFac\rPID|1||123456^^^HOSP^MR||Doe^John~Smith^Jane\r";
        let message = parse(hl7).unwrap();

        // Test first repetition (default)
        assert_eq!(get(&message, "PID.5.1"), Some("Doe"));
        assert_eq!(get(&message, "PID.5.2"), Some("John"));

        // Test second repetition
        assert_eq!(get(&message, "PID.5[2].1"), Some("Smith"));
        assert_eq!(get(&message, "PID.5[2].2"), Some("Jane"));
    }

    #[test]
    fn test_parse_mllp() {
        let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\r";
        let framed = crate::transport::mllp::wrap_mllp(hl7);
        let message = parse_mllp(&framed).unwrap();

        assert_eq!(message.segments.len(), 1);
    }

    #[test]
    fn test_presence_semantics() {
        let hl7 = b"MSH|^~\\&|SendingApp|SendingFac\rPID|1||123456^^^HOSP^MR||Doe^John|||\r";
        let message = parse(hl7).unwrap();

        // Test existing field with value
        match get_presence(&message, "PID.5.1") {
            Presence::Value(val) => assert_eq!(val, "Doe"),
            _ => panic!("Expected Value"),
        }

        // Test existing field with empty value
        match get_presence(&message, "PID.8.1") {
            Presence::Empty => {}
            _ => panic!("Expected Empty"),
        }

        // Test missing field
        match get_presence(&message, "PID.50.1") {
            Presence::Missing => {}
            _ => panic!("Expected Missing"),
        }
    }
}

// Comprehensive test suite modules
#[cfg(test)]
pub mod comprehensive_tests;
