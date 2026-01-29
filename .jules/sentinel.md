## 2024-05-23 - Unused Middleware and Timing Attacks
**Vulnerability:** The authentication middleware was defined but not attached to the API routes, leaving sensitive endpoints exposed. Additionally, it used standard string comparison for API keys, vulnerable to timing attacks.
**Learning:** Defining middleware is not enough; it must be explicitly applied to routes. Dead code in security components is a risk as it gives a false sense of security.
**Prevention:** Use integration tests that specifically target the *absence* of credentials to verify that protection is active. Use constant-time comparison for all secrets.
