## 2024-05-22 - Missing Authentication on API Endpoints
**Vulnerability:** The `/hl7/parse` and `/hl7/validate` endpoints were exposed without authentication, despite `auth_middleware` existing in the codebase.
**Learning:** Middleware definitions are not enough; they must be explicitly applied to routes. The `build_router` function constructed `api_routes` but forgot to layer the authentication middleware.
**Prevention:** Verify middleware application in router construction. Use integration tests that explicitly check for 401 Unauthorized when credentials are missing (negative testing).
