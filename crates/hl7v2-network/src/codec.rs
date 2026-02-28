//! MLLP (Minimal Lower Layer Protocol) codec for encoding and decoding HL7 messages.
//!
//! MLLP wraps HL7 messages with framing characters:
//! - Start: 0x0B (vertical tab)
//! - End: 0x1C 0x0D (file separator + carriage return)

use bytes::{Buf, BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

/// MLLP frame start byte (vertical tab)
const MLLP_START: u8 = 0x0B;

/// MLLP frame end byte 1 (file separator)
const MLLP_END_1: u8 = 0x1C;

/// MLLP frame end byte 2 (carriage return)
const MLLP_END_2: u8 = 0x0D;

/// Maximum size for a single MLLP frame (10MB)
const MAX_FRAME_SIZE: usize = 10 * 1024 * 1024;

/// MLLP codec for encoding and decoding HL7 messages with MLLP framing.
///
/// # Examples
///
/// ```no_run
/// use tokio_util::codec::Framed;
/// use tokio::net::TcpStream;
/// # use hl7v2_network::MllpCodec;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let stream = TcpStream::connect("127.0.0.1:2575").await?;
/// let mut framed = Framed::new(stream, MllpCodec::new());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Default)]
pub struct MllpCodec {
    /// Maximum allowed frame size
    max_frame_size: usize,
}

impl MllpCodec {
    /// Create a new MLLP codec with default settings.
    pub fn new() -> Self {
        Self {
            max_frame_size: MAX_FRAME_SIZE,
        }
    }

    /// Create a new MLLP codec with a custom maximum frame size.
    pub fn with_max_frame_size(max_frame_size: usize) -> Self {
        Self { max_frame_size }
    }
}

impl Decoder for MllpCodec {
    type Item = BytesMut;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Need at least 3 bytes: start byte + at least one content byte + end bytes
        if src.len() < 3 {
            return Ok(None);
        }

        // Find the start byte
        let start_pos = match src.iter().position(|&b| b == MLLP_START) {
            Some(pos) => pos,
            None => {
                // No start byte found, discard all data
                src.clear();
                return Ok(None);
            }
        };

        // If start byte is not at position 0, discard junk data before it
        if start_pos > 0 {
            src.advance(start_pos);
        }

        // Now src[0] should be MLLP_START
        // Look for the end sequence starting from position 1
        // position() returns the index in src[1..] where the end sequence starts
        // This is also the length of the content
        let end_pos = src[1..]
            .windows(2)
            .position(|window| window[0] == MLLP_END_1 && window[1] == MLLP_END_2);

        match end_pos {
            Some(pos) => {
                // pos is the index in src[1..] where we found the end sequence
                // So the actual content length is just pos (not including the start byte)
                let content_len = pos;

                // Check frame size before allocation
                if content_len > self.max_frame_size {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!(
                            "Frame size {} exceeds maximum {}",
                            content_len, self.max_frame_size
                        ),
                    ));
                }

                // We have a complete frame
                // Extract the content (excluding framing bytes)
                // Frame structure: [MLLP_START][content][MLLP_END_1][MLLP_END_2]
                // pos is relative to src[1..], so in original src:
                // - MLLP_START is at index 0
                // - content is at indices 1..=pos
                // - MLLP_END_1 is at index pos+1
                // - MLLP_END_2 is at index pos+2

                // Advance past the start byte
                src.advance(1);

                // Now split off just the content (pos bytes)
                let content = src.split_to(content_len);

                // Advance past the end sequence (MLLP_END_1 + MLLP_END_2)
                src.advance(2);

                Ok(Some(content))
            }
            None => {
                // Check if buffer is getting too large while waiting for end sequence
                if src.len() > self.max_frame_size {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!(
                            "Buffer size {} exceeds maximum {}",
                            src.len(),
                            self.max_frame_size
                        ),
                    ));
                }

                // Not enough data yet, need to wait for more
                Ok(None)
            }
        }
    }
}

impl Encoder<BytesMut> for MllpCodec {
    type Error = std::io::Error;

    fn encode(&mut self, item: BytesMut, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // Check that the message isn't too large
        if item.len() > self.max_frame_size {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "Message size {} exceeds maximum {}",
                    item.len(),
                    self.max_frame_size
                ),
            ));
        }

        // Reserve space: start byte + content + 2 end bytes
        dst.reserve(1 + item.len() + 2);

        // Write MLLP framing
        dst.put_u8(MLLP_START);
        dst.put(item);
        dst.put_u8(MLLP_END_1);
        dst.put_u8(MLLP_END_2);

        Ok(())
    }
}

impl Encoder<&[u8]> for MllpCodec {
    type Error = std::io::Error;

    fn encode(&mut self, item: &[u8], dst: &mut BytesMut) -> Result<(), Self::Error> {
        // Check that the message isn't too large
        if item.len() > self.max_frame_size {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "Message size {} exceeds maximum {}",
                    item.len(),
                    self.max_frame_size
                ),
            ));
        }

        // Reserve space: start byte + content + 2 end bytes
        dst.reserve(1 + item.len() + 2);

        // Write MLLP framing
        dst.put_u8(MLLP_START);
        dst.put_slice(item);
        dst.put_u8(MLLP_END_1);
        dst.put_u8(MLLP_END_2);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode() {
        let mut codec = MllpCodec::new();
        let mut dst = BytesMut::new();
        let msg = BytesMut::from("MSH|^~\\&|TEST\r");

        codec.encode(msg, &mut dst).unwrap();

        assert_eq!(dst[0], MLLP_START);
        assert_eq!(dst[dst.len() - 2], MLLP_END_1);
        assert_eq!(dst[dst.len() - 1], MLLP_END_2);
        assert_eq!(&dst[1..dst.len() - 2], b"MSH|^~\\&|TEST\r");
    }

    #[test]
    fn test_decode() {
        let mut codec = MllpCodec::new();
        let mut src = BytesMut::from(&b"\x0BMSH|^~\\&|TEST\r\x1C\x0D"[..]);

        let result = codec.decode(&mut src).unwrap();
        assert!(result.is_some());

        let content = result.unwrap();
        assert_eq!(&content[..], b"MSH|^~\\&|TEST\r");
    }

    #[test]
    fn test_decode_incomplete() {
        let mut codec = MllpCodec::new();
        let mut src = BytesMut::from(&b"\x0BMSH|^~\\&|TEST\r"[..]);

        let result = codec.decode(&mut src).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_decode_with_junk_before() {
        let mut codec = MllpCodec::new();
        let mut src = BytesMut::from(&b"JUNK\x0BMSH|^~\\&|TEST\r\x1C\x0D"[..]);

        let result = codec.decode(&mut src).unwrap();
        assert!(result.is_some());

        let content = result.unwrap();
        assert_eq!(&content[..], b"MSH|^~\\&|TEST\r");
    }

    #[test]
    fn test_decode_no_start_byte() {
        let mut codec = MllpCodec::new();
        let mut src = BytesMut::from(&b"MSH|^~\\&|TEST\r\x1C\x0D"[..]);

        let result = codec.decode(&mut src).unwrap();
        assert!(result.is_none());
        assert_eq!(src.len(), 0); // Should discard all data
    }

    #[test]
    fn test_max_frame_size() {
        let mut codec = MllpCodec::with_max_frame_size(10);
        let mut dst = BytesMut::new();
        let large_msg = BytesMut::from(&b"12345678901"[..]); // 11 bytes, exceeds limit

        let result = codec.encode(large_msg, &mut dst);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_multiple_frames() {
        let mut codec = MllpCodec::new();
        let mut src = BytesMut::from(&b"\x0BMSG1\r\x1C\x0D\x0BMSG2\r\x1C\x0D"[..]);

        // Decode first frame
        let result1 = codec.decode(&mut src).unwrap();
        assert!(result1.is_some());
        assert_eq!(&result1.unwrap()[..], b"MSG1\r");

        // Decode second frame
        let result2 = codec.decode(&mut src).unwrap();
        assert!(result2.is_some());
        assert_eq!(&result2.unwrap()[..], b"MSG2\r");

        // No more frames
        let result3 = codec.decode(&mut src).unwrap();
        assert!(result3.is_none());
    }
}
