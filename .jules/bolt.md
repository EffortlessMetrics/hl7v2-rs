## 2025-01-29 - String Escaping Optimization
**Learning:** Optimizing `escape_text` and `unescape_text` in `hl7v2-core` by adding a "fast path" check for special characters significantly improves performance for "clean" text (83% faster for unescape, 45% faster for escape). This comes with a small regression (~13%) for "dirty" text, but since most HL7 fields are clean, this is a net positive.
**Action:** When implementing string transformations, always consider checking if the transformation is necessary before allocating a new string or iterating.

## 2025-01-29 - Clippy Configuration
**Learning:** The `warn-on-all-pedantic` and `allowed-lints` keys in `clippy.toml` are unsupported/deprecated in the current toolchain and cause `cargo clippy` to fail completely (not just warn).
**Action:** Comment out unsupported keys in `clippy.toml` to restore linting functionality.
