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
            read_buf: [0u8; 8192],
            read_pos: 0,
            read_len: 0,
            buffer: Vec::new(),
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
                read_buf: [0u8; 8192],
                read_pos: 0,
                read_len: 0,
                buffer: Vec::new(),
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
    /// Internal buffer for reading from the underlying source
    read_buf: [u8; 8192],
    /// Current position in the read buffer
    read_pos: usize,
    /// Number of bytes currently in the read buffer
    read_len: usize,
    /// Buffer for accumulating the current segment or field data
    buffer: Vec<u8>,
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
    pub fn new(reader: D) -> Self {
        Self {
            reader,
            delims: Delims::default(),
            read_buf: [0u8; 8192],
            read_pos: 0,
            read_len: 0,
            buffer: Vec::new(),
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
            read_buf: [0u8; 8192],
            read_pos: 0,
            read_len: 0,
            buffer: Vec::new(),
            pre_msh: true,
            in_message: false,
            event_queue: VecDeque::new(),
            max_message_size,
            current_message_size: 0,
        }
    }

    /// Get the next event from the stream
    pub fn next_event(&mut self) -> Result<Option<Event>, Error> {
        // First check if we have any queued events
        if let Some(event) = self.event_queue.pop_front() {
            return Ok(Some(event));
        }

        loop {
            // Check if we have data in the read buffer
            if self.read_pos >= self.read_len {
                match self.reader.read(&mut self.read_buf) {
                    Ok(0) => {
                        // End of input
                        if !self.buffer.is_empty() {
                            let segment_data = std::mem::take(&mut self.buffer);
                            return self.process_segment(segment_data);
                        }
                        if self.in_message {
                            self.in_message = false;
                            self.pre_msh = true;
                            self.current_message_size = 0;
                            return Ok(Some(Event::EndMessage));
                        }
                        return Ok(None);
                    }
                    Ok(n) => {
                        self.read_len = n;
                        self.read_pos = 0;
                    }
                    Err(_) => return Err(Error::InvalidCharset),
                }
            }

            // Search for segment delimiter \r in the read buffer
            let remaining = &self.read_buf[self.read_pos..self.read_len];
            if let Some(rel_cr_pos) = remaining.iter().position(|&b| b == b'\r') {
                let abs_cr_pos = self.read_pos + rel_cr_pos;
                self.buffer.extend_from_slice(&self.read_buf[self.read_pos..abs_cr_pos]);
                self.read_pos = abs_cr_pos + 1; // Skip the \r

                let segment_data = std::mem::take(&mut self.buffer);
                let result = self.process_segment(segment_data)?;
                if result.is_some() {
                    return Ok(result);
                }
                // If process_segment returned None (e.g. non-segment data), continue searching
            } else {
                // No delimiter in current read_buf, append all to buffer and read more
                self.buffer.extend_from_slice(remaining);
                self.read_pos = self.read_len;
                
                // Safety check: if buffer is growing too large without a \r
                if self.buffer.len() > self.max_message_size {
                     return Err(Error::InvalidFieldFormat {
                        details: format!("Segment size exceeds maximum allowed size {}", self.max_message_size),
                    });
                }
            }
        }
    }

    /// Process a complete segment of data
    fn process_segment(&mut self, segment_data: Vec<u8>) -> Result<Option<Event>, Error> {
        let segment_len = segment_data.len() + 1; // Include the \r

        // Check memory bounds
        if self.in_message {
            self.current_message_size += segment_len;
            if self.current_message_size > self.max_message_size {
                let actual_size = self.current_message_size;
                let max_size = self.max_message_size;
                self.in_message = false;
                self.pre_msh = true;
                self.current_message_size = 0;
                return Err(Error::InvalidFieldFormat {
                    details: format!("Message size {} exceeds maximum {}", actual_size, max_size),
                });
            }
        }

        // Check if this is an MSH segment
        if segment_data.len() >= 3 && &segment_data[0..3] == b"MSH" {
            if self.in_message {
                // End previous message, save MSH to process next
                self.in_message = false;
                self.pre_msh = true;
                self.buffer = segment_data;
                self.current_message_size = 0;
                return Ok(Some(Event::EndMessage));
            }

            // Parse delimiters
            let new_delims = Delims::parse_from_msh(
                std::str::from_utf8(&segment_data).map_err(|_| Error::InvalidCharset)?,
            ).map_err(|e| Error::ParseError {
                segment_id: "MSH".to_string(),
                field_index: 0,
                source: Box::new(e),
            })?;

            self.delims = new_delims.clone();
            self.pre_msh = false;
            self.in_message = true;
            self.current_message_size = segment_len;

            self.generate_msh_field_events(&segment_data)?;
            return Ok(Some(Event::StartMessage { delims: new_delims }));
        }

        // Regular segment
        if self.in_message && segment_data.len() >= 3 && segment_data[0..3].iter().all(u8::is_ascii_alphanumeric) {
            let id = segment_data[0..3].to_vec();
            self.generate_field_events(&segment_data)?;
            return Ok(Some(Event::Segment { id }));
        }

        // Auto-start if pre-MSH and looks like a segment
        if !self.in_message && self.pre_msh && segment_data.len() >= 3 && segment_data[0..3].iter().all(u8::is_ascii_alphanumeric) {
            self.delims = Delims::default();
            self.pre_msh = false;
            self.in_message = true;
            self.current_message_size = segment_len;

            self.generate_field_events(&segment_data)?;
            return Ok(Some(Event::StartMessage { delims: Delims::default() }));
        }

        Ok(None)
    }

    fn generate_field_events(&mut self, segment_data: &[u8]) -> Result<(), Error> {
        if segment_data.len() > 4 {
            let fields_data = &segment_data[4..];
            let field_sep = self.delims.field as u8;
            for (index, field) in fields_data.split(|&b| b == field_sep).enumerate() {
                self.event_queue.push_back(Event::Field {
                    num: (index + 1) as u16,
                    raw: field.to_vec(),
                });
            }
        }
        Ok(())
    }

    fn generate_msh_field_events(&mut self, segment_data: &[u8]) -> Result<(), Error> {
        if segment_data.len() > 8 {
            let fields_data = &segment_data[8..];
            let field_sep = self.delims.field as u8;
            for (index, field) in fields_data.split(|&b| b == field_sep).enumerate() {
                self.event_queue.push_back(Event::Field {
                    num: (index + 1) as u16,
                    raw: field.to_vec(),
                });
            }
        }
        Ok(())
    }

    pub fn current_message_size(&self) -> usize { self.current_message_size }
    pub fn max_message_size(&self) -> usize { self.max_message_size }
    pub fn is_in_message(&self) -> bool { self.in_message }

    pub fn resume_with_data(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
    }

    pub fn clear_buffer(&mut self) {
        self.buffer.clear();
        self.read_pos = 0;
        self.read_len = 0;
    }
}

pub struct AsyncStreamParser {
    receiver: Receiver<Result<Event, StreamError>>,
}

impl AsyncStreamParser {
    pub async fn next(&mut self) -> Option<Result<Event, StreamError>> {
        self.receiver.recv().await
    }
}

#[cfg(test)]
mod comprehensive_tests;
