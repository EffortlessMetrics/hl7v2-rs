# Clippy and Policy Gates

This workspace uses the Effortless Metrics strict Rust policy: one MSRV, one
workspace lint surface, and structured receipts for every exception. The policy
is intentionally applied to production code, examples, benches, and tests.

## Active policy

The root `Cargo.toml` owns the active `[workspace.lints.rust]` and
`[workspace.lints.clippy]` blocks. Every workspace package inherits those lints
with:

```toml
[lints]
workspace = true
```

The baseline is grouped around:

- panic-free production and tests (`unwrap`, `expect`, `panic!`, `todo!`,
  `unimplemented!`, `unreachable!`, and `dbg!` are denied);
- AST/parser/UTF-8/slice safety for HL7 protocol boundaries;
- silent-failure prevention for ignored futures, locks, `Result`s, and errors;
- async, memory, numeric, filesystem, process, API, and reviewability footguns;
- suppression governance.

`policy/clippy-lints.toml` mirrors the active lint block and records planned
Rust 1.94 and 1.95 flips before the MSRV changes.

## Suppression style

Do not use broad `#[allow(...)]` attributes for lint cleanup. Use a narrow
`#[expect(..., reason = "...")]` only when the exception is intentional,
reviewed, and local to the smallest possible item or expression. Any broader or
long-lived exception belongs in `policy/clippy-debt.toml` with an owner, reason,
path, lint, and expiry.

## No test carveouts

`clippy.toml` must not contain test carveouts such as
`allow-unwrap-in-tests = true`, `allow-expect-in-tests = true`,
`allow-panic-in-tests = true`, `allow-indexing-slicing-in-tests = true`, or
`allow-dbg-in-tests = true`. Tests should return `Result` and use explicit
assertion helpers rather than panic-driven setup.

## Structured allowlists

Panic-family exceptions live in `policy/no-panic-allowlist.toml`. Each entry is
identified by `path + family + selector`; `last_seen` line/column data is only an
advisory locator.

Non-Rust programming and policy files are governed by
`policy/non-rust-allowlist.toml`. Entries must explain whether they are matched
by `path` or `glob`, who owns them, why Rust/`xtask` is not the right surface,
what surface they affect, how they are classified, and what check covers them.

## Commands

Run the policy checks with:

```bash
cargo xtask check-lint-policy
cargo xtask check-no-panic-family
cargo xtask check-file-policy
cargo xtask policy-report
```

The standard CI loop remains:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo xtask check-lint-policy
```
