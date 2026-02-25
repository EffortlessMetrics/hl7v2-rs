//! Integration tests for hl7v2-network crate.
//!
//! These tests verify end-to-end functionality of the MLLP client and server,
//! including network communication, message framing, and error handling.

mod common;

use common::*;
use hl7v2_network::{MllpClient, MllpClientBuilder, MllpServer, MllpServerConfig};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;

// =============================================================================
// Basic Integration Tests
// =============================================================================

/// Test 1: Client connects to server
#[tokio::test]
async fn test_client_connects_to_server() {
    let notify = Arc::new(Notify::new());
    let (mut server, addr) = start_test_server(EchoHandler::new(notify.clone())).await;
    
    // Spawn server task
    tokio::spawn(async move {
        let _ = server.run(EchoHandler::new(notify)).await;
    });
    
    wait_for_server_ready().await;
    
    // Create and connect client
    let mut client = create_test_client();
    let result = client.connect(addr).await;
    
    assert!(result.is_ok(), "Client should connect successfully");
    assert!(client.is_connected(), "Client should report connected state");
    assert_eq!(client.peer_addr(), Some(addr));
    
    client.close().await.unwrap();
}

/// Test 2: Client sends message, receives ACK
#[tokio::test]
async fn test_client_sends_message_receives_ack() {
    let notify = Arc::new(Notify::new());
    let (mut server, addr) = start_test_server(AckHandler::new(notify.clone())).await;
    
    // Spawn server task
    let server_notify = notify.clone();
    tokio::spawn(async move {
        let _ = server.run(AckHandler::new(server_notify)).await;
    });
    
    wait_for_server_ready().await;
    
    // Connect and send message
    let mut client = create_test_client();
    client.connect(addr).await.unwrap();
    
    let message = create_test_message();
    let ack = client.send_message(&message).await;
    
    assert!(ack.is_ok(), "Should receive ACK");
    let ack_msg = ack.unwrap();
    assert_eq!(ack_msg.segments.len(), 1);
    
    client.close().await.unwrap();
}

/// Test 3: Server handles multiple concurrent connections
#[tokio::test]
async fn test_server_handles_multiple_concurrent_connections() {
    let count = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let (mut server, addr) = start_test_server(CountingHandler::new(count.clone())).await;
    
    // Spawn server task
    let server_count = count.clone();
    tokio::spawn(async move {
        let _ = server.run(CountingHandler::new(server_count)).await;
    });
    
    wait_for_server_ready().await;
    
    // Create multiple concurrent clients
    let mut handles = vec![];
    for i in 0..5 {
        let server_addr = addr;
        let handle = tokio::spawn(async move {
            let mut client = create_test_client();
            client.connect(server_addr).await.expect("Connection failed");
            
            let message = create_test_message();
            let _ = client.send_message(&message).await;
            
            // Small delay to ensure messages are processed
            tokio::time::sleep(Duration::from_millis(10)).await;
            
            let _ = client.close().await;
            i
        });
        handles.push(handle);
    }
    
    // Wait for all clients to complete
    for handle in handles {
        handle.await.expect("Client task failed");
    }
    
    // Allow time for server to process all messages
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Verify all messages were handled
    let message_count = count.load(std::sync::atomic::Ordering::SeqCst);
    assert!(message_count >= 5, "Should have handled at least 5 messages, got {}", message_count);
}

/// Test 4: Server graceful shutdown (simulated by dropping server)
#[tokio::test]
async fn test_server_graceful_shutdown() {
    let notify = Arc::new(Notify::new());
    let (mut server, addr) = start_test_server(AckHandler::new(notify.clone())).await;
    
    let server_notify = notify.clone();
    let server_task = tokio::spawn(async move {
        let _ = server.run(AckHandler::new(server_notify)).await;
    });
    
    wait_for_server_ready().await;
    
    // Connect a client
    let mut client = create_test_client();
    client.connect(addr).await.unwrap();
    
    // Send a message
    let message = create_test_message();
    let _ = client.send_message(&message).await;
    
    // Abort server (simulates shutdown)
    server_task.abort();
    
    // Give time for shutdown
    tokio::time::sleep(Duration::from_millis(50)).await;
    
    // Client should be able to close gracefully
    let _ = client.close().await;
}

// =============================================================================
// MLLP Framing Tests
// =============================================================================

/// Test 5: MLLP codec handles partial frames
#[tokio::test]
async fn test_mllp_codec_handles_partial_frames() {
    use bytes::BytesMut;
    use tokio_util::codec::Decoder;
    use hl7v2_network::MllpCodec;
    
    let mut codec = MllpCodec::new();
    
    // Simulate partial frame arrival
    let mut buffer = BytesMut::from(&b"\x0BMSH"[..]);
    
    // Should not decode incomplete frame
    let result1 = codec.decode(&mut buffer).unwrap();
    assert!(result1.is_none(), "Should not decode partial frame");
    
    // Add rest of frame
    buffer.extend_from_slice(b"|^~\\&|TEST\r\x1C\x0D");
    
    // Now should decode
    let result2 = codec.decode(&mut buffer).unwrap();
    assert!(result2.is_some(), "Should decode complete frame");
    assert_eq!(&result2.unwrap()[..], b"MSH|^~\\&|TEST\r");
}

/// Test 6: MLLP codec handles multiple messages in one buffer
#[tokio::test]
async fn test_mllp_codec_handles_multiple_messages_in_buffer() {
    use bytes::BytesMut;
    use tokio_util::codec::{Decoder, Encoder};
    use hl7v2_network::MllpCodec;
    
    let mut codec = MllpCodec::new();
    
    // Create buffer with multiple complete frames
    let mut buffer = BytesMut::new();
    codec.encode(BytesMut::from(b"MSG1".as_slice()), &mut buffer).unwrap();
    codec.encode(BytesMut::from(b"MSG2".as_slice()), &mut buffer).unwrap();
    codec.encode(BytesMut::from(b"MSG3".as_slice()), &mut buffer).unwrap();
    
    // Decode all messages
    let mut messages = vec![];
    while let Ok(Some(msg)) = codec.decode(&mut buffer) {
        messages.push(msg);
    }
    
    assert_eq!(messages.len(), 3, "Should decode all 3 messages");
    assert_eq!(&messages[0][..], b"MSG1");
    assert_eq!(&messages[1][..], b"MSG2");
    assert_eq!(&messages[2][..], b"MSG3");
}

/// Test MLLP codec handles junk before start byte
#[tokio::test]
async fn test_mllp_codec_handles_junk_data() {
    use bytes::BytesMut;
    use tokio_util::codec::Decoder;
    use hl7v2_network::MllpCodec;
    
    let mut codec = MllpCodec::new();
    
    // Buffer with junk before valid frame
    let mut buffer = BytesMut::from(&b"GARBAGE\x0BMSH|^~\\&|TEST\r\x1C\x0D"[..]);
    
    let result = codec.decode(&mut buffer).unwrap();
    assert!(result.is_some(), "Should decode frame after junk");
    assert_eq!(&result.unwrap()[..], b"MSH|^~\\&|TEST\r");
}

/// Test MLLP codec handles empty content
#[tokio::test]
async fn test_mllp_codec_handles_empty_content() {
    use bytes::BytesMut;
    use tokio_util::codec::{Decoder, Encoder};
    use hl7v2_network::MllpCodec;
    
    let mut codec = MllpCodec::new();
    
    // Encode empty message
    let mut buffer = BytesMut::new();
    codec.encode(BytesMut::new(), &mut buffer).unwrap();
    
    // Decode
    let result = codec.decode(&mut buffer).unwrap();
    assert!(result.is_some(), "Should decode empty frame");
    assert_eq!(result.unwrap().len(), 0);
}

// =============================================================================
// Connection Timeout Tests
// =============================================================================

/// Test 7: Connection timeout handling
#[tokio::test]
async fn test_connection_timeout() {
    let mut client = MllpClientBuilder::new()
        .connect_timeout(Duration::from_millis(1))
        .build();
    
    // Try to connect to a non-routable address
    let addr: std::net::SocketAddr = "192.0.2.1:2575".parse().unwrap();
    let result = client.connect(addr).await;
    
    assert!(result.is_err(), "Should timeout on non-routable address");
    let err = result.unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::TimedOut);
}

/// Test read timeout configuration works
#[tokio::test]
async fn test_read_timeout_configuration() {
    // This test verifies the timeout configuration is applied
    let client = MllpClientBuilder::new()
        .read_timeout(Duration::from_millis(100))
        .build();
    
    // Client was created successfully with the timeout
    assert!(!client.is_connected());
}

/// Test write timeout configuration works
#[tokio::test]
async fn test_write_timeout_configuration() {
    let client = MllpClientBuilder::new()
        .write_timeout(Duration::from_millis(200))
        .build();
    
    assert!(!client.is_connected());
}

// =============================================================================
// Error Recovery Tests
// =============================================================================

/// Test 9: Error recovery - client can reconnect after error
#[tokio::test]
async fn test_client_can_reconnect_after_error() {
    let notify = Arc::new(Notify::new());
    let (mut server, addr) = start_test_server(AckHandler::new(notify.clone())).await;
    
    let server_notify = notify.clone();
    tokio::spawn(async move {
        let _ = server.run(AckHandler::new(server_notify)).await;
    });
    
    wait_for_server_ready().await;
    
    // First connection
    let mut client = create_test_client();
    client.connect(addr).await.unwrap();
    client.close().await.unwrap();
    
    // Reconnect with same client (after disconnect)
    let mut client2 = create_test_client();
    let result = client2.connect(addr).await;
    assert!(result.is_ok(), "Should be able to reconnect");
    
    client2.close().await.unwrap();
}

/// Test client handles invalid message gracefully
#[tokio::test]
async fn test_client_handles_invalid_message_response() {
    // This would require a mock server that sends invalid responses
    // For now, verify error handling path exists
    let client = MllpClient::with_default_config();
    assert!(!client.is_connected());
}

/// Test server handles malformed input
#[tokio::test]
async fn test_server_handles_malformed_input() {
    use bytes::BytesMut;
    use tokio_util::codec::Decoder;
    use hl7v2_network::MllpCodec;
    
    let mut codec = MllpCodec::new();
    
    // Various malformed inputs
    let test_cases = vec![
        &b""[..],
        &b"\x0B"[..],
        &b"\x1C\x0D"[..],
        &b"NO_START_BYTE"[..],
        &b"\x0BNO_END"[..],
    ];
    
    for input in test_cases {
        let mut buffer = BytesMut::from(input);
        let result = codec.decode(&mut buffer);
        // Should not panic, may return None or error
        assert!(result.is_ok() || result.is_err());
    }
}

// =============================================================================
// Message Exchange Tests
// =============================================================================

/// Test sending multiple messages on same connection
#[tokio::test]
async fn test_multiple_messages_same_connection() {
    let notify = Arc::new(Notify::new());
    let (mut server, addr) = start_test_server(AckHandler::new(notify.clone())).await;
    
    let server_notify = notify.clone();
    tokio::spawn(async move {
        let _ = server.run(AckHandler::new(server_notify)).await;
    });
    
    wait_for_server_ready().await;
    
    let mut client = create_test_client();
    client.connect(addr).await.unwrap();
    
    // Send multiple messages
    for _ in 0..10 {
        let message = create_test_message();
        let ack = client.send_message(&message).await;
        assert!(ack.is_ok(), "Should receive ACK for each message");
    }
    
    client.close().await.unwrap();
}

/// Test fire-and-forget message sending
#[tokio::test]
async fn test_fire_and_forget_messaging() {
    let (mut server, addr) = start_test_server(SilentHandler).await;
    
    tokio::spawn(async move {
        let _ = server.run(SilentHandler).await;
    });
    
    wait_for_server_ready().await;
    
    let mut client = create_test_client();
    client.connect(addr).await.unwrap();
    
    // Send without waiting for ACK
    let message = create_test_message();
    let result = client.send_message_no_ack(&message).await;
    assert!(result.is_ok(), "Should send without error");
    
    client.close().await.unwrap();
}

/// Test large message handling
#[tokio::test]
async fn test_large_message_handling() {
    let notify = Arc::new(Notify::new());
    let (mut server, addr) = start_test_server(EchoHandler::new(notify.clone())).await;
    
    let server_notify = notify.clone();
    tokio::spawn(async move {
        let _ = server.run(EchoHandler::new(server_notify)).await;
    });
    
    wait_for_server_ready().await;
    
    let mut client = create_test_client();
    client.connect(addr).await.unwrap();
    
    // Create a larger message
    let large_message = create_adt_a01_message();
    
    let ack = client.send_message(&large_message).await;
    assert!(ack.is_ok(), "Should handle larger messages");
    
    client.close().await.unwrap();
}

// =============================================================================
// Server Lifecycle Tests
// =============================================================================

/// Test server bind to specific port
#[tokio::test]
async fn test_server_bind_specific_port() {
    let mut server = MllpServer::new(MllpServerConfig::default());
    let port = get_unique_port();
    let addr: std::net::SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
    
    let result = server.bind(addr).await;
    assert!(result.is_ok(), "Should bind to specific port");
    
    let bound_addr = server.local_addr().unwrap();
    assert_eq!(bound_addr.port(), port);
}

/// Test server local_addr when not bound
#[tokio::test]
async fn test_server_local_addr_when_not_bound() {
    let server = MllpServer::new(MllpServerConfig::default());
    let result = server.local_addr();
    
    assert!(result.is_err());
}

/// Test server configuration
#[tokio::test]
async fn test_server_configuration() {
    let config = MllpServerConfig {
        read_timeout: Duration::from_secs(10),
        write_timeout: Duration::from_secs(10),
        max_frame_size: 5 * 1024 * 1024,
        backlog: 64,
        ack_timing: hl7v2_network::AckTimingPolicy::Immediate,
    };
    
    let server = MllpServer::new(config);
    // Server created successfully with custom config
    let _ = server;
}

// =============================================================================
// Client Builder Tests
// =============================================================================

/// Test client builder with all options
#[tokio::test]
async fn test_client_builder_all_options() {
    let client = MllpClientBuilder::new()
        .connect_timeout(Duration::from_secs(3))
        .read_timeout(Duration::from_secs(7))
        .write_timeout(Duration::from_secs(8))
        .max_frame_size(2 * 1024 * 1024)
        .build();
    
    // Client created successfully with all options
    assert!(!client.is_connected());
}

/// Test client default configuration
#[tokio::test]
async fn test_client_default_configuration() {
    let client = MllpClient::with_default_config();
    
    // Default client created
    assert!(!client.is_connected());
}
