# HL7V2-PROP-0001: Source-of-Truth and Release Governance

Status: Proposed
Date: 2026-05-12

## Problem

`hl7v2-rs` now has enough durable evidence infrastructure that prose-only
navigation is too lossy for high-throughput maintenance. Current status, schema
contracts, CI policy, release receipts, Python packaging state, and active work
can all be accurate in isolation while still leaving future agents to infer the
next action from old audits, release notes, and chat context.

The repo needs a source-of-truth stack that separates why work exists, what must
be true, which architecture decisions constrain it, how PRs are sequenced, what
is active now, and what proof closed the loop.

## Users And Surfaces

This proposal serves:

- users who need product claims to map to current proof;
- maintainers who need release and packaging boundaries to stay explicit;
- Codex, Droid, and other agents that need machine-readable execution state;
- reviewers who need each PR to show whether it edits the why, contract,
  decision, plan, active state, or receipt.

The affected surfaces are documentation and planning surfaces only:

- [docs/README.md](../README.md)
- [docs/STATUS.md](../STATUS.md)
- [docs/contracts/evidence-contract-index.md](../contracts/evidence-contract-index.md)
- [docs/ci/](../ci/)
- [docs/guides/python-testpypi-release-proof.md](../guides/python-testpypi-release-proof.md)
- [docs/guides/python-pypi-release.md](../guides/python-pypi-release.md)
- `docs/proposals/`
- `docs/specs/`
- `docs/adr/`
- `plans/1.4.1/`
- `.hl7v2/goals/`
- `policy/ci-*.toml`

## Success Criteria

- A cold checkout can answer what the active campaign is, why it exists, what
  behavior must hold, what decisions constrain it, what PR order is expected,
  what commands prove it, and what external blockers remain.
- `docs/STATUS.md` remains the current feature and release status source of
  truth.
- Evidence artifact contracts remain owned by
  [docs/contracts/evidence-contract-index.md](../contracts/evidence-contract-index.md)
  and `schemas/evidence/`.
- CI economics remain owned by `docs/ci/` and `policy/ci-*.toml`.
- Python release state remains separate from the Rust crates.io publish graph.
- Active campaign state becomes machine-readable in `.hl7v2/goals/active.toml`
  after the referenced proposal, specs, and plans exist.

## Proposed Source-of-Truth Stack

Use this ownership model:

| Claim type | Source of truth |
| --- | --- |
| Current feature status | [docs/STATUS.md](../STATUS.md) |
| Evidence artifact contracts | [docs/contracts/evidence-contract-index.md](../contracts/evidence-contract-index.md) and `schemas/evidence/` |
| CI economics and lane routing | `docs/ci/` and `policy/ci-*.toml` |
| Python release state | Python release guides and audit receipts |
| Active execution state | `.hl7v2/goals/active.toml` |
| Future behavior requirements | `docs/specs/HL7V2-SPEC-*.md` |
| Why the work exists | `docs/proposals/HL7V2-PROP-*.md` |
| Durable architecture choices | `docs/adr/HL7V2-ADR-*.md` or the existing numbered ADR series |
| PR sequence | `plans/<milestone>/implementation-plan.md` |
| Completed proof | `docs/audits/` or `docs/handoffs/` receipts |

The default flow is:

```text
Roadmap / STATUS
  -> Proposal / PRD
    -> Specs
      -> ADRs where needed
        -> Implementation plan
          -> .hl7v2/goals/active.toml
            -> GitHub issues / PRs
              -> proof commands
              -> evidence schemas
              -> policy receipts
              -> closeout audit
```

Small standalone maintenance fixes may skip proposal and spec files when the PR
body marks them maintenance-only and links the proof used.

## Alternatives Rejected

### Keep Expanding STATUS

Rejected because `docs/STATUS.md` should state current product truth, not hold
campaign rationale, PR order, and proof receipts for every active lane.

### Treat Audits As The Plan

Rejected because audits record what happened. They are receipts, not the next
agent execution contract.

### Put Active State In Chat Or PR Bodies Only

Rejected because chat and PR bodies are not stable repo surfaces for cold
checkout operation.

### Make Every Document Repeat Every Claim

Rejected because duplicated status, schema, and release claims drift. Each claim
type needs one home and links elsewhere.

## Specs To Create

- `docs/specs/HL7V2-SPEC-0001-source-of-truth-stack.md`
- `docs/specs/HL7V2-SPEC-0002-python-distribution-proof.md`
- `docs/specs/HL7V2-SPEC-0003-ci-verification-economics.md`
- `docs/specs/HL7V2-SPEC-0004-evidence-contract-support-map.md`
- `docs/specs/HL7V2-SPEC-0005-external-proof-receipts.md`

## ADRs Needed

- `docs/adr/HL7V2-ADR-0001-evidence-artifacts-are-contracts.md`
- `docs/adr/HL7V2-ADR-0002-python-is-separate-distribution-lane.md`

These ADRs should record durable decisions only. They should link to the current
contract and release-proof sources instead of copying their tables.

## Implementation Campaign Shape

Use PR-sized documentation changes:

1. Define the source-of-truth model.
2. Add this governing proposal.
3. Add the source-of-truth stack spec.
4. Add the Python distribution proof spec.
5. Add the CI verification economics spec.
6. Record evidence artifacts as product contracts.
7. Record Python as a separate distribution lane.
8. Add a support tier proof map.
9. Add the v1.4.1 implementation plan.
10. Add the active campaign manifest.

Each PR should change one semantic artifact or one small scaffold layer. Runtime
behavior, CI workflows, evidence schemas, and publishing behavior are outside
this campaign unless a later accepted spec explicitly changes scope.

## Evidence Plan

Docs-only PRs should use:

```powershell
cargo +1.93.0 run -p xtask -- check-doc-links
cargo +1.93.0 run -p xtask -- publish-plan
$env:PYO3_USE_ABI3_FORWARD_COMPATIBILITY='1'; cargo +1.93.0 run -p xtask -- gate --check --changed
```

The publish plan must continue to report only:

```text
hl7v2
hl7v2-server
hl7v2-cli
```

Python TestPyPI proof remains incomplete until issue
[#563](https://github.com/EffortlessMetrics/hl7v2-rs/issues/563) is resolved
externally and a same-commit upload plus install-back receipt lands.

## Risks

- New planning surfaces could become another source of duplicate truth.
- Agents could treat proposed specs as implemented behavior before acceptance.
- Active state could become stale if closeout does not archive or update it.
- Release pressure could blur the TestPyPI/PyPI boundary.

Mitigations:

- Link to status, contract, policy, and receipt sources instead of copying their
  tables.
- Mark proposal/spec/ADR status explicitly.
- Add `.hl7v2/goals/active.toml` only after the linked files exist.
- Keep TestPyPI and production PyPI claims receipt-backed.

## Non-Goals

- No runtime feature changes.
- No evidence schema changes.
- No CI workflow behavior changes.
- No Python publishing.
- No crates.io publishing.
- No claim that TestPyPI or PyPI succeeded before upload and install-back pass.

## Exit Criteria

- The proposal/spec/ADR/plan/active-goal stack is documented.
- Accepted specs describe the source-of-truth stack, Python proof contract, and
  CI verification economics contract.
- ADRs record evidence artifacts as contracts and Python as a separate
  distribution lane.
- A support tier map links product claims to proof commands without replacing
  `docs/STATUS.md`.
- `plans/1.4.1/` sequences the campaign through proof and closeout.
- `.hl7v2/goals/active.toml` names the active work, blockers, proof commands,
  and non-goals.
- TestPyPI issue #563 is either completed with a receipt or remains explicitly
  blocked without token fallback or skip-existing.
