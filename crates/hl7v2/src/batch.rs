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
    let mut in_batch = false;
    let mut has_fhs = false;

    for line in lines {
        if line.starts_with("FHS") {
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

    // Validate that FHS is present for file batches
    if !has_fhs {
        return Err(BatchError::MissingSegment("FHS".to_string()));
    }

    // If message_count is not set from FTS, calculate from batches
    if file_batch.info.message_count.is_none() {
        file_batch.info.message_count = Some(file_batch.total_message_count());
    }

    Ok(file_batch)
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

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn ensure(condition: bool, message: &'static str) -> TestResult {
        if condition {
            Ok(())
        } else {
            Err(std::io::Error::other(message).into())
        }
    }

    fn sample_message() -> Result<Message, ModelError> {
        parse(b"MSH|^~\\&|APP|FAC|RECV|RECVFAC|20250128120000||ADT^A01|MSG001|P|2.5.1\r")
    }

    #[test]
    fn batch_type_equality_distinguishes_single_and_file() -> TestResult {
        ensure(BatchType::Single == BatchType::Single, "Single == Single")?;
        ensure(BatchType::File == BatchType::File, "File == File")?;
        ensure(BatchType::Single != BatchType::File, "Single != File")
    }

    #[test]
    fn batch_info_default_uses_single_batch_type() -> TestResult {
        let info = BatchInfo::default();
        ensure(
            info.batch_type == BatchType::Single,
            "default BatchInfo should be Single",
        )?;
        ensure(info.field_separator.is_none(), "no field separator")?;
        ensure(info.message_count.is_none(), "no message count")
    }

    #[test]
    fn batch_new_default_match_and_count_zero() -> TestResult {
        let new_batch = Batch::new();
        let default_batch = Batch::default();
        ensure(new_batch == default_batch, "default matches new")?;
        ensure(new_batch.message_count() == 0, "empty count zero")?;
        ensure(
            new_batch.iter_messages().count() == 0,
            "iter yields nothing",
        )
    }

    #[test]
    fn batch_add_message_increments_count_and_iter_yields_order() -> TestResult {
        let mut batch = Batch::new();
        let m1 = sample_message()?;
        let m2 = sample_message()?;
        batch.add_message(m1);
        batch.add_message(m2);
        ensure(batch.message_count() == 2, "count after adds")?;
        ensure(batch.iter_messages().count() == 2, "iter count after adds")
    }

    #[test]
    fn file_batch_new_defaults_to_file_batch_type() -> TestResult {
        let fb = FileBatch::new();
        ensure(
            fb.info.batch_type == BatchType::File,
            "FileBatch defaults to File type",
        )?;
        ensure(fb.batches.is_empty(), "no batches")?;
        ensure(fb.header.is_none(), "no header")?;
        ensure(fb.trailer.is_none(), "no trailer")
    }

    #[test]
    fn file_batch_total_message_count_sums_nested_batches() -> TestResult {
        let mut fb = FileBatch::new();
        let mut b1 = Batch::new();
        b1.add_message(sample_message()?);
        b1.add_message(sample_message()?);
        let mut b2 = Batch::new();
        b2.add_message(sample_message()?);
        fb.add_batch(b1);
        fb.add_batch(b2);
        ensure(fb.total_message_count() == 3, "total sums to 3")?;
        ensure(fb.batches.len() == 2, "two batches")
    }

    #[test]
    fn file_batch_iter_all_messages_yields_insertion_order() -> TestResult {
        let mut fb = FileBatch::new();
        let mut b1 = Batch::new();
        b1.add_message(sample_message()?);
        let mut b2 = Batch::new();
        b2.add_message(sample_message()?);
        b2.add_message(sample_message()?);
        fb.add_batch(b1);
        fb.add_batch(b2);
        ensure(fb.iter_all_messages().count() == 3, "iter_all sees 3")
    }

    #[test]
    fn parse_batch_rejects_empty_input() -> TestResult {
        let result = parse_batch(b"");
        ensure(
            matches!(result, Err(BatchError::InvalidStructure(_))),
            "empty input should be InvalidStructure",
        )
    }

    #[test]
    fn parse_batch_rejects_invalid_utf8() -> TestResult {
        let result = parse_batch(b"\xff\xfe");
        ensure(
            matches!(result, Err(BatchError::InvalidStructure(_))),
            "invalid UTF-8 should be InvalidStructure",
        )
    }

    #[test]
    fn parse_batch_rejects_unknown_first_segment() -> TestResult {
        let result = parse_batch(b"XYZ|foo\r");
        ensure(
            matches!(result, Err(BatchError::InvalidStructure(_))),
            "unknown first segment should be InvalidStructure",
        )
    }

    #[test]
    fn parse_batch_handles_valid_single_bhs_bts_batch() -> TestResult {
        let data = b"BHS|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128120000|||BATCH001|Test batch\r\
MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128120001||ADT^A01|MSG001|P|2.5.1\r\
PID|1||123456^^^MRN||Doe^John\r\
BTS|1|End of batch\r";
        let file_batch = parse_batch(data)?;
        ensure(
            file_batch.info.batch_type == BatchType::Single,
            "info.batch_type == Single",
        )?;
        ensure(file_batch.batches.len() == 1, "one nested batch")?;
        ensure(file_batch.total_message_count() == 1, "one message")
    }

    #[test]
    fn parse_batch_handles_valid_file_batch_with_fhs_and_fts() -> TestResult {
        let data = b"FHS|^~\\&|HIS|HOSPITAL|||20250128120000\r\
BHS|^~\\&|HIS|HOSPITAL|LAB|LABHOST|20250128120000|||LAB_BATCH\r\
MSH|^~\\&|HIS|HOSPITAL|LAB|LABHOST|20250128120100||ORM^O01|ORD001|P|2.5.1\r\
PID|1||MRN001^^^HOSP^MR||Patient^One\r\
BTS|1\r\
FTS|1\r";
        let file_batch = parse_batch(data)?;
        ensure(
            file_batch.info.batch_type == BatchType::File,
            "info.batch_type == File",
        )?;
        ensure(file_batch.batches.len() == 1, "one nested batch")?;
        ensure(file_batch.total_message_count() == 1, "one message total")?;
        ensure(
            file_batch.info.message_count == Some(1),
            "FTS-1 message count parsed",
        )
    }

    #[test]
    fn parse_batch_rejects_count_mismatch_in_bts() -> TestResult {
        let data = b"BHS|^~\\&|APP|FAC\r\
MSH|^~\\&|APP|FAC|RECV|RECVFAC|||ADT^A01|MSG|P|2.5.1\r\
BTS|5\r";
        let result = parse_batch(data);
        ensure(
            matches!(result, Err(BatchError::CountMismatch { .. })),
            "BTS count mismatch should be CountMismatch",
        )
    }

    #[test]
    fn parse_batch_accepts_bare_msh_stream() -> TestResult {
        let data = b"MSH|^~\\&|APP|FAC|RECV|RECVFAC|20250128120000||ADT^A01|MSG001|P|2.5.1\r\
PID|1||MRN001^^^HOSP^MR||Patient^One\r";
        let file_batch = parse_batch(data)?;
        ensure(file_batch.batches.len() == 1, "one implicit batch")?;
        ensure(file_batch.total_message_count() == 1, "one message")
    }

    #[test]
    fn batch_error_display_contains_key_text() -> TestResult {
        let err = BatchError::InvalidStructure("oops".to_string());
        ensure(
            err.to_string().contains("oops"),
            "InvalidStructure includes inner detail",
        )?;

        let missing = BatchError::MissingSegment("FHS".to_string());
        ensure(
            missing.to_string().contains("FHS"),
            "MissingSegment includes name",
        )?;

        let mismatch = BatchError::MismatchedHeaders;
        ensure(
            !mismatch.to_string().is_empty(),
            "MismatchedHeaders has display text",
        )?;

        let count = BatchError::CountMismatch {
            expected: 2,
            actual: 3,
        };
        let count_msg = count.to_string();
        ensure(
            count_msg.contains('2') && count_msg.contains('3'),
            "CountMismatch shows counts",
        )?;

        let parse_err = BatchError::ParseError("nope".to_string());
        ensure(
            parse_err.to_string().contains("nope"),
            "ParseError includes inner",
        )
    }

    #[test]
    fn batch_error_clone_round_trips() -> TestResult {
        let err = BatchError::CountMismatch {
            expected: 1,
            actual: 2,
        };
        let cloned = err.clone();
        ensure(
            err.to_string() == cloned.to_string(),
            "clone produces equivalent display",
        )
    }
}
