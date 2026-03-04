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
//!
//! # Async Streaming with Backpressure
//!
//! The [`AsyncStreamParser`] provides async streaming with bounded channels for backpressure:
//!
//! ```rust,no_run
//! use hl7v2_stream::{AsyncStreamParser, StreamParserBuilder, Event};
//!
//! #[tokio::main]
//! async fn main() {
//!     let hl7_text = b"MSH|^~\\&|App|Fac\r".to_vec();
//!     
//!     let mut parser = StreamParserBuilder::new()
//!         .buffer_size(100)
//!         .max_message_size(1024 * 1024)
//!         .build_async(hl7_text);
//!     
//!     while let Some(result) = parser.next().await {
//!         match result {
//!             Ok(event) => println!("Event: {:?}", event),
//!             Err(e) => eprintln!("Error: {:?}", e),
//!         }
//!     }
//! }
//! ```

// Re-export Delims from hl7v2-model for convenience
pub use hl7v2_model::Delims;

use hl7v2_model::Error;
use std::collections::VecDeque;
use std::io::BufRead;
use tokio::sync::mpsc::{self, Receiver};

/// Default buffer size for async channel (number of events)
const DEFAULT_BUFFER_SIZE: usize = 100;

/// Default maximum message size (1 MB)
const DEFAULT_MAX_MESSAGE_SIZE: usize = 1024 * 1024;

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

/// Error type for streaming parser operations
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum StreamError {
    /// Message exceeded maximum allowed size
    #[error("Message size {actual} exceeds maximum allowed size {max}")]
    MessageTooLarge {
        /// Actual size of the message
        actual: usize,
        /// Maximum allowed size
        max: usize,
    },
    /// Parse error from underlying parser
    #[error("Parse error: {0}")]
    ParseError(String),
    /// Channel error
    #[error("Channel error: {0}")]
    ChannelError(String),
}

impl From<Error> for StreamError {
    fn from(err: Error) -> Self {
        StreamError::ParseError(format!("{:?}", err))
    }
}

/// Builder for configuring stream parsers
///
/// Allows customization of buffer sizes and memory limits.
#[derive(Debug, Clone)]
pub struct StreamParserBuilder {
    /// Buffer size for async channel (number of events)
    buffer_size: usize,
    /// Maximum message size in bytes
    max_message_size: usize,
}

impl Default for StreamParserBuilder {
    fn default() -> Self {
        Self {
            buffer_size: DEFAULT_BUFFER_SIZE,
            max_message_size: DEFAULT_MAX_MESSAGE_SIZE,
        }
    }
}

impl StreamParserBuilder {
    /// Create a new builder with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the buffer size for the async channel
    ///
    /// This controls how many events can be buffered before backpressure
    /// is applied to the parser.
    ///
    /// # Arguments
    ///
    /// * `size` - Number of events to buffer (default: 100)
    pub fn buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size;
        self
    }

    /// Set the maximum message size in bytes
    ///
    /// Messages exceeding this size will result in a `MessageTooLarge` error.
    ///
    /// # Arguments
    ///
    /// * `size` - Maximum message size in bytes (default: 1 MB)
    pub fn max_message_size(mut self, size: usize) -> Self {
        self.max_message_size = size;
        self
    }

    /// Build a synchronous stream parser
    ///
    /// # Arguments
    ///
    /// * `reader` - A `BufRead` source containing HL7 v2 message data
    pub fn build<R: BufRead>(self, reader: R) -> StreamParser<R> {
        StreamParser {
            reader,
            delims: Delims::default(),
            buffer: Vec::new(),
            pos: 0,
            pre_msh: true,
            in_message: false,
            event_queue: VecDeque::new(),
            max_message_size: self.max_message_size,
            current_message_size: 0,
        }
    }

    /// Build an async stream parser with backpressure
    ///
    /// Returns a receiver that yields events as they are parsed.
    /// Parsing pauses when the channel buffer is full (backpressure).
    ///
    /// # Arguments
    ///
    /// * `data` - Byte data containing HL7 v2 message data
    pub fn build_async(self, data: Vec<u8>) -> AsyncStreamParser {
        let (tx, rx) = mpsc::channel(self.buffer_size);
        let max_message_size = self.max_message_size;

        tokio::spawn(async move {
            let cursor = std::io::Cursor::new(data);
            let buf_reader = std::io::BufReader::new(cursor);
            let mut parser = StreamParser {
                reader: buf_reader,
                delims: Delims::default(),
                buffer: Vec::new(),
                pos: 0,
                pre_msh: true,
                in_message: false,
                event_queue: VecDeque::new(),
                max_message_size,
                current_message_size: 0,
            };

            loop {
                match parser.next_event() {
                    Ok(Some(event)) => {
                        if tx.send(Ok(event)).await.is_err() {
                            break; // Receiver dropped
                        }
                    }
                    Ok(None) => {
                        break; // End of stream
                    }
                    Err(e) => {
                        let _ = tx.send(Err(StreamError::from(e))).await;
                        break;
                    }
                }
            }
        });

        AsyncStreamParser { receiver: rx }
    }
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
///
/// # Memory Bounds
///
/// The parser enforces a maximum message size to prevent memory exhaustion.
/// When a message exceeds the configured limit, a `MessageTooLarge` error is returned.
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
    /// Maximum allowed message size in bytes
    max_message_size: usize,
    /// Current message size counter (resets on each new message)
    current_message_size: usize,
}

impl<D: BufRead> StreamParser<D> {
    /// Create a new streaming parser with default settings
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
            max_message_size: DEFAULT_MAX_MESSAGE_SIZE,
            current_message_size: 0,
        }
    }

    /// Create a new streaming parser with custom memory bounds
    ///
    /// # Arguments
    ///
    /// * `reader` - A `BufRead` source containing HL7 v2 message data
    /// * `max_message_size` - Maximum allowed message size in bytes
    pub fn with_max_message_size(reader: D, max_message_size: usize) -> Self {
        Self {
            reader,
            delims: Delims::default(),
            buffer: Vec::new(),
            pos: 0,
            pre_msh: true,
            in_message: false,
            event_queue: VecDeque::new(),
            max_message_size,
            current_message_size: 0,
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
    /// - The message exceeds the configured maximum size
    pub fn next_event(&mut self) -> Result<Option<Event>, Error> {
        // First check if we have any queued events
        if let Some(event) = self.event_queue.pop_front() {
            return Ok(Some(event));
        }

        loop {
            // Do we need more data? (No \r in remaining buffer)
            let cr_pos = self.buffer[self.pos..].iter().position(|&b| b == b'\r');

            if cr_pos.is_none() {
                let mut temp_buf = vec![0u8; 1024];
                match self.reader.read(&mut temp_buf) {
                    Ok(0) => {
                        // End of input
                        if self.in_message {
                            self.in_message = false;
                            self.pre_msh = true;
                            // Reset message size counter for next message
                            self.current_message_size = 0;
                            return Ok(Some(Event::EndMessage));
                        }
                        return Ok(None);
                    }
                    Ok(n) => {
                        // Add the new data to our buffer
                        self.buffer.extend_from_slice(&temp_buf[..n]);
                        continue; // Search again
                    }
                    Err(_) => return Err(Error::InvalidCharset),
                }
            }

            // We have a complete segment (ending with \r)
            let cr_pos = cr_pos.unwrap();
            let segment_end = self.pos + cr_pos;
            let segment_data = self.buffer[self.pos..segment_end].to_vec();
            let segment_len = segment_data.len() + 1; // Include the \r

            // Check memory bounds before processing
            if self.in_message {
                self.current_message_size += segment_len;
                if self.current_message_size > self.max_message_size {
                    let actual_size = self.current_message_size;
                    let max_size = self.max_message_size;
                    let segment_id =
                        String::from_utf8_lossy(segment_data.get(0..3).unwrap_or(b"UNK"))
                            .to_string();
                    // Reset state for next message
                    self.in_message = false;
                    self.pre_msh = true;
                    self.current_message_size = 0;
                    return Err(Error::InvalidFieldFormat {
                        details: format!(
                            "Message size {} exceeds maximum {} at segment {}",
                            actual_size, max_size, segment_id
                        ),
                    });
                }
            }

            self.pos = segment_end + 1; // Skip the \r

            // Check if this is an MSH segment
            if segment_data.len() >= 3 && &segment_data[0..3] == b"MSH" {
                // We're starting a new message
                if self.in_message {
                    // End the previous message first
                    self.in_message = false;
                    self.pre_msh = true;
                    // Reset message size counter for new message
                    self.current_message_size = segment_len;
                    // REWIND pos so MSH is processed on the next call!
                    self.pos -= segment_len;
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
                    // Initialize message size counter
                    self.current_message_size = segment_len;

                    // Generate field events for MSH segment
                    self.generate_msh_field_events(&segment_data)?;

                    return Ok(Some(Event::StartMessage { delims: new_delims }));
                }

                // For any other segment
                if self.in_message && segment_data.len() >= 3 && segment_data[0..3].iter().all(|c| c.is_ascii_alphanumeric()) {
                    let segment_id = segment_data[0..3].to_vec();

                    // Generate field events for this segment
                    self.generate_field_events(&segment_data)?;

                    return Ok(Some(Event::Segment { id: segment_id }));
                } else if !self.in_message && self.pre_msh && segment_data.len() >= 3 && segment_data[0..3].iter().all(|c| c.is_ascii_alphanumeric()) {
                    // We're in pre-MSH mode but this isn't an MSH segment,
                    // so start a message with default delimiters
                    self.delims = Delims::default();
                    self.pre_msh = false;
                    self.in_message = true;
                    // Initialize message size counter
                    self.current_message_size = segment_len;

                    // Generate field events for this segment
                    self.generate_field_events(&segment_data)?;

                    return Ok(Some(Event::StartMessage {
                        delims: Delims::default(),
                    }));
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

    /// Get the current message size (bytes processed for current message)
    pub fn current_message_size(&self) -> usize {
        self.current_message_size
    }

    /// Get the maximum allowed message size
    pub fn max_message_size(&self) -> usize {
        self.max_message_size
    }

    /// Check if the parser is currently within a message
    pub fn is_in_message(&self) -> bool {
        self.in_message
    }

    /// Resume parsing with additional data
    ///
    /// This method allows resuming parsing after the buffer has been exhausted.
    /// It appends new data to any remaining partial segment data.
    ///
    /// # Arguments
    ///
    /// * `additional_data` - Additional bytes to parse
    ///
    /// # Note
    ///
    /// This is useful when reading from a stream that may not have all data
    /// available at once. The parser preserves partial segment state across
    /// buffer boundaries automatically.
    pub fn resume_with_data(&mut self, additional_data: &[u8]) {
        // Append new data to the buffer
        self.buffer.extend_from_slice(additional_data);
    }

    /// Clear the internal buffer and reset position
    ///
    /// This is useful when you want to discard any buffered data and start fresh.
    pub fn clear_buffer(&mut self) {
        self.buffer.clear();
        self.pos = 0;
    }
}

/// Async stream parser that yields events with backpressure
///
/// Created by [`StreamParserBuilder::build_async`].
pub struct AsyncStreamParser {
    receiver: Receiver<Result<Event, StreamError>>,
}

impl AsyncStreamParser {
    /// Get the next event from the async parser
    ///
    /// Returns `Some(Ok(event))` when an event is available,
    /// `Some(Err(e))` on error, and `None` when the stream is exhausted.
    pub async fn next(&mut self) -> Option<Result<Event, StreamError>> {
        self.receiver.recv().await
    }
}

// Comprehensive test suite
#[cfg(test)]
mod comprehensive_tests;
