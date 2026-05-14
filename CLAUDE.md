# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Workflow (Enforced)

This repository enforces formatting, linting, and compile checks automatically.

### What happens on commit

If your commit includes Rust or Cargo changes, the pre-commit hook runs:
- `cargo run -p xtask -- lint-fix` (auto-format + best-effort clippy fixes, then verifies)

The hook restages any auto-fixes to the current commit.

### What happens on push

The pre-push hook runs a strict gate:
- `cargo run -p xtask -- gate --check` (CI-parity checks)

### CI parity

CI Stage 1 uses xtask for fmt/clippy, then runs unit + doc tests separately:
- `cargo run -p xtask -- gate --check --only fmt`
- `cargo run -p xtask -- gate --check --only clippy`
- `cargo test --lib --workspace --all-features`
- `cargo test --doc --workspace --all-features`

### One-time setup (per clone)

Enable repository hooks:
- `just setup` (sets `core.hooksPath` to `.githooks`)

## Build & Development Commands

```bash
# Build
cargo build                                         # dev build
cargo build --workspace --all-features              # full workspace build

# Test
cargo test --workspace --all-features               # all tests
cargo run -p xtask -- gate                          # fast local gate (warm graph + compile check)
cargo run -p xtask -- gate --check                  # strict local gate (CI parity)

# Lint & Format
cargo run -p xtask -- lint-fix                      # auto-fix lints and format
cargo fmt --all                                     # manual format
cargo clippy --workspace --all-features --all-targets  # manual lint

# Policy stack
cargo run -p xtask -- check-lint-policy             # lint policy + debt receipts
cargo run -p xtask -- check-no-panic-family         # semantic panic-family ledger
cargo run -p xtask -- check-file-policy             # non-Rust file allowlist
cargo run -p xtask -- no-panic propose              # propose new allowlist entries
cargo run -p xtask -- policy-report                 # rollout + debt + findings summary
```

The policy stack is documented in:
- `docs/CLIPPY_POLICY.md`
- `docs/NO_PANIC_POLICY.md`
- `docs/FILE_POLICY.md`

## Architecture

This repository now uses `hl7v2` as the canonical Rust library and keeps SRP
implementation boundaries as modules inside that crate.

**Public Rust product crates**:
- `hl7v2` — library crate for model, parser, writer, query, transport,
  conformance, ACK, normalization, synthetic data, lifecycle, and experimental
  modules.
- `hl7v2-server` — Axum/Tonic HTTP and gRPC runtime service with metrics,
  health checks, auth, rate limiting, and deployment config.
- `hl7v2-cli` — CLI binary (`hl7v2`) with parsing, validation, ACK,
  normalization, streaming, and generation workflows.

**Separate packaging lane**:
- `hl7v2-python` — publishable PyO3 binding backend crate for the public
  Python `hl7v2` package. It is not the recommended Rust API and should be
  validated with binding-backend and Python/maturin tooling before any registry
  release claim.

**Internal crates and packages**:
- `hl7v2-bench` — benchmark harness.
- `hl7v2-test-utils` — shared testing utilities and fixtures.
- `hl7v2-e2e-tests` — end-to-end tests for full message pipelines.
- `xtask` — workspace automation.
- root `hl7v2-examples` package — examples only; `publish = false`.

Former microcrate package names such as `hl7v2-core`, `hl7v2-model`,
`hl7v2-parser`, and `hl7v2-prof` have been retired from the local workspace.
Their implementation now lives under `hl7v2::...` module paths. Historical
old-name crates.io artifacts may exist, but new code should depend on `hl7v2`.

Dependency flow: `hl7v2` -> `hl7v2-server` / `hl7v2-cli` / wrappers and tests.
All shared dependency versions are declared in the root `[workspace.dependencies]`.

## Conventions

- **Rust edition 2024**, MSRV 1.95
- **Error handling**: Public modules use typed errors with `thiserror` where
  needed. Errors preserve context with `#[source]` chains.
- **Tests**: Unit tests in `src/tests.rs` modules (`#[cfg(test)]`), integration tests in `tests/` directories.
- **Commit messages**: `<type>(<scope>): <subject>` — types: feat, fix, docs, style, refactor, test, chore; scopes should name the current crate or module, such as `hl7v2`, `parser`, `profile`, `cli`, or `server`.
