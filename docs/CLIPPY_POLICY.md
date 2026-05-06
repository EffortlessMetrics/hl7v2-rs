# Clippy and Policy Gates

This workspace follows the Effortless Metrics strict Rust policy: one MSRV,
one panic-free lint surface, explicit suppressions, and structured policy data
for temporary exceptions.

## Workspace baseline

The root `Cargo.toml` owns the active lint block under `[workspace.lints.rust]`
and `[workspace.lints.clippy]`. Workspace crates inherit the block with:

```toml
[lints]
workspace = true
```

The active profile forbids unsafe Rust, denies panic-family APIs, denies silent
failure patterns, denies parser-unsafe string/indexing shapes, and keeps
reviewability lints visible as warnings or denials.

## No test carveouts

Tests are part of the panic-free workspace. Do not add Clippy configuration such
as `allow-unwrap-in-tests`, `allow-expect-in-tests`, `allow-panic-in-tests`,
`allow-indexing-slicing-in-tests`, or `allow-dbg-in-tests`.

Prefer `Result`-returning tests and typed assertion helpers:

```rust
#[test]
fn parses_fixture() -> Result<(), Box<dyn std::error::Error>> {
    let parsed = parse_fixture()?;
    ensure_eq(parsed.segments().len(), 3, "fixture segment count")?;
    Ok(())
}
```

## Suppression style

Use narrow `#[expect(..., reason = "...")]` suppressions only when a reviewed
exception is needed. Do not use silent `#[allow(...)]` attributes. Temporary
repo debt belongs in `policy/clippy-debt.toml` with an owner, reason, path, lint,
and expiry.

## Policy files

- `policy/clippy-lints.toml` is the machine-readable ledger for active lints and
  planned Rust 1.94/1.95 lint flips.
- `policy/clippy-debt.toml` tracks temporary lint debt with expirations.
- `policy/no-panic-allowlist.toml` is reserved for semantic panic-family
  exceptions using `path + family + selector` identity and advisory `last_seen`
  locations.
- `policy/non-rust-allowlist.toml` explains non-Rust files with owner, reason,
  surface, classification, and CI coverage.
- `clippy.toml` is only for repo-specific Clippy configuration such as
  disallowed methods/types/macros. It must not weaken the test policy.

## Checks

Run these before policy changes are merged:

```sh
cargo xtask check-lint-policy
cargo xtask check-file-policy
cargo xtask policy-report
```

`cargo xtask check-no-panic-family` is available as the ratchet command for the
semantic panic-family allowlist. Existing panic-family debt is tracked in
`policy/clippy-debt.toml` while the workspace migrates tests and parser helpers
toward the strict baseline.
