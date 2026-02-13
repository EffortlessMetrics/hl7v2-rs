## 2025-05-23 - Unused Security Middleware
**Vulnerability:** The application had an `auth_middleware` function implemented but it was not applied to any routes, leaving sensitive endpoints completely unauthenticated.
**Learning:** Presence of security code does not imply security enforcement. Developers might implement security features but forget to wire them up.
**Prevention:** Always verify that security middleware is actively applied to the router. Use integration tests that explicitly check for 401/403 responses to confirm protection is active.
