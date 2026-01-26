# Sentinel Journal

## 2024-05-23 - Missing Middleware Application on Protected Routes
**Vulnerability:** The `auth_middleware` was defined in `middleware.rs` but never applied to the `/hl7/*` routes in `routes.rs`. Additionally, the `api_key` was not part of the `AppState`, meaning the middleware (even if applied) would have relied on performance-heavy `std::env` reads or failed.
**Learning:** Defining middleware functions is not enough; they must be explicitly added to the router layer stack. The separation between "defining middleware" and "building router" can lead to gaps if not carefully reviewed.
**Prevention:** Use integration tests that explicitly attempt unauthorized access (negative testing) as part of the default test suite. Always verify that middleware layers are attached to the router. Use type-safe state extraction (`State<T>`) to ensure the required state (like api keys) is actually available to the middleware.
