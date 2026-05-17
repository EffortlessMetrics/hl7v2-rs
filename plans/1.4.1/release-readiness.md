# v1.4.1 Release Readiness Plan

Status: Superseded by v1.5.0 release
Proposal: [HL7V2-PROP-0001](../../docs/proposals/HL7V2-PROP-0001-source-of-truth-and-release-governance.md)
Support map: [SUPPORT_TIERS](../../docs/status/SUPPORT_TIERS.md)

Historical note: this patch-release candidate was not the final release path.
The repo shipped the Rust 1.95 / v1.5.0 quality-ratchet release instead. Use
[`docs/release/1.5.0-readiness.md`](../../docs/release/1.5.0-readiness.md),
[`docs/audits/publish-v1.5.0-2026-05-15.md`](../../docs/audits/publish-v1.5.0-2026-05-15.md),
and [`docs/STATUS.md`](../../docs/STATUS.md) for current release truth.

## Goal

Prepare a patch release candidate only after the source-of-truth stack and
Python proof receipts are durable.

## Candidate Theme

```text
v1.4.1 - Evidence Rails and Python Packaging Proof
```

## Candidate Scope

Likely included after receipts are clean:

- split-train evidence hardening already merged before this lane;
- final source-tree audit and TestPyPI blocker receipts through #564;
- #565 deployment ADR link fix;
- source-of-truth scaffolding, proposal, specs, ADRs, plans, support map, and
  active goal manifest;
- TestPyPI success receipt, if issue #563 is completed.

Production PyPI is included only if an explicit production release decision is
made and upload plus install-back pass from `pypi.org`.

## Required Proof Before Release Decision

```powershell
cargo +1.93.0 run -p xtask -- check-doc-links
cargo +1.93.0 run -p xtask -- publish-plan
$env:PYO3_USE_ABI3_FORWARD_COMPATIBILITY='1'; cargo +1.93.0 run -p xtask -- gate --check --changed
```

Additional proof if TestPyPI closes:

```text
Python TestPyPI Proof from main with publish_to_testpypi=true
TestPyPI upload: success
TestPyPI install-back: success
smoke.py: success
production PyPI: not attempted
```

## Non-Goals

- Do not publish Python to production PyPI by default.
- Do not treat a crates.io `hl7v2-python` binding-backend publish as a
  TestPyPI or PyPI proof for the public Python package `hl7v2`.
- Do not claim TestPyPI success until upload and install-back pass.
- Do not include production PyPI unless production upload and install-back pass.

## Rollback

If any proof is missing, stop at "release candidate not ready" and keep the
blocking issue or receipt open. Do not rewrite release notes to imply proof that
has not passed.
