# HL7V2-ADR-0002: Python Is A Separate Distribution Lane

Status: Accepted
Date: 2026-05-12
Proposal: [HL7V2-PROP-0001](../proposals/HL7V2-PROP-0001-source-of-truth-and-release-governance.md)
Spec: [HL7V2-SPEC-0002](../specs/HL7V2-SPEC-0002-python-distribution-proof.md)

## Context

`hl7v2-python` is built with maturin and distributed as a Python package. It
shares Rust implementation code with the workspace, but its release proof,
package index, Trusted Publishing configuration, and install-back receipts are
Python packaging concerns.

The Rust crates.io product surface is:

```text
hl7v2
hl7v2-server
hl7v2-cli
```

Publishing `hl7v2-python` as a Rust crate would blur package manager boundaries
and create false release evidence.

## Decision

`hl7v2-python` is not a crates.io product crate. It is a Python/maturin
distribution lane.

`publish = false` remains required for `crates/hl7v2-python`.

Python release proof uses TestPyPI and PyPI workflows, not the Rust crates.io
publish graph.

## Consequences

- `cargo +1.93.0 run -p xtask -- publish-plan` must continue to report only
  `hl7v2`, `hl7v2-server`, and `hl7v2-cli`.
- Python TestPyPI and PyPI receipts are separate from Rust crates.io release
  receipts.
- TestPyPI proof remains blocked until external Trusted Publisher setup for
  issue [#563](https://github.com/EffortlessMetrics/hl7v2-rs/issues/563)
  is complete and upload plus install-back pass.
- Production PyPI remains an explicit release decision after same-commit
  TestPyPI proof.
- Token fallback and skip-existing are not valid substitutes for Trusted
  Publishing proof.

## Non-Goals

- This ADR does not publish to TestPyPI or PyPI.
- This ADR does not publish to crates.io.
- This ADR does not change workflow behavior.
- This ADR does not change package metadata.

## Proof Expectations

Docs-only ADR changes use:

```powershell
cargo +1.93.0 run -p xtask -- check-doc-links
cargo +1.93.0 run -p xtask -- publish-plan
$env:PYO3_USE_ABI3_FORWARD_COMPATIBILITY='1'; cargo +1.93.0 run -p xtask -- gate --check --changed
```
