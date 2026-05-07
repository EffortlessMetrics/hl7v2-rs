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
//! use hl7v2::stream::{StreamParser, Event};
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
//! use hl7v2::stream::{AsyncStreamParser, StreamParserBuilder, Event};
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

// Re-export Delims for convenience.
pub use crate::model::Delims;

use crate::model::Error;
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
    StartMessage {
        /// Delimiters inferred for the message.
        delims: Delims,
    },
    /// A segment with its ID
    Segment {
        /// Raw three-byte segment identifier.
        id: Vec<u8>,
    },
    /// A field with its number (1-based) and raw content
    Field {
        /// 1-based field index.
        num: u16,
        /// Raw field bytes (without trailing delimiters).
        raw: Vec<u8>,
    },
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
        StreamError::ParseError(format!("{err:?}"))
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
                        if tx.send(Err(StreamError::from(e))).await.is_err() {
                            break;
                        }
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

    /// Get the next event from the stream.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when input cannot be read as HL7 text, when delimiter
    /// parsing fails, or when the configured message/segment size bounds are
    /// exceeded.
    pub fn next_event(&mut self) -> Result<Option<Event>, Error> {
        // First check if we have any queued events
        if let Some(event) = self.event_queue.pop_front() {
            return Ok(Some(event));
        }

        loop {
            // Check if we have a full segment in the buffer already (could happen via resume_with_data)
            if let Some(pos) = self.buffer.iter().position(|&b| b == b'\r') {
                let segment_data = self
                    .buffer
                    .get(..pos)
                    .ok_or_else(|| Error::InvalidFieldFormat {
                        details: "Internal segment buffer position was out of bounds".to_string(),
                    })?
                    .to_vec();
                self.buffer.drain(..pos.saturating_add(1));
                let result = self.process_segment(segment_data)?;
                if result.is_some() {
                    return Ok(result);
                }
                // If process_segment returned None, continue
                continue;
            }

            // Check if we have data in the read buffer
            if self.read_pos >= self.read_len {
                match self.reader.read(&mut self.read_buf) {
                    Ok(0) => {
                        // End of input
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
            let remaining = self
                .read_buf
                .get(self.read_pos..self.read_len)
                .ok_or_else(|| Error::InvalidFieldFormat {
                    details: "Internal read buffer position was out of bounds".to_string(),
                })?;
            if let Some(rel_cr_pos) = remaining.iter().position(|&b| b == b'\r') {
                let abs_cr_pos = self.read_pos.checked_add(rel_cr_pos).ok_or_else(|| {
                    Error::InvalidFieldFormat {
                        details: "Internal read buffer position overflowed".to_string(),
                    }
                })?;
                let segment_part =
                    self.read_buf
                        .get(self.read_pos..abs_cr_pos)
                        .ok_or_else(|| Error::InvalidFieldFormat {
                            details: "Internal read buffer segment was out of bounds".to_string(),
                        })?;
                self.buffer.extend_from_slice(segment_part);
                self.read_pos = abs_cr_pos.saturating_add(1); // Skip the \r

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
                        details: format!(
                            "Segment size exceeds maximum allowed size {}",
                            self.max_message_size
                        ),
                    });
                }
            }
        }
    }

    /// Process a complete segment of data
    fn process_segment(&mut self, segment_data: Vec<u8>) -> Result<Option<Event>, Error> {
        let segment_len = segment_data.len().saturating_add(1); // Include the \r

        // Check memory bounds
        if self.in_message {
            self.current_message_size = self.current_message_size.saturating_add(segment_len);
            if self.current_message_size > self.max_message_size {
                let actual_size = self.current_message_size;
                let max_size = self.max_message_size;
                self.in_message = false;
                self.pre_msh = true;
                self.current_message_size = 0;
                return Err(Error::InvalidFieldFormat {
                    details: format!("Message size {actual_size} exceeds maximum {max_size}"),
                });
            }
        }

        // Check if this is an MSH segment
        if segment_data.starts_with(b"MSH") {
            let mut end_prev = false;
            if self.in_message {
                end_prev = true;
            }

            // Parse delimiters
            let new_delims = Delims::parse_from_msh(
                std::str::from_utf8(&segment_data).map_err(|_utf8_err| Error::InvalidCharset)?,
            )
            .map_err(|e| Error::ParseError {
                segment_id: "MSH".to_string(),
                field_index: 0,
                source: Box::new(e),
            })?;

            self.delims = new_delims.clone();
            self.pre_msh = false;
            self.in_message = true;
            self.current_message_size = segment_len;

            let start_event = Event::StartMessage { delims: new_delims };
            if end_prev {
                self.event_queue.push_back(start_event);
                self.generate_msh_field_events(&segment_data)?;
                return Ok(Some(Event::EndMessage));
            } else {
                self.generate_msh_field_events(&segment_data)?;
                return Ok(Some(start_event));
            }
        }

        // Regular segment
        if self.in_message
            && segment_data
                .get(..3)
                .is_some_and(|id| id.iter().all(u8::is_ascii_alphanumeric))
        {
            let Some(id) = segment_data.get(..3).map(<[u8]>::to_vec) else {
                return Err(Error::InvalidSegmentId);
            };
            self.generate_field_events(&segment_data)?;
            return Ok(Some(Event::Segment { id }));
        }

        // Auto-start if pre-MSH and looks like a segment
        if !self.in_message
            && self.pre_msh
            && segment_data
                .get(..3)
                .is_some_and(|id| id.iter().all(u8::is_ascii_alphanumeric))
        {
            self.delims = Delims::default();
            self.pre_msh = false;
            self.in_message = true;
            self.current_message_size = segment_len;

            self.generate_field_events(&segment_data)?;
            return Ok(Some(Event::StartMessage {
                delims: Delims::default(),
            }));
        }

        Ok(None)
    }

    fn generate_field_events(&mut self, segment_data: &[u8]) -> Result<(), Error> {
        if segment_data.len() > 4 {
            let Some(fields_data) = segment_data.get(4..) else {
                return Err(Error::InvalidFieldFormat {
                    details: "Segment field data was out of bounds".to_string(),
                });
            };
            let field_sep = self.delims.field as u8;
            for (index, field) in fields_data.split(|&b| b == field_sep).enumerate() {
                self.event_queue.push_back(Event::Field {
                    num: field_number(index)?,
                    raw: field.to_vec(),
                });
            }
        }
        Ok(())
    }

    fn generate_msh_field_events(&mut self, segment_data: &[u8]) -> Result<(), Error> {
        if segment_data.len() > 8 {
            let Some(fields_data) = segment_data.get(8..) else {
                return Err(Error::InvalidFieldFormat {
                    details: "MSH field data was out of bounds".to_string(),
                });
            };
            let field_sep = self.delims.field as u8;
            for (index, field) in fields_data.split(|&b| b == field_sep).enumerate() {
                self.event_queue.push_back(Event::Field {
                    num: field_number(index)?,
                    raw: field.to_vec(),
                });
            }
        }
        Ok(())
    }

    /// Current message size in bytes accumulated so far.
    pub fn current_message_size(&self) -> usize {
        self.current_message_size
    }

    /// Maximum allowed message size in bytes.
    pub fn max_message_size(&self) -> usize {
        self.max_message_size
    }

    /// Returns `true` while a message is currently being parsed.
    pub fn is_in_message(&self) -> bool {
        self.in_message
    }

    /// Push raw input bytes into the internal buffer.
    pub fn resume_with_data(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
    }

    /// Reset parser buffers and internal positions.
    pub fn clear_buffer(&mut self) {
        self.buffer.clear();
        self.read_pos = 0;
        self.read_len = 0;
    }
}

fn field_number(index: usize) -> Result<u16, Error> {
    let one_based = index
        .checked_add(1)
        .ok_or_else(|| Error::InvalidFieldFormat {
            details: "Field index overflowed".to_string(),
        })?;
    u16::try_from(one_based).map_err(|_int_err| Error::InvalidFieldFormat {
        details: format!("Field index {one_based} exceeds u16"),
    })
}

/// Async wrapper around a bounded parser event stream.
pub struct AsyncStreamParser {
    receiver: Receiver<Result<Event, StreamError>>,
}

impl AsyncStreamParser {
    /// Receive the next parsed event, if available.
    pub async fn next(&mut self) -> Option<Result<Event, StreamError>> {
        self.receiver.recv().await
    }
}
