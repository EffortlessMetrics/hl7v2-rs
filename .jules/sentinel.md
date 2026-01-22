## 2024-05-22 - [Middleware Auth Bypass]
**Vulnerability:** The `/hl7/*` endpoints were completely unauthenticated because the `auth_middleware` was defined but never applied to the router.
**Learning:** Axum middleware must be explicitly attached to routers. The assumption that auth was "enforced" led to false confidence. Mismatch between documentation/memory and code reality is a common source of bugs.
**Prevention:** Always verify security controls with negative tests (ensure requests *fail* without credentials) before assuming they work. Use `Router::layer()` carefully and verify the order of execution.
