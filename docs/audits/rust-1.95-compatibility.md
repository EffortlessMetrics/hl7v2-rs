# Rust 1.95 Compatibility Probe

Date: 2026-05-13

This audit records the Rust 1.95 compatibility probe for the
[`Rust 1.95 and 1.5.0 rollout`](../development/RUST_1_95_ROLLOUT.md).
It is a receipt-only probe: it does not raise the declared MSRV, add
`rust-toolchain.toml`, activate new lints, change workflows, bump versions, or
modify Rust source.

## Scope

| Field | Value |
| --- | --- |
| Branch | `probe/rust-1.95-compat` |
| Current workspace version | `1.4.0` |
| Current declared MSRV | Rust `1.93` |
| Probe toolchain | Rust `1.95.0` |
| Target release lane | `1.5.0` |

## Local Environment Notes

Two local-machine conditions affected the first probe attempt:

- The local Python interpreter is newer than PyO3 0.24.2's published support
  window. Workspace checks that include `hl7v2-python` need
  `PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1`, matching the hosted CI posture.
- The first `test --no-run` attempt wrote to the default worktree target
  directory while `H:` had only 147456 bytes free. After stale worktrees were
  removed, the heavy probe was rerun with
  `CARGO_TARGET_DIR=F:\cargo-target\hl7v2-rs-rust-195`.

Neither condition is Rust 1.95 code fallout.

## Commands

| Command | Result | Notes |
| --- | --- | --- |
| `rustup toolchain install 1.95.0 --component rustfmt --component clippy` | pass | Toolchain already installed and current. |
| `cargo +1.95.0 fmt --all -- --check` | pass | Formatting is compatible with Rust 1.95 rustfmt. |
| `cargo +1.95.0 check --workspace --all-targets --all-features --locked` | environment-blocked | Blocked by the local Python 3.14/PyO3 support window without the CI ABI3 compatibility env. |
| `PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo +1.95.0 check --workspace --all-targets --all-features --locked` | pass | No Rust 1.95 check fallout. |
| `PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo +1.95.0 clippy --workspace --all-targets --all-features --locked -- -D warnings` | pass | No Rust 1.95 Clippy fallout before enabling new lint ratchets. |
| `PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo +1.95.0 test --workspace --all-features --locked --no-run` | environment-blocked | First attempt failed with `no space on device` / linker PDB filesystem errors on `H:`. |
| `PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 CARGO_TARGET_DIR=F:\cargo-target\hl7v2-rs-rust-195 cargo +1.95.0 test --workspace --all-features --locked --no-run` | pass | Test binaries compile under Rust 1.95. |
| `PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 CARGO_TARGET_DIR=F:\cargo-target\hl7v2-rs-rust-195 cargo +1.95.0 run -p xtask -- gate --check` | pass | Aggregate gate passed under Rust 1.95. |
| `PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 CARGO_TARGET_DIR=F:\cargo-target\hl7v2-rs-rust-195 cargo +1.95.0 run -p xtask -- check-lint-policy` | pass | 4 workspace packages inherit the baseline; 4 packages are staged. |
| `PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 CARGO_TARGET_DIR=F:\cargo-target\hl7v2-rs-rust-195 cargo +1.95.0 run -p xtask -- check-no-panic-family` | pass | 99 required-inheriting source files scanned; 0 allowlist entries; 304 advisory findings in staged crates. |
| `PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 CARGO_TARGET_DIR=F:\cargo-target\hl7v2-rs-rust-195 cargo +1.95.0 run -p xtask -- check-file-policy` | pass | 482 tracked/untracked non-ignored files checked; 36 allowlist entries. |
| `PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 CARGO_TARGET_DIR=F:\cargo-target\hl7v2-rs-rust-195 cargo +1.95.0 run -p xtask -- evidence-schema-check` | pass | 33 evidence fixtures validated. |

## Conclusion

The current `main` codebase compiles, lints, builds test binaries, runs the
aggregate gate, and validates evidence schemas under Rust 1.95. No code changes
are required before the dedicated MSRV/toolchain bump PR.

Follow-up work remains scoped to later rollout PRs:

- declare Rust 1.95 as the workspace MSRV;
- add `rust-toolchain.toml`;
- update CI MSRV pins and labels;
- activate or explicitly defer Rust 1.94/1.95 lint ratchets;
- prepare the `1.5.0` release only after the policy and release-readiness PRs.
