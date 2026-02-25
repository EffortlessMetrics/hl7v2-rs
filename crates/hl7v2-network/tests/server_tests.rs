//! Server-specific tests for hl7v2-network crate.
//!
//! These tests focus on MllpServer behavior including:
//! - Server lifecycle (bind, run, shutdown)
//! - Connection handling
//! - Message handler behavior
//! - ACK timing policies
//! - Concurrent connections

mod common;

use common::*;
use hl7v2_network::{
    MllpServer, MllpServerConfig, MessageHandler, AckTimingPolicy,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;

// =============================================================================
// Server Lifecycle Tests
// =============================================================================

/// Test server bind to random port
#[tokio::test]
async fn test_server_bind_random_port() {
    let mut server = MllpServer::new(MllpServerConfig::default());
    let bind_addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    
    let result = server.bind(bind_addr).await;
    assert!(result.is_ok());
    
    let addr = server.local_addr();
    assert!(addr.is_ok());
    // Port should be assigned by OS
    assert_ne!(addr.unwrap().port(), 0);
}

/// Test server bind to specific port
#[tokio::test]
async fn test_server_bind_specific_port() {
    let mut server = MllpServer::new(MllpServerConfig::default());
    let port = get_unique_port();
    let bind_addr: std::net::SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
    
    let result = server.bind(bind_addr).await;
    assert!(result.is_ok());
    
    let addr = server.local_addr().unwrap();
    assert_eq!(addr.port(), port);
}

/// Test server bind fails on already bound port
#[tokio::test]
async fn test_server_bind_port_in_use() {
    let mut server1 = MllpServer::new(MllpServerConfig::default());
    let bind_addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    server1.bind(bind_addr).await.unwrap();
    
    let addr = server1.local_addr().unwrap();
    
    // Try to bind another server to the same port
    let mut server2 = MllpServer::new(MllpServerConfig::default());
    let result = server2.bind(addr).await;
    
    assert!(result.is_err());
}

/// Test server local_addr fails when not bound
#[tokio::test]
async fn test_server_local_addr_not_bound() {
    let server = MllpServer::new(MllpServerConfig::default());
    let result = server.local_addr();
    
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::NotConnected);
}

/// Test server creation with default config
#[tokio::test]
async fn test_server_with_default_config() {
    let server = MllpServer::with_default_config();
    // Server created successfully
    let _ = server;
}

/// Test server creation with custom config
#[tokio::test]
async fn test_server_with_custom_config() {
    let config = MllpServerConfig {
        read_timeout: Duration::from_secs(5),
        write_timeout: Duration::from_secs(5),
        max_frame_size: 1024 * 1024,
        backlog: 32,
        ack_timing: AckTimingPolicy::Immediate,
    };
    
    let server = MllpServer::new(config);
    // Server created successfully
    let _ = server;
}

// =============================================================================
// Server Configuration Tests
// =============================================================================

/// Test MllpServerConfig default values
#[tokio::test]
async fn test_server_config_default() {
    let config = MllpServerConfig::default();
    
    assert_eq!(config.read_timeout, Duration::from_secs(30));
    assert_eq!(config.write_timeout, Duration::from_secs(30));
    assert_eq!(config.max_frame_size, 10 * 1024 * 1024);
    assert_eq!(config.backlog, 128);
    assert_eq!(config.ack_timing, AckTimingPolicy::Immediate);
}

/// Test AckTimingPolicy variants
#[tokio::test]
async fn test_ack_timing_policy_immediate() {
    let policy = AckTimingPolicy::Immediate;
    assert_eq!(policy, AckTimingPolicy::Immediate);
    assert_ne!(policy, AckTimingPolicy::OnDemand);
}

#[tokio::test]
async fn test_ack_timing_policy_delayed() {
    let policy = AckTimingPolicy::Delayed(Duration::from_millis(100));
    assert!(matches!(policy, AckTimingPolicy::Delayed(_)));
}

#[tokio::test]
async fn test_ack_timing_policy_on_demand() {
    let policy = AckTimingPolicy::OnDemand;
    assert_eq!(policy, AckTimingPolicy::OnDemand);
}

// =============================================================================
// Message Handler Tests
// =============================================================================

/// Test EchoHandler returns message
#[tokio::test]
async fn test_echo_handler_returns_message() {
    let notify = Arc::new(Notify::new());
    let handler = EchoHandler::new(notify);
    
    let message = create_test_message();
    let result = handler.handle_message(message);
    
    assert!(result.is_ok());
    assert!(result.unwrap().is_some());
}

/// Test AckHandler returns test message
#[tokio::test]
async fn test_ack_handler_returns_ack() {
    let notify = Arc::new(Notify::new());
    let handler = AckHandler::new(notify);
    
    let message = create_test_message();
    let result = handler.handle_message(message);
    
    assert!(result.is_ok());
    let ack = result.unwrap();
    assert!(ack.is_some());
}

/// Test CountingHandler increments count
#[tokio::test]
async fn test_counting_handler_increments() {
    let count = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let handler = CountingHandler::new(count.clone());
    
    let message = create_test_message();
    let _ = handler.handle_message(message.clone());
    let _ = handler.handle_message(message.clone());
    let _ = handler.handle_message(message);
    
    assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 3);
}

/// Test SilentHandler returns None
#[tokio::test]
async fn test_silent_handler_returns_none() {
    let handler = SilentHandler;
    
    let message = create_test_message();
    let result = handler.handle_message(message);
    
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

/// Test ErrorHandler returns error
#[tokio::test]
async fn test_error_handler_returns_error() {
    let handler = ErrorHandler;
    
    let message = create_test_message();
    let result = handler.handle_message(message);
    
    assert!(result.is_err());
}

// =============================================================================
// Server Connection Handling Tests
// =============================================================================

/// Test server accepts single connection
#[tokio::test]
async fn test_server_accepts_connection() {
    let (mut server, addr) = start_test_server(AckHandler::new(Arc::new(Notify::new()))).await;
    
    tokio::spawn(async move {
        let _ = server.run(AckHandler::new(Arc::new(Notify::new()))).await;
    });
    
    wait_for_server_ready().await;
    
    // Connect a client
    let mut client = create_test_client();
    let result = client.connect(addr).await;
    
    assert!(result.is_ok());
    client.close().await.unwrap();
}

/// Test server accepts multiple connections
#[tokio::test]
async fn test_server_accepts_multiple_connections() {
    let (mut server, addr) = start_test_server(AckHandler::new(Arc::new(Notify::new()))).await;
    
    tokio::spawn(async move {
        let _ = server.run(AckHandler::new(Arc::new(Notify::new()))).await;
    });
    
    wait_for_server_ready().await;
    
    // Connect multiple clients sequentially
    for _ in 0..5 {
        let mut client = create_test_client();
        client.connect(addr).await.unwrap();
        client.close().await.unwrap();
    }
}

/// Test server handles concurrent connections
#[tokio::test]
async fn test_server_handles_concurrent_connections() {
    let (mut server, addr) = start_test_server(AckHandler::new(Arc::new(Notify::new()))).await;
    
    tokio::spawn(async move {
        let _ = server.run(AckHandler::new(Arc::new(Notify::new()))).await;
    });
    
    wait_for_server_ready().await;
    
    // Spawn multiple concurrent client tasks
    let mut handles = vec![];
    for _ in 0..5 {
        let server_addr = addr;
        let handle = tokio::spawn(async move {
            let mut client = create_test_client();
            client.connect(server_addr).await.unwrap();
            tokio::time::sleep(Duration::from_millis(50)).await;
            client.close().await.unwrap();
        });
        handles.push(handle);
    }
    
    // Wait for all clients
    for handle in handles {
        handle.await.unwrap();
    }
}

// =============================================================================
// Server Message Processing Tests
// =============================================================================

/// Test server processes message and sends ACK
#[tokio::test]
async fn test_server_processes_message_sends_ack() {
    let notify = Arc::new(Notify::new());
    let (mut server, addr) = start_test_server(AckHandler::new(notify.clone())).await;
    
    let server_notify = notify.clone();
    tokio::spawn(async move {
        let _ = server.run(AckHandler::new(server_notify)).await;
    });
    
    wait_for_server_ready().await;
    
    let mut client = create_test_client();
    client.connect(addr).await.unwrap();
    
    let message = create_test_message();
    let ack = client.send_message(&message).await;
    
    assert!(ack.is_ok(), "Client should receive ACK");
    
    client.close().await.unwrap();
}

/// Test server handles multiple messages from same client
#[tokio::test]
async fn test_server_handles_multiple_messages_same_client() {
    let count = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let (mut server, addr) = start_test_server(CountingHandler::new(count.clone())).await;
    
    let server_count = count.clone();
    tokio::spawn(async move {
        let _ = server.run(CountingHandler::new(server_count)).await;
    });
    
    wait_for_server_ready().await;
    
    let mut client = create_test_client();
    client.connect(addr).await.unwrap();
    
    // Send multiple messages
    for _ in 0..10 {
        let message = create_test_message();
        let _ = client.send_message(&message).await;
    }
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Verify all messages were processed
    assert!(count.load(std::sync::atomic::Ordering::SeqCst) >= 10);
    
    client.close().await.unwrap();
}

/// Test server with SilentHandler doesn't send ACK
#[tokio::test]
async fn test_server_silent_handler_no_ack() {
    let (mut server, addr) = start_test_server(SilentHandler).await;
    
    tokio::spawn(async move {
        let _ = server.run(SilentHandler).await;
    });
    
    wait_for_server_ready().await;
    
    let mut client = create_test_client();
    client.connect(addr).await.unwrap();
    
    // Send message - server won't respond
    let message = create_test_message();
    let result = client.send_message_no_ack(&message).await;
    assert!(result.is_ok());
    
    client.close().await.unwrap();
}

// =============================================================================
// Server Graceful Shutdown Tests
// =============================================================================

/// Test server task can be cancelled
#[tokio::test]
async fn test_server_task_cancellation() {
    let (mut server, _addr) = start_test_server(AckHandler::new(Arc::new(Notify::new()))).await;
    
    let server_task = tokio::spawn(async move {
        let _ = server.run(AckHandler::new(Arc::new(Notify::new()))).await;
    });
    
    wait_for_server_ready().await;
    
    // Cancel the server
    server_task.abort();
    
    // Give time for cancellation
    tokio::time::sleep(Duration::from_millis(50)).await;
    
    // Test passes if no panic occurred
}

/// Test server handles client disconnect gracefully
#[tokio::test]
async fn test_server_handles_client_disconnect() {
    let (mut server, addr) = start_test_server(AckHandler::new(Arc::new(Notify::new()))).await;
    
    tokio::spawn(async move {
        let _ = server.run(AckHandler::new(Arc::new(Notify::new()))).await;
    });
    
    wait_for_server_ready().await;
    
    // Connect and immediately disconnect
    {
        let mut client = create_test_client();
        client.connect(addr).await.unwrap();
        client.close().await.unwrap();
    }
    
    // Server should still accept new connections
    let mut client2 = create_test_client();
    let result = client2.connect(addr).await;
    assert!(result.is_ok());
    client2.close().await.unwrap();
}

// =============================================================================
// MllpConnection Tests (via accept)
// =============================================================================

/// Test MllpConnection peer_addr
#[tokio::test]
async fn test_mllp_connection_peer_addr() {
    let mut server = MllpServer::new(MllpServerConfig::default());
    let bind_addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    server.bind(bind_addr).await.unwrap();
    let server_addr = server.local_addr().unwrap();
    
    // Spawn a task to accept connections
    let accept_task = tokio::spawn(async move {
        let conn = server.accept().await.unwrap();
        conn.peer_addr()
    });
    
    wait_for_server_ready().await;
    
    // Connect a client
    let mut client = create_test_client();
    client.connect(server_addr).await.unwrap();
    
    // Wait for accept
    let peer_addr = accept_task.await.unwrap();
    assert!(peer_addr.to_string().contains("127.0.0.1"));
    
    client.close().await.unwrap();
}

/// Test MllpConnection receive_message
#[tokio::test]
async fn test_mllp_connection_receive_message() {
    let mut server = MllpServer::new(MllpServerConfig::default());
    let bind_addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    server.bind(bind_addr).await.unwrap();
    let server_addr = server.local_addr().unwrap();
    
    // Spawn a task to accept and receive
    let receive_task = tokio::spawn(async move {
        let mut conn = server.accept().await.unwrap();
        conn.receive_message().await
    });
    
    wait_for_server_ready().await;
    
    // Connect and send message
    let mut client = create_test_client();
    client.connect(server_addr).await.unwrap();
    
    let message = create_test_message();
    let _ = client.send_message_no_ack(&message).await;
    
    // Wait for receive
    let result = receive_task.await.unwrap();
    assert!(result.is_ok());
    let received = result.unwrap();
    assert!(received.is_some());
    
    client.close().await.unwrap();
}

/// Test MllpConnection send_message
#[tokio::test]
async fn test_mllp_connection_send_message() {
    let mut server = MllpServer::new(MllpServerConfig::default());
    let bind_addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    server.bind(bind_addr).await.unwrap();
    let server_addr = server.local_addr().unwrap();
    
    // Spawn a task to accept and send
    let send_task = tokio::spawn(async move {
        let mut conn = server.accept().await.unwrap();
        // First receive a message
        let _ = conn.receive_message().await;
        // Then send a response
        let ack = create_test_message();
        conn.send_message(&ack).await
    });
    
    wait_for_server_ready().await;
    
    // Connect and exchange messages
    let mut client = create_test_client();
    client.connect(server_addr).await.unwrap();
    
    let message = create_test_message();
    let _ = client.send_message(&message).await;
    
    // Wait for send
    let result = send_task.await.unwrap();
    assert!(result.is_ok());
    
    client.close().await.unwrap();
}

/// Test MllpConnection close
#[tokio::test]
async fn test_mllp_connection_close() {
    let mut server = MllpServer::new(MllpServerConfig::default());
    let bind_addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    server.bind(bind_addr).await.unwrap();
    let server_addr = server.local_addr().unwrap();
    
    // Spawn a task to accept and close
    let close_task = tokio::spawn(async move {
        let conn = server.accept().await.unwrap();
        conn.close().await
    });
    
    wait_for_server_ready().await;
    
    // Connect
    let mut client = create_test_client();
    client.connect(server_addr).await.unwrap();
    
    // Wait for close
    let result = close_task.await.unwrap();
    assert!(result.is_ok());
    
    client.close().await.unwrap();
}

// =============================================================================
// Server Timeout Configuration Tests
// =============================================================================

/// Test server with short read timeout
#[tokio::test]
async fn test_server_short_read_timeout() {
    let config = MllpServerConfig {
        read_timeout: Duration::from_millis(50),
        ..Default::default()
    };
    
    let mut server = MllpServer::new(config);
    let bind_addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    
    let result = server.bind(bind_addr).await;
    assert!(result.is_ok());
}

/// Test server with short write timeout
#[tokio::test]
async fn test_server_short_write_timeout() {
    let config = MllpServerConfig {
        write_timeout: Duration::from_millis(50),
        ..Default::default()
    };
    
    let mut server = MllpServer::new(config);
    let bind_addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    
    let result = server.bind(bind_addr).await;
    assert!(result.is_ok());
}

/// Test server with small max frame size
#[tokio::test]
async fn test_server_small_max_frame_size() {
    let config = MllpServerConfig {
        max_frame_size: 1024,
        ..Default::default()
    };
    
    let mut server = MllpServer::new(config);
    let bind_addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    
    let result = server.bind(bind_addr).await;
    assert!(result.is_ok());
}

/// Test server with custom backlog
#[tokio::test]
async fn test_server_custom_backlog() {
    let config = MllpServerConfig {
        backlog: 256,
        ..Default::default()
    };
    
    let mut server = MllpServer::new(config);
    let bind_addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    
    let result = server.bind(bind_addr).await;
    assert!(result.is_ok());
}

// =============================================================================
// Server with Different ACK Timing Tests
// =============================================================================

/// Test server with Immediate ACK timing
#[tokio::test]
async fn test_server_immediate_ack_timing() {
    let config = MllpServerConfig {
        ack_timing: AckTimingPolicy::Immediate,
        ..Default::default()
    };
    
    let mut server = MllpServer::new(config);
    let bind_addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    server.bind(bind_addr).await.unwrap();
    let addr = server.local_addr().unwrap();
    
    tokio::spawn(async move {
        let _ = server.run(AckHandler::new(Arc::new(Notify::new()))).await;
    });
    
    wait_for_server_ready().await;
    
    let mut client = create_test_client();
    client.connect(addr).await.unwrap();
    
    let message = create_test_message();
    let ack = client.send_message(&message).await;
    
    assert!(ack.is_ok());
    client.close().await.unwrap();
}

/// Test server with Delayed ACK timing configuration
#[tokio::test]
async fn test_server_delayed_ack_timing_config() {
    let config = MllpServerConfig {
        ack_timing: AckTimingPolicy::Delayed(Duration::from_millis(10)),
        ..Default::default()
    };
    
    let mut server = MllpServer::new(config);
    let bind_addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    
    let result = server.bind(bind_addr).await;
    assert!(result.is_ok());
}

/// Test server with OnDemand ACK timing configuration
#[tokio::test]
async fn test_server_on_demand_ack_timing_config() {
    let config = MllpServerConfig {
        ack_timing: AckTimingPolicy::OnDemand,
        ..Default::default()
    };
    
    let mut server = MllpServer::new(config);
    let bind_addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    
    let result = server.bind(bind_addr).await;
    assert!(result.is_ok());
}
