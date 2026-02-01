## 2024-05-23 - Unsafe Environment Variables in Tests
**Vulnerability:** Race conditions in multi-threaded tests when modifying environment variables.
**Learning:** Rust's `std::env::set_var` is unsafe in multi-threaded environments. This codebase runs tests in parallel.
**Prevention:** Wrap environment variable modifications in `unsafe { ... }` blocks within tests, or use a thread-safe configuration injection pattern instead of global env vars.
