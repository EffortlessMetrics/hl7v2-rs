# Rust 1.95 And 1.5.0 Rollout

This document is the rollout map for moving `hl7v2-rs` from Rust 1.93 to
Rust 1.95 and preparing the next release as `1.5.0`.

This is a planning and operating document. The initial map PR did not change
the active MSRV, toolchain, CI behavior, lint policy, package version, or
release state. The dedicated MSRV PR raises the declared Rust floor and
toolchain pins without activating new lints or changing runtime behavior.

## Executive Call

The workspace should move from Rust 1.93 to Rust 1.95. Because raising MSRV
narrows the supported build surface, the release that carries the ratchet is a
minor release:

```text
1.4.0 -> 1.5.0
```

The rollout sharpens existing rails rather than adding a broad default CI tax.
The current repository already has strict workspace lints, policy ledgers,
staged CI lanes, coverage routing, contract checks, evidence schemas, and
Python publishing boundaries. The upgrade should make those rails more
self-describing and better receipted.

## Current State

| Surface | Current state | Notes |
| --- | --- | --- |
| Rust edition | `2024` | No edition migration is planned. |
| Workspace version | `1.5.0` candidate | v1.4.0 remains the current published crates.io release until v1.5.0 receipts land. |
| Workspace MSRV | `1.95` | Declared in the root `Cargo.toml` after the compatibility probe. |
| Toolchain file | present | `rust-toolchain.toml` pins Rust `1.95.0` with `rustfmt` and `clippy`. |
| Root lint policy | strict | Rust and Clippy lints are already governed from the workspace root. |
| Clippy test carveouts | prohibited | `clippy.toml` explicitly rejects test carveouts. |
| Clippy policy ledger | present | `policy/clippy-lints.toml` records MSRV `1.95`, active lints, and planned 1.94/1.95 flips. |
| Clippy debt ledger | present | `policy/clippy-debt.toml` expires current cleanup debt on 2026-06-30. |
| Clippy exceptions ledger | present | `policy/clippy-exceptions.toml` governs retained Clippy suppressions separately from cleanup debt. |
| No-panic allowlist | present, empty | Exact identity is `path + family + selector_kind + selector_callee + snippet + count`. |
| No-panic baseline | present | Generated no-new-debt baseline follows exact identity. |
| Non-Rust allowlist | present | File presence is governed; companion behavior ledgers split generated, executable, dependency, workflow, process, and network behavior. |
| CI | staged and routed | Fast, standard, MSRV, matrix, extended, benchmark, coverage, contract, and security lanes already exist. |
| Coverage | routed/advisory | Coverage runs on main, dispatch, `coverage`, or `full-ci`. |
| Contracts workflow | present | OpenAPI, proto, schema, and evidence checks already exist. |
| Publish workflow | present | `publish.yml` handles crates.io publish execution, but not full 1.5.0 readiness proof. |
| Release readiness workflow | present | `release-readiness.yml` owns the manual Rust 1.95 / 1.5.0 readiness proof bundle. |
| Python publishing | externally blocked | TestPyPI publish remains blocked by issue #563 until Trusted Publisher is configured. |

## Target State

| Surface | Target state |
| --- | --- |
| Rust toolchain | Rust `1.95.0` |
| Workspace version | `1.5.0` |
| `rust-toolchain.toml` | pinned to `1.95.0` with `rustfmt` and `clippy` |
| `clippy.toml` | `msrv = "1.95"` and no test carveouts |
| `policy/clippy-lints.toml` | `msrv = "1.95"` with 1.94/1.95 lint state resolved |
| Rust compiler lint floor | add the low-risk Rust 1.95 lint floor where clean |
| Clippy ratchets | activate clean 1.94/1.95 lints or defer with expiring debt |
| Clippy exceptions | retained-suppression ledger separate from debt |
| No-panic identity | exact counted identity landed before baseline generation |
| No-panic baseline | generated no-new-debt baseline with reset confined to the baseline PR |
| File policy | companion ledgers for generated, executable, dependency, workflow, process, and network behavior |
| ripr | advisory PR-time static mutation-exposure analysis |
| Mutation testing | targeted runtime backstop, not an ordinary PR tax |
| Release readiness | explicit 1.5.0 readiness workflow and receipt |

## Doctrine

`ripr` is static mutation-exposure analysis.

It catches much of the same signal mutation testing catches: weak test or
oracle exposure. It catches that signal earlier and cheaper because it runs
statically and can run per PR.

Mutation testing remains the slower runtime backstop for what static analysis
cannot prove. `ripr` shifts mutation signal left; it does not make mutation
testing unnecessary.

This matters because industrialized AI changes verification economics. At high
PR volume, per-PR verification cost can exceed LLM cost, and small checks
compound quickly. The answer is not weaker verification. The answer is deep
verification routed by risk: cheap static, schema, lint, policy, and contract
checks at PR time; broader runtime mutation, release, and platform proofs
where the diff and release state justify them.

## PR Ladder

The rollout is split so each PR has one semantic objective.

| PR | Objective | Branch |
| --- | --- | --- |
| 1 | Documentation-only rollout map | `docs/rust-1.95-rollout` |
| 2 | Rust 1.95 compatibility probe | `probe/rust-1.95-compat` |
| 3 | MSRV and toolchain bump | `chore/msrv-rust-1.95` |
| 4 | Rust 1.95 compiler lint floor | `policy/rust-1.95-lints` |
| 5 | Rust 1.94/1.95 Clippy ratchets | `policy/clippy-rust-1.95-ratchets` |
| 6 | Clippy exceptions ledger | `policy/clippy-exceptions-ledger` |
| 7 | Exact no-panic identity | `policy/no-panic-exact-identity` |
| 8 | No-panic baseline and no-new-debt mode | `policy/no-panic-baseline` |
| 9 | No-panic diagnostics | `policy/no-panic-diagnostics` |
| 10 | File-policy companion ledgers | `policy/file-companion-ledgers` |
| 11 | Advisory `ripr` static exposure lane | `ci/ripr-static-exposure` |
| 12 | Targeted mutation lanes | `ci/targeted-mutation-lanes` |
| 13 | Rust 1.95 API cleanup | `refactor/rust-1.95-api-cleanups` |
| 14 | Numeric and protocol lint cleanup | `policy/clippy-protocol-cleanup` |
| 15 | LEM actuals and risk-pack routing | `ci/lem-and-risk-pack-routing` |
| 16 | 1.5.0 release-readiness workflow | `release/readiness-workflow` |
| 17 | Prepare `1.5.0` | `release/1.5.0-prep-rust-1.95` |
| 18 | Release dry-run proof | `release/1.5.0-dry-run` |

## Acceptance Gates

Documentation-only rollout changes use the narrow policy gates:

```bash
cargo run -p xtask -- check-doc-links
cargo run -p xtask -- check-lint-policy
cargo run -p xtask -- check-file-policy
cargo run -p xtask -- policy-report
git diff --check
```

Implementation PRs should add the smallest gate that directly proves the
surface they touch. The MSRV and release PRs must include explicit Rust 1.95,
publish-plan, publish-dry-run, evidence-schema, no-panic, file-policy, and
lint-policy proof where applicable.

## Operating Rules

- Start each PR from clean `origin/main`.
- Keep one objective per PR.
- Open PRs as draft first.
- Do not push directly to `main`.
- Do not combine MSRV bump, lint activation, no-panic baseline, file-policy
  tightening, release prep, and API cleanup.
- Do not add Clippy test carveouts.
- Do not add bare `#[allow(clippy::...)]` suppressions.
- Do not reset the no-panic baseline outside the dedicated baseline PR.
- Do not make `ripr` branch-protection blocking before calibration.
- Do not replace mutation testing with `ripr`.
- Do not put broad runtime mutation on ordinary PRs.
- Do not hide skipped lanes as passed.
- Do not weaken coverage, contract, evidence, Python publishing, or crates.io
  release proofs.

Every PR should include a self-review comment covering scope, files touched,
policy changes, no-panic baseline scope, file-policy scope, CI economics,
`ripr` framing, release evidence, validation, bot comments, and follow-ups.

## Current Follow-Ups

- Issue #563 remains the external TestPyPI Trusted Publisher blocker for the
  public `hl7v2` Python distribution.
- `publish.yml` exists for crates.io publication; `release-readiness.yml` is the
  non-publishing 1.5.0 readiness proof bundle.
