# Sentinel's Journal

## 2025-01-29 - Missing Authentication Middleware Application
**Vulnerability:** The `auth_middleware` was defined in `middleware.rs` but not applied to any routes in `routes.rs`. The API endpoints `/hl7/parse` and `/hl7/validate` were completely exposed and accessible without an API key.
**Learning:** Defining middleware is not enough; it must be explicitly layered onto the router using `.layer()`. Integration tests that rely on `create_test_router` were not verifying the authentication failure case.
**Prevention:** Always verify middleware application with a dedicated negative test case that specifically targets the protected endpoint *without* credentials and asserts a 401/403 response. Configure the application to "Fail Secure" (panic) if the API key is not provided at startup.
