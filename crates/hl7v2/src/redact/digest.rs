use sha2::{Digest, Sha256};

pub(crate) fn compute_sha256(value: &str) -> String {
    compute_sha256_bytes(value.as_bytes())
}

pub(crate) fn compute_sha256_bytes(value: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value);
    format!("{:x}", hasher.finalize())
}
