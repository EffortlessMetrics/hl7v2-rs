## 2024-05-22 - Unapplied Authentication Middleware

**Vulnerability:** The `auth_middleware` was implemented in `middleware.rs` but was NOT applied to the API routes in `routes.rs`, leaving `/hl7/parse` and `/hl7/validate` completely unauthenticated. Additionally, the unused middleware contained a timing attack vulnerability in the API key comparison.

**Learning:** Existence of security code (middleware) does not imply it is active. The disconnection between `routes.rs` (router definition) and `middleware.rs` (security logic) led to a false sense of security.

**Prevention:** Always verify that security middleware is explicitly applied to the relevant routes. Use integration tests that specifically target unauthorized access (negative testing) to confirm enforcement, as implemented in `tests/auth_test.rs`.
