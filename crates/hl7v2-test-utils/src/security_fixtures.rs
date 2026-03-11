//! Deterministic security-oriented test fixtures.

use uselesskey::{Factory, TokenSpec};

/// Returns a deterministic API key for tests.
///
/// This key is derived from the supplied `seed`, making it stable across test
/// runs without storing raw secrets in source control.
pub fn deterministic_api_key(seed: &str) -> String {
    let mut factory = Factory::deterministic(seed);
    let token = factory.token("hl7v2-server-auth", TokenSpec::ApiKey);
    token.secret().to_string()
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
