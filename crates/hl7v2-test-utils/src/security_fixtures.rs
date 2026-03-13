//! Deterministic security-oriented test fixtures.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Returns a deterministic API key for tests.
///
/// This key is derived from the supplied `seed`, making it stable across test
/// runs without storing raw secrets in source control.
pub fn deterministic_api_key(seed: &str) -> String {
    let mut hasher = DefaultHasher::new();
    seed.hash(&mut hasher);
    format!("test-key-{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::deterministic_api_key;

    #[test]
    fn test_deterministic_api_key_is_stable() {
        let first = deterministic_api_key("seed-1");
        let second = deterministic_api_key("seed-1");
        let third = deterministic_api_key("seed-2");

        assert_eq!(first, second);
        assert_ne!(first, third);
    }
}
