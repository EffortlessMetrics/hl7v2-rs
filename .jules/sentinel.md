## 2025-02-19 - Unused Auth Middleware
**Vulnerability:** The `auth_middleware` was implemented in `middleware.rs` but not applied to the API routes in `routes.rs`, leaving `/hl7/*` endpoints exposed.
**Learning:** Axum middleware must be explicitly attached to routers or routes. Implementing the function is not enough.
**Prevention:** Review `routes.rs` to ensure all protected routes have the necessary layers applied. Use integration tests that explicitly check for 401 Unauthorized on protected endpoints.
