# Source-of-Truth Stack Plan

Status: Closed / implemented
Proposal: [HL7V2-PROP-0001](../../docs/proposals/HL7V2-PROP-0001-source-of-truth-and-release-governance.md)
Spec: [HL7V2-SPEC-0001](../../docs/specs/HL7V2-SPEC-0001-source-of-truth-stack.md)
ADRs: [ADR-0001](../../docs/adr/HL7V2-ADR-0001-evidence-artifacts-are-contracts.md),
[ADR-0002](../../docs/adr/HL7V2-ADR-0002-python-is-separate-distribution-lane.md)

Historical note: the source-of-truth stack is now implemented and current
execution state lives in `.hl7v2/goals/active.toml`. This file is retained as
the original PR-sequencing plan, not as the active work queue.

## Goal

Make the documentation operating model explicit so agents and maintainers can
tell whether a change edits the why, contract, decision, PR sequence, active
state, or proof receipt.

## Production Delta

Documentation-only.

## Work Items

| Item | Goal | Acceptance | Proof |
| --- | --- | --- | --- |
| Scaffolding | Define folder roles. | `docs/README.md` links proposals, specs, plans, and active goals. | `cargo +1.93.0 run -p xtask -- check-doc-links` |
| Proposal | Explain why governance exists. | PROP-0001 includes problem, users, success criteria, alternatives, evidence plan, risks, and exit criteria. | docs gate |
| Source stack spec | Define traceability rule. | SPEC-0001 owns proposal/spec/ADR/plan/active-goal/issue/PR/proof/closeout flow. | docs gate |
| Python proof spec | Preserve packaging boundary. | SPEC-0002 blocks token fallback, skip-existing, and unproven PyPI claims. | docs gate plus `publish-plan` |
| CI economics spec | Explain cost-aware depth. | SPEC-0003 links to canonical CI policy and the industrialized-AI article appears once. | `rg -n "assisted-native-industrialized" docs` |
| ADRs | Record durable decisions. | Evidence artifacts are contracts; Python is a separate distribution lane. | docs gate plus `publish-plan` |
| Support map | Map claims to proof. | Support tiers link proof commands without replacing `docs/STATUS.md`. | docs gate |
| Active goal manifest | Record current execution state. | `.hl7v2/goals/active.toml` links real proposal, specs, ADRs, and plans. | docs gate |

## Non-Goals

- Do not duplicate `docs/STATUS.md`.
- Do not duplicate the evidence contract index.
- Do not change CI workflows or policy TOMLs in this plan.
- Do not publish packages.

## Rollback

Revert the documentation PR that introduced the incorrect source ownership or
plan entry. Leave existing receipts intact unless they are factually wrong.
