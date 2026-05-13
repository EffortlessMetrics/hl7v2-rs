# Documentation Index

This directory contains current operating documentation, release receipts, and
historical project records. For live behavior and package-surface truth, start
with the current sources below before reading older planning documents.

## Current Sources

| Need | Start here |
| --- | --- |
| Current release and feature status | [STATUS.md](STATUS.md) |
| Support tiers and proof commands | [status/SUPPORT_TIERS.md](status/SUPPORT_TIERS.md) |
| Contributor workflow | [../CONTRIBUTING.md](../CONTRIBUTING.md), [../DEVELOPMENT.md](../DEVELOPMENT.md) |
| Task-focused evidence workflows | [guides/README.md](guides/README.md) |
| Machine-readable evidence artifacts | [contracts/evidence-contract-index.md](contracts/evidence-contract-index.md) |
| Evidence artifact semantics and provenance | [architecture/evidence-artifacts.md](architecture/evidence-artifacts.md), [architecture/evidence-provenance-versioning.md](architecture/evidence-provenance-versioning.md) |
| JSON schemas | [../schemas/README.md](../schemas/README.md) |
| Current Rust module and package surface | [architecture/module-map.md](architecture/module-map.md) |
| HTTP and gRPC API usage | [API_GUIDE.md](API_GUIDE.md) |
| CI and release validation lanes | [CI_PIPELINE.md](CI_PIPELINE.md) |
| Release process | [../RELEASE_PROCESS.md](../RELEASE_PROCESS.md) |
| Lint, file, and panic-family policies | [CLIPPY_POLICY.md](CLIPPY_POLICY.md), [FILE_POLICY.md](FILE_POLICY.md), [NO_PANIC_POLICY.md](NO_PANIC_POLICY.md) |
| Rust 1.95 / 1.5.0 rollout map | [development/RUST_1_95_ROLLOUT.md](development/RUST_1_95_ROLLOUT.md) |
| Policy allowlist map | [POLICY_ALLOWLISTS.md](POLICY_ALLOWLISTS.md) |

## How Docs Fit Together

Use the smallest document type that owns the claim:

| Document type | Job |
| --- | --- |
| Proposal / PRD | Explain why a campaign exists, who it serves, and what success means. |
| Spec | Define required behavior, contracts, acceptance criteria, and proof. |
| ADR | Record a durable architecture decision and its consequences. |
| Implementation plan | Sequence PR-sized work, rollback, and validation commands. |
| Active goal | Record current agent execution state, blockers, and next work. |
| Status / support | State current product claims and the proof behind them. |
| Policy TOML | Hold exception, CI, lint, file, and package ledgers. |
| Audit / handoff | Preserve what happened, what was validated, and what remains open. |

Start new governance work in [proposals/](proposals/), define durable
requirements in [specs/](specs/), use [adr/](adr/) only for architecture
decisions, sequence execution in [../plans/1.4.1/](../plans/1.4.1/), and keep
the current active state in [../.hl7v2/goals/](../.hl7v2/goals/). Current
feature status still lives in [STATUS.md](STATUS.md); do not duplicate it in
proposal, spec, plan, or receipt documents.

## Evidence Guides

| Guide | Workflow |
| --- | --- |
| [First 10 Minutes](guides/first-10-minutes.md) | Install, diagnose, validate, summarize, bundle, and replay. |
| [Vendor Upgrade Diff](guides/vendor-upgrade-diff.md) | Compare before/after corpora and interpret drift. |
| [Safe Support Bundle](guides/safe-support-bundle.md) | Redact and package replayable support evidence. |
| [Deploy Validation Sidecar](guides/deploy-validation-sidecar.md) | Run `hl7v2-server` as an edge guard. |
| [Python Evidence Workflow](guides/python-evidence-workflow.md) | Use the Python binding for validation reports, corpus diffs, redaction, bundles, and replay. |
| [Python TestPyPI Release Proof](guides/python-testpypi-release-proof.md) | Prove the separate Python packaging lane without changing the Rust crates.io graph. |

## Release And Proof Receipts

| Receipt | Use for |
| --- | --- |
| [v1.4.0 Evidence Contracts release notes](releases/v1.4.0-evidence-contracts.md) | Published release scope and user-facing changes. |
| [v1.4.0 objective audit](audits/v1.4.0-objective-completion-audit.md) | Release-snapshot prompt-to-artifact map for the evidence-layer objective and remaining boundaries. |
| [Final source-tree gap audit](audits/current-source-tree-evidence-objective-gap-audit.md) | Current package-state receipt after the broad local evidence-lane workbench was split and merged. |
| [v1.4.0 publish dry-run receipt](audits/publish-dry-run-v1.4.0-2026-05-09.md) | Package verification before upload. |
| [v1.4.0 publish receipt](audits/publish-v1.4.0-2026-05-09.md) | Dependency-ordered crates.io publication proof. |
| [v1.5.0 Rust 1.95 release candidate notes](releases/v1.5.0-rust-1.95-quality-ratchet.md) | Candidate scope for the Rust 1.95 quality-ratchet release; not a publish receipt. |
| [v1.5.0 release readiness](release/1.5.0-readiness.md) | Receipt home for Rust 1.95 / 1.5.0 readiness workflow results. |
| [Python TestPyPI non-publish proof](audits/python-testpypi-nonpublish-proof-2026-05-09.md) | Python packaging proof that keeps the internal `hl7v2-python` package outside the Rust crates.io graph. |
| [Python TestPyPI publish attempt](audits/python-testpypi-publish-attempt-2026-05-10.md) | Publishing-mode proof attempt; wheel smoke passed, upload is blocked by TestPyPI Trusted Publishing setup. |

## Current Boundaries

- `docs/STATUS.md` is the current-state source of truth.
- `Cargo.toml`, `rust-toolchain.toml`, `clippy.toml`, and
  `policy/clippy-lints.toml` own the active Rust 1.95 MSRV/toolchain state.
  `docs/development/RUST_1_95_ROLLOUT.md` remains the current rollout map for
  the remaining 1.5.0 quality-ratchet lane.
- The v1.4.0 objective audit is a release-snapshot receipt, not proof that the
  full long-range evidence-layer objective is finished.
- The public Python distribution is `hl7v2`, built from the internal
  `hl7v2-python` maturin lane, until a production PyPI release is intentionally
  proven and executed.
- gRPC coverage is useful but still narrower than the full HTTP evidence
  surface; use `docs/API_GUIDE.md` and `docs/STATUS.md` for current endpoint
  claims.

## Historical Documents

These documents are retained for traceability. They should not override the
current package surface, module map, status document, evidence schemas, or
guides. Some historical receipts preserve links or paths to retired crate
folders as evidence of the state they recorded; use the current sources above
for live navigation.

| Historical document | Use for |
| --- | --- |
| [TESTING_ARCHITECTURE.md](TESTING_ARCHITECTURE.md) | Historical testing architecture narrative. Examples are updated where practical, but the rollout story predates the crate collapse. |
| [TESTING_ANALYSIS.md](TESTING_ANALYSIS.md) and [TESTING_SUMMARY.md](TESTING_SUMMARY.md) | Dated testing snapshots from the former microcrate topology. |
| [MICROCRATE_ANALYSIS.md](MICROCRATE_ANALYSIS.md) | Historical analysis of the retired microcrate structure. |
| [ISSUES_AND_NEXT_STEPS.md](ISSUES_AND_NEXT_STEPS.md) | Pre-release planning snapshot, not the current work queue. |
| [TASK_COMPLETION_SUMMARY.md](TASK_COMPLETION_SUMMARY.md) | Earlier documentation alignment receipt. |
| [../ROADMAP.md](../ROADMAP.md) | Historical roadmap snapshot; current status lives in `docs/STATUS.md`. |
| [../TESTING.md](../TESTING.md) | Historical root testing guide; current gates live in `DEVELOPMENT.md` and `docs/CI_PIPELINE.md`. |
| [../SESSION_SUMMARY.md](../SESSION_SUMMARY.md) | Historical session receipt from 2025-11-19. |
| [plans/](plans/) and [audits/](audits/) | Historical plans and verification receipts. |

## Package Surface

The current Rust product surface is:

- `hl7v2`
- `hl7v2-server`
- `hl7v2-cli`

The public Python distribution is `hl7v2`, built from the internal
`hl7v2-python` maturin lane. `hl7v2-python` is not part of the Rust crates.io
publish graph. Historical old microcrate names may exist on crates.io, but they
are not the product surface for new code.
