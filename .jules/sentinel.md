## 2024-05-23 - Authentication Middleware Gap
**Vulnerability:** The `auth_middleware` was defined but not applied to the API routes, leaving `/hl7/parse` and `/hl7/validate` exposed without authentication.
**Learning:** Middleware definitions in Rust/Axum are not automatically applied; they must be explicitly attached to routes or the router. The previous implementation had the code but not the wiring.
**Prevention:** Always verify middleware application by checking `route_layer` or `.layer()` calls in `routes.rs`. Integration tests must verify authentication is enforced (expect 401 without credentials).

## 2024-05-23 - Timing Attack in API Key Validation
**Vulnerability:** The API key validation used standard string equality (`==`), which is vulnerable to timing attacks.
**Learning:** Rust's standard `PartialEq` for strings is not constant-time.
**Prevention:** Use a constant-time comparison function for secrets. Implemented `constant_time_eq` in `middleware.rs` to avoid external dependencies like `subtle` for this simple case.
