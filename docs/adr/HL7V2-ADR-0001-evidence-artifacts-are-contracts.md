# HL7V2-ADR-0001: Evidence Artifacts Are Contracts

Status: Accepted
Date: 2026-05-12
Proposal: [HL7V2-PROP-0001](../proposals/HL7V2-PROP-0001-source-of-truth-and-release-governance.md)
Spec: [HL7V2-SPEC-0001](../specs/HL7V2-SPEC-0001-source-of-truth-stack.md)

## Context

`hl7v2-rs` produces evidence for users, maintainers, release workflows, support
bundles, server endpoints, and Python bindings. Those artifacts are now part of
the product surface. If they drift as incidental output, downstream automation
and support workflows lose trust in the repo's claims.

The repo already has schema and contract ownership in
[docs/contracts/evidence-contract-index.md](../contracts/evidence-contract-index.md)
and `schemas/evidence/`. This ADR records the architecture rule behind that
surface: stable evidence artifacts are contracts.

## Decision

Evidence artifacts are product contracts, not incidental output.

This applies to stable machine-readable outputs including:

- `ValidationReport`
- `ProfileLintReport`
- `ProfileExplainReport`
- `ProfileTestReport`
- `CorpusSummary`
- `CorpusFingerprint`
- `CorpusDiff`
- `RedactionReceipt`
- `EvidenceBundleSummary`
- `EvidenceReplayReport`
- `DoctorReport`
- `QuarantineOutput`
- Python dict/json outputs
- server REST `report_json` outputs
- server gRPC `report_json` outputs

The contract source of truth remains
[docs/contracts/evidence-contract-index.md](../contracts/evidence-contract-index.md)
and `schemas/evidence/`.

## Consequences

- Every stable machine artifact must have a schema or an explicit documented
  reason it does not.
- v2 provenance fields remain opt-in unless a release decision changes defaults.
- CLI, server, and Python surfaces should converge on artifact semantics.
- Documentation must link to the evidence contract index instead of copying or
  forking its table.
- Release receipts should prove artifact behavior with schema-backed or
  golden-tested evidence where practical.
- Future artifact promotions need a spec or plan entry that names the contract,
  proof command, and user-visible compatibility impact.

## Non-Goals

- This ADR does not add or change schemas.
- This ADR does not change runtime output.
- This ADR does not promote experimental artifacts to stable.
- This ADR does not change Python, REST, or gRPC behavior.

## Proof Expectations

Docs-only ADR changes use:

```powershell
cargo +1.93.0 run -p xtask -- check-doc-links
cargo +1.93.0 run -p xtask -- publish-plan
$env:PYO3_USE_ABI3_FORWARD_COMPATIBILITY='1'; cargo +1.93.0 run -p xtask -- gate --check --changed
```
