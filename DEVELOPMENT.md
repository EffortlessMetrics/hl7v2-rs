# Development Guide

Get up and running with hl7v2-rs development.

---

## 🚀 Quick Start

### 1. Setup Environment

We use **Nix** for a reproducible development environment. If you have Nix installed with flakes enabled:

```bash
nix develop
```

This shell includes all necessary tools: `rust`, `cargo-nextest`, `cargo-deny`, `just`, etc.

### 2. Repository Setup

Activate the automated git hooks and prepare the workspace:

```bash
just setup
```

### 3. Verify Everything

Run the "gate" command to verify formatting, lints, and tests:

```bash
just gate
```

---

## 🛠️ Unified Development Workflow

We use `just` as our primary entry point. It wraps `cargo xtask` for complex automation.

### Basic Loop

| Command | Action |
|---------|--------|
| `just lint-fix` | **Mutating**. Auto-formats code and applies safe clippy fixes. |
| `just gate` | Fast local "CI preview". Checks fmt, clippy, and compiles tests. |
| `just gate-check` | **Strict**. Exactly what runs in CI. No mutations allowed. |
| `just gate-changed` | **Fast**. Only runs checks for crates impacted by your changes. |
| `just test` | Runs all tests using `cargo-nextest` (if available) or `cargo test`. |

### Scaffolding New Crates

To create a new microcrate with all standard boilerplate (README, CLAUDE.md, Cargo.toml inheritance):

```bash
just scaffold my-feature "Brief description of the feature"
```

### Documentation

```bash
just docs        # Build and open workspace documentation
just docs-build  # Build docs without opening (for CI)
```

### Quality & Security

```bash
just audit       # Run security vulnerability scan and license check
just outdated    # Check for outdated dependencies
just bench       # Run all benchmarks
```

---

## 🏗️ Project Structure

The project is a Cargo workspace with 28 specialized crates in `crates/`, organized into layers:

1.  **Microcrates** (SRP-focused): Foundational logic like `hl7v2-model`, `hl7v2-parser`, `hl7v2-writer`.
2.  **Service Crates**: Connectivity and validation like `hl7v2-network`, `hl7v2-validation`, `hl7v2-prof`.
3.  **Applications**: User-facing tools like `hl7v2-cli` and `hl7v2-server`.

---

## 🤖 Agent Workflow (Enforced)

If you are an AI agent (Claude, Gemini, etc.), you **must** follow this loop:

1.  Perform your edits.
2.  Run `just lint-fix` to tighten the bolts.
3.  Run `just gate-check` to ensure no regressions.
4.  Only then open a PR.

*Do not rely on CI to discover lint or formatting errors.*

---

## 🧪 Testing Tips

### Focused Testing

```bash
# Test a specific crate
cargo test -p hl7v2-core

# Run a specific test with output
cargo test test_name -- --nocapture
```

### Integration & E2E

Integration tests are in `tests/` directories within each crate. End-to-end tests involving the CLI and network are in `crates/hl7v2-e2e-tests`.

---

## 📜 ADRs (Architecture Decision Records)

All major technical decisions are documented in `docs/adr/`. Please review them before proposing significant architectural changes.

---

**Happy coding!** 🦀
