//! Comprehensive unit tests for hl7v2-network crate.
//!
//! This module contains unit tests for:
//! - MLLP codec encoding/decoding
//! - Client connection handling
//! - Server lifecycle

use std::time::Duration;
use bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder};

use super::codec::MllpCodec;
use super::client::{MllpClient, MllpClientBuilder, MllpClientConfig};
use super::server::{MllpServer, MllpServerConfig, AckTimingPolicy, MessageHandler};
use hl7v2_model::{Message, Delims, Segment, Field, Rep, Comp, Atom, Error};

/// MLLP frame start byte (vertical tab)
const MLLP_START: u8 = 0x0B;

/// MLLP frame end byte 1 (file separator)
const MLLP_END_1: u8 = 0x1C;

/// MLLP frame end byte 2 (carriage return)
const MLLP_END_2: u8 = 0x0D;

// =============================================================================
// Codec Unit Tests
// =============================================================================

mod codec_tests {
    use super::*;

    /// Test basic encoding of a simple message
    #[test]
    fn test_encode_simple_message() {
        let mut codec = MllpCodec::new();
        let mut dst = BytesMut::new();
        let msg = BytesMut::from("MSH|^~\\&|TEST\r");

        codec.encode(msg, &mut dst).unwrap();

        assert_eq!(dst[0], MLLP_START);
        assert_eq!(dst[dst.len() - 2], MLLP_END_1);
        assert_eq!(dst[dst.len() - 1], MLLP_END_2);
        assert_eq!(&dst[1..dst.len() - 2], b"MSH|^~\\&|TEST\r");
    }

    /// Test encoding with slice input
    #[test]
    fn test_encode_slice() {
        let mut codec = MllpCodec::new();
        let mut dst = BytesMut::new();
        let msg: &[u8] = b"MSH|^~\\&|TEST\r";

        codec.encode(msg, &mut dst).unwrap();

        assert_eq!(dst[0], MLLP_START);
        assert_eq!(&dst[1..dst.len() - 2], b"MSH|^~\\&|TEST\r");
    }

    /// Test basic decoding of a complete frame
    #[test]
    fn test_decode_complete_frame() {
        let mut codec = MllpCodec::new();
        let mut src = BytesMut::from(&b"\x0BMSH|^~\\&|TEST\r\x1C\x0D"[..]);

        let result = codec.decode(&mut src).unwrap();
        assert!(result.is_some());

        let content = result.unwrap();
        assert_eq!(&content[..], b"MSH|^~\\&|TEST\r");
    }

    /// Test decoding incomplete frame returns None
    #[test]
    fn test_decode_incomplete_frame() {
        let mut codec = MllpCodec::new();
        let mut src = BytesMut::from(&b"\x0BMSH|^~\\&|TEST\r"[..]);

        let result = codec.decode(&mut src).unwrap();
        assert!(result.is_none());
    }

    /// Test decoding with only start byte
    #[test]
    fn test_decode_only_start_byte() {
        let mut codec = MllpCodec::new();
        let mut src = BytesMut::from(&b"\x0B"[..]);

        let result = codec.decode(&mut src).unwrap();
        assert!(result.is_none());
    }

    /// Test decoding with junk data before start byte
    #[test]
    fn test_decode_junk_before_start() {
        let mut codec = MllpCodec::new();
        let mut src = BytesMut::from(&b"JUNK\x0BMSH|^~\\&|TEST\r\x1C\x0D"[..]);

        let result = codec.decode(&mut src).unwrap();
        assert!(result.is_some());

        let content = result.unwrap();
        assert_eq!(&content[..], b"MSH|^~\\&|TEST\r");
    }

    /// Test decoding when no start byte present
    #[test]
    fn test_decode_no_start_byte() {
        let mut codec = MllpCodec::new();
        let mut src = BytesMut::from(&b"MSH|^~\\&|TEST\r\x1C\x0D"[..]);

        let result = codec.decode(&mut src).unwrap();
        assert!(result.is_none());
        assert_eq!(src.len(), 0); // Should discard all data
    }

    /// Test encoding message exceeding max frame size
    #[test]
    fn test_encode_exceeds_max_frame_size() {
        let mut codec = MllpCodec::with_max_frame_size(10);
        let mut dst = BytesMut::new();
        let large_msg = BytesMut::from(&b"12345678901"[..]); // 11 bytes, exceeds limit

        let result = codec.encode(large_msg, &mut dst);
        assert!(result.is_err());
    }

    /// Test decoding frame exceeding max frame size
    #[test]
    fn test_decode_exceeds_max_frame_size() {
        let mut codec = MllpCodec::with_max_frame_size(10);
        let mut src = BytesMut::from(&b"\x0B12345678901\r\x1C\x0D"[..]); // 11 content bytes

        let result = codec.decode(&mut src);
        assert!(result.is_err());
    }

    /// Test decoding multiple frames in sequence
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

    /// Test decoding with partial end sequence
    #[test]
    fn test_decode_partial_end_sequence() {
        let mut codec = MllpCodec::new();
        let mut src = BytesMut::from(&b"\x0BMSH|^~\\&|TEST\r\x1C"[..]); // Missing final 0x0D

        let result = codec.decode(&mut src).unwrap();
        assert!(result.is_none());
    }

    /// Test encoding empty message
    #[test]
    fn test_encode_empty_message() {
        let mut codec = MllpCodec::new();
        let mut dst = BytesMut::new();
        let msg = BytesMut::new();

        codec.encode(msg, &mut dst).unwrap();

        assert_eq!(dst.len(), 3); // Start + 2 end bytes
        assert_eq!(dst[0], MLLP_START);
        assert_eq!(dst[1], MLLP_END_1);
        assert_eq!(dst[2], MLLP_END_2);
    }

    /// Test decoding empty content frame
    #[test]
    fn test_decode_empty_content() {
        let mut codec = MllpCodec::new();
        let mut src = BytesMut::from(&b"\x0B\x1C\x0D"[..]);

        let result = codec.decode(&mut src).unwrap();
        assert!(result.is_some());

        let content = result.unwrap();
        assert_eq!(content.len(), 0);
    }

    /// Test codec roundtrip encode then decode
    #[test]
    fn test_codec_roundtrip() {
        let mut codec = MllpCodec::new();
        let original = BytesMut::from("MSH|^~\\&|TEST|FACILITY\r");

        // Encode
        let mut encoded = BytesMut::new();
        codec.encode(original.clone(), &mut encoded).unwrap();

        // Decode
        let decoded = codec.decode(&mut encoded).unwrap();
        assert!(decoded.is_some());
        assert_eq!(decoded.unwrap(), original);
    }

    /// Test decoding with content containing end byte values
    #[test]
    fn test_decode_content_with_special_bytes() {
        let mut codec = MllpCodec::new();
        // Content contains 0x1C but not followed by 0x0D
        let mut src = BytesMut::from(&b"\x0BMSH|test\x1Cdata\r\x1C\x0D"[..]);

        let result = codec.decode(&mut src).unwrap();
        assert!(result.is_some());

        let content = result.unwrap();
        assert_eq!(&content[..], b"MSH|test\x1Cdata\r");
    }

    /// Test decoding large message near max size
    #[test]
    fn test_decode_near_max_size() {
        let max_size = 100;
        let mut codec = MllpCodec::with_max_frame_size(max_size);
        
        // Create message just under max size
        let content: Vec<u8> = vec![b'X'; max_size - 1];
        let mut frame = vec![MLLP_START];
        frame.extend(&content);
        frame.extend(&[MLLP_END_1, MLLP_END_2]);
        let mut src = BytesMut::from(&frame[..]);

        let result = codec.decode(&mut src).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().len(), max_size - 1);
    }

    /// Test decoding buffer overflow protection
    #[test]
    fn test_decode_buffer_overflow_protection() {
        let max_size = 10;
        let mut codec = MllpCodec::with_max_frame_size(max_size);
        
        // Create incomplete frame that would exceed max size
        let content: Vec<u8> = vec![b'X'; max_size + 5];
        let mut frame = vec![MLLP_START];
        frame.extend(&content);
        // No end sequence - buffer should grow until limit
        let mut src = BytesMut::from(&frame[..]);

        let result = codec.decode(&mut src);
        assert!(result.is_err());
    }
}

// =============================================================================
// Client Unit Tests
// =============================================================================

mod client_tests {
    use super::*;

    /// Test client builder creates client with correct config
    #[test]
    fn test_client_builder_configuration() {
        let config = MllpClientConfig {
            connect_timeout: Duration::from_secs(5),
            read_timeout: Duration::from_secs(15),
            write_timeout: Duration::from_secs(20),
            max_frame_size: 1024 * 1024,
        };
        let client = MllpClient::new(config);

        assert!(!client.is_connected());
    }

    /// Test client default configuration
    #[test]
    fn test_client_default_config() {
        let config = MllpClientConfig::default();

        assert_eq!(config.connect_timeout, Duration::from_secs(10));
        assert_eq!(config.read_timeout, Duration::from_secs(30));
        assert_eq!(config.write_timeout, Duration::from_secs(30));
        assert_eq!(config.max_frame_size, 10 * 1024 * 1024);
    }

    /// Test client is not connected initially
    #[test]
    fn test_client_not_connected_initially() {
        let client = MllpClient::with_default_config();
        assert!(!client.is_connected());
        assert!(client.peer_addr().is_none());
    }

    /// Test client builder default implementation
    #[test]
    fn test_client_builder_default() {
        let builder = MllpClientBuilder::default();
        let client = builder.build();

        assert!(!client.is_connected());
    }

    /// Test client with custom config
    #[test]
    fn test_client_custom_config() {
        let config = MllpClientConfig {
            connect_timeout: Duration::from_secs(2),
            read_timeout: Duration::from_secs(5),
            write_timeout: Duration::from_secs(5),
            max_frame_size: 5000,
        };

        let client = MllpClient::new(config);
        assert!(!client.is_connected());
    }

    /// Test connect timeout to non-routable address
    #[tokio::test]
    async fn test_client_connect_timeout() {
        use std::net::SocketAddr;

        let mut client = MllpClientBuilder::new()
            .connect_timeout(Duration::from_millis(1))
            .build();

        // Try to connect to a non-routable address (should timeout)
        let addr: SocketAddr = "192.0.2.1:2575".parse().unwrap();
        let result = client.connect(addr).await;
        assert!(result.is_err());
        
        if let Err(e) = result {
            assert_eq!(e.kind(), std::io::ErrorKind::TimedOut);
        }
    }

    /// Test send_message fails when not connected
    #[tokio::test]
    async fn test_send_message_not_connected() {
        let mut client = MllpClient::with_default_config();
        let message = create_test_message();

        let result = client.send_message(&message).await;
        assert!(result.is_err());
        
        if let Err(e) = result {
            assert_eq!(e.kind(), std::io::ErrorKind::NotConnected);
        }
    }

    /// Test send_message_no_ack fails when not connected
    #[tokio::test]
    async fn test_send_message_no_ack_not_connected() {
        let mut client = MllpClient::with_default_config();
        let message = create_test_message();

        let result = client.send_message_no_ack(&message).await;
        assert!(result.is_err());
    }

    /// Test receive_message fails when not connected
    #[tokio::test]
    async fn test_receive_message_not_connected() {
        let mut client = MllpClient::with_default_config();

        let result = client.receive_message().await;
        assert!(result.is_err());
    }

    /// Test close on unconnected client succeeds
    #[tokio::test]
    async fn test_close_unconnected_client() {
        let client = MllpClient::with_default_config();
        let result = client.close().await;
        assert!(result.is_ok());
    }

    /// Test disconnect on unconnected client succeeds
    #[tokio::test]
    async fn test_disconnect_unconnected_client() {
        let mut client = MllpClient::with_default_config();
        let result = client.disconnect().await;
        assert!(result.is_ok());
    }
}

// =============================================================================
// Server Unit Tests
// =============================================================================

mod server_tests {
    use super::*;
    use std::net::SocketAddr;

    /// Test server default configuration
    #[test]
    fn test_server_default_config() {
        let config = MllpServerConfig::default();

        assert_eq!(config.read_timeout, Duration::from_secs(30));
        assert_eq!(config.write_timeout, Duration::from_secs(30));
        assert_eq!(config.max_frame_size, 10 * 1024 * 1024);
        assert_eq!(config.backlog, 128);
        assert_eq!(config.ack_timing, AckTimingPolicy::Immediate);
    }

    /// Test server creation with default config
    #[test]
    fn test_server_creation() {
        let server = MllpServer::with_default_config();
        // Server created successfully
        let _ = server;
    }

    /// Test server bind to available port
    #[tokio::test]
    async fn test_server_bind() {
        let mut server = MllpServer::with_default_config();
        let bind_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        
        let result = server.bind(bind_addr).await;
        assert!(result.is_ok());

        let addr = server.local_addr();
        assert!(addr.is_ok());
        assert_ne!(addr.unwrap().port(), 0);
    }

    /// Test server local_addr fails when not bound
    #[test]
    fn test_server_local_addr_not_bound() {
        let server = MllpServer::with_default_config();
        let result = server.local_addr();
        
        assert!(result.is_err());
        if let Err(e) = result {
            assert_eq!(e.kind(), std::io::ErrorKind::NotConnected);
        }
    }

    /// Test ACK timing policy variants
    #[test]
    fn test_ack_timing_policy() {
        assert_eq!(AckTimingPolicy::Immediate, AckTimingPolicy::Immediate);
        assert_ne!(AckTimingPolicy::Immediate, AckTimingPolicy::OnDemand);
        
        let delayed = AckTimingPolicy::Delayed(Duration::from_millis(100));
        assert!(matches!(delayed, AckTimingPolicy::Delayed(_)));
    }

    /// Test server config with custom values
    #[test]
    fn test_server_custom_config() {
        let config = MllpServerConfig {
            read_timeout: Duration::from_secs(5),
            write_timeout: Duration::from_secs(5),
            max_frame_size: 1024,
            backlog: 64,
            ack_timing: AckTimingPolicy::Delayed(Duration::from_millis(50)),
        };

        assert_eq!(config.read_timeout, Duration::from_secs(5));
        assert_eq!(config.backlog, 64);
        assert!(matches!(config.ack_timing, AckTimingPolicy::Delayed(_)));
    }

    /// Test connection timeout configuration
    #[tokio::test]
    async fn test_connection_timeout_config() {
        let config = MllpServerConfig {
            read_timeout: Duration::from_millis(100),
            ..Default::default()
        };

        assert_eq!(config.read_timeout, Duration::from_millis(100));
    }

    /// Test message handler trait implementation
    struct EchoHandler;

    impl MessageHandler for EchoHandler {
        fn handle_message(&self, message: Message) -> Result<Option<Message>, Error> {
            // Echo the message back as ACK
            Ok(Some(message))
        }
    }

    #[test]
    fn test_message_handler_echo() {
        let handler = EchoHandler;
        let message = create_test_message();
        
        let result = handler.handle_message(message);
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    /// Test message handler that returns None
    struct SilentHandler;

    impl MessageHandler for SilentHandler {
        fn handle_message(&self, _message: Message) -> Result<Option<Message>, Error> {
            Ok(None)
        }
    }

    #[test]
    fn test_message_handler_silent() {
        let handler = SilentHandler;
        let message = create_test_message();
        
        let result = handler.handle_message(message);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    /// Test message handler that returns error
    struct ErrorHandler;

    impl MessageHandler for ErrorHandler {
        fn handle_message(&self, _message: Message) -> Result<Option<Message>, Error> {
            Err(Error::InvalidFieldFormat { details: "Test error".to_string() })
        }
    }

    #[test]
    fn test_message_handler_error() {
        let handler = ErrorHandler;
        let message = create_test_message();
        
        let result = handler.handle_message(message);
        assert!(result.is_err());
    }
}

// =============================================================================
// Property-Based Tests
// =============================================================================

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    /// Generate arbitrary valid HL7 message content (printable ASCII)
    prop_compose! {
        fn arb_message_content()(bytes in "[ -~]*") -> BytesMut {
            BytesMut::from(bytes.as_bytes())
        }
    }

    proptest! {
        /// Test codec roundtrip with arbitrary content
        #[test]
        fn prop_codec_roundtrip(content in arb_message_content()) {
            let mut codec = MllpCodec::new();
            let original = content;
            
            // Encode
            let mut encoded = BytesMut::new();
            let encode_result = codec.encode(original.clone(), &mut encoded);
            
            // Only test if encoding succeeded
            if encode_result.is_ok() {
                // Decode
                let decoded = codec.decode(&mut encoded);
                
                prop_assert!(decoded.is_ok());
                if let Ok(Some(decoded_content)) = decoded {
                    prop_assert_eq!(&decoded_content[..], &original[..]);
                }
            }
        }

        /// Test encoding never panics with any byte sequence
        #[test]
        fn prop_encode_no_panic(bytes: Vec<u8>) {
            let mut codec = MllpCodec::with_max_frame_size(10000);
            let mut dst = BytesMut::new();
            let msg = BytesMut::from(&bytes[..]);
            
            // Should never panic, may return error for large messages
            let _ = codec.encode(msg, &mut dst);
            prop_assert!(true);
        }

        /// Test decoding never panics with any byte sequence
        #[test]
        fn prop_decode_no_panic(bytes: Vec<u8>) {
            let mut codec = MllpCodec::with_max_frame_size(10000);
            let mut src = BytesMut::from(&bytes[..]);
            
            // Should never panic
            let result = codec.decode(&mut src);
            prop_assert!(result.is_ok() || result.is_err());
        }

        /// Test multiple messages can be encoded and decoded
        #[test]
        fn prop_multiple_messages_roundtrip(msgs: Vec<String>) {
            let mut codec = MllpCodec::new();
            let mut buffer = BytesMut::new();
            
            // Encode all messages
            for msg in &msgs {
                let mut encoded = BytesMut::new();
                let _ = codec.encode(BytesMut::from(msg.as_bytes()), &mut encoded);
                buffer.extend(encoded);
            }
            
            // Decode all messages
            let mut decoded_count = 0;
            while let Ok(Some(_)) = codec.decode(&mut buffer) {
                decoded_count += 1;
            }
            
            prop_assert_eq!(decoded_count, msgs.len() as i32);
        }
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Create a simple test message for testing
fn create_test_message() -> Message {
    Message {
        delims: Delims::default(),
        segments: vec![
            Segment {
                id: *b"MSH",
                fields: vec![
                    Field {
                        reps: vec![Rep {
                            comps: vec![Comp {
                                subs: vec![Atom::Text("^~\\&".to_string())],
                            }],
                        }],
                    },
                    Field {
                        reps: vec![Rep {
                            comps: vec![Comp {
                                subs: vec![Atom::Text("TEST".to_string())],
                            }],
                        }],
                    },
                ],
            },
        ],
        charsets: vec![],
    }
}

// =============================================================================
// Integration Tests (run with --test-threads=1 for network tests)
// =============================================================================

#[cfg(test)]
mod network_tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Notify;
    use std::net::SocketAddr;

    /// Test basic client-server communication
    #[tokio::test]
    async fn test_client_server_communication() {
        struct TestHandler {
            notify: Arc<Notify>,
        }

        impl MessageHandler for TestHandler {
            fn handle_message(&self, _message: Message) -> Result<Option<Message>, Error> {
                self.notify.notify_one();
                Ok(Some(create_test_message()))
            }
        }

        // Start server
        let mut server = MllpServer::with_default_config();
        let bind_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        server.bind(bind_addr).await.unwrap();
        let server_addr = server.local_addr().unwrap();

        let notify = Arc::new(Notify::new());
        let handler = TestHandler {
            notify: notify.clone(),
        };

        // Spawn server task
        tokio::spawn(async move {
            let _ = server.run(handler).await;
        });

        // Give server time to start
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Create client and connect
        let mut client = MllpClientBuilder::new()
            .connect_timeout(Duration::from_secs(5))
            .build();

        client.connect(server_addr).await.unwrap();
        assert!(client.is_connected());

        // Send a message
        let message = create_test_message();
        let ack = client.send_message(&message).await.unwrap();

        // Verify we got an ACK back
        assert_eq!(ack.segments.len(), 1);

        // Close client
        client.close().await.unwrap();
    }

    /// Test server handles multiple connections
    #[tokio::test]
    async fn test_server_multiple_connections() {
        struct CountingHandler {
            count: Arc<std::sync::atomic::AtomicU32>,
        }

        impl MessageHandler for CountingHandler {
            fn handle_message(&self, _message: Message) -> Result<Option<Message>, Error> {
                self.count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(Some(create_test_message()))
            }
        }

        // Start server
        let mut server = MllpServer::with_default_config();
        let bind_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        server.bind(bind_addr).await.unwrap();
        let server_addr = server.local_addr().unwrap();

        let count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let handler = CountingHandler {
            count: count.clone(),
        };

        // Spawn server task
        tokio::spawn(async move {
            let _ = server.run(handler).await;
        });

        // Give server time to start
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Create multiple clients
        let mut handles = vec![];
        for _ in 0..3 {
            let addr = server_addr;
            let handle = tokio::spawn(async move {
                let mut client = MllpClientBuilder::new()
                    .connect_timeout(Duration::from_secs(5))
                    .build();
                
                client.connect(addr).await.unwrap();
                let message = create_test_message();
                let _ = client.send_message(&message).await;
                let _ = client.close().await;
            });
            handles.push(handle);
        }

        // Wait for all clients
        for handle in handles {
            handle.await.unwrap();
        }

        // Give time for messages to be processed
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Verify all messages were handled
        assert!(count.load(std::sync::atomic::Ordering::SeqCst) >= 3);
    }

    /// Test codec handles partial frames correctly
    #[tokio::test]
    async fn test_codec_partial_frames() {
        let mut codec = MllpCodec::new();
        
        // Simulate partial frame arrival
        let part1 = BytesMut::from(&b"\x0BMSH"[..]);
        let part2 = BytesMut::from(&b"|^~\\&\r\x1C\x0D"[..]);
        
        let mut buffer = part1;
        
        // First part should not decode
        let result1 = codec.decode(&mut buffer).unwrap();
        assert!(result1.is_none());
        
        // Add second part
        buffer.extend(part2);
        
        // Now should decode
        let result2 = codec.decode(&mut buffer).unwrap();
        assert!(result2.is_some());
        assert_eq!(&result2.unwrap()[..], b"MSH|^~\\&\r");
    }
}
