use crate::model::Error;

use super::{find_complete_mllp_message, unwrap_mllp_owned};

/// An MLLP frame iterator for streaming scenarios.
#[derive(Debug, Default)]
pub struct MllpFrameIterator {
    buffer: Vec<u8>,
}

impl MllpFrameIterator {
    /// Create a new MLLP frame iterator.
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    /// Add bytes to the internal buffer.
    pub fn extend(&mut self, bytes: &[u8]) {
        self.buffer.extend_from_slice(bytes);
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

    /// Clear the internal buffer.
    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}
