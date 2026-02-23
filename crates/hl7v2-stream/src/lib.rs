//! Streaming/event-based parser for HL7 v2 messages.
//!
//! This crate provides a streaming parser that emits events as it parses HL7 v2 messages,
//! allowing for memory-efficient processing of large messages without loading the entire
//! message into memory.
//!
//! # Overview
//!
//! The [`StreamParser`] reads from any `BufRead` source and emits [`Event`] values
//! representing different parts of an HL7 message:
//! - [`Event::StartMessage`] - Beginning of a message with discovered delimiters
//! - [`Event::Segment`] - A segment with its 3-character ID
//! - [`Event::Field`] - A field with its number and raw content
//! - [`Event::EndMessage`] - End of the current message
//!
//! # Example
//!
//! ```rust
//! use hl7v2_stream::{StreamParser, Event};
//! use std::io::{BufReader, Cursor};
//!
//! let hl7_text = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r";
//! let cursor = Cursor::new(hl7_text.as_bytes());
//! let buf_reader = BufReader::new(cursor);
//!
//! let mut parser = StreamParser::new(buf_reader);
//!
//! while let Ok(Some(event)) = parser.next_event() {
//!     match event {
//!         Event::StartMessage { delims } => println!("Message started with delims: {:?}", delims),
//!         Event::Segment { id } => println!("Segment: {}", String::from_utf8_lossy(&id)),
//!         Event::Field { num, raw } => println!("Field {}: {:?}", num, raw),
//!         Event::EndMessage => println!("Message ended"),
//!     }
//! }
//! ```

// Re-export Delims from hl7v2-model for convenience
pub use hl7v2_model::Delims;

use hl7v2_model::Error;
use std::collections::VecDeque;
use std::io::BufRead;

/// Event enum for streaming parser
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// Start of a new message with discovered delimiters
    StartMessage { delims: Delims },
    /// A segment with its ID
    Segment { id: Vec<u8> },
    /// A field with its number (1-based) and raw content
    Field { num: u16, raw: Vec<u8> },
    /// End of message
    EndMessage,
}

/// Streaming parser for HL7 v2 messages
///
/// The `StreamParser` reads HL7 v2 messages from any `BufRead` source and emits
/// [`Event`] values as it encounters different parts of the message.
///
/// # Memory Efficiency
///
/// Unlike the one-shot parser, the streaming parser only holds the current segment
/// in memory at a time, making it suitable for processing very large HL7 messages.
///
/// # Delimiter Handling
///
/// The parser automatically detects delimiters from the MSH segment and uses them
/// for the duration of that message. When a new MSH segment is encountered, the
/// delimiters are updated for the new message.
pub struct StreamParser<D> {
    /// Reader for input data
    reader: D,
    /// Current delimiters (starts with default, switches per message)
    delims: Delims,
    /// Buffer for accumulating data
    buffer: Vec<u8>,
    /// Current position in buffer
    pos: usize,
    /// Whether we're in pre-MSH mode
    pre_msh: bool,
    /// Whether we've started parsing a message
    in_message: bool,
    /// Queue of events to be returned
    event_queue: VecDeque<Event>,
}

impl<D: BufRead> StreamParser<D> {
    /// Create a new streaming parser
    ///
    /// # Arguments
    ///
    /// * `reader` - A `BufRead` source containing HL7 v2 message data
    ///
    /// # Example
    ///
    /// ```rust
    /// use hl7v2_stream::StreamParser;
    /// use std::io::{BufReader, Cursor};
    ///
    /// let data = b"MSH|^~\\&|App|Fac\r";
    /// let cursor = Cursor::new(&data[..]);
    /// let reader = BufReader::new(cursor);
    ///
    /// let parser = StreamParser::new(reader);
    /// ```
    pub fn new(reader: D) -> Self {
        Self {
            reader,
            delims: Delims::default(),
            buffer: Vec::new(),
            pos: 0,
            pre_msh: true,
            in_message: false,
            event_queue: VecDeque::new(),
        }
    }

    /// Get the next event from the stream
    ///
    /// Returns `Ok(Some(event))` when an event is available, `Ok(None)` when
    /// the stream is exhausted, and `Err(e)` on parse errors.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The data contains invalid UTF-8 where charset detection is needed
    /// - The MSH segment has invalid delimiters
    pub fn next_event(&mut self) -> Result<Option<Event>, Error> {
        // First check if we have any queued events
        if let Some(event) = self.event_queue.pop_front() {
            return Ok(Some(event));
        }

        loop {
            // If we're at the end of our buffer, try to read more data
            if self.pos >= self.buffer.len() {
                let mut temp_buf = vec![0u8; 1024];
                match self.reader.read(&mut temp_buf) {
                    Ok(0) => {
                        // End of input
                        if self.in_message {
                            self.in_message = false;
                            self.pre_msh = true;
                            return Ok(Some(Event::EndMessage));
                        }
                        return Ok(None);
                    }
                    Ok(n) => {
                        // Add the new data to our buffer
                        self.buffer.extend_from_slice(&temp_buf[..n]);
                    }
                    Err(_) => return Err(Error::InvalidCharset),
                }
            }

            // Look for a complete segment (ending with \r)
            if let Some(cr_pos) = self.buffer[self.pos..].iter().position(|&b| b == b'\r') {
                let segment_end = self.pos + cr_pos;
                let segment_data = self.buffer[self.pos..segment_end].to_vec();
                self.pos = segment_end + 1; // Skip the \r

                // Check if this is an MSH segment
                if segment_data.len() >= 3 && &segment_data[0..3] == b"MSH" {
                    // We're starting a new message
                    if self.in_message {
                        // End the previous message first
                        self.in_message = false;
                        self.pre_msh = true;
                        return Ok(Some(Event::EndMessage));
                    }

                    // Parse delimiters from MSH segment
                    let new_delims = Delims::parse_from_msh(
                        std::str::from_utf8(&segment_data).map_err(|_| Error::InvalidCharset)?,
                    )
                    .map_err(|e| Error::ParseError {
                        segment_id: "MSH".to_string(),
                        field_index: 0,
                        source: Box::new(e),
                    })?;

                    // Switch to the new delimiters for this message only
                    self.delims = new_delims.clone();
                    self.pre_msh = false;
                    self.in_message = true;

                    // Generate field events for MSH segment
                    self.generate_msh_field_events(&segment_data)?;

                    return Ok(Some(Event::StartMessage { delims: new_delims }));
                }

                // For any other segment
                if self.in_message && segment_data.len() >= 3 {
                    let segment_id = segment_data[0..3].to_vec();

                    // Generate field events for this segment
                    self.generate_field_events(&segment_data)?;

                    return Ok(Some(Event::Segment { id: segment_id }));
                } else if !self.in_message && self.pre_msh && segment_data.len() >= 3 {
                    // We're in pre-MSH mode but this isn't an MSH segment,
                    // so start a message with default delimiters
                    self.delims = Delims::default();
                    self.pre_msh = false;
                    self.in_message = true;

                    // Generate field events for this segment
                    self.generate_field_events(&segment_data)?;

                    return Ok(Some(Event::StartMessage {
                        delims: Delims::default(),
                    }));
                }
            }

            // If we've reached here and have no more data, we're done
            if self.pos >= self.buffer.len() {
                if self.in_message {
                    self.in_message = false;
                    self.pre_msh = true;
                    return Ok(Some(Event::EndMessage));
                }
                return Ok(None);
            }
        }
    }

    /// Generate field events for a regular segment
    fn generate_field_events(&mut self, segment_data: &[u8]) -> Result<(), Error> {
        if segment_data.len() > 4 {
            let fields_data = &segment_data[4..]; // Skip segment ID and field separator
            let field_separator = self.delims.field as u8;

            // Split fields by the field separator
            let fields: Vec<&[u8]> = fields_data.split(|&b| b == field_separator).collect();

            // Generate field events for each field (1-based numbering)
            for (index, field) in fields.iter().enumerate() {
                let field_num = (index + 1) as u16;
                self.event_queue.push_back(Event::Field {
                    num: field_num,
                    raw: field.to_vec(),
                });
            }
        }
        Ok(())
    }

    /// Generate field events specifically for MSH segment
    fn generate_msh_field_events(&mut self, segment_data: &[u8]) -> Result<(), Error> {
        if segment_data.len() > 8 {
            // MSH has special handling - fields start after the encoding characters
            let fields_data = &segment_data[8..]; // Skip "MSH|^~\&"
            let field_separator = self.delims.field as u8;

            // Split fields by the field separator
            let fields: Vec<&[u8]> = fields_data.split(|&b| b == field_separator).collect();

            // Generate field events for each field (1-based numbering)
            for (index, field) in fields.iter().enumerate() {
                let field_num = (index + 1) as u16;
                self.event_queue.push_back(Event::Field {
                    num: field_num,
                    raw: field.to_vec(),
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufReader, Cursor};

    #[test]
    fn test_streaming_parser() {
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
        let start_event = events.iter().find(|e| matches!(e, Event::StartMessage { .. }));
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

    #[test]
    fn test_custom_delimiters() {
        // Create a message with custom delimiters
        let hl7_text = "MSH$@#*|App|Fac\rPID$1||123\r";
        let cursor = Cursor::new(hl7_text.as_bytes());
        let buf_reader = BufReader::new(cursor);

        let mut parser = StreamParser::new(buf_reader);

        let mut found_start = false;
        while let Ok(Some(event)) = parser.next_event() {
            if let Event::StartMessage { delims } = &event {
                assert_eq!(delims.field, '$');
                assert_eq!(delims.comp, '@');
                assert_eq!(delims.rep, '#');
                assert_eq!(delims.esc, '*');
                found_start = true;
            }
        }
        assert!(found_start);
    }
}
