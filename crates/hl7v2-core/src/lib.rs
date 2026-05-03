//! Core parsing and data model for HL7 v2 messages.
//!
//! This crate provides a unified facade for the HL7 v2 microcrates:
//! - `hl7v2-model`: Core data types (Message, Segment, Field, etc.)
//! - `hl7v2-escape`: Escape sequence handling
//! - `hl7v2-mllp`: MLLP framing protocol
//! - `hl7v2-parser`: Message parsing
//! - `hl7v2-writer`: Message serialization
//! - `hl7v2-normalize`: Message normalization
//! - `hl7v2-stream`: Streaming/event-based parsing (optional, enable with `stream` feature)
//! - `hl7v2-network`: Network client/server (optional, enable with `network` feature)
//!
//! For backward compatibility, all types and functions are re-exported here.
//! For new code, consider using the microcrates directly for finer-grained dependencies.
//!
//! # Memory Efficiency
//!
//! The standard parser uses a "zero-allocation where possible" approach rather than true zero-copy.
//! Parsed messages own their data via `Vec<u8>`, prioritizing safety and ergonomics.
//!
//! For memory-constrained environments or very large messages, enable the `stream` feature
//! and use `StreamParser` with configured memory bounds:
//!
//! ```rust,no_run
//! use hl7v2_core::{StreamParser, Event};
//! use std::io::BufReader;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let reader = BufReader::new(std::io::stdin());
//! let mut parser = StreamParser::new(reader);
//!
//! // Stream parse with bounded memory
//! while let Some(event_result) = parser.next_event().transpose() {
//!     match event_result? {
//!         Event::Segment { id } => { /* handle segment */ }
//!         Event::Field { num, raw } => { /* handle field */ }
//!         _ => {}
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Features
//!
//! - `stream`: Enables the streaming parser (`StreamParser` and `Event` types)
//! - `network`: Enables the network module (async client/server)

// Re-export model types
pub use hl7v2_model::{
    Atom, Batch, Comp, Delims, Error, Field, FileBatch, Message, Presence, Rep, Segment,
};

// Re-export escape functions
pub use hl7v2_escape::{escape_text, needs_escaping, needs_unescaping, unescape_text};

// Re-export MLLP types and functions
pub use hl7v2_mllp::{
    MLLP_END_1, MLLP_END_2, MLLP_START, MllpFrameIterator, find_complete_mllp_message,
    is_mllp_framed, unwrap_mllp, unwrap_mllp_owned, wrap_mllp,
};

// Re-export parser functions
pub use hl7v2_parser::{get, get_presence, parse, parse_batch, parse_file_batch, parse_mllp};

// Re-export writer functions
pub use hl7v2_writer::{
    to_json, to_json_string, to_json_string_pretty, write, write_batch, write_file_batch,
    write_mllp,
};

// Re-export normalization function
pub use hl7v2_normalize::normalize;

// Re-export network module when feature is enabled
#[cfg(feature = "network")]
pub use hl7v2_network as network;

// Re-export stream module when feature is enabled
#[cfg(feature = "stream")]
pub use hl7v2_stream::{Event, StreamParser};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_message() {
        let hl7_text = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r";
        let message = parse(hl7_text.as_bytes()).unwrap();

        assert_eq!(message.delims.field, '|');
        assert_eq!(message.delims.comp, '^');
        assert_eq!(message.delims.rep, '~');
        assert_eq!(message.delims.esc, '\\');
        assert_eq!(message.delims.sub, '&');

        assert_eq!(message.segments.len(), 2);

        // Check MSH segment
        assert_eq!(&message.segments[0].id, b"MSH");
        assert_eq!(message.segments[0].fields.len(), 11); // MSH has 11 fields (not counting the field separator)

        // Check PID segment
        assert_eq!(&message.segments[1].id, b"PID");
        assert_eq!(message.segments[1].fields.len(), 5); // PID has 5 fields
    }

    #[test]
    fn test_get_with_repetitions() {
        // Create a message with field repetitions
        let hl7_text =
            "MSH|^~\\&|SendingApp|SendingFac\rPID|1||123456^^^HOSP^MR||Doe^John~Smith^Jane\r";
        let message = parse(hl7_text.as_bytes()).unwrap();

        // Test first repetition (default)
        assert_eq!(get(&message, "PID.5.1"), Some("Doe"));
        assert_eq!(get(&message, "PID.5.2"), Some("John"));

        // Test second repetition
        assert_eq!(get(&message, "PID.5[2].1"), Some("Smith"));
        assert_eq!(get(&message, "PID.5[2].2"), Some("Jane"));

        // Test repetition that doesn't exist
        assert_eq!(get(&message, "PID.5[3].1"), None);
    }

    #[test]
    fn test_mllp_parsing_and_writing() {
        // Create a simple HL7 message
        let hl7_text = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r";
        let original_message = parse(hl7_text.as_bytes()).unwrap();

        // Wrap with MLLP framing
        let mllp_bytes = write_mllp(&original_message);

        // Verify MLLP framing
        assert_eq!(mllp_bytes[0], 0x0B); // Start byte
        assert_eq!(mllp_bytes[mllp_bytes.len() - 2], 0x1C); // End byte 1
        assert_eq!(mllp_bytes[mllp_bytes.len() - 1], 0x0D); // End byte 2

        // Parse from MLLP framed bytes
        let parsed_message = parse_mllp(&mllp_bytes).unwrap();

        // Verify the messages are equivalent
        assert_eq!(
            original_message.segments.len(),
            parsed_message.segments.len()
        );
        assert_eq!(
            std::str::from_utf8(&original_message.segments[0].id).unwrap(),
            std::str::from_utf8(&parsed_message.segments[0].id).unwrap()
        );
        assert_eq!(
            std::str::from_utf8(&original_message.segments[1].id).unwrap(),
            std::str::from_utf8(&parsed_message.segments[1].id).unwrap()
        );
    }

    #[test]
    fn test_presence_semantics() {
        // Create a message with various field types
        let hl7_text = "MSH|^~\\&|SendingApp|SendingFac\rPID|1||123456^^^HOSP^MR||Doe^John|||\r";
        let message = parse(hl7_text.as_bytes()).unwrap();

        // Test existing field with value
        match get_presence(&message, "PID.5.1") {
            Presence::Value(val) => assert_eq!(val, "Doe"),
            _ => panic!("Expected Value, got something else"),
        }

        // Test existing field with empty value (PID.8 in our test message is empty)
        match get_presence(&message, "PID.8.1") {
            Presence::Empty => {}
            _ => panic!("Expected Empty, got something else"),
        }

        // Test missing field (PID.50 doesn't exist)
        match get_presence(&message, "PID.50.1") {
            Presence::Missing => {}
            _ => panic!("Expected Missing, got something else"),
        }

        // Test MSH-1 (special case)
        match get_presence(&message, "MSH.1") {
            Presence::Value(val) => assert_eq!(val, "|"),
            _ => panic!("Expected Value for MSH.1, got something else"),
        }
    }

    #[test]
    #[cfg(feature = "stream")]
    fn test_streaming_parser() {
        use std::io::BufReader;
        use std::io::Cursor;

        // Create a simple HL7 message
        let hl7_text = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r";
        let cursor = Cursor::new(hl7_text.as_bytes());
        let buf_reader = BufReader::new(cursor);

        let mut parser = StreamParser::new(buf_reader);

        // Collect all events
        let mut events = Vec::new();
        while let Ok(Some(event)) = parser.next_event() {
            events.push(event);
        }

        // Verify we got the expected events
        assert!(!events.is_empty());

        // Check for StartMessage event
        let start_event = events
            .iter()
            .find(|e| matches!(e, Event::StartMessage { .. }));
        assert!(start_event.is_some());

        // Check for Segment events (should have PID segment)
        let segment_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, Event::Segment { .. }))
            .collect();
        assert_eq!(segment_events.len(), 1); // PID segment

        // Check that the segment is PID
        if let Event::Segment { id } = &segment_events[0] {
            assert_eq!(id, b"PID");
        }

        // Check for Field events
        let field_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, Event::Field { .. }))
            .collect();
        assert!(!field_events.is_empty());

        // Check for EndMessage event
        let end_event = events.iter().find(|e| matches!(e, Event::EndMessage));
        assert!(end_event.is_some());
    }
}
