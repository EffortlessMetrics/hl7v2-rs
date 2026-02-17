## 2024-05-22 - Unapplied Middleware Gap
**Vulnerability:** The `auth_middleware` was defined but not applied to the API routes, leaving sensitive endpoints (`/hl7/parse`, `/hl7/validate`) publicly accessible.
**Learning:** In Axum, middleware is not automatically applied. It must be explicitly layered onto the router. The existence of middleware code does not imply its enforcement.
**Prevention:** Always verify middleware application with negative tests (e.g., tests that expect 401 Unauthorized when credentials are missing).
