//! Corpus hashing utilities.
//!
//! This module owns stable SHA-256 hashing for raw template content and
//! canonicalized HL7 message bytes.

use super::Message;
use crate::writer::write;
use sha2::{Digest, Sha256};

/// Compute SHA-256 hash of a string
pub fn compute_sha256(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let hash_result = hasher.finalize();
    format!("{hash_result:x}")
}

/// Compute SHA-256 hash of a message
pub fn compute_message_hash(message: &Message) -> String {
    let message_bytes = write(message);
    // Convert bytes to string for hashing (HL7 messages are ASCII-based)
    let message_string = String::from_utf8_lossy(&message_bytes);
    compute_sha256(&message_string)
}
