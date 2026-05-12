# HL7V2-SPEC-0003: CI Verification Economics

Status: Accepted
Date: 2026-05-12
Proposal: [HL7V2-PROP-0001](../proposals/HL7V2-PROP-0001-source-of-truth-and-release-governance.md)
Source-of-truth stack: [HL7V2-SPEC-0001](HL7V2-SPEC-0001-source-of-truth-stack.md)

## Contract

Deep verification and efficient verification are the same goal at industrialized
PR volume.

`hl7v2-rs` must route verification so each CI dollar buys useful risk reduction:

- shift cheap static, schema, contract, link, file-policy, and publish-policy
  checks left into PR-time rails;
- keep expensive runtime verification where it buys meaningful risk reduction;
- make artifacts schema-backed and golden-tested so later checks do less
  guessing;
- use publish-plan, doc-link, file-policy, evidence-schema, Python
  publish-policy, smoke, and contract gates as layered receipts;
- avoid broad always-on expensive lanes where targeted or advisory lanes produce
  the same decision quality.

This spec carries the general CI economics lesson from the perl-lsp planning
thread into `hl7v2-rs`. It does not import tool-specific doctrine unless the
tool is actually wired into this repo.

## Canonical Sources

- CI cost and verification doctrine:
  [docs/ci/cost-and-verification-policy.md](../ci/cost-and-verification-policy.md)
- LEM budgeting: [docs/ci/lem-budgeting.md](../ci/lem-budgeting.md)
- CI labels: [docs/ci/labels.md](../ci/labels.md)
- CI inventory: [docs/ci/inventory.md](../ci/inventory.md)
- Lane whitelist: `policy/ci-lane-whitelist.toml`
- Risk packs: `policy/ci-risk-packs.toml`
- CI budget policy: `policy/ci-budget.toml`

The industrialized-AI article belongs in the canonical CI cost policy, not in
every spec, plan, or receipt.

## Behavior Requirements

### PR-Time Checks Are Layered

Ordinary PRs should start with low-cost proof that is broad enough to catch
common mistakes:

- PR plan and risk-pack classification;
- lint, no-panic-family, and file-policy checks;
- doc-link checks for docs changes;
- Python publish-policy checks for packaging boundaries;
- schema and contract checks when affected surfaces change.

### Expensive Runtime Proof Is Scoped

Windows, macOS, full property tests, benchmarks, coverage, wheel publication,
full API/gRPC suites, and release dry-runs should run when their risk surface is
touched, a label requests them, or main/release/nightly policy requires them.
They should not become broad ordinary-PR defaults without an accepted policy
change.

### Receipts Must Be Decision-Useful

Receipts should explain what risk was reduced, which command or workflow proved
it, and what remains unproven. A green check that does not map to a claim is not
enough for release posture.

### Cost Pressure Must Not Weaken Required Safety

Cost-aware verification must route proof, not remove proof. The hard rules in
the canonical CI policy remain binding.

## Acceptance Criteria

- The canonical CI cost policy explains why deep and efficient verification are
  the same design goal at high PR volume.
- The industrialized-AI article is linked exactly once from the canonical CI
  cost policy.
- The explanation is adapted to `hl7v2-rs` surfaces: evidence schemas, golden
  fixtures, server smoke, gRPC contracts, Python packaging proof, doc/file/
  publish policy rails, publish-plan, and PHI/safety sentinels.
- No CI workflow behavior changes are made by this spec.
- No required-check, budget-enforcement, or publish behavior changes are made by
  this spec.

## Proof Requirements

Docs-only CI economics PRs should run:

```powershell
cargo +1.93.0 run -p xtask -- check-doc-links
cargo +1.93.0 run -p xtask -- publish-plan
$env:PYO3_USE_ABI3_FORWARD_COMPATIBILITY='1'; cargo +1.93.0 run -p xtask -- gate --check --changed
```

## Non-Goals

- No workflow edits.
- No required-check changes.
- No budget enforcement changes.
- No publish behavior changes.
- No evidence schema changes.
