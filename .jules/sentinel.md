## 2024-06-03 - Timing Attack Vulnerability in API Key Comparison
**Vulnerability:** The API key validation in `auth_middleware` (`crates/hl7v2-server/src/middleware.rs`) uses a direct string comparison (`key == expected_key`). This is vulnerable to timing attacks where an attacker can determine the correct key by measuring the time it takes for the comparison to fail.
**Learning:** Security dependencies or custom implementations of constant-time comparisons must be used for secrets like API keys, tokens, and passwords.
**Prevention:** Use a constant-time comparison library or algorithm.
