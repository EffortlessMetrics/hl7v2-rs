//! Mock implementations for testing HL7 network code.
//!
//! This module provides mock implementations of MLLP servers and message handlers
//! for testing network code without requiring actual network connections.
//!
//! # Available Mocks
//!
//! - [`MockMllpServer`] - A mock MLLP server for testing client code
//! - [`MockMessageHandler`] - A configurable message handler for testing
//!
//! # Example
//!
//! ```rust,ignore
//! use hl7v2_test_utils::mocks::{MockMllpServer, MockMessageHandler};
//!
//! #[tokio::test]
//! async fn test_client_sends_message() {
//!     // Create a mock server
//!     let mut server = MockMllpServer::new();
//!     server.start("127.0.0.1:0").await.unwrap();
//!     
//!     // Connect and send a message
//!     let addr = server.local_addr().unwrap();
//!     // ... client code here ...
//!     
//!     // Verify received messages
//!     let messages = server.received_messages().await;
//!     assert_eq!(messages.len(), 1);
//! }
//! ```

use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use hl7v2_model::{Error, Message};
use tokio::net::TcpListener;
use tokio::sync::{RwLock, mpsc};
use tokio::time::timeout;

/// A mock MLLP server for testing network clients.
///
/// This server listens on a configurable address and records all received
/// messages for verification in tests.
///
/// # Example
///
/// ```rust,ignore
/// let mut server = MockMllpServer::new();
/// server.start("127.0.0.1:2575").await.unwrap();
///
/// // ... send messages to the server ...
///
/// let messages = server.received_messages().await;
/// assert_eq!(messages.len(), 1);
/// ```
pub struct MockMllpServer {
    #[allow(dead_code)]
    listener: Option<TcpListener>,
    received: Arc<RwLock<VecDeque<Vec<u8>>>>,
    responses: Arc<RwLock<VecDeque<Vec<u8>>>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
    local_addr: Option<SocketAddr>,
}

impl Default for MockMllpServer {
    fn default() -> Self {
        Self::new()
    }
}

impl MockMllpServer {
    /// Create a new mock MLLP server.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let server = MockMllpServer::new();
    /// ```
    pub fn new() -> Self {
        Self {
            listener: None,
            received: Arc::new(RwLock::new(VecDeque::new())),
            responses: Arc::new(RwLock::new(VecDeque::new())),
            shutdown_tx: None,
            local_addr: None,
        }
    }

    /// Start the server on the given address.
    ///
    /// Use `"127.0.0.1:0"` to bind to a random available port.
    ///
    /// # Arguments
    ///
    /// * `addr` - The address to bind to (e.g., "127.0.0.1:2575" or "127.0.0.1:0")
    ///
    /// # Returns
    ///
    /// `Ok(())` if the server started successfully
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let mut server = MockMllpServer::new();
    /// server.start("127.0.0.1:0").await.unwrap();
    /// ```
    pub async fn start(&mut self, addr: &str) -> Result<(), std::io::Error> {
        let listener = TcpListener::bind(addr).await?;
        self.local_addr = Some(listener.local_addr()?);

        let received = self.received.clone();
        let responses = self.responses.clone();
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    accept_result = listener.accept() => {
                        match accept_result {
                            Ok((stream, _)) => {
                                let received = received.clone();
                                let responses = responses.clone();
                                tokio::spawn(async move {
                                    handle_connection(stream, received, responses).await;
                                });
                            }
                            Err(e) => {
                                eprintln!("Accept error: {}", e);
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Get the local address the server is bound to.
    ///
    /// # Returns
    ///
    /// The local address, or an error if the server is not running
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let addr = server.local_addr().unwrap();
    /// println!("Server listening on {}", addr);
    /// ```
    pub fn local_addr(&self) -> Result<SocketAddr, std::io::Error> {
        self.local_addr.ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotConnected, "Server not started")
        })
    }

    /// Get all received messages.
    ///
    /// Returns a copy of all messages received since the server started
    /// or since the last call to [`clear_received`](Self::clear_received).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let messages = server.received_messages().await;
    /// for msg in &messages {
    ///     println!("Received: {}", String::from_utf8_lossy(msg));
    /// }
    /// ```
    pub async fn received_messages(&self) -> Vec<Vec<u8>> {
        let received = self.received.read().await;
        received.iter().cloned().collect()
    }

    /// Get the number of received messages.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let count = server.received_count().await;
    /// assert_eq!(count, 2);
    /// ```
    pub async fn received_count(&self) -> usize {
        self.received.read().await.len()
    }

    /// Clear all received messages.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// server.clear_received().await;
    /// assert_eq!(server.received_count().await, 0);
    /// ```
    pub async fn clear_received(&self) {
        let mut received = self.received.write().await;
        received.clear();
    }

    /// Queue a response to be sent to the next connecting client.
    ///
    /// Responses are sent in the order they are queued.
    ///
    /// # Arguments
    ///
    /// * `response` - The response bytes to send
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let ack = b"MSH|^~\\&|RecvApp|RecvFac|SendApp|SendFac|20250128120000||ACK^A01|1|P|2.5\rMSA|AA|1|OK\r";
    /// server.queue_response(ack.to_vec()).await;
    /// ```
    pub async fn queue_response(&self, response: Vec<u8>) {
        let mut responses = self.responses.write().await;
        responses.push_back(response);
    }

    /// Queue an MLLP-framed response.
    ///
    /// Wraps the response in MLLP framing (SB ... EB) before queuing.
    ///
    /// # Arguments
    ///
    /// * `response` - The response bytes to wrap and send
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let ack = b"MSH|^~\\&|RecvApp|RecvFac|SendApp|SendFac|20250128120000||ACK^A01|1|P|2.5\rMSA|AA|1|OK\r";
    /// server.queue_mllp_response(ack).await;
    /// ```
    pub async fn queue_mllp_response(&self, response: &[u8]) {
        let mut framed = vec![0x0B]; // SB
        framed.extend_from_slice(response);
        framed.push(0x1C); // EB
        framed.push(0x0D); // CR
        self.queue_response(framed).await;
    }

    /// Stop the server.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// server.stop().await;
    /// ```
    pub async fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }
    }

    /// Wait for a specific number of messages to be received.
    ///
    /// # Arguments
    ///
    /// * `count` - The number of messages to wait for
    /// * `duration` - Maximum time to wait
    ///
    /// # Returns
    ///
    /// `Ok(())` if the expected number of messages was received, `Err(())` on timeout
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// server.wait_for_messages(2, Duration::from_secs(5)).await.unwrap();
    /// ```
    pub async fn wait_for_messages(&self, count: usize, duration: Duration) -> Result<(), ()> {
        timeout(duration, async {
            loop {
                if self.received_count().await >= count {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .map(|_| ())
        .map_err(|_| ())
    }
}

/// Handle a single client connection.
async fn handle_connection(
    stream: tokio::net::TcpStream,
    received: Arc<RwLock<VecDeque<Vec<u8>>>>,
    responses: Arc<RwLock<VecDeque<Vec<u8>>>>,
) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let (mut reader, mut writer) = stream.into_split();

    let received_clone = received.clone();
    let responses_clone = responses.clone();

    // Task to read incoming messages
    let read_task = async move {
        let mut buffer = vec![0u8; 65536];
        loop {
            match reader.read(&mut buffer).await {
                Ok(0) => break, // Connection closed
                Ok(n) => {
                    let data = buffer[..n].to_vec();
                    // Extract MLLP payload if framed
                    let payload = extract_mllp_payload(&data);
                    let mut received = received_clone.write().await;
                    received.push_back(payload.to_vec());
                }
                Err(_) => break,
            }
        }
    };

    // Task to write responses
    let write_task = async move {
        loop {
            let response = {
                let mut responses = responses_clone.write().await;
                responses.pop_front()
            };

            if let Some(resp) = response {
                if writer.write_all(&resp).await.is_err() {
                    break;
                }
            } else {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }
    };

    tokio::select! {
        _ = read_task => {}
        _ = write_task => {}
    }
}

/// Extract the payload from an MLLP-framed message.
fn extract_mllp_payload(data: &[u8]) -> &[u8] {
    // MLLP framing: SB (0x0B) payload EB (0x1C) CR (0x0D)
    if data.len() >= 3 && data[0] == 0x0B {
        // Find the end markers
        for i in 1..data.len() - 1 {
            if data[i] == 0x1C && data[i + 1] == 0x0D {
                return &data[1..i];
            }
        }
        // If no proper end markers, return content after SB
        return &data[1..];
    }
    data
}

/// A mock message handler for testing server code.
///
/// This handler can be configured to return specific responses or errors
/// for testing different scenarios.
///
/// # Example
///
/// ```rust,ignore
/// let handler = MockMessageHandler::new()
///     .with_response(|msg| {
///         // Return an ACK for every message
///         Some(create_ack(msg))
///     });
///
/// // Use with server
/// // server.run(handler).await;
/// ```
pub struct MockMessageHandler {
    #[allow(clippy::type_complexity)]
    responses: Arc<RwLock<VecDeque<Result<Option<Message>, Error>>>>,
    received: Arc<RwLock<Vec<Message>>>,
    #[allow(clippy::type_complexity)]
    response_fn: Option<Arc<dyn Fn(&Message) -> Result<Option<Message>, Error> + Send + Sync>>,
}

impl Default for MockMessageHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl MockMessageHandler {
    /// Create a new mock message handler.
    ///
    /// By default, the handler returns `Ok(None)` for all messages.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let handler = MockMessageHandler::new();
    /// ```
    pub fn new() -> Self {
        Self {
            responses: Arc::new(RwLock::new(VecDeque::new())),
            received: Arc::new(RwLock::new(Vec::new())),
            response_fn: None,
        }
    }

    /// Queue a response to be returned for the next message.
    ///
    /// # Arguments
    ///
    /// * `response` - The response to return
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let handler = MockMessageHandler::new()
    ///     .queue_response(Ok(Some(ack_message)));
    /// ```
    pub fn queue_response(self, response: Result<Option<Message>, Error>) -> Self {
        // Use blocking write since this is a builder pattern (sync)
        // In async context, use queue_response_async instead
        let responses = self.responses.clone();
        if let Ok(mut guard) = responses.try_write() {
            guard.push_back(response);
        }
        self
    }

    /// Queue a response asynchronously.
    pub async fn queue_response_async(&self, response: Result<Option<Message>, Error>) {
        let mut responses = self.responses.write().await;
        responses.push_back(response);
    }

    /// Queue a successful ACK response.
    ///
    /// # Arguments
    ///
    /// * `message` - The ACK message to return
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let handler = MockMessageHandler::new()
    ///     .queue_ack(ack_message);
    /// ```
    pub fn queue_ack(self, message: Message) -> Self {
        self.queue_response(Ok(Some(message)))
    }

    /// Queue an error response.
    ///
    /// # Arguments
    ///
    /// * `error` - The error to return
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let handler = MockMessageHandler::new()
    ///     .queue_error(Error::InvalidCharset);
    /// ```
    pub fn queue_error(self, error: Error) -> Self {
        self.queue_response(Err(error))
    }

    /// Set a custom response function.
    ///
    /// The function will be called for each message and can return
    /// a custom response based on the message content.
    ///
    /// # Arguments
    ///
    /// * `f` - The response function
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let handler = MockMessageHandler::new()
    ///     .with_response(|msg| {
    ///         // Echo back the message type
    ///         Ok(Some(msg.clone()))
    ///     });
    /// ```
    pub fn with_response<F>(mut self, f: F) -> Self
    where
        F: Fn(&Message) -> Result<Option<Message>, Error> + Send + Sync + 'static,
    {
        self.response_fn = Some(Arc::new(f));
        self
    }

    /// Get all received messages.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let messages = handler.get_received().await;
    /// assert_eq!(messages.len(), 1);
    /// ```
    pub async fn get_received(&self) -> Vec<Message> {
        self.received.read().await.clone()
    }

    /// Clear all received messages.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// handler.clear_received().await;
    /// ```
    pub async fn clear_received(&self) {
        self.received.write().await.clear();
    }

    /// Handle a message and return the configured response.
    ///
    /// This method implements the typical handler pattern used by servers.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to handle
    ///
    /// # Returns
    ///
    /// The configured response, or `Ok(None)` if no response is configured
    pub async fn handle(&self, message: Message) -> Result<Option<Message>, Error> {
        // Record the received message
        self.received.write().await.push(message.clone());

        // Check for a queued response first
        let queued = {
            let mut responses = self.responses.write().await;
            responses.pop_front()
        };

        if let Some(response) = queued {
            return response;
        }

        // Use the response function if set
        if let Some(f) = &self.response_fn {
            return f(&message);
        }

        // Default: no response
        Ok(None)
    }
}

/// Test data generator for creating test messages.
pub struct TestDataGenerator;

impl TestDataGenerator {
    /// Generate a random MRN (Medical Record Number).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let mrn = TestDataGenerator::random_mrn();
    /// assert!(mrn.len() >= 6);
    /// ```
    pub fn random_mrn() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        format!("MRN{:08x}", duration.as_nanos() % 0xFFFFFFFF)
    }

    /// Generate a random message control ID.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let id = TestDataGenerator::random_control_id();
    /// assert!(id.starts_with("MSG"));
    /// ```
    pub fn random_control_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        format!("MSG{:012x}", duration.as_nanos())
    }

    /// Generate a timestamp in HL7 format (YYYYMMDDHHmmss).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let ts = TestDataGenerator::current_timestamp();
    /// assert_eq!(ts.len(), 14);
    /// ```
    pub fn current_timestamp() -> String {
        // Use a simple timestamp format
        // In a real implementation, this would use chrono or similar
        "20250128120000".to_string()
    }

    /// Generate a random patient name.
    ///
    /// # Returns
    ///
    /// A tuple of (last_name, first_name)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let (last, first) = TestDataGenerator::random_name();
    /// assert!(!last.is_empty());
    /// assert!(!first.is_empty());
    /// ```
    pub fn random_name() -> (String, String) {
        let last_names = [
            "Smith", "Johnson", "Williams", "Brown", "Jones", "Garcia", "Miller",
        ];
        let first_names = [
            "John", "Jane", "Michael", "Emily", "David", "Sarah", "Robert",
        ];

        use std::time::{SystemTime, UNIX_EPOCH};
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let n = duration.as_nanos() as usize;

        let last = last_names[n % last_names.len()];
        let first = first_names[(n / 7) % first_names.len()];

        (last.to_string(), first.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_mllp_server_start() {
        let mut server = MockMllpServer::new();
        let result = server.start("127.0.0.1:0").await;
        assert!(result.is_ok());

        let addr = server.local_addr();
        assert!(addr.is_ok());

        server.stop().await;
    }

    #[tokio::test]
    async fn test_mock_mllp_server_received_messages() {
        let mut server = MockMllpServer::new();
        server.start("127.0.0.1:0").await.unwrap();

        let messages = server.received_messages().await;
        assert!(messages.is_empty());

        server.clear_received().await;
        assert_eq!(server.received_count().await, 0);

        server.stop().await;
    }

    #[tokio::test]
    async fn test_mock_mllp_server_queue_response() {
        let server = MockMllpServer::new();
        let response = b"ACK".to_vec();
        server.queue_response(response).await;

        // The response will be sent to the next connecting client
    }

    #[tokio::test]
    async fn test_mock_message_handler_new() {
        let handler = MockMessageHandler::new();
        let messages = handler.get_received().await;
        assert!(messages.is_empty());
    }

    #[tokio::test]
    async fn test_mock_message_handler_queue_response() {
        let handler = MockMessageHandler::new();
        let message = Message::new();

        let result = handler.handle(message.clone()).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());

        let received = handler.get_received().await;
        assert_eq!(received.len(), 1);
    }

    #[tokio::test]
    async fn test_mock_message_handler_with_response() {
        let handler = MockMessageHandler::new().with_response(|_| Ok(Some(Message::new())));

        let message = Message::new();
        let result = handler.handle(message).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[test]
    fn test_test_data_generator_random_mrn() {
        let mrn = TestDataGenerator::random_mrn();
        assert!(mrn.starts_with("MRN"));
        assert!(mrn.len() >= 6);
    }

    #[test]
    fn test_test_data_generator_random_control_id() {
        let id = TestDataGenerator::random_control_id();
        assert!(id.starts_with("MSG"));
    }

    #[test]
    fn test_test_data_generator_current_timestamp() {
        let ts = TestDataGenerator::current_timestamp();
        assert_eq!(ts.len(), 14);
    }

    #[test]
    fn test_test_data_generator_random_name() {
        let (last, first) = TestDataGenerator::random_name();
        assert!(!last.is_empty());
        assert!(!first.is_empty());
    }

    #[test]
    fn test_extract_mllp_payload() {
        // MLLP-framed message
        let framed = [0x0B, b'H', b'e', b'l', b'l', b'o', 0x1C, 0x0D];
        let payload = extract_mllp_payload(&framed);
        assert_eq!(payload, b"Hello");

        // Non-framed message
        let unframed = b"Hello";
        let payload = extract_mllp_payload(unframed);
        assert_eq!(payload, b"Hello");
    }
}
