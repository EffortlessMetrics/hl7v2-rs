# Third Party Notices

This project depends on third-party open source software. Below is a summary of the key license families included in our dependency graph.

## Principal Licenses

- **MIT License**: Used by many core Rust ecosystem crates (e.g., `serde`, `tokio`).
- **Apache License 2.0**: Used by many core Rust ecosystem crates (e.g., `tracing`, `hyper`).
- **BSD 2-Clause / 3-Clause**: Used by crates like `zerocopy`.
- **ISC License**: Used by crates like `ring`.
- **Unicode-3.0**: Used by `icu_*` and `yoke` crates.
- **Zlib License**: Used by `adler` and `miniz_oxide`.
- **Unlicense**: Used by `aho-corasick`.
- **CDLA-Permissive-2.0**: Used by `webpki-roots`.

## Full Dependency Audit

You can generate a full machine-readable report of all transitive dependencies and their exact license texts using `cargo-deny`:

```bash
# Verify all dependencies comply with project policy
cargo deny check licenses
```

Or using `cargo-license`:

```bash
# List all dependencies and their licenses
cargo license
```

---

*This file is provided for informational purposes and to satisfy attribution requirements of various open source licenses used by dependencies of this project.*
