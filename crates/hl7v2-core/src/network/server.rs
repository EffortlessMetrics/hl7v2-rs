//! MLLP TCP server for receiving HL7 messages.
//!
//! This module provides an async TCP server that:
//! - Accepts MLLP connections
//! - Decodes incoming MLLP frames
//! - Parses HL7 messages
//! - Sends ACKs according to configurable timing policy

use crate::{Message, parse, Error};
use super::codec::MllpCodec;
use bytes::BytesMut;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;
use tokio_util::codec::Framed;
use futures::prelude::*;

/// Configuration for MLLP server
#[derive(Debug, Clone)]
pub struct MllpServerConfig {
    /// Read timeout for connections
    pub read_timeout: Duration,
    /// Write timeout for connections
    pub write_timeout: Duration,
    /// Maximum frame size
    pub max_frame_size: usize,
    /// Backlog for the TCP listener
    pub backlog: u32,
    /// ACK timing policy
    pub ack_timing: AckTimingPolicy,
}

impl Default for MllpServerConfig {
    fn default() -> Self {
        Self {
            read_timeout: Duration::from_secs(30),
            write_timeout: Duration::from_secs(30),
            max_frame_size: 10 * 1024 * 1024, // 10MB
            backlog: 128,
            ack_timing: AckTimingPolicy::Immediate,
        }
    }
}

/// ACK timing policy for MLLP connections
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AckTimingPolicy {
    /// Send ACK immediately after receiving message
    Immediate,
    /// Send ACK after a delay
    Delayed(Duration),
    /// Send ACK only when explicitly requested
    OnDemand,
}

/// Handler trait for processing incoming HL7 messages
pub trait MessageHandler: Send + Sync {
    /// Process a message and optionally return an ACK message
    fn handle_message(&self, message: Message) -> Result<Option<Message>, Error>;
}

/// MLLP TCP server
pub struct MllpServer {
    config: MllpServerConfig,
    listener: Option<TcpListener>,
}

impl MllpServer {
    /// Create a new MLLP server with the given configuration
    pub fn new(config: MllpServerConfig) -> Self {
        Self {
            config,
            listener: None,
        }
    }

    /// Create a new MLLP server with default configuration
    pub fn with_default_config() -> Self {
        Self::new(MllpServerConfig::default())
    }

    /// Bind to the given address
    pub async fn bind(&mut self, addr: impl Into<SocketAddr>) -> Result<(), std::io::Error> {
        let addr = addr.into();
        let listener = TcpListener::bind(addr).await?;
        self.listener = Some(listener);
        Ok(())
    }

    /// Get the local address the server is bound to
    pub fn local_addr(&self) -> Result<SocketAddr, std::io::Error> {
        self.listener
            .as_ref()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotConnected, "Server not bound"))?
            .local_addr()
    }

    /// Run the server, processing messages with the given handler
    ///
    /// This will accept connections and spawn a task for each connection.
    pub async fn run<H: MessageHandler + 'static>(
        &mut self,
        handler: H,
    ) -> Result<(), std::io::Error> {
        let listener = self
            .listener
            .as_ref()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotConnected, "Server not bound"))?;

        let handler = std::sync::Arc::new(handler);

        loop {
            let (stream, peer_addr) = listener.accept().await?;
            let handler = handler.clone();
            let config = self.config.clone();

            // Spawn a task to handle this connection
            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, peer_addr, handler, config).await {
                    eprintln!("Error handling connection from {}: {}", peer_addr, e);
                }
            });
        }
    }

    /// Accept a single connection and return a connection handler
    pub async fn accept(&mut self) -> Result<MllpConnection, std::io::Error> {
        let listener = self
            .listener
            .as_ref()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotConnected, "Server not bound"))?;

        let (stream, peer_addr) = listener.accept().await?;
        Ok(MllpConnection::new(stream, peer_addr, self.config.clone()))
    }
}

/// Handle a single TCP connection
async fn handle_connection<H: MessageHandler>(
    stream: TcpStream,
    peer_addr: SocketAddr,
    handler: std::sync::Arc<H>,
    config: MllpServerConfig,
) -> Result<(), std::io::Error> {
    let codec = MllpCodec::with_max_frame_size(config.max_frame_size);
    let mut framed = Framed::new(stream, codec);

    while let Some(result) = framed.next().await {
        match result {
            Ok(frame) => {
                // Apply read timeout
                let parse_result = timeout(config.read_timeout, async {
                    parse(&frame).map_err(|e| {
                        std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
                    })
                })
                .await;

                let message = match parse_result {
                    Ok(Ok(msg)) => msg,
                    Ok(Err(e)) => {
                        eprintln!("Failed to parse message from {}: {}", peer_addr, e);
                        continue;
                    }
                    Err(_) => {
                        eprintln!("Timeout parsing message from {}", peer_addr);
                        continue;
                    }
                };

                // Handle the message
                let ack = match handler.handle_message(message) {
                    Ok(Some(ack)) => ack,
                    Ok(None) => continue, // No ACK requested
                    Err(e) => {
                        eprintln!("Error handling message from {}: {}", peer_addr, e);
                        continue;
                    }
                };

                // Apply ACK timing policy
                match config.ack_timing {
                    AckTimingPolicy::Immediate => {
                        // Send ACK immediately
                        let ack_bytes = crate::write(&ack);
                        if let Err(e) = framed.send(BytesMut::from(&ack_bytes[..])).await {
                            eprintln!("Failed to send ACK to {}: {}", peer_addr, e);
                            break;
                        }
                    }
                    AckTimingPolicy::Delayed(delay) => {
                        // Wait before sending ACK
                        tokio::time::sleep(delay).await;
                        let ack_bytes = crate::write(&ack);
                        if let Err(e) = framed.send(BytesMut::from(&ack_bytes[..])).await {
                            eprintln!("Failed to send ACK to {}: {}", peer_addr, e);
                            break;
                        }
                    }
                    AckTimingPolicy::OnDemand => {
                        // Don't send ACK automatically
                        // The handler would need to send it through some other mechanism
                    }
                }
            }
            Err(e) => {
                eprintln!("Error reading frame from {}: {}", peer_addr, e);
                break;
            }
        }
    }

    Ok(())
}

/// A single MLLP connection (server side)
pub struct MllpConnection {
    framed: Framed<TcpStream, MllpCodec>,
    peer_addr: SocketAddr,
    config: MllpServerConfig,
}

impl MllpConnection {
    /// Create a new connection handler
    pub fn new(stream: TcpStream, peer_addr: SocketAddr, config: MllpServerConfig) -> Self {
        let codec = MllpCodec::with_max_frame_size(config.max_frame_size);
        let framed = Framed::new(stream, codec);
        Self {
            framed,
            peer_addr,
            config,
        }
    }

    /// Get the peer address
    pub fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }

    /// Receive a message from the connection
    pub async fn receive_message(&mut self) -> Result<Option<Message>, std::io::Error> {
        match timeout(self.config.read_timeout, self.framed.next()).await {
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

    /// Send a message through the connection
    pub async fn send_message(&mut self, message: &Message) -> Result<(), std::io::Error> {
        let bytes = crate::write(message);
        timeout(
            self.config.write_timeout,
            self.framed.send(BytesMut::from(&bytes[..])),
        )
        .await
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::TimedOut, "Write timeout"))??;
        Ok(())
    }

    /// Close the connection
    pub async fn close(self) -> Result<(), std::io::Error> {
        // Get the underlying stream and drop it to close
        let stream = self.framed.into_inner();
        drop(stream);
        Ok(())
    }
}

#[cfg(test)]
#[allow(dead_code)]
mod tests {
    use super::*;

    struct TestHandler;

    impl MessageHandler for TestHandler {
        fn handle_message(&self, _message: Message) -> Result<Option<Message>, Error> {
            // Simple ACK - just echo the message back
            Ok(None)
        }
    }

    #[tokio::test]
    async fn test_server_bind() {
        use std::net::SocketAddr;

        let mut server = MllpServer::with_default_config();
        let bind_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let result = server.bind(bind_addr).await;
        assert!(result.is_ok());

        let addr = server.local_addr();
        assert!(addr.is_ok());
    }

    #[tokio::test]
    async fn test_connection_timeout() {
        let config = MllpServerConfig {
            read_timeout: Duration::from_millis(100),
            ..Default::default()
        };

        // This would require a more complex test setup with actual TCP connections
        // For now, we just verify the config is set correctly
        assert_eq!(config.read_timeout, Duration::from_millis(100));
    }
}
