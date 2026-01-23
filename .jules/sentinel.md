## 2024-05-22 - Authentication State Management
**Vulnerability:** The server relied on `std::env::var` in the `auth_middleware` for checking the API key, and tests used `unsafe { std::env::set_var }` to configure it.
**Learning:** `unsafe { std::env::set_var }` is dangerous in multi-threaded Rust tests as it modifies the process-global environment, leading to race conditions or undefined behavior.
**Prevention:** Always use dependency injection or application state (like `axum::extract::State`) to pass configuration (secrets) to middleware and handlers. Refactored `AppState` to hold the API key, avoiding global mutable state in tests.
