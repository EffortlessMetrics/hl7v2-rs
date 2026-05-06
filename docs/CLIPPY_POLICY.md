# Clippy policy

`hl7v2-rs` uses the Effortless Metrics strict Rust lint policy as a governed
engineering surface. The root workspace manifest owns the active lint block,
`policy/clippy-lints.toml` records the machine-readable policy ledger, and
`cargo xtask check-lint-policy` verifies that the manifest, policy files, and
crate manifests stay coherent.

## Goals

* Keep production code and tests panic-free: no `unwrap`, `expect`, `panic!`,
  `todo!`, `unimplemented!`, `unreachable!`, unchecked indexing, or string
  slicing by default.
* Prevent silent failures: futures, must-use values, locks, line iteration, and
  error-mapping paths must not be discarded without reviewable intent.
* Protect parser/protocol boundaries: HL7 data is byte-oriented at transport
  seams and text-oriented at parsing seams, so UTF-8, indexing, slicing, and
  numeric conversion lints are part of the baseline.
* Govern suppressions: use narrow `#[expect(..., reason = "...")]` attributes
  only when a local exception is truly better than immediate cleanup.

## Files

* `Cargo.toml` contains `[workspace.lints.rust]` and
  `[workspace.lints.clippy]`, the active policy Cargo applies to all members.
* Every package manifest contains `[lints] workspace = true` so the active root
  policy is inherited uniformly.
* `clippy.toml` is reserved for repo-local policy knobs such as disallowed
  methods, types, or macros. It must not contain test carveouts.
* `policy/clippy-lints.toml` records active lints and planned Rust 1.94/1.95
  flips with reasons.
* `policy/clippy-debt.toml` records temporary, expiring debt when a follow-up PR
  needs to migrate a narrow surface.
* `policy/no-panic-allowlist.toml` reserves the semantic TOML shape for
  path + family + selector panic-family exceptions.
* `policy/non-rust-allowlist.toml` reserves the TOML shape for non-Rust file
  policy exceptions with ownership and CI coverage receipts.

## No test carveouts

The standard is workspace panic-free, not only production panic-free. Do not add
these Clippy carveouts:

```toml
allow-unwrap-in-tests = true
allow-expect-in-tests = true
allow-panic-in-tests = true
allow-indexing-slicing-in-tests = true
allow-dbg-in-tests = true
```

Prefer fallible tests that return `Result` and use `?` for setup and assertions
that can fail.

## Suppression style

Use `#[expect(..., reason = "...")]`, not `#[allow(...)]`, for a local lint
exception. The reason should explain why the exception is safe or why a
follow-up migration is needed.

```rust
#[expect(
    clippy::indexing_slicing,
    reason = "generated parser table is bounds-checked by construction"
)]
fn generated_lookup(table: &[u8], index: usize) -> u8 {
    table[index]
}
```

If an exception cannot be explained narrowly at the call site, record it as
expiring debt in `policy/clippy-debt.toml` instead of weakening the shared
policy.

## Upgrade ledger

The policy ledger tracks planned lints for Rust 1.94 and Rust 1.95 before this
workspace raises its MSRV. `cargo xtask check-lint-policy` fails if planned lints
are activated in `Cargo.toml` before the corresponding MSRV flip is recorded and
reviewed.
