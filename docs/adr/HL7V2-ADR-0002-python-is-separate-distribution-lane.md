# HL7V2-ADR-0002: Python Is A Separate Distribution Lane

Status: Accepted
Date: 2026-05-12
Proposal: [HL7V2-PROP-0001](../proposals/HL7V2-PROP-0001-source-of-truth-and-release-governance.md)
Spec: [HL7V2-SPEC-0002](../specs/HL7V2-SPEC-0002-python-distribution-proof.md)
Amended by:
[HL7V2-ADR-0003](HL7V2-ADR-0003-publishable-binding-backend-crates.md)

## Context

The public Python package is `hl7v2` and is built with maturin from the
`hl7v2-python` Rust/PyO3 binding backend crate. It shares Rust implementation code
with the workspace, but its release proof, package index, Trusted Publishing
configuration, and install-back receipts are Python packaging concerns.

The primary Rust crates.io product surface is:

```text
hl7v2
hl7v2-server
hl7v2-cli
```

Publishing `hl7v2-python` as a primary Rust product crate would blur package
manager boundaries and create false release evidence. A later ADR allows
binding backend crates to be published as implementation packaging artifacts
when they are clearly separated from the primary Rust product graph.

## Decision

`hl7v2-python` is not a primary crates.io product crate. It is the Rust/PyO3
package for the public `hl7v2` Python distribution lane.

Current metadata may make `crates/hl7v2-python` publishable as a governed
binding backend crate. That does not make it part of the primary Rust product
graph, and it does not prove Python TestPyPI or PyPI publication.

Python release proof uses TestPyPI and PyPI workflows, not the Rust crates.io
primary product graph.

## Consequences

- Default `cargo +1.95.0 run -p xtask -- publish-plan` output continues to
  report the primary Rust product graph: `hl7v2`, `hl7v2-server`, and
  `hl7v2-cli`.
- `cargo +1.95.0 run -p xtask -- publish-plan --surface bindings` reports the
  binding backend graph separately. Binding backend crates still require
  release tooling and receipts before any crates.io upload can be claimed.
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
- This ADR does not itself publish or promote binding backend metadata.

## Proof Expectations

Docs-only ADR changes use:

```powershell
cargo +1.95.0 run -p xtask -- check-doc-links
cargo +1.95.0 run -p xtask -- publish-plan
cargo +1.95.0 run -p xtask -- publish-plan --surface bindings
$env:PYO3_USE_ABI3_FORWARD_COMPATIBILITY='1'; cargo +1.95.0 run -p xtask -- gate --check --changed
```
