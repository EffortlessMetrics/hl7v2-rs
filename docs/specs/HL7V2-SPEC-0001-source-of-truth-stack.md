# HL7V2-SPEC-0001: Source-of-Truth Stack

Status: Accepted
Date: 2026-05-12
Proposal: [HL7V2-PROP-0001](../proposals/HL7V2-PROP-0001-source-of-truth-and-release-governance.md)

## Contract

Every durable work item must be traceable through:

```text
proposal -> spec -> ADR-or-none -> plan -> active goal -> issue/PR -> proof -> closeout
```

or explicitly marked as a small standalone maintenance fix.

The trace does not require every document type for every change. It requires
each durable claim to have exactly one owning surface and a clear link path to
proof.

## Source Ownership

| Layer | Owns | Does not own |
| --- | --- | --- |
| Proposal / PRD | Why the work exists, affected users, success criteria, non-goals | Detailed PR order or completed proof |
| Spec | Required behavior, contracts, acceptance criteria, examples, proof requirements | PR sequencing or current feature status |
| ADR | Durable architecture decisions and consequences | General planning or implementation checklists |
| Implementation plan | PR sequence, rollback, commands, closeout order | Product truth or architecture decisions |
| Active goal manifest | Current execution state, next work, blockers, must-not-touch rules | Historical receipts |
| GitHub issue / PR | Reviewable unit of work and discussion | Canonical product status |
| Proof command / receipt | Evidence that a claim was checked or completed | Future behavior requirements |
| Status / support docs | Current product claims and support tier truth | Campaign rationale or PR sequencing |
| Policy TOML | CI, lint, package, file, and exception ledgers | Prose explanation of why a campaign exists |

## Current Canonical Sources

- Current feature and release status: [docs/STATUS.md](../STATUS.md)
- Evidence artifact contracts:
  [docs/contracts/evidence-contract-index.md](../contracts/evidence-contract-index.md)
  and `schemas/evidence/`
- CI economics and lane routing: [docs/ci/](../ci/) and `policy/ci-*.toml`
- Python TestPyPI proof state:
  [docs/guides/python-testpypi-release-proof.md](../guides/python-testpypi-release-proof.md)
  and audit receipts
- Rust crates.io release process: [RELEASE_PROCESS.md](../../RELEASE_PROCESS.md)
- Active execution state: `.hl7v2/goals/active.toml`

If two documents disagree, prefer the source listed above for that claim type
and treat the other document as stale until corrected.

## Behavior Requirements

### Specs Define Behavior And Proof

Specs define what must be true, what acceptance means, and what proof is
required. They may include examples, but they must not become PR checklists.

### Plans Define Sequencing

Plans define the order of PRs, rollback choices, command order, and closeout
steps. Plans link to proposals, specs, and ADRs instead of restating their
content.

### Active Goals Define Current Execution

`.hl7v2/goals/active.toml` is the current machine-readable execution manifest.
It should be added only after the proposal, specs, and plan entries it links to
exist.

### Audits And Receipts Record What Happened

Audits, handoffs, release receipts, and workflow receipts record facts after
execution. They do not define future behavior or active work.

### Status Remains Current Product Truth

`docs/STATUS.md` remains the current feature and release status source of truth.
Specs, plans, ADRs, and receipts must link to status for current product claims
instead of copying current-state tables.

### Evidence Contracts Remain Contract Truth

`docs/contracts/evidence-contract-index.md` and `schemas/evidence/` remain the
evidence artifact contract sources. Other docs may describe why those contracts
matter, but they must not fork the contract table.

### CI Policy Remains CI Truth

`docs/ci/` and `policy/ci-*.toml` remain the CI economics and lane-routing
sources. Specs may require a CI proof, but workflow behavior changes need their
own accepted scope.

### Maintenance Fixes May Be Smaller

A small standalone maintenance fix may skip proposal, spec, ADR, and plan files
when the PR body marks the change maintenance-only and includes direct proof.
Examples include a one-line broken-link fix, typo-only correction, or stale path
repair.

## Acceptance Examples

### Python TestPyPI Work Item

A Python TestPyPI work item links:

- `docs/proposals/HL7V2-PROP-0001-source-of-truth-and-release-governance.md`
- `docs/specs/HL7V2-SPEC-0002-python-distribution-proof.md`
- `plans/1.4.1/testpypi-closure.md`
- `.hl7v2/goals/active.toml`
- issue [#563](https://github.com/EffortlessMetrics/hl7v2-rs/issues/563)
- a receipt PR after upload and install-back proof pass

It must not claim TestPyPI success while the upload is blocked by Trusted
Publisher setup.

### CI Verification Economics Work Item

A CI economics work item links:

- `docs/proposals/HL7V2-PROP-0001-source-of-truth-and-release-governance.md`
- `docs/specs/HL7V2-SPEC-0003-ci-verification-economics.md`
- [docs/ci/cost-and-verification-policy.md](../ci/cost-and-verification-policy.md)
- `policy/ci-lane-whitelist.toml`
- `policy/ci-risk-packs.toml`

It may explain why checks are routed by cost and risk. It must not change CI
workflow behavior unless a later accepted spec and plan authorize that work.

### One-Line Docs Link Fix

A one-line docs link fix may skip proposal and spec files when the PR is marked
maintenance-only and proof is direct, such as:

```text
rg -n "stale-path|new-path" docs/affected-file.md
cargo +1.93.0 run -p xtask -- check-doc-links
```

## Proof Requirements

Docs-only source-of-truth PRs should run:

```powershell
cargo +1.93.0 run -p xtask -- check-doc-links
cargo +1.93.0 run -p xtask -- publish-plan
$env:PYO3_USE_ABI3_FORWARD_COMPATIBILITY='1'; cargo +1.93.0 run -p xtask -- gate --check --changed
```

The publish plan must keep `hl7v2-python` outside the Rust crates.io graph.

## Non-Goals

- No runtime behavior changes.
- No evidence schema changes.
- No CI workflow behavior changes.
- No Python publish behavior changes.
- No duplicate status, evidence-contract, or CI-policy tables.
