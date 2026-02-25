//! HL7 v2 batch message handling (FHS/BHS/FTS/BTS).
//!
//! This crate provides batch processing for HL7 v2 messages, supporting:
//! - File Batch Header (FHS) and Trailer (FTS)
//! - Batch Header (BHS) and Trailer (BTS)
//! - Nested batch structures
//!
//! # Batch Structure
//!
//! ```text
//! FHS - File Header Segment
//!   BHS - Batch Header Segment (optional, can be multiple)
//!     MSH - Message Header (repeated)
//!     ... message segments ...
//!   BTS - Batch Trailer Segment
//! FTS - File Trailer Segment
//! ```
//!
//! # Example
//!
//! ```
//! use hl7v2_batch::{parse_batch, BatchType};
//!
//! let batch_data = b"FHS|^~\\&|App|Fac|\rBHS|^~\\&|App|Fac|\rMSH|^~\\&|...\rBTS|1\rFTS|1\r";
//! let batch = parse_batch(batch_data).unwrap();
//!
//! match batch.info.batch_type {
//!     BatchType::File => println!("File batch"),
//!     BatchType::Single => println!("Single batch"),
//! }
//! ```

use thiserror::Error;
use hl7v2_model::{Message, Segment, Error as ModelError};
use hl7v2_parser::parse;

/// Error type for batch operations
#[derive(Debug, Error)]
pub enum BatchError {
    #[error("Invalid batch structure: {0}")]
    InvalidStructure(String),
    
    #[error("Missing required segment: {0}")]
    MissingSegment(String),
    
    #[error("Mismatched batch headers/trailers")]
    MismatchedHeaders,
    
    #[error("Parse error: {0}")]
    ParseError(String),
    
    #[error("Count mismatch: expected {expected}, got {actual}")]
    CountMismatch { expected: usize, actual: usize },
}

impl From<ModelError> for BatchError {
    fn from(e: ModelError) -> Self {
        BatchError::ParseError(e.to_string())
    }
}

/// Type of batch
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchType {
    /// Single batch (BHS/BTS only)
    Single,
    /// File batch (FHS/FTS with optional nested BHS/BTS)
    File,
}

/// Batch information extracted from header segments
#[derive(Debug, Clone, PartialEq)]
pub struct BatchInfo {
    /// Batch type (file or single)
    pub batch_type: BatchType,
    /// File field separator (from FHS-1)
    pub field_separator: Option<char>,
    /// File encoding characters (from FHS-2)
    pub encoding_characters: Option<String>,
    /// Sending application (from FHS/BHS-3)
    pub sending_application: Option<String>,
    /// Sending facility (from FHS/BHS-4)
    pub sending_facility: Option<String>,
    /// Receiving application (from FHS/BHS-5)
    pub receiving_application: Option<String>,
    /// Receiving facility (from FHS/BHS-6)
    pub receiving_facility: Option<String>,
    /// File creation date/time (from FHS-7)
    pub file_creation_time: Option<String>,
    /// Security (from FHS-8)
    pub security: Option<String>,
    /// Batch name/ID (from FHS/BHS-10)
    pub batch_name: Option<String>,
    /// Batch comment (from FHS/BHS-11)
    pub batch_comment: Option<String>,
    /// Number of messages (from BTS-1 or FTS-1)
    pub message_count: Option<usize>,
    /// Batch comment (from BTS-2 or FTS-2)
    pub trailer_comment: Option<String>,
}

impl Default for BatchInfo {
    fn default() -> Self {
        Self {
            batch_type: BatchType::Single,
            field_separator: None,
            encoding_characters: None,
            sending_application: None,
            sending_facility: None,
            receiving_application: None,
            receiving_facility: None,
            file_creation_time: None,
            security: None,
            batch_name: None,
            batch_comment: None,
            message_count: None,
            trailer_comment: None,
        }
    }
}

/// A single batch containing messages
#[derive(Debug, Clone, PartialEq)]
pub struct Batch {
    /// Batch header segment (BHS), if present
    pub header: Option<Segment>,
    /// Messages contained in the batch
    pub messages: Vec<Message>,
    /// Batch trailer segment (BTS), if present
    pub trailer: Option<Segment>,
    /// Extracted batch info
    pub info: BatchInfo,
}

impl Batch {
    /// Create a new empty batch
    pub fn new() -> Self {
        Self {
            header: None,
            messages: Vec::new(),
            trailer: None,
            info: BatchInfo::default(),
        }
    }
    
    /// Add a message to the batch
    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }
    
    /// Get the number of messages
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }
    
    /// Iterate over messages
    pub fn iter_messages(&self) -> impl Iterator<Item = &Message> {
        self.messages.iter()
    }
}

impl Default for Batch {
    fn default() -> Self {
        Self::new()
    }
}

/// A file batch containing nested batches or messages
#[derive(Debug, Clone, PartialEq)]
pub struct FileBatch {
    /// File header segment (FHS)
    pub header: Option<Segment>,
    /// Nested batches
    pub batches: Vec<Batch>,
    /// File trailer segment (FTS)
    pub trailer: Option<Segment>,
    /// Extracted batch info
    pub info: BatchInfo,
}

impl FileBatch {
    /// Create a new empty file batch
    pub fn new() -> Self {
        Self {
            header: None,
            batches: Vec::new(),
            trailer: None,
            info: BatchInfo {
                batch_type: BatchType::File,
                ..BatchInfo::default()
            },
        }
    }
    
    /// Add a batch to the file
    pub fn add_batch(&mut self, batch: Batch) {
        self.batches.push(batch);
    }
    
    /// Get total message count across all batches
    pub fn total_message_count(&self) -> usize {
        self.batches.iter().map(|b| b.message_count()).sum()
    }
    
    /// Iterate over all messages across all batches
    pub fn iter_all_messages(&self) -> impl Iterator<Item = &Message> {
        self.batches.iter().flat_map(|b| b.messages.iter())
    }
}

impl Default for FileBatch {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse batch data into a FileBatch or single Batch
pub fn parse_batch(data: &[u8]) -> Result<FileBatch, BatchError> {
    let text = std::str::from_utf8(data)
        .map_err(|_| BatchError::InvalidStructure("Invalid UTF-8 data".to_string()))?;
    
    let lines: Vec<&str> = text.split(|c| c == '\r' || c == '\n')
        .filter(|l| !l.is_empty())
        .collect();
    
    if lines.is_empty() {
        return Err(BatchError::InvalidStructure("Empty batch data".to_string()));
    }
    
    // Check first line for batch type
    let first_line = lines[0];
    
    if first_line.starts_with("FHS") {
        parse_file_batch(&lines)
    } else if first_line.starts_with("BHS") {
        // Single batch without file wrapper
        let batch = parse_single_batch(&lines)?;
        let mut file_batch = FileBatch::new();
        file_batch.add_batch(batch);
        Ok(file_batch)
    } else if first_line.starts_with("MSH") {
        // Not a batch, just messages
        let messages = parse_messages(&lines)?;
        let batch = Batch {
            header: None,
            messages,
            trailer: None,
            info: BatchInfo::default(),
        };
        let mut file_batch = FileBatch::new();
        file_batch.add_batch(batch);
        Ok(file_batch)
    } else {
        Err(BatchError::InvalidStructure(
            format!("Unknown first segment: {}", &first_line[..3.min(first_line.len())])
        ))
    }
}

/// Parse a file batch (with FHS/FTS)
fn parse_file_batch(lines: &[&str]) -> Result<FileBatch, BatchError> {
    let mut file_batch = FileBatch::new();
    let mut current_batch_lines: Vec<&str> = Vec::new();
    let mut in_batch = false;
    
    for line in lines {
        if line.starts_with("FHS") {
            file_batch.header = Some(parse_segment(line)?);
            let info = extract_batch_info(line, "FHS")?;
            // Preserve batch_type which is already set to File
            file_batch.info.encoding_characters = info.encoding_characters;
            file_batch.info.sending_application = info.sending_application;
            file_batch.info.sending_facility = info.sending_facility;
            file_batch.info.receiving_application = info.receiving_application;
            file_batch.info.receiving_facility = info.receiving_facility;
            file_batch.info.batch_name = info.batch_name;
            file_batch.info.batch_comment = info.batch_comment;
        } else if line.starts_with("FTS") {
            file_batch.trailer = Some(parse_segment(line)?);
            // Extract message count from FTS-1
            let info = extract_batch_info(line, "FTS")?;
            file_batch.info.message_count = info.message_count;
            file_batch.info.trailer_comment = info.trailer_comment;
        } else if line.starts_with("BHS") {
            in_batch = true;
            current_batch_lines.push(line);
        } else if line.starts_with("BTS") {
            current_batch_lines.push(line);
            let batch = parse_single_batch(&current_batch_lines)?;
            file_batch.add_batch(batch);
            current_batch_lines.clear();
            in_batch = false;
        } else if in_batch {
            current_batch_lines.push(line);
        } else if line.starts_with("MSH") {
            // Message without BHS wrapper
            let messages = parse_messages(std::slice::from_ref(line))?;
            let batch = Batch {
                header: None,
                messages,
                trailer: None,
                info: BatchInfo::default(),
            };
            file_batch.add_batch(batch);
        }
    }
    
    Ok(file_batch)
}

/// Parse a single batch (with BHS/BTS)
fn parse_single_batch(lines: &[&str]) -> Result<Batch, BatchError> {
    let mut batch = Batch::new();
    let mut message_lines: Vec<&str> = Vec::new();
    
    for line in lines {
        if line.starts_with("BHS") {
            batch.header = Some(parse_segment(line)?);
            batch.info = extract_batch_info(line, "BHS")?;
        } else if line.starts_with("BTS") {
            batch.trailer = Some(parse_segment(line)?);
            let info = extract_batch_info(line, "BTS")?;
            batch.info.message_count = info.message_count;
            batch.info.trailer_comment = info.trailer_comment;
        } else if line.starts_with("MSH") {
            if !message_lines.is_empty() {
                // Parse previous message
                let msg_text = message_lines.join("\r");
                let msg = parse(msg_text.as_bytes())?;
                batch.add_message(msg);
                message_lines.clear();
            }
            message_lines.push(line);
        } else {
            message_lines.push(line);
        }
    }
    
    // Parse last message
    if !message_lines.is_empty() {
        let msg_text = message_lines.join("\r");
        let msg = parse(msg_text.as_bytes())?;
        batch.add_message(msg);
    }
    
    // Verify message count if specified
    if let Some(expected) = batch.info.message_count {
        if expected != batch.message_count() {
            return Err(BatchError::CountMismatch {
                expected,
                actual: batch.message_count(),
            });
        }
    }
    
    Ok(batch)
}

/// Parse multiple messages from lines
fn parse_messages(lines: &[&str]) -> Result<Vec<Message>, BatchError> {
    let mut messages = Vec::new();
    let mut message_lines: Vec<&str> = Vec::new();
    
    for line in lines {
        if line.starts_with("MSH") {
            if !message_lines.is_empty() {
                let msg_text = message_lines.join("\r");
                let msg = parse(msg_text.as_bytes())?;
                messages.push(msg);
                message_lines.clear();
            }
        }
        message_lines.push(line);
    }
    
    if !message_lines.is_empty() {
        let msg_text = message_lines.join("\r");
        let msg = parse(msg_text.as_bytes())?;
        messages.push(msg);
    }
    
    Ok(messages)
}

/// Parse a single segment line
fn parse_segment(line: &str) -> Result<Segment, BatchError> {
    // Simple segment parsing for batch headers/trailers
    if line.len() < 3 {
        return Err(BatchError::InvalidStructure(format!("Segment too short: {}", line)));
    }
    
    let id_bytes = line[0..3].as_bytes();
    let id: [u8; 3] = [id_bytes[0], id_bytes[1], id_bytes[2]];
    let field_sep = line.chars().nth(3).unwrap_or('|');
    
    let fields_str = if line.len() > 4 { &line[4..] } else { "" };
    let field_strs: Vec<&str> = fields_str.split(field_sep).collect();
    
    // Convert to Field structures (simplified)
    let fields: Vec<hl7v2_model::Field> = field_strs.iter().map(|s| {
        hl7v2_model::Field {
            reps: vec![hl7v2_model::Rep {
                comps: vec![hl7v2_model::Comp {
                    subs: vec![hl7v2_model::Atom::Text(s.to_string())],
                }],
            }],
        }
    }).collect();
    
    Ok(Segment { id, fields })
}

/// Extract batch info from a segment
fn extract_batch_info(line: &str, segment_type: &str) -> Result<BatchInfo, BatchError> {
    let mut info = BatchInfo::default();
    
    if line.len() < 4 {
        return Ok(info);
    }
    
    let field_sep = line.chars().nth(3).unwrap_or('|');
    let fields: Vec<&str> = line[4..].split(field_sep).collect();
    
    // FTS/BTS-1 is message count, FTS/BTS-2 is trailer comment
    if segment_type == "FTS" || segment_type == "BTS" {
        info.message_count = fields.get(0).and_then(|s| s.parse::<usize>().ok());
        if fields.len() > 1 {
            info.trailer_comment = Some(fields[1].to_string());
        }
        return Ok(info);
    }
    
    // FHS/BHS fields (0-indexed after split):
    // After split from position 4, fields[0] is the first field after separator
    // fields[0] = Encoding Characters (BHS-2)
    // fields[1] = Sending Application (BHS-3)
    // fields[2] = Sending Facility (BHS-4)
    // fields[3] = Receiving Application (BHS-5)
    // fields[4] = Receiving Facility (BHS-6)
    // etc.
    if fields.len() > 0 {
        info.encoding_characters = Some(fields[0].to_string());
    }
    if fields.len() > 1 {
        info.sending_application = Some(fields[1].to_string());
    }
    if fields.len() > 2 {
        info.sending_facility = Some(fields[2].to_string());
    }
    if fields.len() > 3 {
        info.receiving_application = Some(fields[3].to_string());
    }
    if fields.len() > 4 {
        info.receiving_facility = Some(fields[4].to_string());
    }
    if fields.len() > 8 {
        info.batch_name = Some(fields[8].to_string());
    }
    if fields.len() > 9 {
        info.batch_comment = Some(fields[9].to_string());
    }
    
    Ok(info)
}

#[cfg(test)]
mod tests;