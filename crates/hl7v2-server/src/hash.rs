//! Shared hashing helpers for server-side evidence, audit, and API adapters.

use sha2::{Digest, Sha256};

pub(crate) fn compute_sha256(value: &str) -> String {
    compute_sha256_bytes(value.as_bytes())
}

pub(crate) fn compute_sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}
