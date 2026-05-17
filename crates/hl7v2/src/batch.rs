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
//! use hl7v2::batch::{parse_batch, BatchType};
//!
//! let batch_data = b"FHS|^~\\&|App|Fac|\rBHS|^~\\&|App|Fac|\rMSH|^~\\&|...\rBTS|1\rFTS|1\r";
//! let batch = parse_batch(batch_data).unwrap();
//!
//! match batch.info.batch_type {
//!     BatchType::File => println!("File batch"),
//!     BatchType::Single => println!("Single batch"),
//! }
//! ```

use crate::model::{Atom, Comp, Error as ModelError, Field, Message, Rep, Segment};
use crate::parser::parse;
use thiserror::Error;

/// Error type for batch operations
#[derive(Debug, Error, Clone)]
pub enum BatchError {
    /// The batch structure does not match the expected HL7 format.
    #[error("Invalid batch structure: {0}")]
    InvalidStructure(String),

    /// A required segment is missing.
    #[error("Missing required segment: {0}")]
    MissingSegment(String),

    /// Found start and end batch markers that do not align.
    #[error("Mismatched batch headers/trailers")]
    MismatchedHeaders,

    /// General parsing error while reading batch input.
    #[error("Parse error: {0}")]
    ParseError(String),

    /// The batch trailer count does not match observed messages.
    #[error("Count mismatch: expected {expected}, got {actual}")]
    CountMismatch {
        /// Expected message count from batch trailer.
        expected: usize,
        /// Actual number of messages parsed.
        actual: usize,
    },
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
        self.batches.iter().map(Batch::message_count).sum()
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

/// Parse batch data into a `FileBatch` or single batch wrapper.
///
/// # Errors
///
/// Returns [`BatchError`] when the input is not UTF-8, has an unsupported batch
/// structure, is missing required batch segments, contains malformed messages,
/// or declares a trailer message count that does not match the parsed messages.
pub fn parse_batch(data: &[u8]) -> Result<FileBatch, BatchError> {
    let text = std::str::from_utf8(data)
        .map_err(|_err| BatchError::InvalidStructure("Invalid UTF-8 data".to_string()))?;

    let lines: Vec<&str> = text.split(['\r', '\n']).filter(|l| !l.is_empty()).collect();

    if lines.is_empty() {
        return Err(BatchError::InvalidStructure("Empty batch data".to_string()));
    }

    // Check first line for batch type
    let Some(first_line) = lines.first().copied() else {
        return Err(BatchError::InvalidStructure("Empty batch data".to_string()));
    };

    if first_line.starts_with("FHS") {
        parse_file_batch(&lines)
    } else if first_line.starts_with("BHS") {
        // Single batch without file wrapper
        let batch = parse_single_batch(&lines)?;
        let mut file_batch = FileBatch::new();
        // Override batch_type to Single for BHS-only batches
        file_batch.info.batch_type = BatchType::Single;
        // Propagate the nested batch's info to the FileBatch for single batches
        file_batch.info.field_separator = batch.info.field_separator;
        file_batch.info.encoding_characters = batch.info.encoding_characters.clone();
        file_batch.info.sending_application = batch.info.sending_application.clone();
        file_batch.info.sending_facility = batch.info.sending_facility.clone();
        file_batch.info.receiving_application = batch.info.receiving_application.clone();
        file_batch.info.receiving_facility = batch.info.receiving_facility.clone();
        file_batch.info.security = batch.info.security.clone();
        file_batch.info.batch_name = batch.info.batch_name.clone();
        file_batch.info.batch_comment = batch.info.batch_comment.clone();
        file_batch.info.message_count = batch.info.message_count;
        file_batch.info.trailer_comment = batch.info.trailer_comment.clone();
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
        Err(BatchError::InvalidStructure(format!(
            "Unknown first segment: {}",
            segment_prefix(first_line)
        )))
    }
}

/// Parse a file batch (with FHS/FTS)
fn parse_file_batch(lines: &[&str]) -> Result<FileBatch, BatchError> {
    let mut file_batch = FileBatch::new();
    let mut current_batch_lines: Vec<&str> = Vec::new();
    let mut current_message_lines: Vec<&str> = Vec::new();
    let mut in_batch = false;
    let mut has_fhs = false;

    for line in lines {
        if line.starts_with("FHS") {
            add_unwrapped_message_batch(&mut file_batch, &current_message_lines)?;
            current_message_lines.clear();
            has_fhs = true;
            file_batch.header = Some(parse_segment(line)?);
            let info = extract_batch_info(line, "FHS")?;
            // Preserve batch_type which is already set to File
            file_batch.info.encoding_characters = info.encoding_characters;
            file_batch.info.sending_application = info.sending_application;
            file_batch.info.sending_facility = info.sending_facility;
            file_batch.info.receiving_application = info.receiving_application;
            file_batch.info.receiving_facility = info.receiving_facility;
            file_batch.info.file_creation_time = info.file_creation_time;
            file_batch.info.security = info.security;
            file_batch.info.field_separator = info.field_separator;
            file_batch.info.batch_name = info.batch_name;
            file_batch.info.batch_comment = info.batch_comment;
        } else if line.starts_with("FTS") {
            add_unwrapped_message_batch(&mut file_batch, &current_message_lines)?;
            current_message_lines.clear();
            file_batch.trailer = Some(parse_segment(line)?);
            // Extract message count from FTS-1
            let info = extract_batch_info(line, "FTS")?;
            file_batch.info.message_count = info.message_count;
            file_batch.info.trailer_comment = info.trailer_comment;
        } else if line.starts_with("BHS") {
            add_unwrapped_message_batch(&mut file_batch, &current_message_lines)?;
            current_message_lines.clear();
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
            add_unwrapped_message_batch(&mut file_batch, &current_message_lines)?;
            current_message_lines.clear();
            current_message_lines.push(line);
        } else if !current_message_lines.is_empty() {
            current_message_lines.push(line);
        }
    }

    add_unwrapped_message_batch(&mut file_batch, &current_message_lines)?;

    // Validate that FHS is present for file batches
    if !has_fhs {
        return Err(BatchError::MissingSegment("FHS".to_string()));
    }

    let actual_message_count = file_batch.total_message_count();

    // If message_count is not set from FTS, calculate from batches. Otherwise,
    // verify the file trailer count matches the total messages across nested
    // batches, mirroring the single-batch BTS validation.
    match file_batch.info.message_count {
        Some(expected) if expected != actual_message_count => Err(BatchError::CountMismatch {
            expected,
            actual: actual_message_count,
        }),
        Some(_expected) => Ok(file_batch),
        None => {
            file_batch.info.message_count = Some(actual_message_count);
            Ok(file_batch)
        }
    }
}

fn add_unwrapped_message_batch(
    file_batch: &mut FileBatch,
    message_lines: &[&str],
) -> Result<(), BatchError> {
    if message_lines.is_empty() {
        return Ok(());
    }

    let messages = parse_messages(message_lines)?;
    let batch = Batch {
        header: None,
        messages,
        trailer: None,
        info: BatchInfo::default(),
    };
    file_batch.add_batch(batch);
    Ok(())
}

/// Parse a single batch (with BHS/BTS)
fn parse_single_batch(lines: &[&str]) -> Result<Batch, BatchError> {
    let mut batch = Batch::new();
    let mut message_lines: Vec<&str> = Vec::new();
    let mut has_bhs = false;
    let mut has_bts = false;

    for line in lines {
        if line.starts_with("BHS") {
            has_bhs = true;
            batch.header = Some(parse_segment(line)?);
            batch.info = extract_batch_info(line, "BHS")?;
        } else if line.starts_with("BTS") {
            has_bts = true;
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

    // Validate that BHS is present for single batches (if there are messages or BTS)
    if !has_bhs && (has_bts || !batch.messages.is_empty()) {
        return Err(BatchError::MissingSegment("BHS".to_string()));
    }

    // Validate that BTS is present for single batches (if there are messages or BHS)
    if !has_bts && (has_bhs || !batch.messages.is_empty()) {
        return Err(BatchError::MissingSegment("BTS".to_string()));
    }

    // Ensure message_count is set even for empty batches
    if batch.info.message_count.is_none() {
        batch.info.message_count = Some(batch.message_count());
    }

    // Verify message count if specified
    if let Some(expected) = batch.info.message_count
        && expected != batch.message_count()
    {
        return Err(BatchError::CountMismatch {
            expected,
            actual: batch.message_count(),
        });
    }

    Ok(batch)
}

/// Parse multiple messages from lines
fn parse_messages(lines: &[&str]) -> Result<Vec<Message>, BatchError> {
    let mut messages = Vec::new();
    let mut message_lines: Vec<&str> = Vec::new();

    for line in lines {
        if line.starts_with("MSH") && !message_lines.is_empty() {
            let msg_text = message_lines.join("\r");
            let msg = parse(msg_text.as_bytes())?;
            messages.push(msg);
            message_lines.clear();
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
        return Err(BatchError::InvalidStructure(format!(
            "Segment too short: {line}"
        )));
    }

    let Some(id_bytes) = line.as_bytes().get(0..3) else {
        return Err(BatchError::InvalidStructure(format!(
            "Segment too short: {line}"
        )));
    };
    let Ok(id) = <[u8; 3]>::try_from(id_bytes) else {
        return Err(BatchError::InvalidStructure(format!(
            "Segment too short: {line}"
        )));
    };
    let field_sep = line.chars().nth(3).unwrap_or('|');

    let fields_str = fields_after_separator(line);
    let field_strs: Vec<&str> = fields_str.split(field_sep).collect();

    // Convert to Field structures (simplified)
    let fields: Vec<Field> = field_strs
        .iter()
        .map(|s| Field {
            reps: vec![Rep {
                comps: vec![Comp {
                    subs: vec![Atom::Text((*s).to_string())],
                }],
            }],
        })
        .collect();

    Ok(Segment { id, fields })
}

/// Extract batch info from a segment
fn extract_batch_info(line: &str, segment_type: &str) -> Result<BatchInfo, BatchError> {
    let mut info = BatchInfo::default();

    if line.len() < 4 {
        return Ok(info);
    }

    let field_sep = line.chars().nth(3).unwrap_or('|');

    // Store of field separator
    info.field_separator = Some(field_sep);

    // Split fields, preserving empty fields
    let fields: Vec<&str> = fields_after_separator(line).split(field_sep).collect();

    // FTS/BTS-1 is message count, FTS/BTS-2 is trailer comment
    if segment_type == "FTS" || segment_type == "BTS" {
        info.message_count = fields.first().and_then(|s| s.parse::<usize>().ok());
        if let Some(comment) = fields.get(1) {
            info.trailer_comment = Some((*comment).to_string());
        }
        return Ok(info);
    }

    // FHS/BHS fields (0-indexed after split from position 4):
    // line[4..] = "^~\&|SendingApp|..." so fields[0] = encoding chars
    // fields[0] = Encoding Characters (BHS-2 / FHS-2)
    // fields[1] = Sending Application (BHS-3 / FHS-3)
    // fields[2] = Sending Facility (BHS-4 / FHS-4)
    // fields[3] = Receiving Application (BHS-5 / FHS-5)
    // fields[4] = Receiving Facility (BHS-6 / FHS-6)
    // fields[5] = Date/Time (BHS-7 / FHS-7)
    // fields[6] = Security (BHS-8 / FHS-8)
    // fields[7] = (BHS-9 / FHS-9 — unused)
    // fields[8] = Name/ID (BHS-10 / FHS-10)
    // fields[9] = Batch Comment (BHS-11 / FHS-11)
    if let Some(encoding_characters) = fields.first() {
        info.encoding_characters = Some((*encoding_characters).to_string());
    }
    if let Some(sending_application) = fields.get(1) {
        info.sending_application = Some((*sending_application).to_string());
    }
    if let Some(sending_facility) = fields.get(2) {
        info.sending_facility = Some((*sending_facility).to_string());
    }
    if let Some(receiving_application) = fields.get(3) {
        info.receiving_application = Some((*receiving_application).to_string());
    }
    if let Some(receiving_facility) = fields.get(4) {
        info.receiving_facility = Some((*receiving_facility).to_string());
    }
    if let Some(raw_datetime) = fields.get(5) {
        let datetime = (*raw_datetime).to_string();
        if segment_type == "FHS" {
            info.file_creation_time = Some(datetime);
        }
    }
    if let Some(security) = fields.get(6) {
        info.security = Some((*security).to_string());
    }
    if let Some(batch_name) = fields.get(8) {
        info.batch_name = Some((*batch_name).to_string());
    }
    if let Some(batch_comment) = fields.get(9) {
        info.batch_comment = Some((*batch_comment).to_string());
    }

    Ok(info)
}

fn fields_after_separator(line: &str) -> &str {
    line.get(4..).unwrap_or_default()
}

fn segment_prefix(line: &str) -> &str {
    line.get(..3).unwrap_or(line)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::get;
    use std::fmt::Debug;

    fn message(control_id: &str) -> String {
        format!(
            "MSH|^~\\&|APP|FAC|RCV|RCVFAC|202605030101||ADT^A01|{control_id}|P|2.5.1\rPID|1||MRN^^^HOSP^MR||Doe^John"
        )
    }

    fn require_eq<T>(actual: T, expected: T, label: &str) -> Result<(), Box<dyn std::error::Error>>
    where
        T: PartialEq + Debug,
    {
        if actual == expected {
            Ok(())
        } else {
            Err(
                std::io::Error::other(format!("{label}: expected {expected:?}, got {actual:?}"))
                    .into(),
            )
        }
    }

    fn require(condition: bool, message: &'static str) -> Result<(), Box<dyn std::error::Error>> {
        if condition {
            Ok(())
        } else {
            Err(std::io::Error::other(message).into())
        }
    }

    #[test]
    fn parse_batch_rejects_invalid_utf8_before_segment_processing()
    -> Result<(), Box<dyn std::error::Error>> {
        let result = parse_batch(&[0xff, 0xfe, 0xfd]);

        require(
            matches!(
                result,
                Err(BatchError::InvalidStructure(message)) if message == "Invalid UTF-8 data"
            ),
            "invalid UTF-8 should fail before segment processing",
        )?;

        Ok(())
    }

    #[test]
    fn parse_batch_reports_unknown_first_segment_prefix() -> Result<(), Box<dyn std::error::Error>>
    {
        let result = parse_batch(b"ZZ\r");

        require(
            matches!(
                result,
                Err(BatchError::InvalidStructure(message)) if message == "Unknown first segment: ZZ"
            ),
            "unknown first segment should report the prefix",
        )?;

        Ok(())
    }

    #[test]
    fn parse_single_batch_preserves_header_and_trailer_metadata()
    -> Result<(), Box<dyn std::error::Error>> {
        let data = format!(
            "BHS*:+\\&*SEND*SFAC*RECV*RFAC*202605030101*SEC**BATCH42*Nightly import\r{}\rBTS*1*done\r",
            message("CTRL1")
        );
        let batch = parse_batch(data.as_bytes())?;

        require_eq(batch.info.batch_type, BatchType::Single, "batch type")?;
        require_eq(batch.info.field_separator, Some('*'), "field separator")?;
        require_eq(
            batch.info.encoding_characters.as_deref(),
            Some(":+\\&"),
            "encoding characters",
        )?;
        require_eq(
            batch.info.sending_application.as_deref(),
            Some("SEND"),
            "sending application",
        )?;
        require_eq(
            batch.info.sending_facility.as_deref(),
            Some("SFAC"),
            "sending facility",
        )?;
        require_eq(
            batch.info.receiving_application.as_deref(),
            Some("RECV"),
            "receiving application",
        )?;
        require_eq(
            batch.info.receiving_facility.as_deref(),
            Some("RFAC"),
            "receiving facility",
        )?;
        require_eq(batch.info.security.as_deref(), Some("SEC"), "security")?;
        require_eq(
            batch.info.batch_name.as_deref(),
            Some("BATCH42"),
            "batch name",
        )?;
        require_eq(
            batch.info.batch_comment.as_deref(),
            Some("Nightly import"),
            "batch comment",
        )?;
        require_eq(batch.info.message_count, Some(1), "message count")?;
        require_eq(
            batch.info.trailer_comment.as_deref(),
            Some("done"),
            "trailer comment",
        )?;

        Ok(())
    }

    #[test]
    fn parse_file_batch_collects_unwrapped_messages_separately()
    -> Result<(), Box<dyn std::error::Error>> {
        let data = format!(
            "FHS|^~\\&|FILEAPP|FILEFAC|||202605030101\r{}\r{}\rFTS|2|complete\r",
            message("CTRL1"),
            message("CTRL2")
        );
        let batch = parse_batch(data.as_bytes())?;

        require_eq(batch.info.batch_type, BatchType::File, "batch type")?;
        require_eq(
            batch.info.file_creation_time.as_deref(),
            Some("202605030101"),
            "file creation time",
        )?;
        require_eq(batch.info.message_count, Some(2), "message count")?;
        require_eq(
            batch.info.trailer_comment.as_deref(),
            Some("complete"),
            "trailer comment",
        )?;
        require_eq(batch.batches.len(), 2, "implicit batch count")?;
        require_eq(batch.total_message_count(), 2, "total message count")?;

        let first_message = batch
            .batches
            .first()
            .and_then(|batch| batch.messages.first())
            .ok_or_else(|| std::io::Error::other("missing first unwrapped message"))?;
        require_eq(
            first_message.segments.len(),
            2,
            "first unwrapped message segment count",
        )?;
        require_eq(
            get(first_message, "PID.5.1"),
            Some("Doe"),
            "first unwrapped message PID-5.1",
        )?;
        require_eq(
            get(first_message, "PID.5.2"),
            Some("John"),
            "first unwrapped message PID-5.2",
        )?;

        Ok(())
    }

    #[test]
    fn parse_segment_preserves_empty_fields_with_custom_separator()
    -> Result<(), Box<dyn std::error::Error>> {
        let segment = parse_segment("BTS*2**comment")?;

        require_eq(segment.id, *b"BTS", "segment id")?;
        require_eq(segment.fields.len(), 3, "field count")?;
        require_eq(
            segment.fields.first().and_then(Field::first_text),
            Some("2"),
            "first field",
        )?;
        require_eq(
            segment.fields.get(1).and_then(Field::first_text),
            Some(""),
            "empty second field",
        )?;
        require_eq(
            segment.fields.get(2).and_then(Field::first_text),
            Some("comment"),
            "third field",
        )?;

        Ok(())
    }

    #[test]
    fn fields_after_separator_handles_short_and_multibyte_segments() {
        assert_eq!(fields_after_separator("BHS|fields"), "fields");
        assert_eq!(fields_after_separator("MSH"), "");
        assert_eq!(fields_after_separator("AAAÅ|fields"), "");
    }
}
