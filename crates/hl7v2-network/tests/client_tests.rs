//! Client-specific tests for hl7v2-network crate.
//!
//! These tests focus on MllpClient behavior including:
//! - Connection management
//! - Message sending and receiving
//! - Error handling
//! - Timeout behavior
//! - Reconnection logic

mod common;

use common::*;
use hl7v2_network::{MllpClient, MllpClientBuilder, MllpClientConfig};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;

// =============================================================================
// Client Connection Tests
// =============================================================================

/// Test client connection to a live server
#[tokio::test]
async fn test_client_connection_success() {
    let notify = Arc::new(Notify::new());
    let (mut server, addr) = start_test_server(AckHandler::new(notify.clone())).await;

    tokio::spawn(async move {
        let _ = server.run(AckHandler::new(notify)).await;
    });

    wait_for_server_ready().await;

    let mut client = create_test_client();
    let result = client.connect(addr).await;

    assert!(result.is_ok());
    assert!(client.is_connected());
    assert_eq!(client.peer_addr(), Some(addr));

    client.close().await.unwrap();
}

/// Test client connection to non-existent server
#[tokio::test]
async fn test_client_connection_refused() {
    let mut client = MllpClientBuilder::new()
        .connect_timeout(Duration::from_secs(1))
        .build();

    // Use a port that's unlikely to be in use
    let addr: std::net::SocketAddr = "127.0.0.1:59999".parse().unwrap();
    let result = client.connect(addr).await;

    assert!(result.is_err());
    assert!(!client.is_connected());
}

/// Test client connection timeout
#[tokio::test]
async fn test_client_connection_timeout() {
    let mut client = MllpClientBuilder::new()
        .connect_timeout(Duration::from_millis(1))
        .build();

    // 192.0.2.1 is a TEST-NET address that won't respond
    let addr: std::net::SocketAddr = "192.0.2.1:2575".parse().unwrap();
    let result = client.connect(addr).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::TimedOut);
}

/// Test client is_connected returns false initially
#[tokio::test]
async fn test_client_not_connected_initially() {
    let client = create_test_client();
    assert!(!client.is_connected());
    assert!(client.peer_addr().is_none());
}

/// Test client peer_addr returns correct address after connection
#[tokio::test]
async fn test_client_peer_addr_after_connection() {
    let notify = Arc::new(Notify::new());
    let (mut server, addr) = start_test_server(AckHandler::new(notify.clone())).await;

    tokio::spawn(async move {
        let _ = server.run(AckHandler::new(notify)).await;
    });

    wait_for_server_ready().await;

    let mut client = create_test_client();
    client.connect(addr).await.unwrap();

    let peer = client.peer_addr();
    assert!(peer.is_some());
    assert_eq!(peer.unwrap(), addr);

    client.close().await.unwrap();
}

// =============================================================================
// Client Message Sending Tests
// =============================================================================

/// Test send_message returns ACK from server
#[tokio::test]
async fn test_client_send_message_receives_ack() {
    let notify = Arc::new(Notify::new());
    let (mut server, addr) = start_test_server(AckHandler::new(notify.clone())).await;

    tokio::spawn(async move {
        let _ = server.run(AckHandler::new(notify)).await;
    });

    wait_for_server_ready().await;

    let mut client = create_test_client();
    client.connect(addr).await.unwrap();

    let message = create_test_message();
    let ack = client.send_message(&message).await;

    assert!(ack.is_ok());
    let ack_msg = ack.unwrap();
    // AckHandler returns a simple test message
    assert_eq!(ack_msg.segments.len(), 1);

    client.close().await.unwrap();
}

/// Test send_message fails when not connected
#[tokio::test]
async fn test_client_send_message_not_connected() {
    let mut client = create_test_client();
    let message = create_test_message();

    let result = client.send_message(&message).await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::NotConnected);
}

/// Test send_message_no_ack succeeds
#[tokio::test]
async fn test_client_send_message_no_ack() {
    let (mut server, addr) = start_test_server(SilentHandler).await;

    tokio::spawn(async move {
        let _ = server.run(SilentHandler).await;
    });

    wait_for_server_ready().await;

    let mut client = create_test_client();
    client.connect(addr).await.unwrap();

    let message = create_test_message();
    let result = client.send_message_no_ack(&message).await;

    assert!(result.is_ok());

    client.close().await.unwrap();
}

/// Test send_message_no_ack fails when not connected
#[tokio::test]
async fn test_client_send_message_no_ack_not_connected() {
    let mut client = create_test_client();
    let message = create_test_message();

    let result = client.send_message_no_ack(&message).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::NotConnected);
}

/// Test sending multiple messages sequentially
#[tokio::test]
async fn test_client_send_multiple_messages() {
    let notify = Arc::new(Notify::new());
    let (mut server, addr) = start_test_server(AckHandler::new(notify.clone())).await;

    tokio::spawn(async move {
        let _ = server.run(AckHandler::new(notify)).await;
    });

    wait_for_server_ready().await;

    let mut client = create_test_client();
    client.connect(addr).await.unwrap();

    for i in 0..5 {
        let message = create_test_message();
        let ack = client.send_message(&message).await;
        assert!(ack.is_ok(), "Message {} should succeed", i);
    }

    client.close().await.unwrap();
}

// =============================================================================
// Client Message Receiving Tests
// =============================================================================

/// Test receive_message fails when not connected
#[tokio::test]
async fn test_client_receive_message_not_connected() {
    let mut client = create_test_client();

    let result = client.receive_message().await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::NotConnected);
}

// =============================================================================
// Client Close/Disconnect Tests
// =============================================================================

/// Test close succeeds on connected client
#[tokio::test]
async fn test_client_close_connected() {
    let notify = Arc::new(Notify::new());
    let (mut server, addr) = start_test_server(AckHandler::new(notify.clone())).await;

    tokio::spawn(async move {
        let _ = server.run(AckHandler::new(notify)).await;
    });

    wait_for_server_ready().await;

    let mut client = create_test_client();
    client.connect(addr).await.unwrap();
    assert!(client.is_connected());

    let result = client.close().await;
    assert!(result.is_ok());
}

/// Test close succeeds on unconnected client
#[tokio::test]
async fn test_client_close_unconnected() {
    let client = create_test_client();
    let result = client.close().await;
    assert!(result.is_ok());
}

/// Test disconnect succeeds on connected client
#[tokio::test]
async fn test_client_disconnect_connected() {
    let notify = Arc::new(Notify::new());
    let (mut server, addr) = start_test_server(AckHandler::new(notify.clone())).await;

    tokio::spawn(async move {
        let _ = server.run(AckHandler::new(notify)).await;
    });

    wait_for_server_ready().await;

    let mut client = create_test_client();
    client.connect(addr).await.unwrap();
    assert!(client.is_connected());

    let result = client.disconnect().await;
    assert!(result.is_ok());
    assert!(!client.is_connected());
    assert!(client.peer_addr().is_none());
}

/// Test disconnect succeeds on unconnected client
#[tokio::test]
async fn test_client_disconnect_unconnected() {
    let mut client = create_test_client();
    let result = client.disconnect().await;
    assert!(result.is_ok());
}

// =============================================================================
// Client Reconnection Tests
// =============================================================================

/// Test client can reconnect after disconnect
#[tokio::test]
async fn test_client_reconnect_after_disconnect() {
    let notify = Arc::new(Notify::new());
    let (mut server, addr) = start_test_server(AckHandler::new(notify.clone())).await;

    tokio::spawn(async move {
        let _ = server.run(AckHandler::new(notify)).await;
    });

    wait_for_server_ready().await;

    let mut client = create_test_client();

    // First connection
    client.connect(addr).await.unwrap();
    assert!(client.is_connected());

    // Disconnect
    client.disconnect().await.unwrap();
    assert!(!client.is_connected());

    // Reconnect
    let result = client.connect(addr).await;
    assert!(result.is_ok());
    assert!(client.is_connected());

    client.close().await.unwrap();
}

/// Test client can be reused after close
#[tokio::test]
async fn test_client_reuse_after_close() {
    let notify = Arc::new(Notify::new());
    let (mut server, addr) = start_test_server(AckHandler::new(notify.clone())).await;

    tokio::spawn(async move {
        let _ = server.run(AckHandler::new(notify)).await;
    });

    wait_for_server_ready().await;

    // First client
    let mut client = create_test_client();
    client.connect(addr).await.unwrap();
    client.close().await.unwrap();

    // Create new client (close consumes the client)
    let mut client2 = create_test_client();
    let result = client2.connect(addr).await;
    assert!(result.is_ok());

    client2.close().await.unwrap();
}

// =============================================================================
// Client Builder Tests
// =============================================================================

/// Test builder creates client with custom connect timeout
#[tokio::test]
async fn test_client_builder_connect_timeout() {
    let client = MllpClientBuilder::new()
        .connect_timeout(Duration::from_secs(3))
        .build();

    assert!(!client.is_connected());
}

/// Test builder creates client with custom read timeout
#[tokio::test]
async fn test_client_builder_read_timeout() {
    let client = MllpClientBuilder::new()
        .read_timeout(Duration::from_secs(15))
        .build();

    assert!(!client.is_connected());
}

/// Test builder creates client with custom write timeout
#[tokio::test]
async fn test_client_builder_write_timeout() {
    let client = MllpClientBuilder::new()
        .write_timeout(Duration::from_secs(20))
        .build();

    assert!(!client.is_connected());
}

/// Test builder creates client with custom max frame size
#[tokio::test]
async fn test_client_builder_max_frame_size() {
    let client = MllpClientBuilder::new().max_frame_size(1024).build();

    assert!(!client.is_connected());
}

/// Test builder chains all options
#[tokio::test]
async fn test_client_builder_chain_all_options() {
    let client = MllpClientBuilder::new()
        .connect_timeout(Duration::from_secs(1))
        .read_timeout(Duration::from_secs(2))
        .write_timeout(Duration::from_secs(3))
        .max_frame_size(4096)
        .build();

    assert!(!client.is_connected());
}

/// Test builder default
#[tokio::test]
async fn test_client_builder_default() {
    let builder = MllpClientBuilder::default();
    let client = builder.build();

    assert!(!client.is_connected());
}

// =============================================================================
// Client Configuration Tests
// =============================================================================

/// Test MllpClientConfig default values
#[tokio::test]
async fn test_client_config_default() {
    let config = MllpClientConfig::default();

    assert_eq!(config.connect_timeout, Duration::from_secs(10));
    assert_eq!(config.read_timeout, Duration::from_secs(30));
    assert_eq!(config.write_timeout, Duration::from_secs(30));
    assert_eq!(config.max_frame_size, 10 * 1024 * 1024);
}

/// Test MllpClient::new with custom config
#[tokio::test]
async fn test_client_new_with_config() {
    let config = MllpClientConfig {
        connect_timeout: Duration::from_secs(5),
        read_timeout: Duration::from_secs(10),
        write_timeout: Duration::from_secs(15),
        max_frame_size: 2048,
    };

    let client = MllpClient::new(config);

    assert!(!client.is_connected());
}

/// Test MllpClient::with_default_config
#[tokio::test]
async fn test_client_with_default_config() {
    let client = MllpClient::with_default_config();

    assert!(!client.is_connected());
}

// =============================================================================
// Client Error Handling Tests
// =============================================================================

/// Test client handles server closing connection gracefully
#[tokio::test]
async fn test_client_handles_server_close() {
    let notify = Arc::new(Notify::new());
    let (mut server, addr) = start_test_server(AckHandler::new(notify.clone())).await;

    let server_task = tokio::spawn(async move {
        // Accept one connection then exit
        let _ = server.run(AckHandler::new(notify)).await;
    });

    wait_for_server_ready().await;

    let mut client = create_test_client();
    client.connect(addr).await.unwrap();

    // Send a message
    let message = create_test_message();
    let _ = client.send_message(&message).await;

    // Abort server
    server_task.abort();

    // Give time for connection to close
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Client should handle this gracefully
    let _ = client.close().await;
}

/// Test client with very short timeout
#[tokio::test]
async fn test_client_short_timeout() {
    let mut client = MllpClientBuilder::new()
        .connect_timeout(Duration::from_millis(1))
        .read_timeout(Duration::from_millis(1))
        .write_timeout(Duration::from_millis(1))
        .build();

    // Connection to slow/non-existent endpoint should timeout
    let addr: std::net::SocketAddr = "192.0.2.1:2575".parse().unwrap();
    let result = client.connect(addr).await;

    assert!(result.is_err());
}
