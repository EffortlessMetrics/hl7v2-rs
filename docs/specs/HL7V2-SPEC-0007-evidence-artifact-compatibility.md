# HL7V2-SPEC-0007: Evidence Artifact Compatibility

Status: Accepted
Date: 2026-05-19
Source-of-truth stack: [HL7V2-SPEC-0001](HL7V2-SPEC-0001-source-of-truth-stack.md)
Related parity spec: [HL7V2-SPEC-0006](HL7V2-SPEC-0006-cross-surface-evidence-parity.md)

## Contract

Evidence artifacts are product contracts once documented as stable. They are not logs.

For each stable artifact contract we define: stable fields, advisory fields, `schema_version` behavior, PHI posture, replayability/shareability posture, semver impact for breaking changes, proof command, and owning schema path.

Covered artifacts: ValidationReport, RedactionReceipt, EvidenceBundle, ReplayReport, CorpusSummary, CorpusFingerprint, CorpusDiff, SafeError, ProfileLintReport, ProfileExplainReport, ProfileTestReport, QuarantineSummary.

## Machine Rails

- Schemas: `schemas/evidence/**`
- Contract index: `docs/contracts/evidence-contract-index.md`
- Parity manifest: `policy/evidence-parity.toml`
- Required checks:
  - `cargo +1.95.0 run -p xtask -- evidence-schema-check`
  - `cargo +1.95.0 run -p xtask -- check-schema-version-parity`
  - `cargo +1.95.0 run -p xtask -- check-safe-error-phi-parity`

## Acceptance Rule

A new stable evidence field must update schema, contract index, parity manifest, fixture/golden proof, and this compatibility spec whenever the public contract behavior changes.
