## 2024-05-21 - [Missing Authentication Implementation]
**Vulnerability:** The `hl7v2-server` codebase contains artifacts (`auth_middleware` in `middleware.rs`) suggesting API Key authentication is implemented, but the middleware is not applied to the router in `routes.rs`. Furthermore, `ServerConfig` and `AppState` lack the fields to store the API key, rendering the `auth_middleware` (which relies on checking env vars directly) disconnected from the application state pattern intended by the design.
**Learning:** Code presence != Code execution. Dead code or unwired middleware can create a dangerous illusion of security. The discrepancy between "what is implemented" (middleware exists) and "what runs" (router config) is a classic gap.
**Prevention:**
1. Security controls must have positive verification tests (ensure they block unauthorized access).
2. Use "secure by default" frameworks where middleware is applied globally or via type-safe builders that enforce auth.
3. Regularly audit router configurations against security requirements.
