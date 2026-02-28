//! Common test utilities for hl7v2-network integration tests.
//!
//! This module provides shared test fixtures, helpers, and utilities
//! used across integration tests.

use hl7v2_model::{Atom, Comp, Delims, Field, Message, Rep, Segment};
use hl7v2_network::{MessageHandler, MllpClient, MllpClientBuilder, MllpServer, MllpServerConfig};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;

/// Create a simple test message for testing.
pub fn create_test_message() -> Message {
    Message {
        delims: Delims::default(),
        segments: vec![Segment {
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
        }],
        charsets: vec![],
    }
}

/// Create a more realistic ADT^A01 test message.
pub fn create_adt_a01_message() -> Message {
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
                                subs: vec![Atom::Text("SENDING_APP".to_string())],
                            }],
                        }],
                    },
                    Field {
                        reps: vec![Rep {
                            comps: vec![Comp {
                                subs: vec![Atom::Text("SENDING_FAC".to_string())],
                            }],
                        }],
                    },
                    Field {
                        reps: vec![Rep {
                            comps: vec![Comp {
                                subs: vec![Atom::Text("RECV_APP".to_string())],
                            }],
                        }],
                    },
                    Field {
                        reps: vec![Rep {
                            comps: vec![Comp {
                                subs: vec![Atom::Text("RECV_FAC".to_string())],
                            }],
                        }],
                    },
                    Field {
                        reps: vec![Rep {
                            comps: vec![Comp {
                                subs: vec![Atom::Text("202401011200".to_string())],
                            }],
                        }],
                    },
                    Field {
                        reps: vec![Rep {
                            comps: vec![Comp {
                                subs: vec![Atom::Text("ADT^A01".to_string())],
                            }],
                        }],
                    },
                    Field {
                        reps: vec![Rep {
                            comps: vec![Comp {
                                subs: vec![Atom::Text("MSG00001".to_string())],
                            }],
                        }],
                    },
                    Field {
                        reps: vec![Rep {
                            comps: vec![Comp {
                                subs: vec![Atom::Text("P".to_string())],
                            }],
                        }],
                    },
                    Field {
                        reps: vec![Rep {
                            comps: vec![Comp {
                                subs: vec![Atom::Text("2.5.1".to_string())],
                            }],
                        }],
                    },
                ],
            },
            Segment {
                id: *b"PID",
                fields: vec![
                    Field {
                        reps: vec![Rep {
                            comps: vec![Comp {
                                subs: vec![Atom::Text("1".to_string())],
                            }],
                        }],
                    },
                    Field {
                        reps: vec![Rep {
                            comps: vec![Comp {
                                subs: vec![Atom::Text("12345".to_string())],
                            }],
                        }],
                    },
                    Field {
                        reps: vec![Rep {
                            comps: vec![Comp {
                                subs: vec![Atom::Text("DOE^JOHN^M".to_string())],
                            }],
                        }],
                    },
                ],
            },
        ],
        charsets: vec![],
    }
}

/// A simple echo handler that returns the received message as ACK.
pub struct EchoHandler {
    pub notify: Arc<Notify>,
}

impl EchoHandler {
    pub fn new(notify: Arc<Notify>) -> Self {
        Self { notify }
    }
}

impl MessageHandler for EchoHandler {
    fn handle_message(&self, message: Message) -> Result<Option<Message>, hl7v2_model::Error> {
        self.notify.notify_one();
        Ok(Some(message))
    }
}

/// A handler that returns a custom ACK message.
pub struct AckHandler {
    pub notify: Arc<Notify>,
}

impl AckHandler {
    pub fn new(notify: Arc<Notify>) -> Self {
        Self { notify }
    }
}

impl MessageHandler for AckHandler {
    fn handle_message(&self, _message: Message) -> Result<Option<Message>, hl7v2_model::Error> {
        self.notify.notify_one();
        // Create a simple ACK
        Ok(Some(create_test_message()))
    }
}

/// A handler that delays before responding.
#[allow(dead_code)]
pub struct DelayedHandler {
    pub delay: Duration,
    pub notify: Arc<Notify>,
}

impl DelayedHandler {
    pub fn new(delay: Duration, notify: Arc<Notify>) -> Self {
        Self { delay, notify }
    }
}

impl MessageHandler for DelayedHandler {
    fn handle_message(&self, message: Message) -> Result<Option<Message>, hl7v2_model::Error> {
        // Note: This is synchronous, actual delay would need async handling
        self.notify.notify_one();
        Ok(Some(message))
    }
}

/// A handler that tracks message count.
pub struct CountingHandler {
    pub count: Arc<std::sync::atomic::AtomicU32>,
}

impl CountingHandler {
    pub fn new(count: Arc<std::sync::atomic::AtomicU32>) -> Self {
        Self { count }
    }
}

impl MessageHandler for CountingHandler {
    fn handle_message(&self, _message: Message) -> Result<Option<Message>, hl7v2_model::Error> {
        self.count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(Some(create_test_message()))
    }
}

/// A handler that returns an error.
pub struct ErrorHandler;

impl MessageHandler for ErrorHandler {
    fn handle_message(&self, _message: Message) -> Result<Option<Message>, hl7v2_model::Error> {
        Err(hl7v2_model::Error::InvalidFieldFormat {
            details: "Test error".to_string(),
        })
    }
}

/// A handler that returns None (no ACK).
pub struct SilentHandler;

impl MessageHandler for SilentHandler {
    fn handle_message(&self, _message: Message) -> Result<Option<Message>, hl7v2_model::Error> {
        Ok(None)
    }
}

/// Helper to start a test server on a random port.
pub async fn start_test_server<H: MessageHandler + 'static>(
    _handler: H,
) -> (MllpServer, SocketAddr) {
    let mut server = MllpServer::new(MllpServerConfig::default());
    let bind_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    server.bind(bind_addr).await.expect("Failed to bind server");
    let addr = server.local_addr().expect("Failed to get server address");
    (server, addr)
}

/// Helper to create a test client with default settings.
pub fn create_test_client() -> MllpClient {
    MllpClientBuilder::new()
        .connect_timeout(Duration::from_secs(5))
        .read_timeout(Duration::from_secs(5))
        .write_timeout(Duration::from_secs(5))
        .build()
}

/// Helper to create a test client with short timeouts for timeout testing.
pub fn create_quick_timeout_client() -> MllpClient {
    MllpClientBuilder::new()
        .connect_timeout(Duration::from_millis(100))
        .read_timeout(Duration::from_millis(100))
        .write_timeout(Duration::from_millis(100))
        .build()
}

/// Wait for server to be ready.
pub async fn wait_for_server_ready() {
    tokio::time::sleep(Duration::from_millis(50)).await;
}

/// Generate a unique port for parallel test execution.
pub fn get_unique_port() -> u16 {
    use std::sync::atomic::{AtomicU16, Ordering};
    static PORT_COUNTER: AtomicU16 = AtomicU16::new(26000);
    PORT_COUNTER.fetch_add(1, Ordering::SeqCst)
}
