//! MLLP TCP client for sending HL7 messages.
//!
//! This module provides an async TCP client that:
//! - Connects to MLLP servers
//! - Encodes and sends HL7 messages with MLLP framing
//! - Receives and decodes ACK responses

use crate::{Message, parse};
use super::codec::MllpCodec;
use bytes::BytesMut;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_util::codec::Framed;
use futures::prelude::*;

/// Configuration for MLLP client
#[derive(Debug, Clone)]
pub struct MllpClientConfig {
    /// Connection timeout
    pub connect_timeout: Duration,
    /// Read timeout for responses
    pub read_timeout: Duration,
    /// Write timeout for sending messages
    pub write_timeout: Duration,
    /// Maximum frame size
    pub max_frame_size: usize,
}

impl Default for MllpClientConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(10),
            read_timeout: Duration::from_secs(30),
            write_timeout: Duration::from_secs(30),
            max_frame_size: 10 * 1024 * 1024, // 10MB
        }
    }
}

/// MLLP TCP client
pub struct MllpClient {
    config: MllpClientConfig,
    framed: Option<Framed<TcpStream, MllpCodec>>,
    peer_addr: Option<SocketAddr>,
}

impl MllpClient {
    /// Create a new MLLP client with the given configuration
    pub fn new(config: MllpClientConfig) -> Self {
        Self {
            config,
            framed: None,
            peer_addr: None,
        }
    }

    /// Create a new MLLP client with default configuration
    pub fn with_default_config() -> Self {
        Self::new(MllpClientConfig::default())
    }

    /// Connect to a remote MLLP server
    pub async fn connect(&mut self, addr: impl Into<SocketAddr>) -> Result<(), std::io::Error> {
        let addr = addr.into();

        let stream = timeout(self.config.connect_timeout, TcpStream::connect(addr))
            .await
            .map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::TimedOut, "Connection timeout")
            })??;

        let codec = MllpCodec::with_max_frame_size(self.config.max_frame_size);
        self.framed = Some(Framed::new(stream, codec));
        self.peer_addr = Some(addr);

        Ok(())
    }

    /// Check if the client is connected
    pub fn is_connected(&self) -> bool {
        self.framed.is_some()
    }

    /// Get the peer address if connected
    pub fn peer_addr(&self) -> Option<SocketAddr> {
        self.peer_addr
    }

    /// Send a message and wait for an ACK response
    pub async fn send_message(&mut self, message: &Message) -> Result<Message, std::io::Error> {
        let framed = self
            .framed
            .as_mut()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotConnected, "Not connected"))?;

        // Serialize the message
        let bytes = crate::write(message);

        // Send the message with timeout
        timeout(
            self.config.write_timeout,
            framed.send(BytesMut::from(&bytes[..])),
        )
        .await
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::TimedOut, "Write timeout"))??;

        // Wait for ACK response with timeout
        let response = timeout(self.config.read_timeout, framed.next())
            .await
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::TimedOut, "Read timeout"))?
            .ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "Connection closed")
            })??;

        // Parse the ACK
        let ack = parse(&response)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;

        Ok(ack)
    }

    /// Send a message without waiting for a response (fire-and-forget)
    pub async fn send_message_no_ack(&mut self, message: &Message) -> Result<(), std::io::Error> {
        let framed = self
            .framed
            .as_mut()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotConnected, "Not connected"))?;

        // Serialize the message
        let bytes = crate::write(message);

        // Send the message with timeout
        timeout(
            self.config.write_timeout,
            framed.send(BytesMut::from(&bytes[..])),
        )
        .await
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::TimedOut, "Write timeout"))??;

        Ok(())
    }

    /// Receive a message from the server
    pub async fn receive_message(&mut self) -> Result<Option<Message>, std::io::Error> {
        let framed = self
            .framed
            .as_mut()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotConnected, "Not connected"))?;

        match timeout(self.config.read_timeout, framed.next()).await {
            Ok(Some(Ok(frame))) => {
                let message = parse(&frame).map_err(|e| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
                })?;
                Ok(Some(message))
            }
            Ok(Some(Err(e))) => Err(e),
            Ok(None) => Ok(None), // Connection closed
            Err(_) => Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "Read timeout",
            )),
        }
    }

    /// Close the connection
    pub async fn close(mut self) -> Result<(), std::io::Error> {
        if let Some(framed) = self.framed.take() {
            // Get the underlying stream and shut it down
            let stream = framed.into_inner();
            // Just dropping the stream will close it
            drop(stream);
        }
        Ok(())
    }

    /// Disconnect without consuming the client (allows reconnection)
    pub async fn disconnect(&mut self) -> Result<(), std::io::Error> {
        if let Some(framed) = self.framed.take() {
            // Get the underlying stream and shut it down
            let stream = framed.into_inner();
            // Just dropping the stream will close it
            drop(stream);
        }
        self.peer_addr = None;
        Ok(())
    }
}

/// Builder for creating MLLP clients with custom configuration
pub struct MllpClientBuilder {
    config: MllpClientConfig,
}

impl MllpClientBuilder {
    /// Create a new client builder with default configuration
    pub fn new() -> Self {
        Self {
            config: MllpClientConfig::default(),
        }
    }

    /// Set the connection timeout
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.config.connect_timeout = timeout;
        self
    }

    /// Set the read timeout
    pub fn read_timeout(mut self, timeout: Duration) -> Self {
        self.config.read_timeout = timeout;
        self
    }

    /// Set the write timeout
    pub fn write_timeout(mut self, timeout: Duration) -> Self {
        self.config.write_timeout = timeout;
        self
    }

    /// Set the maximum frame size
    pub fn max_frame_size(mut self, size: usize) -> Self {
        self.config.max_frame_size = size;
        self
    }

    /// Build the client
    pub fn build(self) -> MllpClient {
        MllpClient::new(self.config)
    }
}

impl Default for MllpClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_builder() {
        let client = MllpClientBuilder::new()
            .connect_timeout(Duration::from_secs(5))
            .read_timeout(Duration::from_secs(15))
            .write_timeout(Duration::from_secs(15))
            .max_frame_size(1024 * 1024)
            .build();

        assert_eq!(client.config.connect_timeout, Duration::from_secs(5));
        assert_eq!(client.config.read_timeout, Duration::from_secs(15));
        assert_eq!(client.config.write_timeout, Duration::from_secs(15));
        assert_eq!(client.config.max_frame_size, 1024 * 1024);
    }

    #[test]
    fn test_client_not_connected() {
        let client = MllpClient::with_default_config();
        assert!(!client.is_connected());
        assert!(client.peer_addr().is_none());
    }

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
    }
}
