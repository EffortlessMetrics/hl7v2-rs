use crate::model::Error;

use super::{MllpError, find_complete_mllp_message, unwrap_mllp_owned};

/// Default maximum size of the buffered MLLP input (10 MiB).
const DEFAULT_MAX_BUFFER_SIZE: usize = 10 * 1024 * 1024;

/// An MLLP frame iterator for streaming scenarios.
#[derive(Debug)]
pub struct MllpFrameIterator {
    buffer: Vec<u8>,
    max_buffer_size: usize,
}

impl MllpFrameIterator {
    /// Create a new MLLP frame iterator.
    pub fn new() -> Self {
        Self::with_max_buffer_size(DEFAULT_MAX_BUFFER_SIZE)
    }

    /// Create a new MLLP frame iterator with a custom buffer limit.
    pub fn with_max_buffer_size(max_buffer_size: usize) -> Self {
        Self {
            buffer: Vec::new(),
            max_buffer_size,
        }
    }

    /// Add bytes to the internal buffer without exceeding its configured limit.
    ///
    /// If the bytes would exceed the limit, the buffer is unchanged and the
    /// caller can recover by consuming a frame or calling [`Self::clear`].
    pub fn extend(&mut self, bytes: &[u8]) -> Result<(), MllpError> {
        let attempted_size = self.buffer.len().saturating_add(bytes.len());
        if attempted_size > self.max_buffer_size {
            return Err(MllpError::BufferLimitExceeded {
                max_size: self.max_buffer_size,
                attempted_size,
            });
        }

        self.buffer.extend_from_slice(bytes);
        Ok(())
    }

    /// Try to extract the next complete MLLP frame.
    pub fn next_frame(&mut self) -> Option<Vec<u8>> {
        let total_len = find_complete_mllp_message(&self.buffer)?;
        let frame: Vec<u8> = self.buffer.drain(..total_len).collect();
        Some(frame)
    }

    /// Try to extract the next complete MLLP frame and unwrap it.
    pub fn next_message(&mut self) -> Option<Result<Vec<u8>, Error>> {
        let frame = self.next_frame()?;
        Some(unwrap_mllp_owned(&frame))
    }

    /// Get the current buffer size.
    pub fn buffer_len(&self) -> usize {
        self.buffer.len()
    }

    /// Get the configured maximum buffer size.
    pub fn max_buffer_size(&self) -> usize {
        self.max_buffer_size
    }

    /// Clear the internal buffer.
    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

impl Default for MllpFrameIterator {
    fn default() -> Self {
        Self::new()
    }
}
