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
| gRPC server | Typed service transport with evidence parity for the implemented RPCs; service lifecycle and operational hardening remain tracked separately from artifact semantics. |
| Python package `hl7v2` | Python user package backed by `hl7v2-python`; release proof is separate from crates.io backend proof. |
| TypeScript package `@effortlessmetrics/hl7v2` | Planned package governed by [HL7V2-SPEC-0005](HL7V2-SPEC-0005-npm-wasm-binding-package-model.md). |

## Parity Matrix

| Contract | Rust | CLI | REST | gRPC | Python | TypeScript |
| --- | --- | --- | --- | --- | --- | --- |
| parse | Stable | Stable | Stable | Stable | Stable local binding | Planned |
| write / normalize | Stable | Stable | Stable | Stable | Stable local binding | Planned |
| validate | Stable | Stable | Stable | Stable | Stable local binding | Planned |
| ACK | Stable | Stable where exposed | Stable where exposed | Stable | Stable local binding | Planned |
| profile lint / explain / test | Stable | Stable | Stable where exposed | Profile lint/explain/test stable | Stable local helper | Planned |
| redaction receipt | Stable | Stable | Stable | Stable via `ValidateRedacted` | Stable local binding | Planned |
| quarantine output | Not applicable | Not applicable | Stable | Stable via `ValidateRedacted` when configured | Planned or not claimed | Planned |
| corpus summary | Stable | Stable | Stable | Stable for inline messages | Stable local helper | Planned |
| corpus fingerprint / diff | Stable | Stable | Stable | Stable for inline messages | Stable local helper where exposed | Planned |
| bundle / replay | Stable | Stable | Stable | Bundle creation/replay stable | Stable local helper where exposed | Planned |
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

The machine-readable parity manifest lives in
[`policy/evidence-parity.toml`](../../policy/evidence-parity.toml). It records
the current surface state, proof commands, fixture families, and known gaps so
future PRs can update parity state without scraping audit prose. The manifest
does not create a new release, registry, or runtime claim by itself. Use
`cargo run -p xtask -- check-evidence-parity` to verify the manifest keeps the
required surfaces, contracts, proof links, and non-claim boundaries.
Use `cargo run -p xtask -- check-evidence-parity-acceptance` as the default
local Rust/CLI/REST/gRPC acceptance suite for the implemented parity runners.
Pass `--include-python` only after a local `hl7v2` wheel is installed; Python
registry availability remains governed by the separate TestPyPI/PyPI proof
lane.

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

The shared dirty-corpus fixture family lives in
`test_data/dirty-real-world/`. It currently proves Rust core, CLI, REST server,
gRPC server, and local Python wheel corpus summary, fingerprint, and diff parity
for Z-segments, large OBX expansion, legacy encoding declarations, malformed
delimiters, partial batch-like input, odd MSH sender/receiver metadata,
generated MLLP-framed input, and generated truncated-MLLP failure input.
It also proves CLI, REST, and gRPC validate/redact/bundle/replay workflows
against the shared Z-segment fixture so dirty-corpus evidence is not limited to
feed-level corpus commands. The local Python wheel proof uses the same
Z-segment fixture for validate, redact, bundle, and replay semantics when the
Python smoke lane is included.
TypeScript parity must use the same fixture family or explicitly explain why a
transport-specific fixture is required.
The default acceptance runner is
`cargo run -p xtask -- check-dirty-corpus-parity`; it composes the existing
Rust, CLI, REST, and gRPC dirty-corpus checks. Pass `--include-python` only
after a local `hl7v2` wheel is installed; that adds the Python dirty-corpus
smoke and dirty validate/redact/bundle/replay smoke, but Python package
availability remains governed by the separate TestPyPI/PyPI proof lane.

The shared safe-error and PHI sentinel fixture lives in
`test_data/security/safe-error-phi-parity.json`. It supplies the synthetic PHI
message, safe-analysis policy, malformed-input payload, malformed-profile
payload, expected safe diagnostics, and forbidden sentinel values used by Rust
core, CLI, REST, gRPC, and local Python proof. Surface tests may stay
transport-specific, but they must use this fixture when claiming safe-error or
PHI sentinel parity. The default acceptance runner is
`cargo run -p xtask -- check-safe-error-phi-parity`; it composes the
fixture-backed Rust, CLI, REST, and gRPC checks. Pass `--include-python` only
after a local `hl7v2` wheel is installed, because Python package availability
remains governed by the separate TestPyPI/PyPI proof lane.

The shared schema-version fixture lives in
`test_data/evidence/schema-version-parity.json`. It records the accepted v2
request value, expected v2 artifact `schema_version`, unsupported request
version, unsupported-version diagnostic fragment, surface tool names, and the
validation shape used by representative CLI, REST, gRPC, and local Python
proof. Artifact-specific tests may remain near their surfaces, but they should
consume this fixture when claiming schema-version behavior parity.
The default acceptance runner is
`cargo run -p xtask -- check-schema-version-parity`; it composes the
fixture-backed Rust, CLI, REST, and gRPC checks plus evidence schema
validation. Pass `--include-python` only after a local `hl7v2` wheel is
installed, because Python package availability remains governed by the
separate TestPyPI/PyPI proof lane.

The default bundle/replay acceptance runner is
`cargo run -p xtask -- check-bundle-replay-parity`; it composes the existing
Rust, CLI, REST, and gRPC bundle/replay checks. Pass `--include-python` only
after a local `hl7v2` wheel is installed, because Python package availability
remains governed by the separate TestPyPI/PyPI proof lane.

The default aggregate local acceptance runner is
`cargo run -p xtask -- check-evidence-parity-acceptance`. It verifies the
manifest and then runs the shared safe-error/PHI, schema-version, dirty-corpus,
and bundle/replay parity runners for Rust, CLI, REST, and gRPC.

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

`GenerateAck` can be described as gRPC ACK parity when the proto contract
accepts application ACK codes `AA`, `AE`, and `AR` plus commit ACK codes `CA`,
`CE`, and `CR`, returns an ACK payload whose `MSA.1` and `MSA.2` preserve the
requested code and original control ID, returns a parsed ACK shape for
verification, and passes gRPC contract tests.

`CreateEvidenceBundle` can be described as gRPC evidence bundle creation parity
when the RPC accepts inline message, profile, and redaction policy inputs,
writes only under the configured server bundle root, rejects unsafe bundle IDs,
returns the shared bundle summary shape without configured root paths or raw
bundle IDs, supports opt-in v2 bundle artifacts if claimed, avoids raw HL7,
profile, and policy echo in diagnostics, and passes gRPC contract tests.

`ReplayEvidenceBundle` can be described as gRPC evidence replay parity when the
RPC reads only from the configured server bundle root, rejects unsafe bundle
IDs, returns the shared replay report shape without configured root paths or raw
bundle IDs, supports opt-in v2 replay reports if claimed, reports missing
bundles as not found, fails closed on tampered bundles through the shared replay
checks, and passes gRPC contract tests.

`ValidateRedacted` can be described as gRPC quarantine output parity when the
RPC writes configured quarantine output only after failed redacted validation,
omits quarantine output for valid reports or disabled quarantine config, fails
closed when quarantine is enabled without a configured root, returns only
root-relative output IDs, supports opt-in v2 quarantine summaries if claimed,
avoids raw HL7 and configured filesystem roots in responses and diagnostics, and
passes gRPC contract tests.

### Correct Python Claim

A local Python helper can be described as locally proven when a wheel install,
`import hl7v2`, and helper-specific smoke or parity test pass.

It must not be described as TestPyPI-proven or PyPI-released until upload and
install-back receipts from those registries exist.

The Python ACK helper can be described as local binding parity when the Python
wheel smoke proves default `AA`, explicit code mapping, control ID
preservation, and unsupported-code failure. The checked Python evidence
workflow may also use ACK artifacts as part of the local evidence loop. This
does not prove TestPyPI or PyPI availability.

### Correct TypeScript Claim

The planned TypeScript user package is `@effortlessmetrics/hl7v2`. Future
TypeScript parity starts with package review, install/import smoke, and shared
parse/validate/redaction fixtures. It must not use `hl7v2-rs` as the public SDK
identity.
