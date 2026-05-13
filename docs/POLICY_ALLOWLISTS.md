# Policy Allowlists

This page explains how the repository policy ledgers fit together. It is a
map, not a replacement for the TOML files. The TOML ledgers remain the source
of truth for exceptions and policy state.

## Current Ledgers

| Ledger | Current job |
| --- | --- |
| `policy/clippy-lints.toml` | Active Rust and Clippy lint policy, staged packages, and planned Rust 1.94/1.95 lint flips. |
| `policy/clippy-debt.toml` | Temporary cleanup debt that should expire by 2026-06-30. |
| `policy/clippy-exceptions.toml` | Retained Clippy suppressions, separate from broad cleanup debt. |
| `policy/no-panic-allowlist.toml` | Panic-family exception ledger; currently empty with exact counted identity semantics. |
| `policy/no-panic-baseline.toml` | Generated exact counted no-new-debt baseline for current panic-family findings. |
| `policy/non-rust-allowlist.toml` | Tracked non-Rust file presence ledger. |
| `policy/generated-allowlist.toml` | Generated artifacts and generators. |
| `policy/executable-allowlist.toml` | Scripts and executable entrypoints. |
| `policy/dependency-surface-allowlist.toml` | Non-Rust package manager and tool dependencies. |
| `policy/workflow-allowlist.toml` | Workflow behavior beyond file presence. |
| `policy/process-allowlist.toml` | Process execution surfaces. |
| `policy/network-allowlist.toml` | Network access surfaces. |
| `policy/ripr-suppressions.toml` | Advisory static mutation-exposure suppressions. |
| `policy/ci-lane-whitelist.toml` | Allowed CI lanes, ownership, trigger class, and LEM estimate. |
| `policy/ci-risk-packs.toml` | Risk-pack routing for CI lanes. |
| `policy/ci-budget.toml` | CI budget policy. |

Generated local diagnostics, such as
`target/policy/no-panic-report.md` and
`target/policy/no-panic-report.json`, are operator evidence for the current
checkout. They are not policy ledgers and should not be committed.

## Planned Ledgers

The Rust 1.95 / 1.5.0 rollout should add more companion ledgers only where
they govern behavior that does not belong in `policy/non-rust-allowlist.toml`.

| Planned ledger | Purpose |
| --- | --- |
| None currently. | Add only when a behavior cannot be governed by an existing ledger. |

## Rules

- Do not duplicate source-of-truth tables from the TOML ledgers.
- Broad globs need a concrete reason.
- Production OpenAPI, protobuf, schema, profile, and publishing surfaces need
  a real `covered_by` command or workflow.
- Python, Node, Go, Docker, shell, process, and network behavior belongs in
  companion policies instead of broad file-presence entries.
- `policy/non-rust-allowlist.toml` answers "may this file exist?" Companion
  ledgers answer "what behavior may this file perform?"
- Exceptions must be owned, reviewable, and expiring unless they are permanent
  platform metadata with a stable proof.

## Local Gates

```bash
cargo run -p xtask -- check-file-policy
cargo run -p xtask -- check-lint-policy
cargo run -p xtask -- check-no-panic-family
cargo run -p xtask -- policy-report
```
