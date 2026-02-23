//! MLLP (Minimal Lower Layer Protocol) framing for HL7 v2.
//!
//! This crate provides functions for wrapping and unwrapping HL7 v2 messages
//! with MLLP framing, as defined in the HL7 v2 specification.
//!
//! # MLLP Protocol
//!
//! MLLP is a simple framing protocol used to transmit HL7 messages over TCP.
//! Each message is wrapped with:
//! - Start byte: `0x0B` (vertical tab)
//! - Message content (HL7 message)
//! - End bytes: `0x1C 0x0D` (file separator + carriage return)
//!
//! # Example
//!
//! ```
//! use hl7v2_mllp::{wrap_mllp, unwrap_mllp};
//!
//! let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\r";
//! let framed = wrap_mllp(hl7);
//! assert_eq!(framed[0], 0x0B); // Start byte
//! assert_eq!(framed[framed.len()-2], 0x1C); // End byte 1
//! assert_eq!(framed[framed.len()-1], 0x0D); // End byte 2
//!
//! let unwrapped = unwrap_mllp(&framed).unwrap();
//! assert_eq!(unwrapped, hl7);
//! ```

use hl7v2_model::Error;

/// MLLP start byte (vertical tab)
pub const MLLP_START: u8 = 0x0B;

/// MLLP end byte 1 (file separator)
pub const MLLP_END_1: u8 = 0x1C;

/// MLLP end byte 2 (carriage return)
pub const MLLP_END_2: u8 = 0x0D;

/// Wrap HL7 message bytes with MLLP framing.
///
/// This function adds the MLLP start and end bytes to an HL7 message.
///
/// # Arguments
///
/// * `bytes` - The HL7 message bytes to wrap
///
/// # Returns
///
/// The MLLP-framed message bytes
///
/// # Example
///
/// ```
/// use hl7v2_mllp::wrap_mllp;
///
/// let hl7 = b"MSH|^~\\&|TEST\r";
/// let framed = wrap_mllp(hl7);
/// assert_eq!(framed[0], 0x0B);
/// assert_eq!(framed[framed.len()-2], 0x1C);
/// assert_eq!(framed[framed.len()-1], 0x0D);
/// ```
pub fn wrap_mllp(bytes: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(bytes.len() + 3);
    
    // Add MLLP start byte
    buf.push(MLLP_START);
    
    // Add HL7 message content
    buf.extend_from_slice(bytes);
    
    // Add MLLP end sequence
    buf.push(MLLP_END_1);
    buf.push(MLLP_END_2);
    
    buf
}

/// Unwrap MLLP-framed bytes to extract the HL7 message.
///
/// This function removes the MLLP framing and returns the HL7 message content.
///
/// # Arguments
///
/// * `bytes` - The MLLP-framed bytes to unwrap
///
/// # Returns
///
/// The HL7 message bytes, or an error if the framing is invalid
///
/// # Example
///
/// ```
/// use hl7v2_mllp::{wrap_mllp, unwrap_mllp};
///
/// let hl7 = b"MSH|^~\\&|TEST\r";
/// let framed = wrap_mllp(hl7);
/// let unwrapped = unwrap_mllp(&framed).unwrap();
/// assert_eq!(unwrapped, hl7);
/// ```
pub fn unwrap_mllp(bytes: &[u8]) -> Result<&[u8], Error> {
    // Check if this is MLLP framed (starts with start byte)
    if bytes.is_empty() || bytes[0] != MLLP_START {
        return Err(Error::InvalidCharset); // TODO: Add specific MLLP error
    }
    
    // Find the end sequence
    let end_pos = find_mllp_end(bytes)?;
    
    // Extract the HL7 message content (excluding framing bytes)
    Ok(&bytes[1..end_pos])
}

/// Unwrap MLLP-framed bytes and return owned data.
///
/// This is a convenience function that returns an owned Vec<u8>.
///
/// # Arguments
///
/// * `bytes` - The MLLP-framed bytes to unwrap
///
/// # Returns
///
/// The HL7 message bytes as an owned Vec, or an error if the framing is invalid
pub fn unwrap_mllp_owned(bytes: &[u8]) -> Result<Vec<u8>, Error> {
    unwrap_mllp(bytes).map(|s| s.to_vec())
}

/// Find the MLLP end sequence position.
///
/// # Arguments
///
/// * `bytes` - The MLLP-framed bytes
///
/// # Returns
///
/// The position of the start of the end sequence, or an error if not found
fn find_mllp_end(bytes: &[u8]) -> Result<usize, Error> {
    // Look for the end sequence (0x1C 0x0D)
    for i in 0..bytes.len().saturating_sub(1) {
        if bytes[i] == MLLP_END_1 && bytes[i + 1] == MLLP_END_2 {
            return Ok(i);
        }
    }
    Err(Error::InvalidCharset) // TODO: Add specific MLLP error
}

/// Check if bytes are MLLP-framed.
///
/// # Arguments
///
/// * `bytes` - The bytes to check
///
/// # Returns
///
/// `true` if the bytes appear to be MLLP-framed
pub fn is_mllp_framed(bytes: &[u8]) -> bool {
    !bytes.is_empty() && bytes[0] == MLLP_START
}

/// Find the end of a complete MLLP message in a buffer.
///
/// This is useful for streaming scenarios where you need to determine
/// if a complete MLLP message has been received.
///
/// # Arguments
///
/// * `bytes` - The buffer to search
///
/// # Returns
///
/// `Some(len)` if a complete MLLP message is found, where `len` is the
/// total length of the framed message (including start and end bytes).
/// Returns `None` if no complete message is found.
pub fn find_complete_mllp_message(bytes: &[u8]) -> Option<usize> {
    // Check for start byte
    if bytes.is_empty() || bytes[0] != MLLP_START {
        return None;
    }
    
    // Look for the end sequence
    for i in 1..bytes.len().saturating_sub(1) {
        if bytes[i] == MLLP_END_1 && bytes[i + 1] == MLLP_END_2 {
            // Return the total length including end bytes
            return Some(i + 2);
        }
    }
    
    None
}

/// An MLLP frame iterator for streaming scenarios.
///
/// This struct helps process a stream of bytes that may contain multiple
/// MLLP-framed messages.
#[derive(Debug, Default)]
pub struct MllpFrameIterator {
    buffer: Vec<u8>,
}

impl MllpFrameIterator {
    /// Create a new MLLP frame iterator.
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
        }
    }
    
    /// Add bytes to the internal buffer.
    pub fn extend(&mut self, bytes: &[u8]) {
        self.buffer.extend_from_slice(bytes);
    }
    
    /// Try to extract the next complete MLLP frame.
    ///
    /// Returns `Some(frame)` if a complete frame is available,
    /// or `None` if more data is needed.
    pub fn next_frame(&mut self) -> Option<Vec<u8>> {
        let total_len = find_complete_mllp_message(&self.buffer)?;
        
        // Extract the frame
        let frame: Vec<u8> = self.buffer.drain(..total_len).collect();
        Some(frame)
    }
    
    /// Try to extract the next complete MLLP frame and unwrap it.
    ///
    /// Returns `Some(message)` if a complete frame is available,
    /// or `None` if more data is needed.
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

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_wrap_mllp() {
        let hl7 = b"MSH|^~\\&|TEST\r";
        let framed = wrap_mllp(hl7);
        
        assert_eq!(framed[0], MLLP_START);
        assert_eq!(framed[framed.len() - 2], MLLP_END_1);
        assert_eq!(framed[framed.len() - 1], MLLP_END_2);
        
        // Check content
        assert_eq!(&framed[1..framed.len() - 2], hl7);
    }
    
    #[test]
    fn test_unwrap_mllp() {
        let hl7 = b"MSH|^~\\&|TEST\r";
        let framed = wrap_mllp(hl7);
        let unwrapped = unwrap_mllp(&framed).unwrap();
        assert_eq!(unwrapped, hl7);
    }
    
    #[test]
    fn test_unwrap_mllp_invalid() {
        // No start byte
        let result = unwrap_mllp(b"MSH|TEST");
        assert!(result.is_err());
    }
    
    #[test]
    fn test_is_mllp_framed() {
        let hl7 = b"MSH|^~\\&|TEST\r";
        let framed = wrap_mllp(hl7);
        
        assert!(is_mllp_framed(&framed));
        assert!(!is_mllp_framed(hl7));
        assert!(!is_mllp_framed(b""));
    }
    
    #[test]
    fn test_find_complete_mllp_message() {
        let hl7 = b"MSH|^~\\&|TEST\r";
        let framed = wrap_mllp(hl7);
        
        let len = find_complete_mllp_message(&framed).unwrap();
        assert_eq!(len, framed.len());
        
        // Incomplete message
        assert!(find_complete_mllp_message(&framed[..5]).is_none());
        
        // No start byte
        assert!(find_complete_mllp_message(hl7).is_none());
    }
    
    #[test]
    fn test_frame_iterator() {
        let mut iter = MllpFrameIterator::new();
        
        let hl7 = b"MSH|^~\\&|TEST\r";
        let framed = wrap_mllp(hl7);
        
        // Add the framed message
        iter.extend(&framed);
        
        // Extract the message
        let msg = iter.next_message().unwrap().unwrap();
        assert_eq!(&msg, hl7);
    }
    
    #[test]
    fn test_frame_iterator_multiple() {
        let mut iter = MllpFrameIterator::new();
        
        let hl7_1 = b"MSH|^~\\&|TEST1\r";
        let hl7_2 = b"MSH|^~\\&|TEST2\r";
        let framed_1 = wrap_mllp(hl7_1);
        let framed_2 = wrap_mllp(hl7_2);
        
        // Add both messages
        iter.extend(&framed_1);
        iter.extend(&framed_2);
        
        // Extract first message
        let msg_1 = iter.next_message().unwrap().unwrap();
        assert_eq!(&msg_1, hl7_1);
        
        // Extract second message
        let msg_2 = iter.next_message().unwrap().unwrap();
        assert_eq!(&msg_2, hl7_2);
        
        // No more messages
        assert!(iter.next_message().is_none());
    }
    
    #[test]
    fn test_frame_iterator_partial() {
        let mut iter = MllpFrameIterator::new();
        
        let hl7 = b"MSH|^~\\&|TEST\r";
        let framed = wrap_mllp(hl7);
        
        // Add partial message
        iter.extend(&framed[..5]);
        assert!(iter.next_message().is_none());
        
        // Add the rest
        iter.extend(&framed[5..]);
        
        // Now we can extract
        let msg = iter.next_message().unwrap().unwrap();
        assert_eq!(&msg, hl7);
    }
}