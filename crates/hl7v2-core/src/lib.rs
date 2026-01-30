//! Core parsing and data model for HL7 v2 messages.
//!
//! This crate provides the foundational data structures and parsing logic
//! for HL7 v2 messages, including:
//! - Message parsing from raw bytes
//! - Data model representation (Message, Segment, Field, etc.)
//! - Escape sequence handling
//! - JSON serialization
//! - Batch message handling (FHS/BHS/BTS/FTS)

#[cfg(feature = "network")]
pub mod network;

/// Error type for HL7 v2 operations
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid segment ID")]
    InvalidSegmentId,
    
    #[error("Bad delimiter length")]
    BadDelimLength,
    
    #[error("Duplicate delimiters")]
    DuplicateDelims,
    
    #[error("Unbalanced escape")]
    UnbalancedEscape,
    
    #[error("Invalid escape token")]
    InvalidEscapeToken,
    
    #[error("MSH field malformed")]
    MshFieldMalformed,
    
    #[error("MSH-10 missing")]
    Msh10Missing,
    
    #[error("Invalid processing ID")]
    InvalidProcessingId,
    
    #[error("Unrecognized version")]
    UnrecognizedVersion,
    
    #[error("Invalid charset")]
    InvalidCharset,
    
    #[error("Write failed")]
    WriteFailed,
    
    // New comprehensive error types
    #[error("Parse error at segment {segment_id} field {field_index}: {source}")]
    ParseError {
        segment_id: String,
        field_index: usize,
        #[source]
        source: Box<Error>,
    },
    
    #[error("Invalid field format: {details}")]
    InvalidFieldFormat {
        details: String,
    },
    
    #[error("Invalid repetition format: {details}")]
    InvalidRepFormat {
        details: String,
    },
    
    #[error("Invalid component format: {details}")]
    InvalidCompFormat {
        details: String,
    },
    
    #[error("Invalid subcomponent format: {details}")]
    InvalidSubcompFormat {
        details: String,
    },
    
    #[error("Batch parsing error: {details}")]
    BatchParseError {
        details: String,
    },
    
    #[error("Invalid batch header: {details}")]
    InvalidBatchHeader {
        details: String,
    },
    
    #[error("Invalid batch trailer: {details}")]
    InvalidBatchTrailer {
        details: String,
    },
}

/// Delimiters used in HL7 v2 messages
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delims {
    pub field: char,
    pub comp: char,
    pub rep: char,
    pub esc: char,
    pub sub: char,
}

impl Default for Delims {
    /// Create default delimiters (|^~\&)
    fn default() -> Self {
        Self {
            field: '|',
            comp: '^',
            rep: '~',
            esc: '\\',
            sub: '&',
        }
    }
}

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
                    let new_delims = parse_delimiters(std::str::from_utf8(&segment_data).map_err(|_| Error::InvalidCharset)?)
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

/// Main message structure
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub delims: Delims,
    pub segments: Vec<Segment>,
    /// Character sets used in the message (from MSH-18)
    pub charsets: Vec<String>,
}

/// A batch of HL7 messages
#[derive(Debug, Clone, PartialEq)]
pub struct Batch {
    pub header: Option<Segment>, // BHS segment
    pub messages: Vec<Message>,
    pub trailer: Option<Segment>, // BTS segment
}

/// A file containing batches of HL7 messages
#[derive(Debug, Clone, PartialEq)]
pub struct FileBatch {
    pub header: Option<Segment>, // FHS segment
    pub batches: Vec<Batch>,
    pub trailer: Option<Segment>, // FTS segment
}

/// A segment in an HL7 message
#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    pub id: [u8; 3],
    pub fields: Vec<Field>,
}

/// A field in a segment
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub reps: Vec<Rep>,
}

/// A repetition of a field
#[derive(Debug, Clone, PartialEq)]
pub struct Rep {
    pub comps: Vec<Comp>,
}

/// A component of a field
#[derive(Debug, Clone, PartialEq)]
pub struct Comp {
    pub subs: Vec<Atom>,
}

/// An atomic value in the message
#[derive(Debug, Clone, PartialEq)]
pub enum Atom {
    Text(String),
    Null,
}

/// Presence semantics for HL7 v2 fields
#[derive(Debug, Clone, PartialEq)]
pub enum Presence {
    /// Field is not present in the message (index out of range)
    Missing,
    /// Field is present but empty (zero-length)
    Empty,
    /// Field contains a literal NULL value ("")
    Null,
    /// Field contains a value
    Value(String),
}

/// Parse HL7 v2 message from bytes
pub fn parse(bytes: &[u8]) -> Result<Message, Error> {
    // Convert bytes to string
    let text = std::str::from_utf8(bytes).map_err(|_| Error::InvalidCharset)?;
    
    // Split into lines (segments)
    let lines: Vec<&str> = text.split('\r').filter(|line| !line.is_empty()).collect();
    
    if lines.is_empty() {
        return Err(Error::InvalidSegmentId);
    }
    
    // First segment must be MSH
    if !lines[0].starts_with("MSH") {
        return Err(Error::InvalidSegmentId);
    }
    
    // Parse delimiters from MSH segment
    let delims = parse_delimiters(lines[0]).map_err(|e| Error::ParseError {
        segment_id: "MSH".to_string(),
        field_index: 0,
        source: Box::new(e),
    })?;
    
    // Parse all segments
    let mut segments = Vec::new();
    for line in lines {
        let segment = parse_segment(line, &delims).map_err(|e| Error::ParseError {
            segment_id: if line.len() >= 3 { line[..3].to_string() } else { line.to_string() },
            field_index: 0,
            source: Box::new(e),
        })?;
        segments.push(segment);
    }
    
    // Extract charset information from MSH-18 if present
    let charsets = extract_charsets(&segments);
    
    Ok(Message { delims, segments, charsets })
}

/// Extract character sets from MSH-18 field
fn extract_charsets(segments: &[Segment]) -> Vec<String> {
    // Look for the MSH segment (should be the first one)
    if let Some(msh_segment) = segments.first() {
        // Check if this is an MSH segment
        if &msh_segment.id == b"MSH" {
            // MSH-18 is field index 17 (1-based indexing)
            // In parsed fields, this would be index 17 (0-based indexing)
            // But we need to account for the special MSH handling:
            // - MSH-1 (field separator) is not a parsed field
            // - MSH-2 (encoding characters) is parsed field 0
            // - MSH-3 is parsed field 1
            // - ...
            // - MSH-18 is parsed field 17
            
            // So we need at least 18 parsed fields (indices 0-17)
            if msh_segment.fields.len() > 17 {
                let field_18 = &msh_segment.fields[17];
                
                // Get the first repetition
                if !field_18.reps.is_empty() {
                    let rep = &field_18.reps[0];
                    
                    // For MSH-18, we collect all components and filter out empty ones
                    let mut charsets = Vec::new();
                    for comp in &rep.comps {
                        if !comp.subs.is_empty() {
                            match &comp.subs[0] {
                                Atom::Text(text) => {
                                    if !text.is_empty() {
                                        charsets.push(text.clone());
                                    }
                                },
                                Atom::Null => continue, // Skip NULL values
                            }
                        }
                    }
                    
                    return charsets;
                }
            }
        }
    }
    vec![]
}

/// Parse HL7 v2 message from MLLP framed bytes
pub fn parse_mllp(bytes: &[u8]) -> Result<Message, Error> {
    // Check if this is MLLP framed (starts with 0x0B)
    if bytes.is_empty() || bytes[0] != 0x0B {
        return Err(Error::InvalidCharset);
    }
    
    // Find the end sequence (0x1C 0x0D)
    let end_pos = bytes.windows(2).position(|window| window[0] == 0x1C && window[1] == 0x0D);
    
    if let Some(end_pos) = end_pos {
        // Extract the HL7 message content (excluding framing bytes)
        let hl7_content = &bytes[1..end_pos];
        
        // Parse the HL7 message
        parse(hl7_content)
    } else {
        Err(Error::InvalidCharset)
    }
}

/// Parse HL7 v2 batch from bytes
pub fn parse_batch(bytes: &[u8]) -> Result<Batch, Error> {
    // Convert bytes to string
    let text = std::str::from_utf8(bytes).map_err(|_| Error::InvalidCharset)?;
    
    // Split into lines (segments)
    let lines: Vec<&str> = text.split('\r').filter(|line| !line.is_empty()).collect();
    
    if lines.is_empty() {
        return Err(Error::InvalidSegmentId);
    }
    
    // Check if this is a batch (starts with BHS) or regular message (starts with MSH)
    let first_line = lines[0];
    if first_line.starts_with("BHS") {
        parse_batch_with_header(&lines)
    } else if first_line.starts_with("MSH") {
        // This is a single message, wrap it in a batch
        let message = parse(bytes)?;
        Ok(Batch {
            header: None,
            messages: vec![message],
            trailer: None,
        })
    } else {
        Err(Error::InvalidSegmentId)
    }
}

/// Parse HL7 v2 file batch from bytes
pub fn parse_file_batch(bytes: &[u8]) -> Result<FileBatch, Error> {
    // Convert bytes to string
    let text = std::str::from_utf8(bytes).map_err(|_| Error::InvalidCharset)?;
    
    // Split into lines (segments)
    let lines: Vec<&str> = text.split('\r').filter(|line| !line.is_empty()).collect();
    
    if lines.is_empty() {
        return Err(Error::InvalidSegmentId);
    }
    
    // Check if this is a file batch (starts with FHS)
    let first_line = lines[0];
    if first_line.starts_with("FHS") {
        parse_file_batch_with_header(&lines)
    } else if first_line.starts_with("BHS") || first_line.starts_with("MSH") {
        // This is a batch or single message, wrap it in a file batch
        let batch_data = parse_batch(bytes)?;
        Ok(FileBatch {
            header: None,
            batches: vec![batch_data],
            trailer: None,
        })
    } else {
        Err(Error::InvalidSegmentId)
    }
}

/// Parse a batch that starts with BHS
fn parse_batch_with_header(lines: &[&str]) -> Result<Batch, Error> {
    // First line should be BHS
    if !lines[0].starts_with("BHS") {
        return Err(Error::InvalidBatchHeader {
            details: "Batch must start with BHS segment".to_string(),
        });
    }
    
    // Parse delimiters from the first MSH segment we find
    let delims = find_and_parse_delimiters(lines).map_err(|e| Error::BatchParseError {
        details: format!("Failed to parse delimiters: {}", e),
    })?;
    
    let mut header = None;
    let mut messages = Vec::new();
    let mut trailer = None;
    let mut current_message_lines = Vec::new();
    
    for &line in lines {
        if line.starts_with("BHS") {
            // Parse BHS segment
            let bhs_segment = parse_segment(line, &delims).map_err(|e| Error::InvalidBatchHeader {
                details: format!("Failed to parse BHS segment: {}", e),
            })?;
            header = Some(bhs_segment);
        } else if line.starts_with("BTS") {
            // Parse BTS segment
            let bts_segment = parse_segment(line, &delims).map_err(|e| Error::InvalidBatchTrailer {
                details: format!("Failed to parse BTS segment: {}", e),
            })?;
            trailer = Some(bts_segment);
        } else if line.starts_with("MSH") {
            // Start of a new message
            if !current_message_lines.is_empty() {
                // Parse the previous message
                let message_text = current_message_lines.to_vec().join("\r");
                let message = parse(message_text.as_bytes()).map_err(|e| Error::BatchParseError {
                    details: format!("Failed to parse message in batch: {}", e),
                })?;
                messages.push(message);
                current_message_lines.clear();
            }
            current_message_lines.push(line);
        } else {
            // Part of current message
            current_message_lines.push(line);
        }
    }
    
    // Parse the last message
    if !current_message_lines.is_empty() {
        let message_text = current_message_lines.to_vec().join("\r");
        let message = parse(message_text.as_bytes()).map_err(|e| Error::BatchParseError {
            details: format!("Failed to parse final message in batch: {}", e),
        })?;
        messages.push(message);
    }
    
    Ok(Batch {
        header,
        messages,
        trailer,
    })
}

/// Parse a file batch that starts with FHS
fn parse_file_batch_with_header(lines: &[&str]) -> Result<FileBatch, Error> {
    // First line should be FHS
    if !lines[0].starts_with("FHS") {
        return Err(Error::InvalidBatchHeader {
            details: "File batch must start with FHS segment".to_string(),
        });
    }
    
    // Parse delimiters from the first MSH segment we find
    let delims = find_and_parse_delimiters(lines).map_err(|e| Error::BatchParseError {
        details: format!("Failed to parse delimiters: {}", e),
    })?;
    
    let mut header = None;
    let mut batches = Vec::new();
    let mut trailer = None;
    let mut current_batch_lines = Vec::new();
    
    for &line in lines {
        if line.starts_with("FHS") {
            // Parse FHS segment
            let fhs_segment = parse_segment(line, &delims).map_err(|e| Error::InvalidBatchHeader {
                details: format!("Failed to parse FHS segment: {}", e),
            })?;
            header = Some(fhs_segment);
        } else if line.starts_with("FTS") {
            // Parse FTS segment
            let fts_segment = parse_segment(line, &delims).map_err(|e| Error::InvalidBatchTrailer {
                details: format!("Failed to parse FTS segment: {}", e),
            })?;
            trailer = Some(fts_segment);
        } else if line.starts_with("BHS") {
            // Start of a new batch
            if !current_batch_lines.is_empty() {
                // Parse the previous batch
                let batch_text = current_batch_lines.to_vec().join("\r");
                match parse_batch(batch_text.as_bytes()) {
                    Ok(batch) => batches.push(batch),
                    Err(e) => {
                        // If parsing as batch fails, try as single message
                        let message = parse(batch_text.as_bytes()).map_err(|_| e)?;
                        batches.push(Batch {
                            header: None,
                            messages: vec![message],
                            trailer: None,
                        });
                    }
                }
                current_batch_lines.clear();
            }
            current_batch_lines.push(line);
        } else if line.starts_with("MSH") && current_batch_lines.is_empty() {
            // Start of a message when no batch has started
            current_batch_lines.push(line);
        } else {
            // Part of current batch
            current_batch_lines.push(line);
        }
    }
    
    // Parse the last batch
    if !current_batch_lines.is_empty() {
        let batch_text = current_batch_lines.to_vec().join("\r");
        match parse_batch(batch_text.as_bytes()) {
            Ok(batch) => batches.push(batch),
            Err(e) => {
                // If parsing as batch fails, try as single message
                let message = parse(batch_text.as_bytes()).map_err(|_| e)?;
                batches.push(Batch {
                    header: None,
                    messages: vec![message],
                    trailer: None,
                });
            }
        }
    }
    
    Ok(FileBatch {
        header,
        batches,
        trailer,
    })
}

/// Find and parse delimiters from the first MSH segment in the lines
fn find_and_parse_delimiters(lines: &[&str]) -> Result<Delims, Error> {
    for line in lines {
        if line.starts_with("MSH") {
            return parse_delimiters(line);
        }
    }
    // If no MSH segment found, use default delimiters
    Ok(Delims::default())
}

/// Parse delimiters from MSH segment
fn parse_delimiters(msh: &str) -> Result<Delims, Error> {
    if msh.len() < 8 {
        return Err(Error::BadDelimLength);
    }
    
    // Extract the encoding characters directly without parsing them as regular fields
    // MSH has a special format: MSH|^~\&|... where ^~\& are the encoding characters
    let field_sep = msh.chars().nth(3).ok_or(Error::BadDelimLength)?;
    let comp_char = msh.chars().nth(4).ok_or(Error::BadDelimLength)?;
    let rep_char = msh.chars().nth(5).ok_or(Error::BadDelimLength)?;
    let esc_char = msh.chars().nth(6).ok_or(Error::BadDelimLength)?;
    let sub_char = msh.chars().nth(7).ok_or(Error::BadDelimLength)?;
    
    // Check that all delimiters are distinct
    let delimiters = [field_sep, comp_char, rep_char, esc_char, sub_char];
    for i in 0..delimiters.len() {
        for j in (i + 1)..delimiters.len() {
            if delimiters[i] == delimiters[j] {
                return Err(Error::DuplicateDelims);
            }
        }
    }
    
    Ok(Delims {
        field: field_sep,
        comp: comp_char,
        rep: rep_char,
        esc: esc_char,
        sub: sub_char,
    })
}

/// Parse a single segment
fn parse_segment(line: &str, delims: &Delims) -> Result<Segment, Error> {
    if line.len() < 3 {
        return Err(Error::InvalidSegmentId);
    }
    
    // Parse segment ID
    let id_bytes = &line.as_bytes()[0..3];
    let mut id = [0u8; 3];
    id.copy_from_slice(id_bytes);
    
    // Ensure segment ID is all uppercase ASCII letters or digits
    for &byte in &id {
        if !(byte.is_ascii_uppercase() || byte.is_ascii_digit()) {
            return Err(Error::InvalidSegmentId);
        }
    }
    
    // Parse fields
    let fields_str = if line.len() > 4 {
        &line[4..] // Skip segment ID and field separator
    } else {
        ""
    };
    
    let mut fields = parse_fields(fields_str, delims).map_err(|e| Error::ParseError {
        segment_id: String::from_utf8_lossy(&id).to_string(),
        field_index: 0,
        source: Box::new(e),
    })?;
    
    // Special handling for MSH segment
    if &id == b"MSH" {
        // MSH-2 (the encoding characters) should be treated as a single atomic value
        // Currently it's being parsed incorrectly, so we need to fix it
        if !fields.is_empty() {
            // Create a field with the encoding characters as a single atomic value
            // Use direct string construction instead of format! to avoid allocation
            let encoding_chars = String::from_iter([
                delims.comp, delims.rep, delims.esc, delims.sub
            ]);
            
            let encoding_field = Field {
                reps: vec![Rep {
                    comps: vec![Comp {
                        subs: vec![Atom::Text(encoding_chars)],
                    }],
                }],
            };
            // Replace the first field with the corrected encoding field
            fields[0] = encoding_field;
        }
        Ok(Segment {
            id,
            fields,
        })
    } else {
        Ok(Segment {
            id,
            fields,
        })
    }
}

/// Parse fields from a segment
fn parse_fields(fields_str: &str, delims: &Delims) -> Result<Vec<Field>, Error> {
    if fields_str.is_empty() {
        return Ok(vec![]);
    }
    
    // Count fields first to pre-allocate the vector
    let field_count = fields_str.matches(delims.field).count() + 1;
    let mut fields = Vec::with_capacity(field_count);
    
    // Use split iterator directly instead of collecting into intermediate vector
    for (i, field_str) in fields_str.split(delims.field).enumerate() {
        let field = parse_field(field_str, delims).map_err(|e| Error::ParseError {
            segment_id: "UNKNOWN".to_string(), // This will be filled in by the caller
            field_index: i,
            source: Box::new(e),
        })?;
        fields.push(field);
    }
    
    Ok(fields)
}

/// Parse a single field
fn parse_field(field_str: &str, delims: &Delims) -> Result<Field, Error> {
    // Validate field format
    if field_str.contains('\n') || field_str.contains('\r') {
        return Err(Error::InvalidFieldFormat {
            details: "Field contains invalid line break characters".to_string(),
        });
    }
    
    // Count repetitions first to pre-allocate the vector
    let rep_count = field_str.matches(delims.rep).count() + 1;
    let mut reps = Vec::with_capacity(rep_count);
    
    // Use split iterator directly instead of collecting into intermediate vector
    for (i, rep_str) in field_str.split(delims.rep).enumerate() {
        let rep = parse_rep(rep_str, delims).map_err(|e| {
            match e {
                Error::InvalidRepFormat { .. } => e,
                _ => Error::InvalidRepFormat {
                    details: format!("Repetition {}: {}", i, e),
                }
            }
        })?;
        reps.push(rep);
    }
    
    Ok(Field { reps })
}

/// Parse a repetition
fn parse_rep(rep_str: &str, delims: &Delims) -> Result<Rep, Error> {
    // Handle NULL value
    if rep_str == "\"\"" {
        return Ok(Rep {
            comps: vec![Comp {
                subs: vec![Atom::Null],
            }],
        });
    }
    
    // Validate repetition format
    if rep_str.contains('\n') || rep_str.contains('\r') {
        return Err(Error::InvalidRepFormat {
            details: "Repetition contains invalid line break characters".to_string(),
        });
    }
    
    // Count components first to pre-allocate the vector
    let comp_count = rep_str.matches(delims.comp).count() + 1;
    let mut comps = Vec::with_capacity(comp_count);
    
    // Use split iterator directly instead of collecting into intermediate vector
    for (i, comp_str) in rep_str.split(delims.comp).enumerate() {
        let comp = parse_comp(comp_str, delims).map_err(|e| {
            match e {
                Error::InvalidCompFormat { .. } => e,
                _ => Error::InvalidCompFormat {
                    details: format!("Component {}: {}", i, e),
                }
            }
        })?;
        comps.push(comp);
    }
    
    Ok(Rep { comps })
}

/// Parse a component
fn parse_comp(comp_str: &str, delims: &Delims) -> Result<Comp, Error> {
    // Validate component format
    if comp_str.contains('\n') || comp_str.contains('\r') {
        return Err(Error::InvalidCompFormat {
            details: "Component contains invalid line break characters".to_string(),
        });
    }
    
    // Count subcomponents first to pre-allocate the vector
    let sub_count = comp_str.matches(delims.sub).count() + 1;
    let mut subs = Vec::with_capacity(sub_count);
    
    // Use split iterator directly instead of collecting into intermediate vector
    for (i, sub_str) in comp_str.split(delims.sub).enumerate() {
        let atom = parse_atom(sub_str, delims).map_err(|e| {
            match e {
                Error::InvalidSubcompFormat { .. } => e,
                _ => Error::InvalidSubcompFormat {
                    details: format!("Subcomponent {}: {}", i, e),
                }
            }
        })?;
        subs.push(atom);
    }
    
    Ok(Comp { subs })
}

/// Parse an atom (unescaped text or NULL)
fn parse_atom(atom_str: &str, delims: &Delims) -> Result<Atom, Error> {
    // Handle NULL value
    if atom_str == "\"\"" {
        return Ok(Atom::Null);
    }
    
    // Validate atom format
    if atom_str.contains('\n') || atom_str.contains('\r') {
        return Err(Error::InvalidSubcompFormat {
            details: "Subcomponent contains invalid line break characters".to_string(),
        });
    }
    
    // Unescape the text
    let unescaped = unescape_text(atom_str, delims)?;
    Ok(Atom::Text(unescaped))
}

/// Unescape text according to HL7 v2 rules
pub fn unescape_text(text: &str, delims: &Delims) -> Result<String, Error> {
    // Fast path: if no escape character is present, return the text as is
    if !text.contains(delims.esc) {
        return Ok(text.to_string());
    }

    // Pre-allocate result with estimated capacity to reduce reallocations
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    
    while let Some(ch) = chars.next() {
        if ch == delims.esc {
            // Start of escape sequence
            let mut escape_seq = String::new();
            let mut found_end = false;
            
            for esc_ch in chars.by_ref() {
                if esc_ch == delims.esc {
                    found_end = true;
                    break;
                }
                escape_seq.push(esc_ch);
            }
            
            if !found_end {
                // If we don't find the closing escape character, this might be a literal backslash
                // in the encoding characters. Let's check if this is the special case of the
                // MSH encoding characters "^~\&"
                // Use direct comparison instead of format! to avoid allocation
                if text.len() == 4 && 
                   text.starts_with(delims.comp) &&
                   text.chars().nth(1) == Some(delims.rep) &&
                   text.chars().nth(2) == Some(delims.esc) &&
                   text.chars().nth(3) == Some(delims.sub) {
                    // This is the MSH encoding characters, treat as literal
                    result.push(delims.comp);
                    result.push(delims.rep);
                    result.push(delims.esc);
                    result.push(delims.sub);
                    // Skip the rest of the processing since we've handled the special case
                    return Ok(result);
                }
                
                // For other cases, treat the text as-is
                result.push(delims.esc);
                result.push_str(&escape_seq);
                continue;
            }
            
            // Process escape sequence
            match escape_seq.as_str() {
                "F" => {
                    result.push(delims.field);
                },
                "S" => {
                    result.push(delims.comp);
                },
                "R" => {
                    result.push(delims.rep);
                },
                "E" => {
                    result.push(delims.esc);
                },
                "T" => {
                    result.push(delims.sub);
                },
                _ => {
                    // Unknown escape sequences are passed through
                    result.push(delims.esc);
                    result.push_str(&escape_seq);
                    result.push(delims.esc);
                }
            }
        } else {
            result.push(ch);
        }
    }
    
    Ok(result)
}

/// Write HL7 message to bytes
pub fn write(msg: &Message) -> Vec<u8> {
    let mut buf = Vec::new();
    
    // Write segments
    for segment in &msg.segments {
        // Write segment ID
        buf.extend_from_slice(&segment.id);
        
        // Special handling for MSH segment
        if &segment.id == b"MSH" {
            // Write field separator
            buf.push(msg.delims.field as u8);
            
            // Write encoding characters as a single field
            buf.push(msg.delims.comp as u8);
            buf.push(msg.delims.rep as u8);
            buf.push(msg.delims.esc as u8);
            buf.push(msg.delims.sub as u8);
            
            // Write the rest of the fields
            for field in &segment.fields[1..] { // Skip the encoding characters field
                buf.push(msg.delims.field as u8);
                write_field(&mut buf, field, &msg.delims);
            }
        } else {
            // Write fields
            for field in &segment.fields {
                buf.push(msg.delims.field as u8);
                write_field(&mut buf, field, &msg.delims);
            }
        }
        
        // End segment with carriage return
        buf.push(b'\r');
    }
    
    buf
}

/// Wrap HL7 message bytes with MLLP framing
pub fn wrap_mllp(bytes: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(bytes.len() + 3);
    
    // Add MLLP start byte (0x0B)
    buf.push(0x0B);
    
    // Add HL7 message content
    buf.extend_from_slice(bytes);
    
    // Add MLLP end sequence (0x1C 0x0D)
    buf.push(0x1C);
    buf.push(0x0D);
    
    buf
}

/// Write HL7 message with MLLP framing
pub fn write_mllp(msg: &Message) -> Vec<u8> {
    let hl7_bytes = write(msg);
    wrap_mllp(&hl7_bytes)
}

/// Write a field to bytes (with escaping)
fn write_field(output: &mut Vec<u8>, field: &Field, delims: &Delims) {
    for (i, rep) in field.reps.iter().enumerate() {
        if i > 0 {
            output.push(delims.rep as u8);
        }
        write_rep(output, rep, delims);
    }
}

/// Write a repetition to bytes (with escaping)
fn write_rep(output: &mut Vec<u8>, rep: &Rep, delims: &Delims) {
    for (i, comp) in rep.comps.iter().enumerate() {
        if i > 0 {
            output.push(delims.comp as u8);
        }
        write_comp(output, comp, delims);
    }
}

/// Write a component to bytes (with escaping)
fn write_comp(output: &mut Vec<u8>, comp: &Comp, delims: &Delims) {
    for (i, atom) in comp.subs.iter().enumerate() {
        if i > 0 {
            output.push(delims.sub as u8);
        }
        write_atom(output, atom, delims);
    }
}

/// Write an atom to bytes (with escaping)
fn write_atom(output: &mut Vec<u8>, atom: &Atom, delims: &Delims) {
    match atom {
        Atom::Text(text) => {
            // Escape special characters
            let escaped = escape_text(text, delims);
            output.extend_from_slice(escaped.as_bytes());
        }
        Atom::Null => {
            output.extend_from_slice(b"\"\"");
        }
    }
}

/// Escape text according to HL7 v2 rules
pub fn escape_text(text: &str, delims: &Delims) -> String {
    // Fast path: if no special characters are present, return the text as is
    let special_chars = [delims.field, delims.comp, delims.rep, delims.esc, delims.sub];
    if !text.contains(&special_chars[..]) {
        return text.to_string();
    }

    // Pre-calculate maximum possible size to reduce reallocations
    // In worst case, every character might need escaping (3 chars each)
    let max_size = text.len() * 3;
    let mut result = String::with_capacity(max_size);
    
    for ch in text.chars() {
        match ch {
            c if c == delims.field => {
                result.push(delims.esc);
                result.push('F');
                result.push(delims.esc);
            }
            c if c == delims.comp => {
                result.push(delims.esc);
                result.push('S');
                result.push(delims.esc);
            }
            c if c == delims.rep => {
                result.push(delims.esc);
                result.push('R');
                result.push(delims.esc);
            }
            c if c == delims.esc => {
                result.push(delims.esc);
                result.push('E');
                result.push(delims.esc);
            }
            c if c == delims.sub => {
                result.push(delims.esc);
                result.push('T');
                result.push(delims.esc);
            }
            _ => result.push(ch),
        }
    }
    
    result
}

/// Normalize HL7 v2 message
/// 
/// This function parses and rewrites an HL7 message, optionally converting
/// it to canonical delimiters (|^~\&).
pub fn normalize(bytes: &[u8], canonical_delims: bool) -> Result<Vec<u8>, Error> {
    // Parse the message
    let mut message = parse(bytes)?;
    
    // If canonical delimiters are requested, update the message delimiters
    if canonical_delims {
        message.delims = Delims::default();
    }
    
    // Write the normalized message
    Ok(write(&message))
}

/// Normalize HL7 v2 batch
/// 
/// This function parses and rewrites an HL7 batch message, optionally converting
/// it to canonical delimiters (|^~\&).
pub fn normalize_batch(bytes: &[u8], canonical_delims: bool) -> Result<Vec<u8>, Error> {
    // Parse the batch
    let mut batch = parse_batch(bytes)?;
    
    // If canonical delimiters are requested, update all message delimiters
    if canonical_delims {
        let canonical = Delims::default();
        for message in &mut batch.messages {
            message.delims = canonical.clone();
        }
    }
    
    // Write the normalized batch
    Ok(write_batch(&batch))
}

/// Normalize HL7 v2 file batch
/// 
/// This function parses and rewrites an HL7 file batch message, optionally converting
/// it to canonical delimiters (|^~\&).
pub fn normalize_file_batch(bytes: &[u8], canonical_delims: bool) -> Result<Vec<u8>, Error> {
    // Parse the file batch
    let mut file_batch = parse_file_batch(bytes)?;
    
    // If canonical delimiters are requested, update all message delimiters
    if canonical_delims {
        let canonical = Delims::default();
        for batch in &mut file_batch.batches {
            for message in &mut batch.messages {
                message.delims = canonical.clone();
            }
        }
    }
    
    // Write the normalized file batch
    Ok(write_file_batch(&file_batch))
}

/// Write batch back to HL7 v2 format
pub fn write_batch(batch: &Batch) -> Vec<u8> {
    let mut result = Vec::new();
    
    // Write BHS if present
    if let Some(header) = &batch.header {
        result.extend_from_slice(&header.id);
        // We need to get delimiters from the first message or use defaults
        let delims = if let Some(first_msg) = batch.messages.first() {
            &first_msg.delims
        } else {
            &Delims::default()
        };
        result.push(delims.field as u8);
        write_segment_fields(header, &mut result, delims);
        result.push(b'\r');
    }
    
    // Write all messages
    for message in &batch.messages {
        result.extend(write(message));
    }
    
    // Write BTS if present
    if let Some(trailer) = &batch.trailer {
        result.extend_from_slice(&trailer.id);
        let delims = if let Some(first_msg) = batch.messages.first() {
            &first_msg.delims
        } else {
            &Delims::default()
        };
        result.push(delims.field as u8);
        write_segment_fields(trailer, &mut result, delims);
        result.push(b'\r');
    }
    
    result
}

/// Write file batch back to HL7 v2 format
pub fn write_file_batch(file_batch: &FileBatch) -> Vec<u8> {
    let mut result = Vec::new();
    
    // Write FHS if present
    if let Some(header) = &file_batch.header {
        result.extend_from_slice(&header.id);
        // We need to get delimiters from the first message or use defaults
        let delims = get_delimiters_from_file_batch(file_batch);
        result.push(delims.field as u8);
        write_segment_fields(header, &mut result, &delims);
        result.push(b'\r');
    }
    
    // Write all batches
    for batch in &file_batch.batches {
        result.extend(write_batch(batch));
    }
    
    // Write FTS if present
    if let Some(trailer) = &file_batch.trailer {
        result.extend_from_slice(&trailer.id);
        let delims = get_delimiters_from_file_batch(file_batch);
        result.push(delims.field as u8);
        write_segment_fields(trailer, &mut result, &delims);
        result.push(b'\r');
    }
    
    result
}

/// Get value at path (e.g., "PID.5[1].1")
pub fn get<'a>(msg: &'a Message, path: &str) -> Option<&'a str> {
    // Parse the path
    // Format: SEGMENT.FIELD[REP].COMPONENT
    // Examples: "PID.5.1", "PID.5[1].1", "MSH.9"
    
    let mut parts = path.split('.');
    let segment_id = parts.next()?;
    
    // Find the segment
    let segment = msg.segments.iter().find(|s| {
        std::str::from_utf8(&s.id) == Ok(segment_id)
    })?;
    
    // Parse field index (1-based)
    let field_part = parts.next()?;
    let (field_index, rep_index) = parse_field_and_rep(field_part)?;
    
    // Special handling for MSH segments
    if segment_id == "MSH" {
        if field_index == 1 {
            // MSH-1 is the field separator character
            // We can't return a reference to a temporary string, so we don't support this case
            // Users should access msg.delims.field directly for the field separator
            None
        } else if field_index == 2 {
            // MSH-2 is the encoding characters
            // This should be the first parsed field (index 0)
            if segment.fields.is_empty() {
                return None;
            }
            let field = &segment.fields[0];
            // Get the repetition
            if rep_index == 0 || rep_index > field.reps.len() {
                return None;
            }
            let rep = &field.reps[rep_index - 1];
            // Get the component
            let comp_index = if let Some(comp_part) = parts.next() {
                comp_part.parse::<usize>().ok()?
            } else {
                1
            };
            if comp_index == 0 || comp_index > rep.comps.len() {
                return None;
            }
            let comp = &rep.comps[comp_index - 1];
            // Get the subcomponent
            if comp.subs.is_empty() {
                return None;
            }
            match &comp.subs[0] {
                Atom::Text(text) => Some(text.as_str()),
                Atom::Null => None,
            }
        } else {
            // MSH-3 and beyond
            // Adjust index: MSH-3 maps to parsed field 1, MSH-4 to parsed field 2, etc.
            let adjusted_field_index = field_index - 2;
            if adjusted_field_index >= segment.fields.len() {
                return None;
            }
            let field = &segment.fields[adjusted_field_index];
            // Get the repetition
            if rep_index == 0 || rep_index > field.reps.len() {
                return None;
            }
            let rep = &field.reps[rep_index - 1];
            // Get the component
            let comp_index = if let Some(comp_part) = parts.next() {
                comp_part.parse::<usize>().ok()?
            } else {
                1
            };
            if comp_index == 0 || comp_index > rep.comps.len() {
                return None;
            }
            let comp = &rep.comps[comp_index - 1];
            // Get the subcomponent
            if comp.subs.is_empty() {
                return None;
            }
            match &comp.subs[0] {
                Atom::Text(text) => Some(text.as_str()),
                Atom::Null => None,
            }
        }
    } else {
        // For non-MSH segments, convert directly to 0-based indexing
        if field_index == 0 {
            return None;
        }
        let zero_based_field_index = field_index - 1;
        
        // Get the field
        if zero_based_field_index >= segment.fields.len() {
            return None;
        }
        let field = &segment.fields[zero_based_field_index];
        
        // Get the repetition (convert to 0-based indexing)
        if rep_index == 0 || rep_index > field.reps.len() {
            return None;
        }
        let rep = &field.reps[rep_index - 1];
        
        // Parse component index if provided
        let comp_index = if let Some(comp_part) = parts.next() {
            comp_part.parse::<usize>().ok()?
        } else {
            1 // Default to first component
        };
        
        // Get the component (convert to 0-based indexing)
        if comp_index == 0 || comp_index > rep.comps.len() {
            return None;
        }
        let comp = &rep.comps[comp_index - 1];
        
        // Get the first subcomponent as text
        if comp.subs.is_empty() {
            return None;
        }
        
        match &comp.subs[0] {
            Atom::Text(text) => Some(text.as_str()),
            Atom::Null => None,
        }
    }
}

/// Get presence semantics for a field at path (e.g., "PID.5[1].1")
pub fn get_presence(msg: &Message, path: &str) -> Presence {
    // Parse the path
    // Format: SEGMENT.FIELD[REP].COMPONENT
    // Examples: "PID.5.1", "PID.5[1].1", "MSH.9"
    
    let mut parts = path.split('.');
    let segment_id = match parts.next() {
        Some(id) => id,
        None => return Presence::Missing,
    };
    
    // Find the segment
    let segment = match msg.segments.iter().find(|s| {
        std::str::from_utf8(&s.id) == Ok(segment_id)
    }) {
        Some(seg) => seg,
        None => return Presence::Missing,
    };
    
    // Parse field index (1-based)
    let field_part = match parts.next() {
        Some(part) => part,
        None => return Presence::Missing,
    };
    
    let (field_index, rep_index) = match parse_field_and_rep(field_part) {
        Some(indices) => indices,
        None => return Presence::Missing,
    };
    
    // Special handling for MSH segments
    if segment_id == "MSH" {
        if field_index == 1 {
            // MSH-1 is the field separator character
            // We treat this as a special case - present with the field separator value
            Presence::Value(msg.delims.field.to_string())
        } else if field_index == 2 {
            // MSH-2 is the encoding characters
            // This should be the first parsed field (index 0)
            if segment.fields.is_empty() {
                return Presence::Missing;
            }
            let field = &segment.fields[0];
            // Check repetition bounds
            if rep_index == 0 || rep_index > field.reps.len() {
                return Presence::Missing;
            }
            let rep = &field.reps[rep_index - 1];
            // Get the component
            let comp_index = if let Some(comp_part) = parts.next() {
                match comp_part.parse::<usize>() {
                    Ok(index) => index,
                    Err(_) => return Presence::Missing,
                }
            } else {
                1
            };
            if comp_index == 0 || comp_index > rep.comps.len() {
                return Presence::Missing;
            }
            let comp = &rep.comps[comp_index - 1];
            // Get the subcomponent
            if comp.subs.is_empty() {
                return Presence::Missing;
            }
            match &comp.subs[0] {
                Atom::Text(text) => {
                    if text.is_empty() {
                        Presence::Empty
                    } else {
                        Presence::Value(text.clone())
                    }
                },
                Atom::Null => Presence::Null,
            }
        } else {
            // MSH-3 and beyond
            // Adjust index: MSH-3 maps to parsed field 1, MSH-4 to parsed field 2, etc.
            let adjusted_field_index = field_index - 2;
            if adjusted_field_index >= segment.fields.len() {
                return Presence::Missing;
            }
            let field = &segment.fields[adjusted_field_index];
            // Check repetition bounds
            if rep_index == 0 || rep_index > field.reps.len() {
                return Presence::Missing;
            }
            let rep = &field.reps[rep_index - 1];
            // Get the component
            let comp_index = if let Some(comp_part) = parts.next() {
                match comp_part.parse::<usize>() {
                    Ok(index) => index,
                    Err(_) => return Presence::Missing,
                }
            } else {
                1
            };
            if comp_index == 0 || comp_index > rep.comps.len() {
                return Presence::Missing;
            }
            let comp = &rep.comps[comp_index - 1];
            // Get the subcomponent
            if comp.subs.is_empty() {
                return Presence::Missing;
            }
            match &comp.subs[0] {
                Atom::Text(text) => {
                    if text.is_empty() {
                        Presence::Empty
                    } else {
                        Presence::Value(text.clone())
                    }
                },
                Atom::Null => Presence::Null,
            }
        }
    } else {
        // For non-MSH segments, convert directly to 0-based indexing
        if field_index == 0 {
            return Presence::Missing;
        }
        let zero_based_field_index = field_index - 1;
        
        // Check field bounds
        if zero_based_field_index >= segment.fields.len() {
            return Presence::Missing;
        }
        let field = &segment.fields[zero_based_field_index];
        
        // Check repetition bounds
        if rep_index == 0 || rep_index > field.reps.len() {
            return Presence::Missing;
        }
        let rep = &field.reps[rep_index - 1];
        
        // Parse component index if provided
        let comp_index = if let Some(comp_part) = parts.next() {
            match comp_part.parse::<usize>() {
                Ok(index) => index,
                Err(_) => return Presence::Missing,
            }
        } else {
            1 // Default to first component
        };
        
        // Check component bounds
        if comp_index == 0 || comp_index > rep.comps.len() {
            return Presence::Missing;
        }
        let comp = &rep.comps[comp_index - 1];
        
        // Get the first subcomponent
        if comp.subs.is_empty() {
            return Presence::Missing;
        }
        
        match &comp.subs[0] {
            Atom::Text(text) => {
                if text.is_empty() {
                    Presence::Empty
                } else {
                    Presence::Value(text.clone())
                }
            },
            Atom::Null => Presence::Null,
        }
    }
}

/// Parse field and repetition indices from a string like "5" or "5[1]"
fn parse_field_and_rep(field_str: &str) -> Option<(usize, usize)> {
    if let Some(bracket_pos) = field_str.find('[') {
        // Has repetition index
        let field_index = field_str[..bracket_pos].parse::<usize>().ok()?;
        let rep_part = &field_str[bracket_pos + 1..];
        if let Some(end_bracket) = rep_part.find(']') {
            let rep_index = rep_part[..end_bracket].parse::<usize>().ok()?;
            Some((field_index, rep_index))
        } else {
            None
        }
    } else {
        // No repetition index, default to 1
        let field_index = field_str.parse::<usize>().ok()?;
        Some((field_index, 1))
    }
}

/// Convert message to canonical JSON
pub fn to_json(msg: &Message) -> serde_json::Value {
    use serde_json::json;
    
    let segments: Vec<serde_json::Value> = msg
        .segments
        .iter()
        .map(|segment| {
            let segment_id = String::from_utf8_lossy(&segment.id).to_string();
            let fields: serde_json::Map<String, serde_json::Value> = segment
                .fields
                .iter()
                .enumerate()
                .filter_map(|(index, field)| {
                    if field.reps.is_empty() {
                        None
                    } else {
                        let field_value = field_to_json(field);
                        Some(((index + 1).to_string(), field_value))
                    }
                })
                .collect();
            
            json!({
                "id": segment_id,
                "fields": fields
            })
        })
        .collect();
    
    json!({
        "meta": {
            "delims": {
                "field": msg.delims.field.to_string(),
                "comp": msg.delims.comp.to_string(),
                "rep": msg.delims.rep.to_string(),
                "esc": msg.delims.esc.to_string(),
                "sub": msg.delims.sub.to_string()
            },
            "charsets": msg.charsets
        },
        "segments": segments
    })
}

/// Convert a field to JSON
fn field_to_json(field: &Field) -> serde_json::Value {
    use serde_json::json;
    
    let reps: Vec<serde_json::Value> = field
        .reps
        .iter()
        .map(|rep| {
            let comps: Vec<serde_json::Value> = rep
                .comps
                .iter()
                .map(|comp| {
                    let subs: Vec<serde_json::Value> = comp
                        .subs
                        .iter()
                        .map(|atom| match atom {
                            Atom::Text(text) => json!(text),
                            Atom::Null => json!("__NULL__"),
                        })
                        .collect();
                    json!(subs)
                })
                .collect();
            json!(comps)
        })
        .collect();
    
    json!(reps)
}

/// Helper function to write segment fields
fn write_segment_fields(segment: &Segment, output: &mut Vec<u8>, delims: &Delims) {
    for (i, field) in segment.fields.iter().enumerate() {
        if i > 0 {
            output.push(delims.field as u8);
        }
        write_field(output, field, delims);
    }
}

/// Helper function to get delimiters from a file batch
fn get_delimiters_from_file_batch(file_batch: &FileBatch) -> Delims {
    // Try to get delimiters from the first message in the first batch
    if let Some(first_batch) = file_batch.batches.first()
        && let Some(first_message) = first_batch.messages.first() {
        return first_message.delims.clone();
    }
    // Fallback to default delimiters
    Delims::default()
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
    fn test_debug_segments() {
        let hl7_text = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\rPV1|1|O|OP^PAREG^CHAREG|3|||DOE^JOHN^A^III^^^^MD|^DR.^JANE^B^^^^RN|||SURG||||ADM|||||20250128152300\r";
        println!("Testing HL7 text: {:?}", hl7_text);
        
        // Split into lines to see what we're getting
        let lines: Vec<&str> = hl7_text.split('\r').filter(|line| !line.is_empty()).collect();
        println!("Lines: {:?}", lines);
        
        for (i, line) in lines.iter().enumerate() {
            println!("Line {}: '{}' (len: {})", i, line, line.len());
            if line.len() >= 3 {
                println!("  First 3 chars: '{}'", &line[0..3]);
            }
        }
        
        let result = parse(hl7_text.as_bytes());
        match result {
            Ok(message) => {
                println!("Successfully parsed message with {} segments", message.segments.len());
                for (i, segment) in message.segments.iter().enumerate() {
                    let segment_id = std::str::from_utf8(&segment.id).unwrap();
                    println!("Segment {}: {} with {} fields", i, segment_id, segment.fields.len());
                }
            },
            Err(e) => {
                println!("Error parsing message: {}", e);
            }
        }
    }

    #[test]
    fn test_debug_segments_detailed() {
        let hl7_text = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\rPV1|1|O|OP^PAREG^CHAREG|3|||DOE^JOHN^A^III^^^^MD|^DR.^JANE^B^^^^RN|||SURG||||ADM|||||20250128152300\r";
        
        // Split into lines to see what we're getting
        let lines: Vec<&str> = hl7_text.split('\r').filter(|line| !line.is_empty()).collect();
        println!("Lines: {:?}", lines);
        
        for (i, line) in lines.iter().enumerate() {
            println!("Line {}: '{}' (len: {})", i, line, line.len());
            if line.len() >= 3 {
                let segment_id = &line[0..3];
                println!("  First 3 chars: '{}' (bytes: {:?})", segment_id, segment_id.as_bytes());
                
                // Check each byte
                for (j, byte) in segment_id.bytes().enumerate() {
                    println!("    Byte {}: {} ({})", j, byte, byte as char);
                    if !byte.is_ascii_uppercase() {
                        println!("      INVALID BYTE: {} is not between A-Z", byte as char);
                    }
                }
            }
        }
        
        let result = parse(hl7_text.as_bytes());
        match result {
            Ok(message) => {
                println!("Successfully parsed message with {} segments", message.segments.len());
                for (i, segment) in message.segments.iter().enumerate() {
                    let segment_id = std::str::from_utf8(&segment.id).unwrap();
                    println!("Segment {}: {} with {} fields", i, segment_id, segment.fields.len());
                }
            },
            Err(e) => {
                println!("Error parsing message: {}", e);
            }
        }
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
        
        // Test repetition that doesn's exist
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
    fn debug_normalization() {
        // Read the MLLP file
        let contents = std::fs::read("test_mllp.hl7").expect("Failed to read test_mllp.hl7");
        println!("File size: {} bytes", contents.len());
        
        // Parse as MLLP
        let message = parse_mllp(&contents).expect("Failed to parse MLLP");
        println!("Successfully parsed MLLP message");
        
        // Write back to bytes
        let output = write(&message);
        println!("Output size: {} bytes", output.len());
        
        // Print the output as a string
        let output_str = String::from_utf8(output).expect("Failed to convert to UTF-8");
        println!("Output:\n{}", output_str);
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
            Presence::Empty => {},
            _ => panic!("Expected Empty, got something else"),
        }
        
        // Test missing field (PID.50 doesn't exist)
        match get_presence(&message, "PID.50.1") {
            Presence::Missing => {},
            _ => panic!("Expected Missing, got something else"),
        }
        
        // Test MSH-1 (special case)
        match get_presence(&message, "MSH.1") {
            Presence::Value(val) => assert_eq!(val, "|"),
            _ => panic!("Expected Value for MSH.1, got something else"),
        }
    }
    
    #[test]
    fn debug_charset_extraction() {
        // Create a message with charset information in MSH-18
        let hl7_text = "MSH|^~\\&|SendingApp|SendingFac|||||||||||||||UTF-8^ISO-8859-1\rPID|1||123456^^^HOSP^MR||Doe^John\r";
        let message = parse(hl7_text.as_bytes()).unwrap();
        
        println!("Number of segments: {}", message.segments.len());
        for (i, segment) in message.segments.iter().enumerate() {
            let segment_id = std::str::from_utf8(&segment.id).unwrap();
            println!("Segment {}: {} with {} fields", i, segment_id, segment.fields.len());
            if segment_id == "MSH" {
                for (j, field) in segment.fields.iter().enumerate() {
                    println!("  Field {}: {} reps", j, field.reps.len());
                    for (k, rep) in field.reps.iter().enumerate() {
                        println!("    Rep {}: {} comps", k, rep.comps.len());
                        for (l, comp) in rep.comps.iter().enumerate() {
                            println!("      Comp {}: {} subs", l, comp.subs.len());
                            for (m, sub) in comp.subs.iter().enumerate() {
                                match sub {
                                    Atom::Text(text) => println!("        Sub {}: Text({})", m, text),
                                    Atom::Null => println!("        Sub {}: Null", m),
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Test the extract_charsets function directly
        let charsets = extract_charsets(&message.segments);
        println!("Extracted charsets: {:?}", charsets);
    }

    #[test]
    fn test_charset_extraction() {
        // Create a message with charset information in MSH-18
        let hl7_text = "MSH|^~\\&|SendingApp|SendingFac|||||||||||||||UTF-8^ISO-8859-1\rPID|1||123456^^^HOSP^MR||Doe^John\r";
        let message = parse(hl7_text.as_bytes()).unwrap();
        
        // Verify that charsets were extracted correctly
        assert_eq!(message.charsets, vec!["UTF-8", "ISO-8859-1"]);
        
        // Create a message without charset information
        let hl7_text_no_charset = "MSH|^~\\&|SendingApp|SendingFac\rPID|1||123456^^^HOSP^MR||Doe^John\r";
        let message_no_charset = parse(hl7_text_no_charset.as_bytes()).unwrap();
        
        // Verify that charsets is empty
        assert!(message_no_charset.charsets.is_empty());
        
        // Test JSON output includes charsets
        let json_value = to_json(&message);
        let meta = json_value.get("meta").unwrap();
        let charsets = meta.get("charsets").unwrap().as_array().unwrap();
        assert_eq!(charsets.len(), 2);
        assert_eq!(charsets[0], "UTF-8");
        assert_eq!(charsets[1], "ISO-8859-1");
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
