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
```

## Architecture

This is a Cargo workspace with 28 crates in `crates/`, organized in three layers:

**Microcrates** (minimal dependencies, single-responsibility):
- `hl7v2-model` — Core data types: Message, Segment, Field, Rep, Comp, Atom, Delims
- `hl7v2-escape` — HL7v2 escape sequences (\F\, \S\, \R\, \E\, \T\)
- `hl7v2-mllp` — MLLP framing (VT...FS CR)
- `hl7v2-parser` — Message parsing from bytes, delimiter discovery from MSH
- `hl7v2-writer` — Message serialization to HL7 wire format and JSON
- `hl7v2-json` — JSON serialization/deserialization for HL7 messages
- `hl7v2-normalize` — Message normalization and delimiter transformation
- `hl7v2-datetime` — Date/time parsing and validation
- `hl7v2-datatype` — Data type validation (CX, PN, TS, etc.)
- `hl7v2-path` — Field path parsing/resolution (e.g., `PID.5[1].1`)
- `hl7v2-query` — Fast path-based data extraction
- `hl7v2-batch` — Batch message handling (FHS/BHS/BTS/FTS)
- `hl7v2-network` — Async TCP/TLS MLLP client and server
- `hl7v2-stream` — Event-based streaming parser for large messages
- `hl7v2-validation` — Rule-based message validation engine
- `hl7v2-ack` — Automatic ACK generation logic
- `hl7v2-faker` — Realistic synthetic data generation
- `hl7v2-template` — Template-based message generation
- `hl7v2-template-values` — Values and generators for templates
- `hl7v2-corpus` — Pre-defined HL7 sample messages

**Mid-level crates**:
- `hl7v2-core` — Facade re-exporting all microcrates.
- `hl7v2-prof` — Profile-based validation with inheritance (YAML profiles in `profiles/`)
- `hl7v2-gen` — Synthetic message generation facade
- `hl7v2-bench` — High-performance benchmarks for all layers

**Application crates**:
- `hl7v2-cli` — CLI binary (`hl7v2`) with streaming support
- `hl7v2-server` — Axum HTTP API server with metrics, health checks, rate limiting

**Testing crates**:
- `hl7v2-test-utils` — Shared testing utilities and fixtures
- `hl7v2-e2e-tests` — Integration tests for full message pipelines

Dependency flow: microcrates → core → prof/gen → cli/server. All shared dependency versions are declared in the root `[workspace.dependencies]`.

## Conventions

- **Rust edition 2024**, MSRV 1.92
- **Error handling**: Each crate has its own error type using `thiserror`. Errors preserve context with `#[source]` chains.
- **Tests**: Unit tests in `src/tests.rs` modules (`#[cfg(test)]`), integration tests in `tests/` directories.
- **Commit messages**: `<type>(<scope>): <subject>` — types: feat, fix, docs, style, refactor, test, chore; scopes: core, prof, gen, cli, network, etc.
