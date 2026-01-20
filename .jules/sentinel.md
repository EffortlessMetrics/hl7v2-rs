## 2025-05-23 - [Ignored Security Configuration]
**Vulnerability:** The `max_body_size` configuration in `ServerConfig` was completely ignored, causing the server to rely on Axum's default (2MB) regardless of user intent.
**Learning:** Configuration structs in Rust are just data; they don't apply themselves. Middleware like `DefaultBodyLimit` must be explicitly added to the router with the config value.
**Prevention:** Always write integration tests that specifically verify configuration boundaries (e.g., "does it fail when > limit?" and "does it succeed when < limit but > default?").
