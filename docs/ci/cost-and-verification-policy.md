# CI Cost and Verification Policy

## Principle

`hl7v2-rs` is not reducing verification. It is routing verification.

HL7 parsing, validation, normalization, profile validation, MLLP, HTTP/gRPC service behavior,
Python bindings, OpenAPI/schema contracts, and evidence bundles all need strong proof. The goal
is to stop making every ordinary PR pay for every proof surface.

We optimize for **proof per Linux-equivalent minute (LEM)**.

## Industrialized AI Verification Economics

Industrialized AI changes the cost center. At high PR volume, per-PR verification cost can dominate
LLM cost, so deep verification and efficient verification are the same design goal. Steven's
[industrialized-AI framing](https://effortlesssteven.com/assisted-native-industrialized/) explains
why Suggestions -> Assisted -> Native -> Industrialized changes PR volume, review assumptions, and
verification load.

`hl7v2-rs` keeps verification layered: cheap static, schema, and contract rails run early, while
heavier runtime proofs remain scoped to the changes that need them. Evidence schemas, golden
fixtures, doc-link checks, file-policy rails, Python publish-policy checks, server smoke, gRPC
contracts, PHI sentinels, and publish-plan receipts are not bureaucracy. They are how the repo keeps
high-throughput agentic work safe without making every PR pay for every possible proof.

> **CI economics doctrine**: industrialized AI turns per-PR verification cost into a dominant
> operating cost. We keep verification deep, but scope expensive runtime work with cheaper static,
> schema, smoke, and contract checks where possible. The goal is not less verification - it is more
> useful verification per CI dollar.

## What a LEM Is

One LEM equals one GitHub Actions minute on `ubuntu-latest`. Runner multipliers convert
wall-clock minutes on other platforms to LEM equivalents:

| Runner              | Multiplier |
| ------------------- | ---------: |
| `ubuntu-latest`     |        1.0 |
| `windows-latest`    |        2.0 |
| `macos-latest`      |       10.0 |
| Python wheel build  |        2.0 |
| Docker build        |        6.0 |
| External AI review  |        4.0 |

A one-minute macOS job costs 10 LEM, not 1.

## Target LEM Budget per PR

| PR kind                           | Preferred | Limit |
| --------------------------------- | --------: | ----: |
| Docs only                         |       2–8 |    15 |
| Ordinary Rust (non-parser)        |      13–25 |    35 |
| Parser / protocol / service       |      25–45 |    75 |
| API / contract / schema change    |      30–55 |    75 |
| Python / platform / release       | label/main |   125 |

The `$1/PR` figure is the **hard ceiling**, not the design target. Ordinary Rust PRs should
usually stay below 35 LEM and preferably below $0.50 (≈ 62 LEM at $0.008/LEM).

## Lane Routing Doctrine

### Default PR path (every ordinary PR)

```text
PR Plan (LEM estimate, risk pack classification)
CI lane whitelist / policy checks
Rust Fast Gate
  fmt / clippy / lint policy / no-panic / file policy
  selected unit and doc tests
Standard risk-pack tests
  only surfaces touched by the diff
ripr advisory (non-blocking)
PR Gate Success (single required summary check)
```

### Available but not default

These lanes are available on labels, main, nightly, release branches, or `workflow_dispatch`.
They are never default for ordinary PRs unless the diff explicitly touches the surface.

| Lane                          | Trigger                              |
| ----------------------------- | ------------------------------------ |
| Windows / macOS matrix        | `platform-matrix`, `full-ci`, main   |
| Full property tests           | `property-tests`, `full-ci`, main    |
| Benchmarks                    | `benchmarks`, `full-ci`, main        |
| Python wheel smoke            | `python`, `full-ci`, Python paths    |
| Full API/gRPC contract suite  | `api-contract`, `full-ci`, API paths |
| Publish dry-run               | `release-check`, release branches, manual dispatch |
| Coverage                      | `coverage`, `full-ci`, main          |
| ripr soft-gate                | After calibration period             |

## Rust 1.95 / 1.5.0 Rollout

The Rust 1.95 rollout keeps this policy intact: sharper rails, not heavier
ordinary PRs. The rollout map lives in
[../development/RUST_1_95_ROLLOUT.md](../development/RUST_1_95_ROLLOUT.md).
It plans a Rust 1.95 MSRV ratchet, exact no-panic identity, companion policy
ledgers, advisory `ripr` static mutation-exposure analysis, targeted mutation
lanes, and release-readiness proof for `1.5.0`.

`ripr` is static mutation-exposure analysis. It shifts mutation signal left at
PR time; it does not replace runtime mutation testing. Runtime mutation remains
the slower backstop for high-risk, nightly, and release lanes.

## Hard Rules

The following are non-negotiable regardless of cost pressure:

- Do not weaken the strict Clippy profile.
- Do not add test carveouts for `unwrap` / `expect` / `panic` / indexing / `dbg!`.
- Do not add bare `#[allow(...)]`.
- Do not weaken `unsafe_code = "forbid"`.
- Do not remove deep validation lanes; route them instead.
- Do not make ripr blocking before calibration data exists.
- Do not hard-enforce learned budgets before actuals exist.
- Do not make Python/maturin, macOS, Windows, coverage, full property tests, OpenAPI/gRPC
  contract generation, or release checks broad ordinary-PR defaults unless the risk surface
  requires them.

## Labels Reference

See `docs/ci/labels.md` for the full label inventory. Labels override default routing without
requiring CI changes.
