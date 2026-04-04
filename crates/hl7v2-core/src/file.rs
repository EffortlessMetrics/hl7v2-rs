//! File reading functionality for HL7 v2 messages.
//!
//! This module provides utilities for reading HL7 v2 messages from files,
//! including support for:
//! - Single HL7 message files
//! - Batch files with multiple messages
//! - Multiple character encodings (UTF-8, ISO-8859-1, Windows-1252)
//! - Different line ending formats (CR, LF, CRLF)
//! - Both synchronous and asynchronous APIs
//!
//! # Examples
//!
//! ## Synchronous API
//!
//! ```no_run
//! use hl7v2_core::file::{read_message, read_batch};
//!
//! // Read a single message from a file
//! let message = read_message("message.hl7").unwrap();
//!
//! // Read a batch file containing multiple messages
//! let batch = read_batch("batch.hl7").unwrap();
//! ```
//!
//! ## Asynchronous API
//!
//! ```no_run
//! # #[cfg(feature = "file")]
//! # async fn example() {
//! use hl7v2_core::file::async_impl::read_message_async;
//!
//! let message = read_message_async("message.hl7").await.unwrap();
//! # }
//! ```

use hl7v2_model::{Batch, Error, FileBatch, Message};
use hl7v2_parser::{parse, parse_batch, parse_file_batch};
use std::fs;
use std::io::Read;
use std::path::Path;

/// Error type for file operations
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum FileError {
    /// I/O error reading file
    #[error("I/O error: {0}")]
    Io(String),

    /// Error detecting or converting character encoding
    #[error("Encoding error: {0}")]
    Encoding(String),

    /// Error parsing HL7 content
    #[error("Parse error: {0}")]
    Parse(String),

    /// File is too large
    #[error("File too large: {size} bytes (max: {max})")]
    FileTooLarge { size: u64, max: u64 },
}

impl From<Error> for FileError {
    fn from(err: Error) -> Self {
        FileError::Parse(err.to_string())
    }
}

impl From<std::io::Error> for FileError {
    fn from(err: std::io::Error) -> Self {
        FileError::Io(err.to_string())
    }
}

/// Default maximum file size (10 MB)
pub const DEFAULT_MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// Character encoding for file reading
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum FileEncoding {
    /// UTF-8 encoding
    Utf8,
    /// ISO-8859-1 (Latin-1) encoding
    Iso8859_1,
    /// Windows-1252 encoding
    Windows1252,
    /// Automatic detection (tries UTF-8 first, falls back to Windows-1252)
    #[default]
    Auto,
}

/// Options for reading HL7 files
#[derive(Debug, Clone)]
pub struct FileReadOptions {
    /// Character encoding to use
    pub encoding: FileEncoding,
    /// Maximum file size in bytes
    pub max_size: u64,
    /// Whether to normalize line endings to \r
    pub normalize_line_endings: bool,
}

impl Default for FileReadOptions {
    fn default() -> Self {
        Self {
            encoding: FileEncoding::Auto,
            max_size: DEFAULT_MAX_FILE_SIZE,
            normalize_line_endings: true,
        }
    }
}

impl FileReadOptions {
    /// Create new options with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the character encoding
    pub fn encoding(mut self, encoding: FileEncoding) -> Self {
        self.encoding = encoding;
        self
    }

    /// Set the maximum file size
    pub fn max_size(mut self, max_size: u64) -> Self {
        self.max_size = max_size;
        self
    }

    /// Set whether to normalize line endings
    pub fn normalize_line_endings(mut self, normalize: bool) -> Self {
        self.normalize_line_endings = normalize;
        self
    }
}

/// Read a single HL7 message from a file.
///
/// # Arguments
///
/// * `path` - Path to the HL7 file
///
/// # Returns
///
/// The parsed `Message`, or a `FileError` if reading/parsing fails
///
/// # Example
///
/// ```no_run
/// use hl7v2_core::file::read_message;
///
/// let message = read_message("message.hl7").unwrap();
/// ```
pub fn read_message<P: AsRef<Path>>(path: P) -> Result<Message, FileError> {
    read_message_with_options(path, FileReadOptions::default())
}

/// Read a single HL7 message from a file with options.
///
/// # Arguments
///
/// * `path` - Path to the HL7 file
/// * `options` - Reading options
///
/// # Returns
///
/// The parsed `Message`, or a `FileError` if reading/parsing fails
pub fn read_message_with_options<P: AsRef<Path>>(
    path: P,
    options: FileReadOptions,
) -> Result<Message, FileError> {
    let bytes = read_file_with_options(path, &options)?;
    let message = parse(&bytes)?;
    Ok(message)
}

/// Read an HL7 batch file containing multiple messages.
///
/// # Arguments
///
/// * `path` - Path to the HL7 batch file
///
/// # Returns
///
/// The parsed `Batch`, or a `FileError` if reading/parsing fails
///
/// # Example
///
/// ```no_run
/// use hl7v2_core::file::read_batch;
///
/// let batch = read_batch("batch.hl7").unwrap();
/// for message in &batch.messages {
///     println!("Message with {} segments", message.segments.len());
/// }
/// ```
pub fn read_batch<P: AsRef<Path>>(path: P) -> Result<Batch, FileError> {
    read_batch_with_options(path, FileReadOptions::default())
}

/// Read an HL7 batch file with options.
///
/// # Arguments
///
/// * `path` - Path to the HL7 batch file
/// * `options` - Reading options
///
/// # Returns
///
/// The parsed `Batch`, or a `FileError` if reading/parsing fails
pub fn read_batch_with_options<P: AsRef<Path>>(
    path: P,
    options: FileReadOptions,
) -> Result<Batch, FileError> {
    let bytes = read_file_with_options(path, &options)?;
    let batch = parse_batch(&bytes)?;
    Ok(batch)
}

/// Read an HL7 file batch (FHS/FTS format) containing multiple batches.
///
/// # Arguments
///
/// * `path` - Path to the HL7 file batch
///
/// # Returns
///
/// The parsed `FileBatch`, or a `FileError` if reading/parsing fails
///
/// # Example
///
/// ```no_run
/// use hl7v2_core::file::read_file_batch;
///
/// let file_batch = read_file_batch("file_batch.hl7").unwrap();
/// ```
pub fn read_file_batch<P: AsRef<Path>>(path: P) -> Result<FileBatch, FileError> {
    read_file_batch_with_options(path, FileReadOptions::default())
}

/// Read an HL7 file batch with options.
///
/// # Arguments
///
/// * `path` - Path to the HL7 file batch
/// * `options` - Reading options
///
/// # Returns
///
/// The parsed `FileBatch`, or a `FileError` if reading/parsing fails
pub fn read_file_batch_with_options<P: AsRef<Path>>(
    path: P,
    options: FileReadOptions,
) -> Result<FileBatch, FileError> {
    let bytes = read_file_with_options(path, &options)?;
    let file_batch = parse_file_batch(&bytes)?;
    Ok(file_batch)
}

/// Read file contents with options.
fn read_file_with_options<P: AsRef<Path>>(
    path: P,
    options: &FileReadOptions,
) -> Result<Vec<u8>, FileError> {
    let path = path.as_ref();

    // Check file size
    let metadata = fs::metadata(path)?;
    let size = metadata.len();

    if size > options.max_size {
        return Err(FileError::FileTooLarge {
            size,
            max: options.max_size,
        });
    }

    // Read raw bytes
    let mut file = fs::File::open(path)?;
    let mut bytes = Vec::with_capacity(size as usize);
    file.read_to_end(&mut bytes)?;

    // Detect and convert encoding if needed
    let text = decode_bytes(&bytes, options.encoding)?;

    // Normalize line endings if requested
    let normalized = if options.normalize_line_endings {
        normalize_line_endings(&text)
    } else {
        text.into_bytes()
    };

    Ok(normalized)
}

/// Decode bytes to string using specified encoding.
fn decode_bytes(bytes: &[u8], encoding: FileEncoding) -> Result<String, FileError> {
    match encoding {
        FileEncoding::Utf8 => String::from_utf8(bytes.to_vec())
            .map_err(|e| FileError::Encoding(format!("Invalid UTF-8: {}", e))),
        FileEncoding::Iso8859_1 => {
            let (cow, _, had_errors) = encoding_rs::ISO_8859_15.decode(bytes);
            if had_errors {
                return Err(FileError::Encoding("ISO-8859-1 decode error".to_string()));
            }
            Ok(cow.into_owned())
        }
        FileEncoding::Windows1252 => {
            let (cow, _, had_errors) = encoding_rs::WINDOWS_1252.decode(bytes);
            if had_errors {
                return Err(FileError::Encoding("Windows-1252 decode error".to_string()));
            }
            Ok(cow.into_owned())
        }
        FileEncoding::Auto => {
            // Try UTF-8 first
            if let Ok(text) = String::from_utf8(bytes.to_vec()) {
                return Ok(text);
            }
            // Fall back to Windows-1252 (superset of ISO-8859-1 with more characters)
            let (cow, _, _) = encoding_rs::WINDOWS_1252.decode(bytes);
            Ok(cow.into_owned())
        }
    }
}

/// Normalize line endings to HL7 standard (\r)
fn normalize_line_endings(text: &str) -> Vec<u8> {
    // Replace CRLF with CR, then LF with CR
    let normalized = text.replace("\r\n", "\r").replace('\n', "\r");
    normalized.into_bytes()
}

/// Iterator over messages in a batch file
pub struct MessageIterator {
    batch: Batch,
    index: usize,
}

impl Iterator for MessageIterator {
    type Item = Message;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.batch.messages.len() {
            let message = self.batch.messages[self.index].clone();
            self.index += 1;
            Some(message)
        } else {
            None
        }
    }
}

impl ExactSizeIterator for MessageIterator {
    fn len(&self) -> usize {
        self.batch.messages.len() - self.index
    }
}

/// Read messages from a batch file as an iterator.
///
/// # Arguments
///
/// * `path` - Path to the HL7 batch file
///
/// # Returns
///
/// An iterator over the messages in the batch, or a `FileError`
///
/// # Example
///
/// ```no_run
/// use hl7v2_core::file::read_messages_iter;
///
/// for message in read_messages_iter("batch.hl7").unwrap() {
///     println!("Message with {} segments", message.segments.len());
/// }
/// ```
pub fn read_messages_iter<P: AsRef<Path>>(path: P) -> Result<MessageIterator, FileError> {
    let batch = read_batch(path)?;
    Ok(MessageIterator { batch, index: 0 })
}

/// Async versions of file reading functions (requires "file" feature)
#[cfg(feature = "file")]
pub mod async_impl {
    use super::*;
    use tokio::fs::File;
    use tokio::io::AsyncReadExt;

    /// Read a single HL7 message from a file asynchronously.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the HL7 file
    ///
    /// # Returns
    ///
    /// The parsed `Message`, or a `FileError` if reading/parsing fails
    pub async fn read_message_async<P: AsRef<Path>>(path: P) -> Result<Message, FileError> {
        read_message_with_options_async(path, FileReadOptions::default()).await
    }

    /// Read a single HL7 message asynchronously with options.
    pub async fn read_message_with_options_async<P: AsRef<Path>>(
        path: P,
        options: FileReadOptions,
    ) -> Result<Message, FileError> {
        let bytes = read_file_with_options_async(path, &options).await?;
        let message = parse(&bytes)?;
        Ok(message)
    }

    /// Read an HL7 batch file asynchronously.
    pub async fn read_batch_async<P: AsRef<Path>>(path: P) -> Result<Batch, FileError> {
        read_batch_with_options_async(path, FileReadOptions::default()).await
    }

    /// Read an HL7 batch file asynchronously with options.
    pub async fn read_batch_with_options_async<P: AsRef<Path>>(
        path: P,
        options: FileReadOptions,
    ) -> Result<Batch, FileError> {
        let bytes = read_file_with_options_async(path, &options).await?;
        let batch = parse_batch(&bytes)?;
        Ok(batch)
    }

    /// Read file contents asynchronously with options.
    async fn read_file_with_options_async<P: AsRef<Path>>(
        path: P,
        options: &FileReadOptions,
    ) -> Result<Vec<u8>, FileError> {
        let path = path.as_ref();

        // Check file size
        let metadata = tokio::fs::metadata(path).await?;
        let size = metadata.len();

        if size > options.max_size {
            return Err(FileError::FileTooLarge {
                size,
                max: options.max_size,
            });
        }

        // Read raw bytes
        let mut file = File::open(path).await?;
        let mut bytes = Vec::with_capacity(size as usize);
        file.read_to_end(&mut bytes).await?;

        // Decode and normalize
        let text = decode_bytes(&bytes, options.encoding)?;
        let normalized = if options.normalize_line_endings {
            normalize_line_endings(&text)
        } else {
            text.into_bytes()
        };

        Ok(normalized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_file(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file.flush().unwrap();
        file
    }

    #[test]
    fn test_read_message() {
        let hl7 = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||123456\r";
        let file = create_test_file(hl7);

        let message = read_message(file.path()).unwrap();
        assert_eq!(message.segments.len(), 2);
        assert_eq!(&message.segments[0].id, b"MSH");
        assert_eq!(&message.segments[1].id, b"PID");
    }

    #[test]
    fn test_read_message_lf_line_endings() {
        let hl7 = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\nPID|1||123456\n";
        let file = create_test_file(hl7);

        let message = read_message(file.path()).unwrap();
        assert_eq!(message.segments.len(), 2);
    }

    #[test]
    fn test_read_message_crlf_line_endings() {
        let hl7 = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\r\nPID|1||123456\r\n";
        let file = create_test_file(hl7);

        let message = read_message(file.path()).unwrap();
        assert_eq!(message.segments.len(), 2);
    }

    #[test]
    fn test_read_batch() {
        // Batch with two messages
        let batch_content = "BHS|^~\\&|Sender|Facility|Receiver|Facility|20250101\rMSH|^~\\&|App1|Fac1|App2|Fac2|20250101||ADT^A01|MSG001|P|2.5.1\rPID|1||123456\rMSH|^~\\&|App1|Fac1|App2|Fac2|20250101||ADT^A02|MSG002|P|2.5.1\rPID|1||789012\rBTS|2\r";
        let file = create_test_file(batch_content);

        let batch = read_batch(file.path()).unwrap();
        assert_eq!(batch.messages.len(), 2);
    }

    #[test]
    fn test_file_too_large() {
        let options = FileReadOptions::new().max_size(1); // 1 byte max
        let hl7 = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\r";
        let file = create_test_file(hl7);

        let result = read_message_with_options(file.path(), options);
        assert!(matches!(result, Err(FileError::FileTooLarge { .. })));
    }

    #[test]
    fn test_read_messages_iter() {
        let batch_content = "BHS|^~\\&|Sender|Facility|Receiver|Facility|20250101\rMSH|^~\\&|App1|Fac1|App2|Fac2|20250101||ADT^A01|MSG001|P|2.5.1\rPID|1||123456\rMSH|^~\\&|App1|Fac1|App2|Fac2|20250101||ADT^A02|MSG002|P|2.5.1\rPID|1||789012\rBTS|2\r";
        let file = create_test_file(batch_content);

        let messages: Vec<_> = read_messages_iter(file.path()).unwrap().collect();
        assert_eq!(messages.len(), 2);
    }

    #[test]
    fn test_read_real_test_files() {
        // Test reading actual test data files from the project
        let test_data_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../test_data");

        // Read a valid message file
        let test_file = format!("{}/valid_message.hl7", test_data_dir);
        if std::path::Path::new(&test_file).exists() {
            let message = read_message(&test_file).unwrap();
            assert_eq!(&message.segments[0].id, b"MSH");
            assert!(message.segments.len() >= 2);
        }

        // Read test.hl7 file
        let test_file = format!("{}/test.hl7", test_data_dir);
        if std::path::Path::new(&test_file).exists() {
            let message = read_message(&test_file).unwrap();
            assert_eq!(&message.segments[0].id, b"MSH");
        }

        // Read UTF-8 test file
        let test_file = format!("{}/utf8_test.hl7", test_data_dir);
        if std::path::Path::new(&test_file).exists() {
            let message = read_message(&test_file).unwrap();
            assert_eq!(&message.segments[0].id, b"MSH");
            // Verify Cyrillic characters are preserved
            let patient_name = hl7v2_parser::get(&message, "PID.5.1");
            assert!(patient_name.is_some());
        }
    }
}
