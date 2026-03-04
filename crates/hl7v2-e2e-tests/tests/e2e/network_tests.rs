//! Network communication tests for MLLP over TCP.
//!
//! These tests validate:
//! - Starting server and connecting clients
//! - MLLP framing over TCP
//! - Multiple concurrent connections
//! - Message exchange patterns

use bytes::BytesMut;
use hl7v2_ack::{AckCode, ack};
use hl7v2_network::{MessageHandler, MllpClientBuilder, MllpCodec, MllpServer, MllpServerConfig};
use hl7v2_parser::parse;
use hl7v2_test_utils::{MockMllpServer, SampleMessages};
use hl7v2_writer::write;
use tokio_util::codec::{Decoder, Encoder};

use super::common::init_tracing;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{sleep, timeout};

static NEXT_PORT: std::sync::atomic::AtomicU16 = std::sync::atomic::AtomicU16::new(45000);

// =========================================================================
// Basic MLLP Framing Tests
// =========================================================================

mod mllp_framing {
    use super::*;

    #[tokio::test]
    async fn test_mllp_encode_decode_roundtrip() {
        init_tracing();

        let message = SampleMessages::adt_a01();
        let mut codec = MllpCodec::new();

        // Encode using Encoder trait
        let mut dst = BytesMut::new();
        codec
            .encode(BytesMut::from(message.as_bytes()), &mut dst)
            .expect("Should encode");

        // Verify MLLP framing: SB (0x0B) + message + EB (0x1C) + CR (0x0D)
        assert_eq!(dst[0], 0x0B, "Should start with SB");
        assert_eq!(dst[dst.len() - 2], 0x1C, "Should end with EB");
        assert_eq!(dst[dst.len() - 1], 0x0D, "Should end with CR");

        // Decode using Decoder trait
        let decoded = codec
            .decode(&mut dst)
            .expect("Should decode")
            .expect("Should have data");

        assert_eq!(decoded.as_ref(), message.as_bytes());
    }

    #[tokio::test]
    async fn test_mllp_framing_with_special_characters() {
        init_tracing();

        let message =
            SampleMessages::edge_case("special_chars").expect("Should have special_chars");
        let mut codec = MllpCodec::new();

        // Encode
        let mut dst = BytesMut::new();
        codec
            .encode(BytesMut::from(message.as_bytes()), &mut dst)
            .expect("Should encode");

        // Decode
        let decoded = codec
            .decode(&mut dst)
            .expect("Should decode")
            .expect("Should have data");

        assert_eq!(decoded.as_ref(), message.as_bytes());
    }
}

// =========================================================================
// Mock Server Tests
// =========================================================================

mod mock_server_tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_server_starts_and_stops() {
        init_tracing();

        let mut server = MockMllpServer::new();
        server
            .start("127.0.0.1:0")
            .await
            .expect("Server should start");

        let addr = server.local_addr().expect("Should have address");
        assert!(addr.port() > 0);

        server.stop().await;
    }

    #[tokio::test]
    async fn test_mock_server_receives_message() {
        init_tracing();

        let mut server = MockMllpServer::new();
        server
            .start("127.0.0.1:0")
            .await
            .expect("Server should start");
        let addr = server.local_addr().expect("Should have address");

        // Connect and send a message
        let mut stream = TcpStream::connect(addr).await.expect("Should connect");

        let message = SampleMessages::adt_a01();
        let framed = frame_mllp(message.as_bytes());

        stream.write_all(&framed).await.expect("Should write");
        stream.flush().await.expect("Should flush");

        // Wait for message to be received
        server
            .wait_for_messages(1, Duration::from_secs(5))
            .await
            .unwrap();

        let messages = server.received_messages().await;
        assert_eq!(messages.len(), 1);

        server.stop().await;
    }

    #[tokio::test]
    async fn test_mock_server_sends_response() {
        init_tracing();

        let mut server = MockMllpServer::new();
        server
            .start("127.0.0.1:0")
            .await
            .expect("Server should start");
        let addr = server.local_addr().expect("Should have address");

        // Queue a response
        let ack =
            b"MSH|^~\\&|RecvApp|RecvFac|SendApp|SendFac|20250128120000||ACK|1|P|2.5\rMSA|AA|1|OK\r";
        server.queue_mllp_response(ack).await;

        // Connect and send a message
        let mut stream = TcpStream::connect(addr).await.expect("Should connect");

        let message = SampleMessages::adt_a01();
        let framed = frame_mllp(message.as_bytes());

        stream.write_all(&framed).await.expect("Should write");
        stream.flush().await.expect("Should flush");

        // Read response
        let mut buffer = vec![0u8; 4096];
        let n = timeout(Duration::from_secs(5), stream.read(&mut buffer))
            .await
            .expect("Should receive response")
            .expect("Read should succeed");

        assert!(n > 0, "Should receive response");

        server.stop().await;
    }

    /// Helper to frame a message with MLLP
    fn frame_mllp(data: &[u8]) -> Vec<u8> {
        let mut framed = Vec::with_capacity(data.len() + 3);
        framed.push(0x0B); // SB
        framed.extend_from_slice(data);
        framed.push(0x1C); // EB
        framed.push(0x0D); // CR
        framed
    }
}

// =========================================================================
// Real Network Client-Server Tests
// =========================================================================

mod client_server_tests {
    use super::*;
    use std::net::SocketAddr;

    #[tokio::test]
    async fn test_client_connects_to_server() {
        init_tracing();

        let port = find_available_port().await;
        let addr: SocketAddr = format!("127.0.0.1:{}", port)
            .parse()
            .expect("Should parse address");

        // Start server in background
        let server_task = tokio::spawn(async move {
            let config = MllpServerConfig::default();
            let mut server = MllpServer::new(config);
            server.bind(addr).await.expect("Server should bind");

            // Accept one connection
            let _conn = server.accept().await.expect("Should accept connection");

            // Keep connection alive briefly
            sleep(Duration::from_millis(100)).await;
        });

        // Give server time to start
        sleep(Duration::from_millis(100)).await;

        // Client connects
        let client_result = timeout(Duration::from_secs(5), async {
            let mut client = MllpClientBuilder::new()
                .connect_timeout(Duration::from_secs(5))
                .build();
            client.connect(addr).await
        })
        .await;

        assert!(client_result.is_ok(), "Client should connect");

        let _ = timeout(Duration::from_secs(2), server_task).await;
    }

    #[tokio::test]
    async fn test_client_send_and_receive() {
        init_tracing();

        let port = find_available_port().await;
        let addr: SocketAddr = format!("127.0.0.1:{}", port)
            .parse()
            .expect("Should parse address");

        // Start server with handler
        let server_task = tokio::spawn(async move {
            let config = MllpServerConfig::default();
            let mut server = MllpServer::new(config);
            server.bind(addr).await.expect("Server should bind");

            let mut conn = server.accept().await.expect("Should accept");

            // Process one message
            if let Some(msg) = conn.receive_message().await.expect("Should receive") {
                let ack_msg = ack(&msg, AckCode::AA).expect("Should generate ACK");
                conn.send_message(&ack_msg).await.expect("Should send ACK");
            }
        });

        // Give server time to start
        sleep(Duration::from_millis(100)).await;

        // Client sends message and receives ACK
        let client_task = async {
            let mut client = MllpClientBuilder::new()
                .connect_timeout(Duration::from_secs(5))
                .read_timeout(Duration::from_secs(5))
                .build();
            client.connect(addr).await.expect("Should connect");

            let message =
                parse(SampleMessages::adt_a01().as_bytes()).expect("Should parse test message");

            let ack = client
                .send_message(&message)
                .await
                .expect("Should send and receive");
            ack
        };

        let (server_result, client_result) = tokio::join!(
            timeout(Duration::from_secs(10), server_task),
            timeout(Duration::from_secs(10), client_task)
        );

        assert!(server_result.is_ok(), "Server should complete");
        assert!(client_result.is_ok(), "Client should complete");

        let ack = client_result.unwrap();
        let msg_type = hl7v2_core::get(&ack, "MSH.9");
        assert!(msg_type.is_some());
    }

    async fn find_available_port() -> u16 {
        super::NEXT_PORT.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }
}

// =========================================================================
// Multiple Concurrent Connections Tests
// =========================================================================

mod concurrent_connections {
    use super::*;
    use std::net::SocketAddr;

    async fn find_available_port() -> u16 {
        super::NEXT_PORT.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    #[tokio::test]
    async fn test_multiple_concurrent_clients() {
        init_tracing();

        let port = find_available_port().await;
        let addr: std::net::SocketAddr = format!("127.0.0.1:{}", port)
            .parse()
            .expect("Should parse address");

        let connection_count = Arc::new(AtomicUsize::new(0));
        let message_count = Arc::new(AtomicUsize::new(0));

        // Start server
        let connection_count_clone = connection_count.clone();
        let message_count_clone = message_count.clone();

        let server_task = tokio::spawn(async move {
            let config = MllpServerConfig::default();
            let mut server = MllpServer::new(config);
            server.bind(addr).await.expect("Server should bind");

            // Accept connections for a short time
            for _ in 0..3 {
                if let Ok(conn) = timeout(Duration::from_millis(500), server.accept()).await {
                    if let Ok(mut conn) = conn {
                        connection_count_clone.fetch_add(1, Ordering::SeqCst);

                        let msg_count = message_count_clone.clone();
                        tokio::spawn(async move {
                            while let Ok(Some(msg)) = conn.receive_message().await {
                                msg_count.fetch_add(1, Ordering::SeqCst);
                                if let Ok(ack_msg) = ack(&msg, AckCode::AA) {
                                    let _ = conn.send_message(&ack_msg).await;
                                }
                            }
                        });
                    }
                }
            }
        });

        // Give server time to start
        sleep(Duration::from_millis(100)).await;

        // Launch multiple clients concurrently
        let mut client_handles = vec![];

        for _ in 0..3 {
            let client_task = async move {
                let mut client = MllpClientBuilder::new()
                    .connect_timeout(Duration::from_secs(2))
                    .read_timeout(Duration::from_secs(2))
                    .build();

                // Try to connect, may fail if server is busy
                if client.connect(addr).await.is_ok() {
                    let msg = parse(SampleMessages::adt_a01().as_bytes()).expect("Should parse");
                    let _ = client.send_message(&msg).await;
                    true
                } else {
                    false
                }
            };
            client_handles.push(tokio::spawn(client_task));
        }

        // Wait for all clients
        let mut successful_clients = 0;
        for handle in client_handles {
            if let Ok(Ok(true)) = timeout(Duration::from_secs(5), handle).await {
                successful_clients += 1;
            }
        }

        // At least one client should succeed
        assert!(
            successful_clients >= 1,
            "At least one client should connect and send"
        );

        let _ = timeout(Duration::from_secs(2), server_task).await;
    }

    #[tokio::test]
    async fn test_sequential_messages_same_connection() {
        init_tracing();

        let port = find_available_port().await;
        let addr: std::net::SocketAddr = format!("127.0.0.1:{}", port)
            .parse()
            .expect("Should parse address");

        // Start server
        let server_task = tokio::spawn(async move {
            let config = MllpServerConfig::default();
            let mut server = MllpServer::new(config);
            server.bind(addr).await.expect("Server should bind");

            let mut conn = server.accept().await.expect("Should accept");

            // Process multiple messages on same connection
            for _ in 0..5 {
                if let Ok(Some(msg)) = conn.receive_message().await {
                    if let Ok(ack_msg) = ack(&msg, AckCode::AA) {
                        let _ = conn.send_message(&ack_msg).await;
                    }
                }
            }
        });

        sleep(Duration::from_millis(100)).await;

        // Client sends multiple messages
        let client_task = async {
            let mut client = MllpClientBuilder::new()
                .connect_timeout(Duration::from_secs(5))
                .read_timeout(Duration::from_secs(5))
                .build();
            client.connect(addr).await.expect("Should connect");

            let messages = vec![
                SampleMessages::adt_a01(),
                SampleMessages::adt_a04(),
                SampleMessages::oru_r01(),
            ];

            let mut acks_received = 0;
            for msg_str in &messages {
                let msg = parse(msg_str.as_bytes()).expect("Should parse");
                if client.send_message(&msg).await.is_ok() {
                    acks_received += 1;
                }
            }
            acks_received
        };

        let (server_result, client_result) = tokio::join!(
            timeout(Duration::from_secs(15), server_task),
            timeout(Duration::from_secs(15), client_task)
        );

        assert!(server_result.is_ok());
        assert!(client_result.is_ok());
        assert!(
            client_result.unwrap() >= 1,
            "Should receive at least one ACK"
        );
    }
}

// =========================================================================
// Error Handling Tests
// =========================================================================

mod error_handling {
    use super::*;
    use std::net::SocketAddr;

    async fn find_available_port() -> u16 {
        super::NEXT_PORT.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    #[tokio::test]
    async fn test_connection_timeout() {
        init_tracing();

        let mut client = MllpClientBuilder::new()
            .connect_timeout(Duration::from_millis(100))
            .build();

        // Try to connect to a non-existent server
        let addr: SocketAddr = "127.0.0.1:59999".parse().unwrap();
        let result = client.connect(addr).await;
        assert!(
            result.is_err(),
            "Should fail to connect to non-existent server"
        );
    }

    #[tokio::test]
    async fn test_server_handles_malformed_message() {
        init_tracing();

        let port = find_available_port().await;
        let addr: std::net::SocketAddr = format!("127.0.0.1:{}", port)
            .parse()
            .expect("Should parse address");

        // Start server
        let server_task = tokio::spawn(async move {
            let config = MllpServerConfig::default();
            let mut server = MllpServer::new(config);
            server.bind(addr).await.expect("Server should bind");

            let mut conn = server.accept().await.expect("Should accept");

            // Try to receive - should handle malformed data gracefully
            if let Ok(Some(msg)) = conn.receive_message().await {
                // Even malformed messages that parse should get an ACK
                if let Ok(ack_msg) = ack(&msg, AckCode::AE) {
                    let _ = conn.send_message(&ack_msg).await;
                }
            }
        });

        sleep(Duration::from_millis(100)).await;

        // Send raw MLLP frame with invalid HL7
        let client_task = async {
            let mut stream = TcpStream::connect(addr).await.expect("Should connect");

            // MLLP frame with invalid content
            let invalid = b"\x0BINVALID HL7 MESSAGE\x1C\x0D";
            stream.write_all(invalid).await.expect("Should write");
            stream.flush().await.expect("Should flush");

            sleep(Duration::from_millis(200)).await;
        };

        let _ = tokio::join!(
            timeout(Duration::from_secs(5), server_task),
            timeout(Duration::from_secs(5), client_task)
        );
        // Test passes if neither panics
    }
}

// =========================================================================
// Network Stress Tests
// =========================================================================

mod stress_tests {
    use super::*;
    use std::net::SocketAddr;

    async fn find_available_port() -> u16 {
        super::NEXT_PORT.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    #[tokio::test]
    #[ignore = "Stress test - run manually"]
    async fn test_high_throughput_messages() {
        init_tracing();

        let port = find_available_port().await;
        let addr: std::net::SocketAddr = format!("127.0.0.1:{}", port)
            .parse()
            .expect("Should parse address");

        let total_messages = Arc::new(AtomicUsize::new(0));
        let total_clone = total_messages.clone();

        // Start server
        let server_task = tokio::spawn(async move {
            let config = MllpServerConfig::default();
            let mut server = MllpServer::new(config);
            server.bind(addr).await.expect("Server should bind");

            for _ in 0..100 {
                if let Ok(conn) = timeout(Duration::from_millis(100), server.accept()).await {
                    if let Ok(mut conn) = conn {
                        let count = total_clone.clone();
                        tokio::spawn(async move {
                            while let Ok(Some(msg)) = conn.receive_message().await {
                                count.fetch_add(1, Ordering::SeqCst);
                                if let Ok(ack_msg) = ack(&msg, AckCode::AA) {
                                    let _ = conn.send_message(&ack_msg).await;
                                }
                            }
                        });
                    }
                }
            }
        });

        sleep(Duration::from_millis(100)).await;

        // Send many messages
        let start = std::time::Instant::now();
        let message_count = 100;

        for _ in 0..message_count {
            let _ = async {
                let mut client = MllpClientBuilder::new()
                    .connect_timeout(Duration::from_millis(500))
                    .read_timeout(Duration::from_millis(500))
                    .build();
                let addr: std::net::SocketAddr =
                    format!("127.0.0.1:{}", addr.port()).parse().unwrap();
                if client.connect(addr).await.is_ok() {
                    let msg = parse(SampleMessages::adt_a01().as_bytes()).unwrap();
                    let _ = client.send_message(&msg).await;
                }
            }
            .await;
        }

        let elapsed = start.elapsed();
        let msgs_per_sec = message_count as f64 / elapsed.as_secs_f64();

        println!("Throughput: {:.2} messages/second", msgs_per_sec);

        let _ = timeout(Duration::from_secs(2), server_task).await;
    }
}
