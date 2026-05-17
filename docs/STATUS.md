# HL7v2-rs Implementation Status

This document provides a transparent view of which features are fully implemented, partially implemented, or planned.

> **Last Updated**: 2026-05-17
> **Project Status**: v1.5.0 is published to crates.io for the selected Rust graph: `hl7v2`, `hl7v2-python`, `hl7v2-server`, and `hl7v2-cli`. `hl7v2-python` is published only as binding backend infrastructure for the public Python `hl7v2` package, not as the recommended Rust API.

## Core Components

| Crate | Status | Coverage | Notes |
|-------|--------|----------|-------|
| `hl7v2` | ✅ 100% | 92% | Canonical Rust library crate for parsing, writing, validation, transport framing, ACK, normalization, and generation. Foundation model, escape, and MLLP implementations now live here. |
| `hl7v2-server` | ✅ 100% | 80% | HTTP REST API with metrics, auth, ACK, normalization, redacted validation, configured-root bundle/replay, inline corpus evidence, readiness, quarantine, and redacted structured logs. |
| `hl7v2-cli` | ✅ 100% | 75% | Full-featured CLI with streaming support. |
| Python binding (`hl7v2` distribution) | 🟡 Experimental | Smoke | Public Python distribution built from the `hl7v2-python` PyO3 binding backend; not part of the primary Rust product graph and validated through the Python/maturin wheel smoke lane before any PyPI release. |
| retired old package names | ✅ Retired locally | N/A | Old microcrate package names, including `hl7v2-model`, `hl7v2-escape`, and `hl7v2-mllp`, are no longer local workspace crates. Some historical old-name `1.2.0` artifacts already exist on crates.io and should not be treated as the current product surface. |

## Published Feature Set (v1.5.0)

### 🚀 Connectivity
- ✅ **MLLP Over TCP**: Fully implemented async client and server.
- ✅ **TLS Support**: Secure framing using `rustls`.
- ✅ **HTTP REST API**: Axum-based JSON endpoints for parse, validate, ACK, and normalize.
- 🟡 **gRPC Service**: v1.5.0 unary RPCs have contract tests. Current `main` serves gRPC with `hl7v2 serve --mode grpc`, implements `ParseStream` as one request message into one response message, `GenerateAck`, `Normalize`, `ProfileLint`, `ProfileExplain`, and `ProfileTest` for inline profile evidence, `ValidateRedacted` with opt-in v2 validation, redaction receipt, and configured quarantine output evidence, `CreateEvidenceBundle` for configured-root evidence bundle creation, `ReplayEvidenceBundle` for configured-root evidence replay, `CorpusSummarize` for inline corpus summary evidence, `CorpusFingerprint` for inline corpus fingerprint evidence, and `CorpusDiff` for inline before/after corpus diff evidence with opt-in v2 provenance.

### 🛡️ Security & Observability
- ✅ **API Authentication**: Constant-time API Key validation.
- ✅ **Rate Limiting**: Per-IP throttling to prevent DoS.
- ✅ **Prometheus Metrics**: Throughput, latency, and error tracking.
- ✅ **Audit Ready**: Server metrics and structured runtime logs are available. Redacted evidence-workflow logs with hashed message-control and bundle identifiers are part of v1.4.0.

### 🧪 Quality Assurance
- ✅ **BDD Tests**: Real validation scenarios verified with Cucumber.
- ✅ **E2E Tests**: Subprocess CLI and network integration tests.
- ✅ **Property Testing**: Robust parsing and escaping edge-case coverage.
- ✅ **Security Workflow**: Dependency audit, cargo-deny, Semgrep, Trivy, and secret scanning are green on current `main`.

## Release and Publish Readiness

- ✅ **Main workflows**: required CI success, Security, and CI Policy were observed green for the final pre-publish merge. Coverage is unchanged/skipped for this docs/package release lane. Extended tests and benchmark artifacts remain non-publish performance lanes.
- ✅ **Publish order**: `cargo run -p xtask -- publish-plan` defaults to the primary Rust product graph: `hl7v2`, `hl7v2-server`, and `hl7v2-cli`. Use `cargo run -p xtask -- publish-plan --surface bindings` to inspect binding backend crates separately.
- ✅ **Published Rust graph**: `hl7v2`, `hl7v2-python`, `hl7v2-server`, and `hl7v2-cli` v1.5.0 are published and visible in the crates.io index. See [`docs/audits/publish-v1.5.0-2026-05-15.md`](audits/publish-v1.5.0-2026-05-15.md).
- ✅ **Dry-run publish**: Workspace-patched dry-run verification and dependency-ordered direct dry-runs were completed before upload. See [`docs/audits/publish-dry-run-v1.5.0-2026-05-15-final-prepublish.md`](audits/publish-dry-run-v1.5.0-2026-05-15-final-prepublish.md).
- ✅ **v1.5.0 published Rust graph**: v1.5.0 is published to crates.io for `hl7v2`, `hl7v2-python`, `hl7v2-server`, and `hl7v2-cli`. The release is tagged as `v1.5.0`, has a GitHub release, and has a registry-resolution and public install-back receipt. A repeatable public crates smoke now rechecks the `hl7v2`, `hl7v2-cli`, and `hl7v2-server` first-use paths from crates.io. `hl7v2-python` is included only as a binding backend crate. See [`docs/audits/publish-v1.5.0-2026-05-15.md`](audits/publish-v1.5.0-2026-05-15.md) and [`docs/audits/public-crates-install-first-use-2026-05-16.md`](audits/public-crates-install-first-use-2026-05-16.md).
- 🟡 **Python binding lane**: `hl7v2-python` is published to crates.io as a governed binding backend, but it is not part of the primary Rust product graph and does not prove the public Python package. The public Python distribution is `hl7v2`. Current `main` has a v1.5.0 local wheel build, fresh-venv install, import smoke, and evidence workflow receipt; see [`docs/audits/python-local-wheel-proof-2026-05-15.md`](audits/python-local-wheel-proof-2026-05-15.md). The hosted non-publishing TestPyPI proof mode passed again on current `main` after the SRP refactor wave; see [`docs/audits/python-testpypi-nonpublish-proof-2026-05-16.md`](audits/python-testpypi-nonpublish-proof-2026-05-16.md). The hosted production PyPI non-publishing rehearsal also passed and is recorded in [`docs/audits/python-pypi-nonpublish-proof-2026-05-16.md`](audits/python-pypi-nonpublish-proof-2026-05-16.md). The manual TestPyPI publishing proof remains required before any production PyPI release. The 2026-05-17 TestPyPI upload attempt built and smoke-tested the wheel but failed with `invalid-publisher` because the TestPyPI Trusted Publisher is not configured. Production PyPI upload and install-back have not been run. A 2026-05-17 package-state check still found no visible `hl7v2` package on TestPyPI or production PyPI.
- ✅ **Package registry state**: A 2026-05-15 pre-release registry audit found crates.io `hl7v2`, `hl7v2-server`, and `hl7v2-cli` at `1.4.0`, no crates.io `hl7v2-python`, and no PyPI/TestPyPI `hl7v2` package before publish. The v1.5.0 publish receipt supersedes the crates.io portion of that audit while preserving the PyPI/TestPyPI absence claim. See [`docs/audits/package-registry-state-2026-05-15.md`](audits/package-registry-state-2026-05-15.md) and [`docs/audits/publish-v1.5.0-2026-05-15.md`](audits/publish-v1.5.0-2026-05-15.md).
- 🟡 **Objective completion audit**: The pre-publish 2026-05-15 prompt-to-artifact audit correctly kept the lane open before crates.io release. The post-release audit now records the current state: crates.io, tag, GitHub release, and public Rust/CLI/server install-back portions are complete; TestPyPI upload/install-back proof, a production PyPI decision, and any future npm/WASM implementation receipts remain separate. See [`docs/audits/v1.5.0-objective-completion-audit-2026-05-15.md`](audits/v1.5.0-objective-completion-audit-2026-05-15.md), [`docs/audits/v1.5.0-objective-completion-audit-2026-05-15-post-release.md`](audits/v1.5.0-objective-completion-audit-2026-05-15-post-release.md), and [`docs/audits/publish-v1.5.0-2026-05-15.md`](audits/publish-v1.5.0-2026-05-15.md).
- ✅ **Binding-backend closeout**: #604 accepted the binding-backend ADR, #605 refreshed the yanked `metrics` lock entry that blocked security checks, #606 added `publish-plan --surface primary|bindings|all-publishable`, #607 framed `hl7v2-python` as the PyO3 backend for the public Python `hl7v2` package, and #608 fixed Python wheel cache behavior. This closeout did not publish `hl7v2-python`, TestPyPI, PyPI, or any v1.5.0 crates.io artifact.
- ✅ **Binding-backend readiness audit**: #610 added the binding-backend release-proof spec, #611 added the binding backend dry-run surface, #612 prepared `hl7v2-python` as publishable backend metadata, #613 defined the future npm/WASM package model, and #614 added a publish-surface classification guard. See [`docs/audits/binding-backend-readiness-2026-05-14.md`](audits/binding-backend-readiness-2026-05-14.md). This audit does not claim a crates.io backend upload, PyPI/TestPyPI upload, npm package, tag, GitHub release, or v1.5.0 publish.
- ⚠️ **Registry history**: crates.io already contains historical `1.2.0` artifacts for several old microcrate names. The current release plan does not publish those names again unless a deliberate deprecation-only compatibility release is chosen.
- ✅ **Tag alignment policy**: the existing `v1.2.0` tag points at an older commit and remains historical. Fresh `v1.2.1`, `v1.3.0`, `v1.4.0`, and `v1.5.0` tags point at their release heads.

## Package Boundary Model

- Primary Rust product crates: `hl7v2`, `hl7v2-server`, `hl7v2-cli`.
- Language packages: PyPI `hl7v2`, future npm `@effortlessmetrics/hl7v2`.
- Binding backend crates: `hl7v2-python`, future `hl7v2-wasm`, future
  `hl7v2-node`.
- Internal/dev crates: benches, e2e tests, test utilities, examples, and
  `xtask`.

Binding backend crates are real language-boundary APIs, but they are not the
recommended Rust API. `xtask publish-plan --surface bindings` reports this
separate graph. Current `hl7v2-python` metadata describes it as the PyO3
extension crate backing the Python `hl7v2` package and v1.5.0 published it to
crates.io as binding infrastructure only. #610-#614 added the binding-backend
release-proof spec, dry-run surface, publishable metadata, npm/WASM package
model, and publish surface guard; the v1.5.0 publish receipt owns the
`hl7v2-python` crates.io upload and registry-resolution proof. That crates.io
backend receipt is still not a TestPyPI or PyPI proof for the public Python
package.
Future TypeScript package work is governed by
[HL7V2-SPEC-0005](specs/HL7V2-SPEC-0005-npm-wasm-binding-package-model.md):
the public npm package is `@effortlessmetrics/hl7v2`, while Rust backend crates
such as `hl7v2-wasm` or `hl7v2-node` remain binding infrastructure.
Cross-surface evidence parity is governed by
[HL7V2-SPEC-0006](specs/HL7V2-SPEC-0006-cross-surface-evidence-parity.md):
Rust, CLI, REST, gRPC, Python, and future TypeScript claims must map to shared
semantics, safe diagnostics, schema-backed artifacts, and receipts before they
are described as equivalent.

## Evidence Contracts Release And Current Main

v1.4.0 is the Evidence Contracts and Server Sidecar release line around
deterministic HL7 interface evidence. It is tagged, released on GitHub, and
uploaded to crates.io for the primary Rust product graph.

Current `main` contains opt-in v2 provenance producers, maintained schema
validation through `xtask evidence-schema-check`, server replay and inline
corpus endpoints, redacted structured evidence logs, Docker sidecar smoke
coverage, broader PHI sentinel tests, Python/TestPyPI proof rails, the server
bundle replay message-type fix, Rust 1.95 policy ratchets, profile evidence,
Python profile helpers, gRPC profile lint/explain/test, configured-root
evidence bundle creation/replay, and inline corpus summary/fingerprint/diff
parity, advisory `ripr`, RIPR evidence endpoints, targeted mutation routing,
the cross-surface evidence parity spec, dirty real-world corpus proof, the
nightly property-test command repair, finite numeric validation, the v1.5.0
release-readiness workflow, the focused SRP module split train through #691,
and shared dirty-corpus parity across Rust core, CLI, REST/gRPC server, and
local Python wheel surfaces through #695. Current `main` also includes
normalization parity across CLI and local Python helper surfaces; CLI ACK parity
for command success, ACK code/control ID preservation, MSH-9 ACK message type,
and MLLP framing; REST ACK parity for all six supported ACK codes with MLLP
framing; gRPC ACK payload and parsed segment-shape contract coverage for all
six supported ACK codes; Python enhanced ACK smoke parity through #702; server
handler helper tests through #710; and conformance datatype/datetime coverage
with strict fractional timestamp syntax enforcement through #709.

For navigation across current docs, historical receipts, and evidence workflow
guides, start with [the documentation index](README.md). For the current
final source-tree gap audit after the local workbench split, see
[`docs/audits/current-source-tree-evidence-objective-gap-audit.md`](audits/current-source-tree-evidence-objective-gap-audit.md).

| Area | Status | Notes |
|------|--------|-------|
| First-run diagnostics | ✅ Stable | `hl7v2 doctor` verifies CLI version, sample parse, profile loading, JSON output, optional server reachability, and optional Python binding presence. |
| Typed validation evidence | ✅ Stable | `ValidationReport` is shared by library, CLI, server validation, and Python bindings. |
| Profiles as code | ✅ Stable | `profile lint`, `profile test`, and `profile explain` produce machine-readable profile evidence. |
| Corpus observability | ✅ Stable | `corpus summarize`, `corpus fingerprint`, and `corpus diff` produce feed-level evidence for regression and migration review. Current proof includes shared dirty-corpus coverage for Z-segments, legacy MSH/encoding fields, generated MLLP bytes, large OBX expansion, malformed delimiters, partial batch-like input, and safe parse-error output across Rust core, CLI, REST server, gRPC server, and local Python wheel surfaces. |
| Safe support packets | ✅ Stable | `redact`, `bundle`, and `replay` produce redacted evidence bundles with manifest checks and replay verification. |
| Evidence contracts | ✅ Stable | v1.4.0 ships opt-in v2 provenance schemas/producers and an `xtask evidence-schema-check` gate. |
| CLI automation contract | ✅ Stable | Evidence commands use stable exit codes, primary stdout, diagnostic stderr, and output-file/quiet/no-color flags. |
| Server edge guard | ✅ Stable | v1.4.0 ships `/hl7/replay`, inline-message corpus endpoints that do not read request filesystem paths, bundle artifact schema opt-in, redacted structured evidence logs with hashed message-control and bundle identifiers, evidence metrics, Docker smoke coverage, and the bundle replay message-type fix. Current `main` also includes gRPC configured-root evidence bundle creation and replay parity. |
| Python evidence lane | 🟡 Separate lane | Python wheel proof and minimum API parity cover parse, JSON export, normalize, ACK, generated fixtures, profile evidence helpers, validation, corpus, redaction, bundle, and replay. v1.4.0 adds v2 parity, PHI sentinel coverage, Python evidence docs, and a manual TestPyPI proof workflow. Python package proof remains separate from the primary Rust product graph. |

## v1.3.0 Readiness Checklist

Release notes: [`docs/releases/v1.3.0-evidence-loop.md`](releases/v1.3.0-evidence-loop.md).
Dry-run receipt: [`docs/audits/publish-dry-run-2026-05-09.md`](audits/publish-dry-run-2026-05-09.md).
Publish receipt: [`docs/audits/publish-2026-05-09.md`](audits/publish-2026-05-09.md).

- ✅ **Publish plan**: `cargo run -p xtask -- publish-plan` resolves `hl7v2`, `hl7v2-server`, and `hl7v2-cli`.
- ✅ **Full gate**: `cargo run -p xtask -- gate --check` passes on the v1.3.0 package line.
- ✅ **Dry-runs**: workspace-patched publish verification passes for the full graph and direct `cargo publish --dry-run` passes in dependency order after each dependency is visible in the crates.io index.
- ✅ **Python proof**: the maturin wheel build/install/import smoke lane passes without publishing `hl7v2-python` to crates.io.
- ✅ **Release notes and tag**: `v1.3.0` is tagged and the GitHub release is published.

## v1.4.0 Readiness Checklist

Release notes: [`docs/releases/v1.4.0-evidence-contracts.md`](releases/v1.4.0-evidence-contracts.md).
Dry-run receipt: [`docs/audits/publish-dry-run-v1.4.0-2026-05-09.md`](audits/publish-dry-run-v1.4.0-2026-05-09.md).
Publish receipt: [`docs/audits/publish-v1.4.0-2026-05-09.md`](audits/publish-v1.4.0-2026-05-09.md).
Objective audit: [`docs/audits/v1.4.0-objective-completion-audit.md`](audits/v1.4.0-objective-completion-audit.md).
Current source-tree truth audit: [`docs/audits/current-source-tree-evidence-objective-gap-audit.md`](audits/current-source-tree-evidence-objective-gap-audit.md).

- ✅ **Publish plan**: `cargo run -p xtask -- publish-plan` resolves `hl7v2`, `hl7v2-server`, and `hl7v2-cli`.
- ✅ **Evidence schemas**: `cargo run -p xtask -- evidence-schema-check` passes on the v1.4.0 package line.
- ✅ **API contracts**: local OpenAPI lint, proto lint, and packaged proto/OpenAPI drift tests pass on the v1.4.0 package line.
- ✅ **Full gate**: `cargo run -p xtask -- gate --check` passes on the v1.4.0 package line.
- ✅ **Dry-runs**: direct `hl7v2` dry-run and workspace-patched full-graph dry-run pass. Direct dependent dry-runs correctly wait for `hl7v2` v1.4.0 to exist in the crates.io index during the real publish sequence.
- ✅ **Python proof**: the maturin wheel build/install/import smoke proof passes for the v1.4.0 Python lane package without publishing `hl7v2-python` to crates.io.
- ✅ **Release notes and tag**: `v1.4.0` is tagged and the GitHub release is published.

## v1.5.0 Readiness Checklist

Release notes: [`docs/releases/v1.5.0-rust-1.95-quality-ratchet.md`](releases/v1.5.0-rust-1.95-quality-ratchet.md).
Readiness receipt: [`docs/release/1.5.0-readiness.md`](release/1.5.0-readiness.md).
Dry-run receipt: [`docs/audits/publish-dry-run-v1.5.0-2026-05-13.md`](audits/publish-dry-run-v1.5.0-2026-05-13.md).
Current-main refresh: [`docs/audits/publish-dry-run-v1.5.0-2026-05-14.md`](audits/publish-dry-run-v1.5.0-2026-05-14.md).
Parity refresh: [`docs/audits/publish-dry-run-v1.5.0-2026-05-14-parity-refresh.md`](audits/publish-dry-run-v1.5.0-2026-05-14-parity-refresh.md).
Corpus refresh: [`docs/audits/publish-dry-run-v1.5.0-2026-05-14-corpus-refresh.md`](audits/publish-dry-run-v1.5.0-2026-05-14-corpus-refresh.md).
gRPC status refresh: [`docs/audits/publish-dry-run-v1.5.0-2026-05-14-grpc-status-refresh.md`](audits/publish-dry-run-v1.5.0-2026-05-14-grpc-status-refresh.md).
gRPC corpus evidence refresh: [`docs/audits/publish-dry-run-v1.5.0-2026-05-14-grpc-corpus-refresh.md`](audits/publish-dry-run-v1.5.0-2026-05-14-grpc-corpus-refresh.md).
Numeric validation refresh: [`docs/audits/publish-dry-run-v1.5.0-2026-05-15-numeric-refresh.md`](audits/publish-dry-run-v1.5.0-2026-05-15-numeric-refresh.md).
gRPC profile lint refresh: [`docs/audits/publish-dry-run-v1.5.0-2026-05-15-grpc-profile-lint-refresh.md`](audits/publish-dry-run-v1.5.0-2026-05-15-grpc-profile-lint-refresh.md).
gRPC profile explain refresh: [`docs/audits/publish-dry-run-v1.5.0-2026-05-15-grpc-profile-explain-refresh.md`](audits/publish-dry-run-v1.5.0-2026-05-15-grpc-profile-explain-refresh.md).
gRPC profile test refresh: [`docs/audits/publish-dry-run-v1.5.0-2026-05-15-grpc-profile-test-refresh.md`](audits/publish-dry-run-v1.5.0-2026-05-15-grpc-profile-test-refresh.md).
gRPC bundle creation refresh: [`docs/audits/publish-dry-run-v1.5.0-2026-05-15-grpc-bundle-refresh.md`](audits/publish-dry-run-v1.5.0-2026-05-15-grpc-bundle-refresh.md).
gRPC replay refresh: [`docs/audits/publish-dry-run-v1.5.0-2026-05-15-grpc-replay-refresh.md`](audits/publish-dry-run-v1.5.0-2026-05-15-grpc-replay-refresh.md).
gRPC quarantine refresh: [`docs/audits/publish-dry-run-v1.5.0-2026-05-15-grpc-quarantine-refresh.md`](audits/publish-dry-run-v1.5.0-2026-05-15-grpc-quarantine-refresh.md).
Parity documentation refresh: [`docs/audits/publish-dry-run-v1.5.0-2026-05-15-parity-doc-refresh.md`](audits/publish-dry-run-v1.5.0-2026-05-15-parity-doc-refresh.md).
Final pre-publish proof: [`docs/audits/publish-dry-run-v1.5.0-2026-05-15-final-prepublish.md`](audits/publish-dry-run-v1.5.0-2026-05-15-final-prepublish.md).
Post-release SRP refresh: [`docs/audits/publish-dry-run-v1.5.0-2026-05-16-post-srp-refresh.md`](audits/publish-dry-run-v1.5.0-2026-05-16-post-srp-refresh.md).
Dirty-corpus parity refresh:
[`docs/audits/publish-dry-run-v1.5.0-2026-05-16-dirty-corpus-parity-refresh.md`](audits/publish-dry-run-v1.5.0-2026-05-16-dirty-corpus-parity-refresh.md).
Normalization and ACK parity refresh:
[`docs/audits/publish-dry-run-v1.5.0-2026-05-16-normalization-ack-refresh.md`](audits/publish-dry-run-v1.5.0-2026-05-16-normalization-ack-refresh.md).
Evidence parity refresh:
[`docs/audits/publish-dry-run-v1.5.0-2026-05-16-evidence-parity-refresh.md`](audits/publish-dry-run-v1.5.0-2026-05-16-evidence-parity-refresh.md).
gRPC enhanced ACK refresh:
[`docs/audits/publish-dry-run-v1.5.0-2026-05-16-grpc-enhanced-ack-refresh.md`](audits/publish-dry-run-v1.5.0-2026-05-16-grpc-enhanced-ack-refresh.md).
Test-coverage refresh:
[`docs/audits/publish-dry-run-v1.5.0-2026-05-17-test-coverage-refresh.md`](audits/publish-dry-run-v1.5.0-2026-05-17-test-coverage-refresh.md).
Publish receipt: [`docs/audits/publish-v1.5.0-2026-05-15.md`](audits/publish-v1.5.0-2026-05-15.md).
Release graph decision: [`docs/audits/v1.5.0-release-graph-decision-2026-05-14.md`](audits/v1.5.0-release-graph-decision-2026-05-14.md).
RIPR calibration: [`docs/audits/ripr-calibration-2026-05-15.md`](audits/ripr-calibration-2026-05-15.md).

- ✅ **Published release**: workspace package versions are published as `1.5.0` for the selected Rust crates.io graph: `hl7v2`, `hl7v2-python`, `hl7v2-server`, and `hl7v2-cli`.
- ✅ **Rust floor**: MSRV is Rust 1.95 and `rust-toolchain.toml` pins Rust 1.95.0 with `rustfmt` and `clippy`.
- ✅ **Verification rails**: lint policy, Clippy exceptions, no-panic exact identity and no-new-debt baseline, file-policy companion ledgers, advisory `ripr`, and targeted mutation routing are present.
- ✅ **Release readiness workflow**: `.github/workflows/release-readiness.yml` records the non-publishing readiness proof bundle.
- ✅ **Dry-run receipt**: hosted release-readiness dry-run passed on `main` at `b0bb5b5392354273946f36f797f39d741d318fc1`; current-main primary and binding surface dry-runs passed locally at `b4b7962e6f3f9d7ae5d91adf603e6328e3d13297` after #616-#618, at `cc1e3046e2496ea0c10a25239b9d077641d01c36` after #621-#622, at `425ff79b959ef5ceff1bdd072cbe074ff2a7ab04` after #624-#625, at `eb518e948d57bc07e96512ced306fe0db2dc2990` after #627, at `acfeacec0eda6d632d52e61440f2c85fda93d95f` after #629-#630, at `b0018c8760f07d738b3a6b7a11eeeda300786ef2` after #632-#633, at `fe8680c551bc3ce9248063906cbbe90ffe732a70` after #635-#636, at `483fb572a42006c07bd2e857fb7638ec91615b7d` after #638, at `915578b5cab5ab3abde7f806e5ad39c063d9cc82` after #640, at `47ba93201aaa15c95157d341bb5ec4f5f3871741` after #642, at `addbbe3e6962dc7f1629053c8a9c6810c7e37cc1` after #645, at `a12c6fdf61396de74f971c27f4887dd7c451543b` after #647, at `686dbff26afbbdcb97e4cc1915d1e33bd1404d14` after #649-#650, at `9fc95604d8950b565b6b6b7941ad275fd5624178` after #653, at `4cf501ddc0f7fc3d027b3ce2459e899fe4aa7092` after #684-#691, at `1564a1ac6a028146471e53c80fe3dbca22a32497` after #693-#695, at `c888405000384f391b24864e3e572dd0e9b6ba6a` after #696-#698, at `65c3c30ec52c9c5d65b58dae245b62f0bee9d198` after #699-#702, at `a8ac4cf5b9ab9e33a1a0aa61287901e00b13ab04` after #705, and at `aa548e1579f659c089162f0dd8e445219d0a2414` after #708-#710/#709.
- ✅ **Release graph decision**: v1.5.0 selects `hl7v2`, `hl7v2-python`, `hl7v2-server`, and `hl7v2-cli`, with `hl7v2-python` included only as binding backend infrastructure.
- ✅ **Publish receipt**: crates.io upload, registry resolution, `v1.5.0` tag, GitHub release, and public Rust/CLI/server install-back smoke are recorded in [`docs/audits/publish-v1.5.0-2026-05-15.md`](audits/publish-v1.5.0-2026-05-15.md). The repeatable install-back harness is recorded in [`docs/audits/public-crates-install-first-use-2026-05-16.md`](audits/public-crates-install-first-use-2026-05-16.md).
- 🟡 **Python proof**: the public Python distribution is `hl7v2`, remains separate from the primary Rust product graph, has current-main local wheel/import/evidence smoke proof, and the 2026-05-17 publishing-mode TestPyPI run again passed wheel smoke before failing at `invalid-publisher`; Trusted Publisher upload/install-back proof is still required before any TestPyPI or PyPI success claim.

## Historical Plans
Old planning documents have been moved to `docs/plans/` for archival reference.

---

**Current published release**: v1.5.0 is tested, package-verified, tagged, released on GitHub, and published to crates.io for the selected Rust graph: `hl7v2`, `hl7v2-python`, `hl7v2-server`, and `hl7v2-cli`. `hl7v2-python` is binding backend infrastructure, not the recommended Rust API.

**Current main**: records the v1.5.0 Rust 1.95 quality-ratchet publish receipt
and the post-release test-coverage readiness refresh at
`aa548e1579f659c089162f0dd8e445219d0a2414`; the latest main state referenced
by the current TestPyPI publishing attempt is
`54cde6b686f1904d76a980a03a1e104631ccce7b`.
The public Python `hl7v2` TestPyPI/PyPI lane remains separate and still needs
upload and install-back proof before any Python release claim. The latest
publishing-mode TestPyPI attempt is
[`docs/audits/python-testpypi-publish-attempt-2026-05-17.md`](audits/python-testpypi-publish-attempt-2026-05-17.md).
