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

#[cfg(test)]
mod tests {
    use super::{compute_message_hash, compute_sha256};
    use crate::model::{Delims, Field, Message, Segment};
    use crate::writer::write;

    #[test]
    fn empty_string_sha256_matches_known_constant() {
        // Known SHA-256("") from `echo -n "" | sha256sum`.
        assert_eq!(
            compute_sha256(""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn ascii_abc_sha256_matches_known_constant() {
        // Known SHA-256("abc") from `echo -n "abc" | sha256sum`.
        assert_eq!(
            compute_sha256("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn multibyte_unicode_sha256_is_deterministic_and_hex() {
        let h1 = compute_sha256("héllo");
        let h2 = compute_sha256("héllo");
        assert_eq!(
            h1, h2,
            "hashing the same input twice should be deterministic"
        );
        assert_eq!(h1.len(), 64, "sha256 hex string should be 64 chars long");
        assert!(
            h1.chars().all(|c| c.is_ascii_hexdigit()),
            "hash {h1} should be all hex"
        );
        // Distinct from the ASCII "hello" hash.
        assert_ne!(h1, compute_sha256("hello"));
    }

    #[test]
    fn different_inputs_produce_different_hashes() {
        assert_ne!(compute_sha256("a"), compute_sha256("b"));
        assert_ne!(compute_sha256(""), compute_sha256(" "));
    }

    #[test]
    fn long_input_still_returns_64_char_hex() {
        let payload = "x".repeat(10_000);
        let digest = compute_sha256(&payload);
        assert_eq!(digest.len(), 64);
        assert!(digest.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn compute_message_hash_matches_canonical_wire_bytes() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"MSH",
                fields: vec![Field::from_text("|^~\\&"), Field::from_text("APP")],
            }],
            charsets: vec![],
        };
        let expected_wire = "MSH|^~\\&|APP\r";

        assert_eq!(write(&message), expected_wire.as_bytes());
        assert_eq!(
            compute_message_hash(&message),
            compute_sha256(expected_wire)
        );
    }

    #[test]
    fn compute_message_hash_differs_when_message_differs() {
        let base = Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"PID",
                fields: vec![Field::from_text("1")],
            }],
            charsets: vec![],
        };
        let mut other = base.clone();
        other.segments.push(Segment {
            id: *b"OBX",
            fields: vec![Field::from_text("2")],
        });

        assert_eq!(write(&base), b"PID|1\r");
        assert_eq!(write(&other), b"PID|1\rOBX|2\r");
        assert_ne!(compute_message_hash(&base), compute_message_hash(&other));
    }
}
