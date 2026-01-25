## 2024-05-23 - Unapplied Authentication Middleware
**Vulnerability:** The `auth_middleware` was fully implemented in `middleware.rs` but was not applied to any routes in `routes.rs`.
**Learning:** Having middleware code exists does not mean it is active. In Axum, middleware must be explicitly attached to the router or routes. The disconnect between "having auth code" and "enforcing auth" led to a critical gap.
**Prevention:** Always verify middleware application with integration tests that explicitly attempt to bypass security controls (red-team tests), rather than assuming presence of code equals security.
