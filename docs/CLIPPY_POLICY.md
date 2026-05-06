# Clippy Policy

`hl7v2-rs` uses the Effortless Metrics strict Rust lint policy as a governed
engineering surface. The policy is intentionally workspace-wide: production
code, examples, benches, integration tests, and unit tests inherit the same
panic-free and parser-safe defaults.

## Goals

- Keep HL7 parser/protocol code panic-free by default.
- Prevent silent failure patterns such as discarded futures, ignored `Result`s,
  and swallowed error context.
- Make UTF-8, string slicing, indexing, and numeric conversion risks visible in
  review before they can affect healthcare message parsing.
- Require every suppression to carry a narrow, reviewable reason.
- Track Rust 1.94 and 1.95 lint flips before the MSRV changes.

## Workspace baseline

The root `Cargo.toml` owns the active lint surface in
`[workspace.lints.rust]` and `[workspace.lints.clippy]`. Packages join the
blocking baseline by inheriting it with:

```toml
[lints]
workspace = true
```

The baseline forbids unsafe code, denies panic-family lints, denies unchecked
string/indexing hazards, denies silent-failure lints, and warns on selected
reviewability and numeric-correctness lints that are useful but may need staged
cleanup. This first policy PR opts in `xtask`; subsequent stacked PRs can add
`[lints] workspace = true` to product crates as their lint debt is retired.

## No test carveouts

This repository is workspace panic-free, not only production panic-free. Do not
add Clippy test carveouts such as:

```toml
allow-unwrap-in-tests = true
allow-expect-in-tests = true
allow-panic-in-tests = true
allow-indexing-slicing-in-tests = true
allow-dbg-in-tests = true
```

Tests should return `Result` when setup can fail, use explicit assertions, and
prefer helper functions that propagate structured errors instead of `unwrap`,
`expect`, or panic-driven fixture setup.

## Suppression style

Use `#[expect(..., reason = "...")]` for a narrow, temporary exception when the
lint is right in general but a local call site needs staged cleanup. Do not use
silent `#[allow]` attributes for lint debt.

Good:

```rust
#[expect(
    clippy::indexing_slicing,
    reason = "generated HL7 table access; tracked in policy/clippy-debt.toml"
)]
fn lookup_generated_table(index: usize) -> &'static str {
    TABLE[index]
}
```

Bad:

```rust
#[allow(clippy::indexing_slicing)]
fn lookup_generated_table(index: usize) -> &'static str {
    TABLE[index]
}
```

## Policy ledgers

- `policy/clippy-lints.toml` is the machine-readable source of truth for active
  lints and planned Rust 1.94/1.95 flips.
- `policy/clippy-debt.toml` records temporary exceptions with `lint`, `path`,
  `owner`, `reason`, and `expires`.
- `policy/no-panic-allowlist.toml` reserves the semantic path + family +
  selector schema for panic-family exceptions.
- `policy/non-rust-allowlist.toml` reserves the structured schema for non-Rust
  programming-file exceptions.
- `clippy.toml` is only for repo-local disallowed methods, types, macros, or
  fields. It must not weaken the test posture.

## Gate

Run the policy gate with:

```bash
cargo run -p xtask -- check-lint-policy
```

The gate checks that the workspace MSRV matches the policy ledger, packages
inherit workspace lints, active lints match the root manifest, planned 1.94/1.95
lints are still planned until the MSRV bump, Clippy test carveouts are absent,
and debt entries are complete and unexpired.
