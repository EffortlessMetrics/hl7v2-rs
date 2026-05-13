# v1.5.0 Publish Dry-Run Receipt

This receipt records non-publishing release-readiness proof for the
`hl7v2-rs` Rust 1.95 / v1.5.0 quality-ratchet candidate.

It is not a crates.io publish receipt. It is not a TestPyPI or PyPI receipt.

## Candidate

| Field | Value |
| --- | --- |
| Version | `1.5.0` |
| Commit SHA | `38043c62b6684c3bd074661ae6e6089250593132` |
| Workflow | Rust 1.95 / 1.5.0 Release Readiness |
| Run URL | <https://github.com/EffortlessMetrics/hl7v2-rs/actions/runs/25810853128> |
| Job URL | <https://github.com/EffortlessMetrics/hl7v2-rs/actions/runs/25810853128/job/75826420754> |
| Result | Success |

## Rust Publish Graph

The publish plan remained:

1. `hl7v2`
2. `hl7v2-server`
3. `hl7v2-cli`

The internal `hl7v2-python` Rust crate remained outside the crates.io graph.

## Hosted Proof

The release-readiness workflow passed on `main` with Rust `1.95.0` and
`PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1`.

Passed steps:

- `cargo fmt --all -- --check`
- `cargo check --workspace --all-targets --all-features --locked`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- `cargo test --workspace --all-features --locked`
- `cargo test --doc --workspace --all-features --locked`
- `cargo run -p xtask -- check-lint-policy`
- `cargo run -p xtask -- check-no-panic-family`
- `cargo run -p xtask -- check-file-policy`
- `cargo run -p xtask -- check-ci-lane-whitelist`
- `cargo run -p xtask -- evidence-schema-check`
- contract workflow coverage check for OpenAPI, protobuf, JSON Schema, and
  evidence schema rails
- `cargo run -p xtask -- publish-plan`
- `cargo run -p xtask -- publish-dry-run --workspace-patches --allow-dirty`
- `cargo run -p xtask -- check-python-publish-policy`

The dry-run packaged and verified `hl7v2`, `hl7v2-server`, and `hl7v2-cli` as
`1.5.0`, then aborted uploads because the run was a dry-run.

## Python Boundary

The workflow confirmed:

- public Python distribution: `hl7v2`;
- internal Rust binding crate: `hl7v2-python`;
- Rust crates.io publish graph: `hl7v2`, `hl7v2-server`, `hl7v2-cli`;
- no TestPyPI or PyPI success claim was made.

Issue [#563](https://github.com/EffortlessMetrics/hl7v2-rs/issues/563)
remains the external TestPyPI Trusted Publisher blocker.

## Known Non-Blockers

- Hosted Actions emitted Node 20 deprecation annotations for existing actions.
- Production PyPI has not been released.
- crates.io publish, tag, and GitHub release have not been run.

## Rollback Path

Before crates.io publish, supersede this candidate with another release-prep
commit or revert the v1.5.0 candidate commits.

After crates.io publish, prefer a forward-fix patch unless a security, legal, or
metadata integrity issue requires yanking.
