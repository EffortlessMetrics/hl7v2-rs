# v1.4.1 Source-of-Truth Implementation Plan

Status: Active
Date: 2026-05-13
Proposal: [HL7V2-PROP-0001](../../docs/proposals/HL7V2-PROP-0001-source-of-truth-and-release-governance.md)
Primary spec: [HL7V2-SPEC-0001](../../docs/specs/HL7V2-SPEC-0001-source-of-truth-stack.md)

## Goal

Prepare a small release-discipline lane that makes `hl7v2-rs`
self-describing: durable claims have one home, active work has machine-readable
state, and release closure is blocked on receipt-backed proof rather than prose
memory.

## Production Delta

No runtime, workflow, schema, packaging, or publish behavior changes are planned
in this documentation campaign.

## Work Items

| Item | Status | Linked artifacts | Goal |
| --- | --- | --- | --- |
| Finish deployment ADR link fix | Done | PR #565 | Remove stale deployment ADR links. |
| Source-of-truth scaffold | Done | PR #567 | Add README scaffolding for proposals, specs, plans, and active goals. |
| Governing proposal | Done | [PROP-0001](../../docs/proposals/HL7V2-PROP-0001-source-of-truth-and-release-governance.md) | Explain why the source-of-truth stack exists. |
| Source-of-truth stack spec | Done | [SPEC-0001](../../docs/specs/HL7V2-SPEC-0001-source-of-truth-stack.md) | Define claim ownership and traceability rules. |
| Python distribution proof spec | Done | [SPEC-0002](../../docs/specs/HL7V2-SPEC-0002-python-distribution-proof.md) | Preserve TestPyPI/PyPI proof boundaries. |
| CI verification economics spec | Done | [SPEC-0003](../../docs/specs/HL7V2-SPEC-0003-ci-verification-economics.md) | Explain deep, cost-aware verification. |
| Evidence artifacts ADR | Done | [ADR-0001](../../docs/adr/HL7V2-ADR-0001-evidence-artifacts-are-contracts.md) | Record evidence artifacts as product contracts. |
| Python distribution ADR | Done | [ADR-0002](../../docs/adr/HL7V2-ADR-0002-python-is-separate-distribution-lane.md) | Record Python as separate from crates.io. |
| Support tier map | Done | [SUPPORT_TIERS](../../docs/status/SUPPORT_TIERS.md) | Map product surfaces to proof commands. |
| TestPyPI Trusted Publisher proof | Blocked | [testpypi-closure.md](testpypi-closure.md), issue [#563](https://github.com/EffortlessMetrics/hl7v2-rs/issues/563) | Complete upload and install-back after external setup. |
| Active goal manifest | Ready | `.hl7v2/goals/active.toml` | Record machine-readable current campaign state. |
| v1.4.1 release readiness | Waiting | [release-readiness.md](release-readiness.md) | Decide patch candidate only after receipts are clean. |

## PR Order

1. Merge the completed source-of-truth documentation PRs.
2. Add `.hl7v2/goals/active.toml` after this plan exists.
3. Resolve TestPyPI Trusted Publisher setup outside the repo.
4. Rerun the Python TestPyPI Proof workflow from `main`.
5. If upload and install-back pass, add a receipt PR and close #563.
6. Decide separately whether production PyPI should be attempted.
7. Prepare v1.4.1 only after the source-of-truth stack and Python proof
   receipts are durable.

## Non-Goals

- No runtime feature changes.
- No CI workflow behavior changes.
- No evidence schema changes.
- No Python publishing from this plan PR.
- No production PyPI decision by default.

## Proof Commands

Docs-only plan changes use:

```powershell
cargo +1.93.0 run -p xtask -- check-doc-links
cargo +1.93.0 run -p xtask -- publish-plan
$env:PYO3_USE_ABI3_FORWARD_COMPATIBILITY='1'; cargo +1.93.0 run -p xtask -- gate --check --changed
```

## Rollback

Plans are documentation-only. Revert the plan PR if it misstates sequencing or
claim ownership. Do not change runtime behavior to make a plan true.
