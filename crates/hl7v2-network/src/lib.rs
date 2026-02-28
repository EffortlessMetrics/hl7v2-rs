//! Network functionality for HL7 v2 MLLP connections.
//!
//! This crate provides:
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
//! use hl7v2_network::{MllpClient, MllpClientBuilder};
//! use hl7v2_model::{Message, Delims};
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
//! let addr: std::net::SocketAddr = "127.0.0.1:2575".parse()?;
//! client.connect(addr).await?;
//!
//! // Send a message (assumes you have a Message)
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
//! use hl7v2_network::{MllpServer, MllpServerConfig, MessageHandler};
//! use hl7v2_model::{Message, Error};
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
//! let addr: std::net::SocketAddr = "127.0.0.1:2575".parse()?;
//! server.bind(addr).await?;
//! server.run(MyHandler).await?;
//! # Ok(())
//! # }
//! ```

mod client;
mod codec;
mod server;

pub use client::{MllpClient, MllpClientBuilder, MllpClientConfig};
pub use codec::MllpCodec;
pub use server::{AckTimingPolicy, MessageHandler, MllpConnection, MllpServer, MllpServerConfig};

#[cfg(test)]
mod tests;
