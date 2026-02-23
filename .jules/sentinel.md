# Sentinel Journal - Security Learnings

## 2025-05-18 - Missing Authentication Middleware Application
**Vulnerability:** The `auth_middleware` was defined but never applied to the `/hl7` routes, leaving critical endpoints completely exposed.
**Learning:** Middleware in Axum (and many frameworks) must be explicitly layered. Defining the function is not enough. The disconnect happened because `auth_middleware` was not part of the `build_router` chain.
**Prevention:** Always verify middleware application by tracing the router construction. Use integration tests that specifically target the protected endpoints without credentials to ensure they are rejected (fail-secure testing).
