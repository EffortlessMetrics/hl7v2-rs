//! Network functionality for HL7 v2 MLLP connections.
//!
//! This module provides:
//! - **MLLP Codec**: Encoding and decoding of MLLP frames using Tokio's codec framework
//! - **MLLP Client**: Async TCP client for sending HL7 messages and receiving ACKs
//! - **MLLP Server**: Async TCP server for receiving HL7 messages and sending ACKs
//!
//! # MLLP Protocol
//!
//! MLLP (Minimal Lower Layer Protocol) is a simple framing protocol used to transmit
//! HL7 messages over TCP. Each message is wrapped with:
//! - Start byte: `0x0B` (vertical tab)
//! - Message content (HL7 message)
//! - End bytes: `0x1C 0x0D` (file separator + carriage return)
//!
//! # Examples
//!
//! ## Client Example
//!
//! ```no_run
//! use hl7v2_core::network::{MllpClient, MllpClientBuilder};
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a client
//! let mut client = MllpClientBuilder::new()
//!     .connect_timeout(Duration::from_secs(5))
//!     .read_timeout(Duration::from_secs(30))
//!     .build();
//!
//! // Connect to server
//! client.connect("127.0.0.1:2575").await?;
//!
//! // Send a message (assumes you have a Message)
//! # use hl7v2_core::{Message, Delims};
//! # let message = Message { delims: Delims::default(), segments: vec![], charsets: vec![] };
//! let ack = client.send_message(&message).await?;
//!
//! // Close connection
//! client.close().await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Server Example
//!
//! ```no_run
//! use hl7v2_core::network::{MllpServer, MllpServerConfig, MessageHandler};
//! use hl7v2_core::{Message, Error};
//!
//! struct MyHandler;
//!
//! impl MessageHandler for MyHandler {
//!     fn handle_message(&self, message: Message) -> Result<Option<Message>, Error> {
//!         // Process the message and optionally return an ACK
//!         println!("Received message with {} segments", message.segments.len());
//!         Ok(None) // Return Some(ack_message) to send an ACK
//!     }
//! }
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut server = MllpServer::new(MllpServerConfig::default());
//! server.bind("127.0.0.1:2575").await?;
//! server.run(MyHandler).await?;
//! # Ok(())
//! # }
//! ```

mod codec;
mod client;
mod server;

pub use codec::MllpCodec;
pub use client::{MllpClient, MllpClientBuilder, MllpClientConfig};
pub use server::{
    MllpServer, MllpServerConfig, MllpConnection, MessageHandler, AckTimingPolicy,
};

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::{Message, Delims, Segment, Field, Rep, Comp, Atom, parse, write};
    use tokio::time::Duration;

    /// Create a simple test message
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

    #[tokio::test]
    async fn test_client_server_integration() {
        use std::sync::Arc;
        use tokio::sync::Notify;
        use std::net::SocketAddr;

        struct TestHandler {
            notify: Arc<Notify>,
        }

        impl MessageHandler for TestHandler {
            fn handle_message(&self, _message: Message) -> Result<Option<Message>, crate::Error> {
                self.notify.notify_one();
                Ok(Some(create_test_message())) // Echo back as ACK
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
        tokio::time::sleep(Duration::from_millis(100)).await;

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

    #[tokio::test]
    async fn test_codec_roundtrip() {
        use bytes::BytesMut;
        use tokio_util::codec::{Decoder, Encoder};

        let mut codec = MllpCodec::new();
        let original = BytesMut::from("MSH|^~\\&|TEST\r");

        // Encode
        let mut encoded = BytesMut::new();
        codec.encode(original.clone(), &mut encoded).unwrap();

        // Decode
        let decoded = codec.decode(&mut encoded).unwrap();
        assert!(decoded.is_some());
        assert_eq!(decoded.unwrap(), original);
    }
}
