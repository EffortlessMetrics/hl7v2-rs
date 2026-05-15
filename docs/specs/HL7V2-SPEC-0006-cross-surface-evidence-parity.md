# HL7V2-SPEC-0006: Cross-Surface Evidence Parity

Status: Accepted
Date: 2026-05-14
Proposal: [HL7V2-PROP-0001](../proposals/HL7V2-PROP-0001-source-of-truth-and-release-governance.md)
Source-of-truth stack: [HL7V2-SPEC-0001](HL7V2-SPEC-0001-source-of-truth-stack.md)
Related Python proof spec: [HL7V2-SPEC-0002](HL7V2-SPEC-0002-python-distribution-proof.md)
Related backend proof spec: [HL7V2-SPEC-0004](HL7V2-SPEC-0004-binding-backend-release-proof.md)
Related npm/WASM package model: [HL7V2-SPEC-0005](HL7V2-SPEC-0005-npm-wasm-binding-package-model.md)

## Contract

Evidence parity means the same HL7 message, profile, corpus, redaction policy,
bundle, or replay input has the same product meaning across supported
surfaces. It does not require every language package or server transport to
expose every command immediately, and it does not require byte-for-byte wrapper
APIs. It requires shared semantics, safe error shape, schema-backed artifacts,
and proof receipts for every claimed surface.

Current and future surfaces are:

| Surface | Role |
| --- | --- |
| Rust crate `hl7v2` | Canonical parser, validator, normalizer, evidence model, and artifact semantics. |
| CLI `hl7v2-cli` | Operator and CI interface for evidence commands and support receipts. |
| REST server | HTTP sidecar for validation, redaction, corpus, bundle, replay, and service integration. |
| gRPC server | Typed service transport; narrower than REST until focused parity PRs land. |
| Python package `hl7v2` | Python user package backed by `hl7v2-python`; release proof is separate from crates.io backend proof. |
| TypeScript package `@effortlessmetrics/hl7v2` | Planned package governed by [HL7V2-SPEC-0005](HL7V2-SPEC-0005-npm-wasm-binding-package-model.md). |

## Parity Matrix

| Contract | Rust | CLI | REST | gRPC | Python | TypeScript |
| --- | --- | --- | --- | --- | --- | --- |
| parse | Stable | Stable | Stable | Stable | Stable local binding | Planned |
| write / normalize | Stable | Stable | Stable | Stable | Stable local binding | Planned |
| validate | Stable | Stable | Stable | Stable | Stable local binding | Planned |
| ACK | Stable | Stable where exposed | Stable where exposed | Stable | Planned or not claimed | Planned |
| profile lint / explain / test | Stable | Stable | Stable where exposed | Profile lint/explain/test stable | Stable local helper | Planned |
| redaction receipt | Stable | Stable | Stable | Stable via `ValidateRedacted` | Stable local binding | Planned |
| corpus summary | Stable | Stable | Stable | Stable for inline messages | Stable local helper | Planned |
| corpus fingerprint / diff | Stable | Stable | Stable | Stable for inline messages | Stable local helper where exposed | Planned |
| bundle / replay | Stable | Stable | Stable | Bundle creation stable; replay planned | Stable local helper where exposed | Planned |
| safe error shape | Stable | Stable | Stable | Stable for implemented RPCs | Required for every claimed helper | Planned |
| `schema_version` behavior | Stable | Stable | Stable | Stable for implemented v2 evidence RPCs | Required for every claimed artifact | Planned |
| PHI sentinel behavior | Stable | Stable | Stable | Required for every evidence RPC | Required for every claimed helper | Planned |

`Stable local binding` means local Python wheel/import smoke and parity tests
exist for the helper surface. It is not a TestPyPI or production PyPI release
claim. Python distribution proof remains governed by
[HL7V2-SPEC-0002](HL7V2-SPEC-0002-python-distribution-proof.md).

`Planned until implemented` means the product claim must stay narrower until a
focused implementation PR adds the surface, docs, and proof receipts.

## Required Proof

Every parity claim must map to at least one local or hosted receipt:

| Claim | Minimum proof |
| --- | --- |
| Rust artifact semantics | `cargo test -p hl7v2 --all-features`; `cargo run -p xtask -- evidence-schema-check` |
| CLI evidence command | CLI integration or BDD test for the command and artifact shape |
| REST endpoint | Server endpoint contract test plus schema or PHI sentinel proof where applicable |
| gRPC RPC | `cargo test -p hl7v2-server --test grpc_contract_tests` and proto lint/packaging proof |
| Python helper | Local wheel install/import smoke plus helper-specific parity test |
| Python distribution | TestPyPI or PyPI upload and install-back receipt from the target registry |
| TypeScript package | npm package review, install/import smoke, and parity fixtures after implementation |
| Evidence artifact | Schema validation against `schemas/evidence/` and golden fixture coverage |
| Publish or registry claim | Upload plus registry-resolution or install-back proof |

## Fixture Rules

Parity fixtures should be shared across surfaces where practical. A fixture set
may be transport-specific only when the transport adds a real concern such as
MLLP framing, gRPC streaming, HTTP request/response metadata, Python packaging,
or TypeScript/WASM serialization.

Required fixture families:

- parse and write round trip;
- validation success, warning, and error shape;
- normalization with canonical delimiters and optional MLLP framing;
- ACK code mapping and control ID preservation;
- profile lint, explain, and fixture test output;
- redaction receipt with PHI sentinels;
- corpus summary, fingerprint, and diff;
- evidence bundle creation and replay;
- v1 and v2 `schema_version` behavior where an artifact supports both;
- malformed input that proves safe diagnostics without echoing raw PHI.

## Non-Goals

- No new runtime implementation in this spec.
- No crates.io, TestPyPI, PyPI, npm, tag, or GitHub release claim.
- No requirement that gRPC expose every REST endpoint in one PR.
- No requirement that Python or TypeScript users import binding backend crates.
- No return to public Rust implementation microcrates for parser, model,
  redaction, MLLP, batch, stream, or evidence internals.

## Acceptance Examples

### Correct gRPC Parity Claim

`CorpusSummarize` can be described as gRPC corpus summary parity when the RPC
accepts inline messages, returns the shared corpus summary fields, supports
opt-in v2 provenance if claimed, rejects unsupported schema versions, avoids
request filesystem reads, and passes gRPC contract tests.

`CorpusFingerprint` can be described as gRPC corpus fingerprint parity when the
RPC accepts inline messages, returns the shared corpus fingerprint fields,
supports optional inline profile validation issue-code counts, supports opt-in
v2 provenance if claimed, rejects unsupported schema versions, avoids request
filesystem reads, and passes gRPC contract tests.

`CorpusDiff` can be described as gRPC corpus diff parity when the RPC accepts
inline before/after message sets, returns the shared corpus diff fields,
supports optional inline profile validation issue-code deltas, supports opt-in
v2 provenance if claimed, rejects unsupported schema versions, avoids request
filesystem reads, and passes gRPC contract tests.

`ProfileLint` can be described as gRPC profile lint parity when the RPC accepts
inline profile YAML, returns the shared profile lint report fields, supports
opt-in v2 provenance if claimed, rejects unsupported schema versions, avoids
raw profile echo in malformed-profile diagnostics, and passes gRPC contract
tests.

`ProfileExplain` can be described as gRPC profile explain parity when the RPC
accepts inline profile YAML, returns the shared profile explain report fields,
supports opt-in v2 provenance if claimed, rejects unsupported schema versions,
avoids raw profile echo in malformed-profile diagnostics, and passes gRPC
contract tests.

`ProfileTest` can be described as gRPC profile test parity when the RPC accepts
an inline profile and inline fixture messages, returns the shared profile test
report fields, supports opt-in v2 provenance if claimed, rejects unsupported
schema versions, avoids request filesystem reads, avoids raw profile echo in
malformed-profile diagnostics, and passes gRPC contract tests.

`CreateEvidenceBundle` can be described as gRPC evidence bundle creation parity
when the RPC accepts inline message, profile, and redaction policy inputs,
writes only under the configured server bundle root, rejects unsafe bundle IDs,
returns the shared bundle summary shape without configured root paths or raw
bundle IDs, supports opt-in v2 bundle artifacts if claimed, avoids raw HL7,
profile, and policy echo in diagnostics, and passes gRPC contract tests.

These claims must not imply gRPC replay parity until a replay RPC and tests
land.

### Correct Python Claim

A local Python helper can be described as locally proven when a wheel install,
`import hl7v2`, and helper-specific smoke or parity test pass.

It must not be described as TestPyPI-proven or PyPI-released until upload and
install-back receipts from those registries exist.

### Correct TypeScript Claim

The planned TypeScript user package is `@effortlessmetrics/hl7v2`. Future
TypeScript parity starts with package review, install/import smoke, and shared
parse/validate/redaction fixtures. It must not use `hl7v2-rs` as the public SDK
identity.
