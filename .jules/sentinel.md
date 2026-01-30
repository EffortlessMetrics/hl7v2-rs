## 2024-05-22 - Missing Middleware Application
**Vulnerability:** Authentication middleware was defined but not applied to the router, leaving sensitive endpoints exposed.
**Learning:** Defining middleware is not enough; explicit application to the router is required. Code review must verify *usage*, not just *existence*.
**Prevention:** Use integration tests that explicitly verify 401/403 responses for unauthenticated requests, not just 200 for happy paths.
