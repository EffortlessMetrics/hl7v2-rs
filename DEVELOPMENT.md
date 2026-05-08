# Development Guide

Get up and running with `hl7v2-rs` development.

## Quick Start

### 1. Setup Environment

We use Nix for a reproducible development environment. If you have Nix
installed with flakes enabled:

```bash
nix develop
```

This shell includes the expected Rust, Cargo, `cargo-nextest`, `cargo-deny`,
and `just` tooling.

### 2. Repository Setup

Activate the automated git hooks and prepare the workspace:

```bash
just setup
```

### 3. Verify Everything

Run the gate command to verify formatting, lints, and tests:

```bash
just gate
```

## Unified Development Workflow

We use `just` as the primary entry point. It wraps `cargo xtask` for complex
automation.

### Basic Loop

| Command | Action |
| --- | --- |
| `just lint-fix` | Mutating. Auto-formats code and applies safe clippy fixes. |
| `just gate` | Fast local CI preview. Checks fmt, clippy, and compiles tests. |
| `just gate-check` | Strict CI-parity gate. No mutations allowed. |
| `just gate-changed` | Faster checks for crates impacted by your changes. |
| `just test` | Runs all tests using `cargo-nextest` when available, otherwise `cargo test`. |

### Adding New Implementation

Normal feature work should add modules under `crates/hl7v2/src`. Add a new
workspace crate only when the design needs a separate product, binary, service,
foreign-language binding, benchmark, test, or tool boundary.

### Documentation

```bash
just docs        # Build and open workspace documentation
just docs-build  # Build docs without opening, for CI
```

### Quality And Security

```bash
just audit       # Run security vulnerability scan and license check
just outdated    # Check for outdated dependencies
just bench       # Run all benchmarks
```

## Project Structure

The project is a Cargo workspace with one public Rust library crate, two public
Rust wrapper crates, a separate Python binding lane, and private test/tool
crates:

1. **Library**: `hl7v2`, which owns parser, writer, query, transport,
   conformance, synthetic, lifecycle, and operational modules.
2. **Rust products**: `hl7v2-server` and `hl7v2-cli`, which depend on `hl7v2`.
3. **Python lane**: `hl7v2-python`, kept `publish = false` for crates.io and
   released through Python packaging tooling.
4. **Internal support**: `hl7v2-e2e-tests`, `hl7v2-test-utils`, `hl7v2-bench`,
   `xtask`, and the root examples package.

## Agent Workflow

If you are an AI agent, follow this loop:

1. Perform your edits.
2. Run `just lint-fix` when the lane allows mutating fixes.
3. Run `just gate-check` before opening a PR.
4. Do not rely on CI to discover lint or formatting errors.

## Testing Tips

### Focused Testing

```bash
# Test the canonical library crate
cargo test -p hl7v2

# Run a specific test with output
cargo test test_name -- --nocapture
```

### Integration And E2E

Integration tests live in `tests/` directories within retained crates.
End-to-end tests involving the CLI and network are in
`crates/hl7v2-e2e-tests`.

## ADRs

All major technical decisions are documented in `docs/adr/`. Review them before
proposing significant architectural changes.
