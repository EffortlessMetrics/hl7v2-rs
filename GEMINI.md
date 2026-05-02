# Gemini Project Instructions

## Pull Request Workflow

- **Incremental Improvements:** Each Pull Request (PR) MUST be improved and validated on its own dedicated branch.
- **Incremental Naming:** For re-implementations or major improvements to existing PRs, use the `-v2` suffix (e.g., `feature-python-bindings-v2`).
- **Continuous Integration (CI):** All PRs MUST pass all CI checks (unit tests, integration tests, linting, security scans) before being merged into the `main` branch.
- **No Consolidated Merges:** Do NOT consolidate multiple PRs into a single large merge request unless specifically instructed. Bypassing individual PR validation is strictly prohibited.
- **Conflict Resolution:** Resolve merge conflicts locally on the PR branch by merging or rebasing with `main`.
- **Validation:** Always run `cargo check`, `cargo clippy`, and relevant tests locally before pushing changes to ensure a high probability of green CI. Use `$env:PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1` if working on Python-related crates locally.

## Development Standards

- **Microcrate Architecture:** Follow the microcrate architecture (SRP-focused crates in `crates/`). New features should typically live in their own crate.
- **Security First:** Adhere to the security practices outlined in `SECURITY.md`, including constant-time comparisons (`subtle::ct_eq`) for sensitive operations like API key validation.
- **Performance:** Prioritize stack-allocated buffers and efficient string scanning (`str::contains` fast-paths) for core parsing logic.
- **Ecosystem Sync:** Ensure OpenAPI specifications (`schemas/openapi/`) are always in sync with the actual implementation in `crates/hl7v2-server`.
- **Licensing:** Use AGPL-3.0-or-later for all new source files. No MIT/Apache dependencies without explicit approval.
