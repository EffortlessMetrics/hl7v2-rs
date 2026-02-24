## 2024-05-23 - API Authentication Refactor
**Vulnerability:** API endpoints `/hl7/parse` and `/hl7/validate` were exposed without authentication despite middleware definition.
**Learning:** `auth_middleware` was defined but never applied to the router. Also, standard string comparison leaked timing information.
**Prevention:**
1. Always apply middleware explicitly using `.layer()`.
2. Use constant-time comparison for secrets.
3. Move `AppState` to a separate module to avoid circular dependencies when middleware needs state access.
