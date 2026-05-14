# HL7V2-ADR-0003: Publishable Binding Backend Crates

Status: Accepted
Date: 2026-05-13
Proposal: [HL7V2-PROP-0001](../proposals/HL7V2-PROP-0001-source-of-truth-and-release-governance.md)
Specs:
[HL7V2-SPEC-0001](../specs/HL7V2-SPEC-0001-source-of-truth-stack.md),
[HL7V2-SPEC-0002](../specs/HL7V2-SPEC-0002-python-distribution-proof.md)
Amends:
[HL7V2-ADR-0002](HL7V2-ADR-0002-python-is-separate-distribution-lane.md)

## Context

The demicrocrating work collapsed ordinary SRP implementation boundaries into
the canonical `hl7v2` Rust library crate. That decision remains correct:
parser, model, redaction, profile, transport, and evidence implementation units
are modules, not independent product crates.

Language bindings are a different kind of boundary. A binding crate can be a
thin packaging backend that gives external package managers reproducible source
availability, version anchoring, and build provenance without becoming the main
Rust API.

The current Python surface already has this shape:

| Layer | Name | Registry | User-facing |
| --- | --- | --- | --- |
| Core Rust API | `hl7v2` | crates.io | yes |
| Rust service and CLI wrappers | `hl7v2-server`, `hl7v2-cli` | crates.io | yes |
| Python public package | `hl7v2` | PyPI | yes |
| Python Rust backend crate | `hl7v2-python` | crates.io, if promoted by policy and tooling | no |

The public Python package and import remain `hl7v2`. The Rust/PyO3 binding
backend crate remains `hl7v2-python`.

## Decision

The repo distinguishes primary Rust product crates from binding backend crates.

Primary Rust product crates are the crates users should choose when they want a
Rust API, service, or CLI:

```text
hl7v2
hl7v2-server
hl7v2-cli
```

Binding backend crates may be publishable to crates.io when they support an
external language package:

```text
hl7v2-python
future hl7v2-wasm
future hl7v2-node
```

A binding backend crate may be published only when it is:

- thin;
- version-locked to the workspace release;
- clearly described as a binding backend;
- not documented as the main user-facing Rust API;
- covered by release proof for its own package boundary;
- prevented from becoming a broad implementation microcrate.

Publishing a binding backend is allowed for packaging provenance. It is not a
signal that the crate is a primary Rust product API. These crates are honest
APIs at the foreign-language boundary, but their audience is packagers and
binding maintainers rather than ordinary Rust users.

## Consequences

- `hl7v2-python` may become publishable in a later PR, but this ADR does not
  change Cargo metadata.
- Default Rust release planning continues to show the primary Rust product
  graph separately from binding backend graphs.
- `xtask publish-plan --surface primary|bindings|all-publishable` prints
  package surfaces such as:

```text
Primary Rust product graph:
1. hl7v2
2. hl7v2-server
3. hl7v2-cli

Binding backend graph:
1. hl7v2-python
```

- Python TestPyPI and PyPI proof remain separate from crates.io proof. A
  crates.io backend publish does not prove Python upload or install-back.
- Rust users should still be routed to `hl7v2` unless they are working on the
  binding backend itself.
- Future JS/TS package work should prefer `@effortlessmetrics/hl7v2` as the
  public npm package and use backend crate names such as `hl7v2-wasm` or
  `hl7v2-node` only for implementation packaging.

## Non-Goals

- This ADR does not publish any crate.
- This ADR does not change `crates/hl7v2-python/Cargo.toml`.
- This ADR does not rename the Python PyPI package or import module.
- This ADR does not reopen retired implementation microcrates such as
  `hl7v2-parser`, `hl7v2-model`, or `hl7v2-redact`.
- This ADR does not make TestPyPI or PyPI claims.
- This ADR does not change CI workflow behavior.

## Follow-Up Work

- Decide whether to make `hl7v2-python` publishable as a binding backend.
- If `hl7v2-python` becomes publishable, update its metadata and README before
  any crates.io dry-run or upload.
- Define a future npm binding backend spec before adding JS/TS packages.

## Proof Expectations

Docs-only ADR changes use:

```powershell
cargo +1.95.0 run -p xtask -- check-doc-links
cargo +1.95.0 run -p xtask -- publish-plan
cargo +1.95.0 run -p xtask -- publish-plan --surface bindings
cargo +1.95.0 run -p xtask -- publish-plan --surface all-publishable
cargo +1.95.0 run -p xtask -- check-file-policy
git diff --check
```
