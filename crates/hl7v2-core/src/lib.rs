//! Core parsing and data model for HL7 v2 messages.
//!
//! This crate provides a unified facade for the HL7 v2 microcrates:
//! - `hl7v2-model`: Core data types (Message, Segment, Field, etc.)
//! - `hl7v2-escape`: Escape sequence handling
//! - `hl7v2-mllp`: MLLP framing protocol
//! - `hl7v2-parser`: Message parsing
//! - `hl7v2-writer`: Message serialization
//!
//! For backward compatibility, all types and functions are re-exported here.
//! For new code, consider using the microcrates directly for finer-grained dependencies.

// Re-export model types
pub use hl7v2_model::{
    Atom, Batch, Comp, Delims, Error, Field, FileBatch, Message, Presence, Rep, Segment,
};

// Re-export escape functions
pub use hl7v2_escape::{
    escape_text, needs_escaping, needs_unescaping, unescape_text,
};

// Re-export MLLP types and functions
pub use hl7v2_mllp::{
    is_mllp_framed, find_complete_mllp_message, unwrap_mllp, unwrap_mllp_owned, wrap_mllp,
    MllpFrameIterator, MLLP_END_1, MLLP_END_2, MLLP_START,
};

// Re-export parser functions
pub use hl7v2_parser::{
    get, get_presence, parse, parse_batch, parse_file_batch, parse_mllp,
};

// Re-export writer functions
pub use hl7v2_writer::{
    normalize, to_json, to_json_string, to_json_string_pretty, write, write_batch, write_file_batch,
    write_mllp,
};

#[cfg(feature = "network")]
pub mod network;

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
    event_queue: std::collections::VecDeque<Event>,
}

impl<D: std::io::BufRead> StreamParser<D> {
    /// Create a new streaming parser
    pub fn new(reader: D) -> Self {
        Self {
            reader,
            delims: Delims::default(),
            buffer: Vec::new(),
            pos: 0,
            pre_msh: true,
            in_message: false,
            event_queue: std::collections::VecDeque::new(),
        }
    }

    /// Get the next event from the stream
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
                        std::str::from_utf8(&segment_data).map_err(|_| Error::InvalidCharset)?
                    ).map_err(|e| Error::ParseError {
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
                    
                    return Ok(Some(Event::StartMessage { delims: Delims::default() }));
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
            let fields: Vec<&[u8]> = fields_data
                .split(|&b| b == field_separator)
                .collect();
            
            // Generate field events for each field (1-based numbering)
            for (index, field) in fields.iter().enumerate() {
                let field_num = (index + 1) as u16;
                self.event_queue.push_back(Event::Field { 
                    num: field_num, 
                    raw: field.to_vec() 
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
            let fields: Vec<&[u8]> = fields_data
                .split(|&b| b == field_separator)
                .collect();
            
            // Generate field events for each field (1-based numbering)
            for (index, field) in fields.iter().enumerate() {
                let field_num = (index + 1) as u16;
                self.event_queue.push_back(Event::Field { 
                    num: field_num, 
                    raw: field.to_vec() 
                });
            }
        }
        Ok(())
    }
}

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
        let hl7_text = "MSH|^~\\&|SendingApp|SendingFac\rPID|1||123456^^^HOSP^MR||Doe^John~Smith^Jane\r";
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
        assert_eq!(mllp_bytes[mllp_bytes.len()-2], 0x1C); // End byte 1
        assert_eq!(mllp_bytes[mllp_bytes.len()-1], 0x0D); // End byte 2
        
        // Parse from MLLP framed bytes
        let parsed_message = parse_mllp(&mllp_bytes).unwrap();
        
        // Verify the messages are equivalent
        assert_eq!(original_message.segments.len(), parsed_message.segments.len());
        assert_eq!(std::str::from_utf8(&original_message.segments[0].id).unwrap(), 
                   std::str::from_utf8(&parsed_message.segments[0].id).unwrap());
        assert_eq!(std::str::from_utf8(&original_message.segments[1].id).unwrap(), 
                   std::str::from_utf8(&parsed_message.segments[1].id).unwrap());
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
            Presence::Empty => assert!(true),
            _ => panic!("Expected Empty, got something else"),
        }
        
        // Test missing field (PID.50 doesn't exist)
        match get_presence(&message, "PID.50.1") {
            Presence::Missing => assert!(true),
            _ => panic!("Expected Missing, got something else"),
        }
        
        // Test MSH-1 (special case)
        match get_presence(&message, "MSH.1") {
            Presence::Value(val) => assert_eq!(val, "|"),
            _ => panic!("Expected Value for MSH.1, got something else"),
        }
    }

    #[test]
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
        let start_event = events.iter().find(|e| matches!(e, Event::StartMessage { .. }));
        assert!(start_event.is_some());
        
        // Check for Segment events (should have PID segment)
        let segment_events: Vec<_> = events.iter().filter(|e| matches!(e, Event::Segment { .. })).collect();
        assert_eq!(segment_events.len(), 1); // PID segment
        
        // Check that the segment is PID
        if let Event::Segment { id } = &segment_events[0] {
            assert_eq!(id, b"PID");
        }
        
        // Check for Field events
        let field_events: Vec<_> = events.iter().filter(|e| matches!(e, Event::Field { .. })).collect();
        assert!(!field_events.is_empty());
        
        // Check for EndMessage event
        let end_event = events.iter().find(|e| matches!(e, Event::EndMessage));
        assert!(end_event.is_some());
    }
}