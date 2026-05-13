# Clippy Policy

`hl7v2-rs` uses the Effortless Metrics strict Rust lint policy as a governed
engineering surface. The policy is defined at the workspace root and rolls out
through explicit package inheritance so lint debt is visible instead of hidden.

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
cleanup.

Initial blocking inheritance is intentionally narrow:

```text
hl7v2
hl7v2-server
hl7v2-cli
xtask
```

The Python binding crate and private test/benchmark crates are staged in
`policy/clippy-lints.toml`; the Python binding has an explicit debt receipt in
`policy/clippy-debt.toml` while its PyO3 packaging lane stays isolated:

```text
hl7v2-python
internal test and benchmark crates
```

`hl7v2-server` and `hl7v2-cli` inherit the baseline now. Any pre-existing
server or CLI lint debt that was not appropriate to clean up in this policy PR
must be represented by a reasoned `#[expect(...)]` and an expiring
`policy/clippy-debt.toml` receipt.

Former implementation microcrates have been retired locally. Their code now
lives under `hl7v2`, so the canonical library crate's inherited lint baseline
is the enforcement point for that implementation surface.

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
- `policy/clippy-debt.toml` records temporary cleanup debt with `lint`,
  `path`, `owner`, `reason`, and `expires`.
- `policy/clippy-exceptions.toml` records retained suppressions that are
  intentional exceptions rather than broad cleanup debt. Entries must carry an
  `id`, `lint`, `path`, `selector`, `owner`, `reason`, `covered_by`, and
  `expires`.
- `policy/no-panic-allowlist.toml` reserves the semantic path + family +
  selector schema for panic-family exceptions.
- `policy/non-rust-allowlist.toml` reserves the structured schema for non-Rust
  programming-file exceptions.
- [POLICY_ALLOWLISTS.md](POLICY_ALLOWLISTS.md) maps current and planned policy
  ledgers without replacing the TOML sources of truth.
- `clippy.toml` is only for repo-local disallowed methods, types, macros, or
  fields. It must not weaken the test posture.

## Rust 1.95 rollout

The Rust 1.95 / 1.5.0 rollout is mapped in
[development/RUST_1_95_ROLLOUT.md](development/RUST_1_95_ROLLOUT.md). The
current state is Rust 2024, workspace version `1.4.0`, and MSRV `1.95`.

During the rollout, planned Rust 1.94/1.95 lints in
`policy/clippy-lints.toml` should either become active with clean proof or stay
planned with an expiring receipt. Do not use the MSRV bump as a reason to add
test carveouts or bare `#[allow(clippy::...)]` suppressions.

## Gate

Run the policy gate with:

```bash
cargo run -p xtask -- check-lint-policy
```

Print the current rollout and debt summary with:

```bash
cargo run -p xtask -- policy-report
```

The gate checks that the workspace MSRV matches the policy ledger, required
packages inherit workspace lints, staged package rollout is declared, active
lints match the root manifest, planned 1.94/1.95 lints are still planned until
the MSRV bump, Clippy test carveouts are absent, and debt entries are complete
and unexpired. It also validates the retained-exceptions ledger shape.
