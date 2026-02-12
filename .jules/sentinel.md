## 2024-05-22 - Missing Middleware on Sensitive Endpoints
**Vulnerability:** `/hl7/parse` and `/hl7/validate` endpoints were publicly exposed without authentication checks because the authentication middleware (`auth_middleware`) was defined but not applied to the router.
**Learning:** The middleware was present in the codebase but unused. This highlights the risk of "dead code" or unused security components. Also, standard string comparison was used for API keys, which is vulnerable to timing attacks.
**Prevention:** Ensure all sensitive routes explicitly apply authentication middleware. Use tests to verify *unauthorized* access is rejected (negative testing). Use constant-time comparison for secrets.
