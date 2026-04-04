# Sentinel Journal

## 2026-02-15 - Missing Authentication and Timing Attack
**Vulnerability:** The `/hl7` API endpoints were completely unprotected because `auth_middleware` (although implemented) was not applied to the routes. Additionally, the middleware used standard string equality (`==`) for API key validation, making it vulnerable to timing attacks.
**Learning:** Middleware definitions are useless if not explicitly applied to routes. The presence of `middleware.rs` created a false sense of security. Also, standard `==` on strings short-circuits, leaking information about matching prefixes.
**Prevention:** Always verify middleware application in route configuration. Use `constant_time_eq` utilities for comparing secrets. Add integration tests that explicitly check for 401 Unauthorized to ensure auth is active.
