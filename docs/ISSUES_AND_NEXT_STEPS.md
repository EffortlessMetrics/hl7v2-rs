# Issues, Blockers, Next Steps, and Friction Points

> Last updated: 2026-05-19

This document is a current navigation aid for the remaining product gaps. It
does not replace the status, support-tier, roadmap, parity, or receipt sources
of truth:

- [docs/STATUS.md](STATUS.md)
- [docs/status/SUPPORT_TIERS.md](status/SUPPORT_TIERS.md)
- [ROADMAP.md](../ROADMAP.md)
- [policy/evidence-parity.toml](../policy/evidence-parity.toml)
- [docs/guides/README.md](guides/README.md)

The stale v1.2/v1.3 planning snapshot that previously lived here has been
retired. It described gRPC, Python bindings, and enterprise evidence work as
not started, which is no longer true for current `main`.

## Current Blockers

### Public Python package proof

The public Python package is `hl7v2`; the crates.io crate `hl7v2-python` is
only the PyO3 binding backend. The backend crate is published as binding
infrastructure, but that does not prove the public Python package.

The remaining blocker is external Trusted Publisher setup for TestPyPI:

- Issue: [#563](https://github.com/EffortlessMetrics/hl7v2-rs/issues/563)
- Project: `hl7v2`
- Owner: `EffortlessMetrics`
- Repository: `hl7v2-rs`
- Workflow: `python-testpypi.yml`
- Environment: `testpypi`

The workflow can now be run from `main` with
`diagnose_trusted_publisher=true` and `publish_to_testpypi=false` to record the
actual GitHub `audience=pypi` OIDC claims through the `testpypi` environment
without uploading. That is diagnostic evidence only; the blocker remains open
until TestPyPI upload and install-back succeed.

Required proof after setup:

```text
cargo +1.95.0 run -p xtask -- python-public-registry-proof --index testpypi --version <version>
```

The receipt must record upload, install-back, `import hl7v2`, Python smoke,
evidence workflow, and dirty evidence workflow success. No token fallback and
no `skip-existing` are allowed.

### Production PyPI decision

Production PyPI remains unreleased. It requires an explicit decision after
same-commit TestPyPI proof exists. TestPyPI success must not silently become a
production PyPI release.

Required proof after approval:

```text
cargo +1.95.0 run -p xtask -- python-public-registry-proof --index pypi --version <version>
```

### Public Python parity promotion

Python evidence parity is strong for local wheel proof, but it remains
local-wheel-scoped until TestPyPI or PyPI install-back succeeds. After public
registry proof, update the parity state and support tiers to distinguish
TestPyPI proof from production PyPI proof.

## Current Maintenance Work

### Evidence parity

Keep parity claims mapped to executable proof commands rather than prose. The
current shared semantics are tracked in
[HL7V2-SPEC-0006](specs/HL7V2-SPEC-0006-cross-surface-evidence-parity.md) and
[policy/evidence-parity.toml](../policy/evidence-parity.toml).

Core acceptance commands include:

```text
cargo +1.95.0 run -p xtask -- check-evidence-parity
cargo +1.95.0 run -p xtask -- check-evidence-parity-acceptance
cargo +1.95.0 run -p xtask -- check-dirty-corpus-parity
cargo +1.95.0 run -p xtask -- check-bundle-replay-parity
cargo +1.95.0 run -p xtask -- check-safe-error-phi-parity
cargo +1.95.0 run -p xtask -- check-schema-version-parity
```

### Operator workflows

The first-use, sidecar, safe support-bundle, artifact interpretation, vendor
upgrade diff, and operator error guidance paths are now executable guide smokes.
Keep future changes tied to the guide commands listed in
[docs/status/SUPPORT_TIERS.md](status/SUPPORT_TIERS.md).

### Dirty real-world corpus proof

The dirty corpus should continue expanding with synthetic or redacted fixtures
that represent vendor-shaped HL7, including:

- Z-segments
- odd MSH sender/receiver fields
- malformed delimiters
- legacy timestamp and encoding variants
- partial batch-like inputs
- large OBX payloads
- MLLP wrapper and failure traces
- redacted support bundles

New fixture classes should update the corpus receipts and keep Rust, CLI,
REST/gRPC, and local Python wheel expectations aligned.

### gRPC operational hardening

gRPC evidence semantics are broadly covered by contract tests, but the support
tier remains beta while transport lifecycle and operational deployment hardening
catch up with the REST sidecar surface.

### Advisory RIPR calibration

RIPR remains advisory static mutation-exposure analysis. Continue calibrating it
against real PR traffic before considering any blocking policy.

## Later Work

### TypeScript and WASM

Do not start npm/WASM implementation until Python public proof is resolved or
explicitly parked. The public npm package identity is:

```text
@effortlessmetrics/hl7v2
```

Do not use `hl7v2-rs` as the public npm package. Future backend crates such as
`hl7v2-wasm` or `hl7v2-node` are binding infrastructure, not a return to public
parser/model/redaction/MLLP microcrates.

## No Longer Current

The following old claims are no longer valid for current `main`:

- v1.2.0 is the current production-readiness target.
- v1.3.0 is still only planning.
- gRPC support has not started.
- language bindings have not started.
- the old parser/model/redaction/MLLP microcrate topology is the product
  surface.
- local Python wheel proof should be treated as public TestPyPI or PyPI proof.

Use [docs/STATUS.md](STATUS.md) and [ROADMAP.md](../ROADMAP.md) for current
release and next-work truth before acting on older planning receipts.
