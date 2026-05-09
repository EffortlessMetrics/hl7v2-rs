# hl7v2-rs CI Economics Rollout Plan

## Current State (as of 2026-05-09)

`hl7v2-rs` already satisfies the following baseline:

- Rust 1.93 / edition 2024
- Strict Clippy profile at workspace root (panic-free production + tests, AST/string/indexing
  safety, silent-failure lints, async/concurrency rails, numeric rails, reviewability lints,
  suppression governance)
- `policy/clippy-lints.toml` with active lints, planned 1.94/1.95 flips, no test carveouts,
  required/staged inheritance tracking
- `xtask` with `check-lint-policy`, `check-no-panic-family`, `no-panic propose`,
  `check-file-policy`, `policy-report`
- `RUSTFLAGS=-Dwarnings`, `RUSTDOCFLAGS=-Dwarnings`, broad `--workspace --all-features` tests
- CI with Fast, Standard, Matrix, Extended, Benchmarks, and `ci-success` jobs
- Pre-commit hook: `cargo run -p xtask -- lint-fix`
- Pre-push hook: `cargo run -p xtask -- gate --check`

## Assumptions

- `hl7v2-rs` already satisfies the MSRV 1.93 strict-lint baseline.
- The current rollout focuses on CI lane economics, lane inventory, risk routing, ripr
  advisory, and actuals — not adding stricter Clippy enforcement.
- The ordinary PR target is below 35 LEM where possible.
- Full matrix, Python wheel, coverage, full property tests, release/publish, OpenAPI/gRPC
  deep contract validation, and broader platform checks remain available on main, nightly,
  release, or labels.
- Existing strict Clippy policy and semantic policy checks are preserved.

## What This Rollout Is NOT

- Not reducing verification.
- Not weakening the Clippy profile.
- Not adding test carveouts.
- Not making deep lanes disappear.

## What This Rollout IS

Making CI economics explicit so that:

1. Every CI lane has a documented purpose, cost, and failure mode.
2. Expensive lanes run where they buy signal, not everywhere by default.
3. Ordinary Rust PRs stay below 35 LEM.
4. One required summary check (`PR Gate Success`) decouples branch protection from individual
   lane churn.
5. ripr provides oracle-gap signal at static-analysis prices.
6. LEM actuals feed back into learned estimates.

## PR Stack

| PR | Title                                              | Status  |
| -- | -------------------------------------------------- | ------- |
| 01 | docs(ci): add verification economics rollout plan  | target  |
| 02 | policy(ci): add CI lane whitelist                  | planned |
| 03 | ci(policy): check workflow lanes against whitelist | planned |
| 04 | ci(plan): add advisory PR Plan with LEM estimate   | planned |
| 05 | ci: add PR Gate Success summary workflow           | planned |
| 06 | perf(ci): normalize cache and cancellation policy  | planned |
| 07 | perf(ci): route platform matrix to main and labels | planned |
| 08 | ci: route standard tests by risk pack              | planned |
| 09 | ci(ripr): add advisory static exposure analysis    | planned |
| 10 | ci(python): route maturin wheel smoke by risk      | planned |
| 11 | ci(api): route API and gRPC contract checks        | planned |
| 12 | ci(release): route publish and package checks      | planned |
| 13 | ci(test): add nextest and structured test telemetry| planned |
| 14 | ci(telemetry): emit LEM actuals                    | planned |
| 15 | ci(budget): warn on elevated LEM                   | planned |
| 16 | ci: make PR Gate Success the required check        | planned |
| 17 | ci(metrics): use observed actuals for LEM estimates| planned |
| 18 | ci(ripr): require ack for high-confidence gaps     | planned |

## Natural Stacks

```
01 → 02 → 03 → 04 → 05
09 → 18
13 → 14 → 15 → 17
15 → 16
```

Independent: 06, 07, 08, 10, 11, 12

## Immediate Highest-Value Changes

1. Move platform matrix off ordinary PR path (PR 07)
2. Add PR Plan + LEM (PR 04)
3. Normalize cache saves (PR 06)
4. Add `PR Gate Success` (PR 05)
5. Route Python/API/release lanes by risk (PRs 10, 11, 12)
6. Add `ripr` advisory (PR 09)
