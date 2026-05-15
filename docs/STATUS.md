# HL7v2-rs Implementation Status

This document provides a transparent view of which features are fully implemented, partially implemented, or planned.

> **Last Updated**: 2026-05-15
> **Project Status**: v1.4.0 is published to crates.io for the primary Rust product graph: `hl7v2`, `hl7v2-server`, and `hl7v2-cli`. Current `main` is prepared as the v1.5.0 Rust 1.95 quality-ratchet candidate; v1.5.0 is not published until explicit crates.io publish and tag receipts land.

## Core Components

| Crate | Status | Coverage | Notes |
|-------|--------|----------|-------|
| `hl7v2` | ✅ 100% | 92% | Canonical Rust library crate for parsing, writing, validation, transport framing, ACK, normalization, and generation. Foundation model, escape, and MLLP implementations now live here. |
| `hl7v2-server` | ✅ 100% | 80% | HTTP REST API with metrics, auth, ACK, normalization, redacted validation, configured-root bundle/replay, inline corpus evidence, readiness, quarantine, and redacted structured logs. |
| `hl7v2-cli` | ✅ 100% | 75% | Full-featured CLI with streaming support. |
| Python binding (`hl7v2` distribution) | 🟡 Experimental | Smoke | Public Python distribution built from the `hl7v2-python` PyO3 binding backend; not part of the primary Rust product graph and validated through the Python/maturin wheel smoke lane before any PyPI release. |
| retired old package names | ✅ Retired locally | N/A | Old microcrate package names, including `hl7v2-model`, `hl7v2-escape`, and `hl7v2-mllp`, are no longer local workspace crates. Some historical old-name `1.2.0` artifacts already exist on crates.io and should not be treated as the current product surface. |

## Published Feature Set (v1.4.0)

### 🚀 Connectivity
- ✅ **MLLP Over TCP**: Fully implemented async client and server.
- ✅ **TLS Support**: Secure framing using `rustls`.
- ✅ **HTTP REST API**: Axum-based JSON endpoints for parse, validate, ACK, and normalize.
- 🟡 **gRPC Service**: v1.4.0 unary RPCs have contract tests. Current `main` also serves gRPC with `hl7v2 serve --mode grpc`, implements `ParseStream` as one request message into one response message, `GenerateAck`, `Normalize`, `ProfileLint`, `ProfileExplain`, and `ProfileTest` for inline profile evidence, `ValidateRedacted` with opt-in v2 validation, redaction receipt, and configured quarantine output evidence, `CreateEvidenceBundle` for configured-root evidence bundle creation, `ReplayEvidenceBundle` for configured-root evidence replay, `CorpusSummarize` for inline corpus summary evidence, `CorpusFingerprint` for inline corpus fingerprint evidence, and `CorpusDiff` for inline before/after corpus diff evidence with opt-in v2 provenance.

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

- ✅ **Main workflows**: required CI success, Security, Python Wheels, and API Contracts are green on the v1.4.0 release head. Coverage is unchanged/skipped for this docs/package release lane. Extended tests and benchmark artifacts remain non-publish performance lanes.
- ✅ **Publish order**: `cargo run -p xtask -- publish-plan` defaults to the primary Rust product graph: `hl7v2`, `hl7v2-server`, and `hl7v2-cli`. Use `cargo run -p xtask -- publish-plan --surface bindings` to inspect binding backend crates separately.
- ✅ **Published Rust graph**: `hl7v2`, `hl7v2-server`, and `hl7v2-cli` v1.4.0 are published and visible in the crates.io index. See `docs/audits/publish-v1.4.0-2026-05-09.md`.
- ✅ **Dry-run publish**: Workspace-patched dry-run verification and dependency-ordered direct dry-runs were completed before upload. See `docs/audits/publish-dry-run-v1.4.0-2026-05-09.md`.
- 🟡 **v1.5.0 candidate**: Current `main` carries the Rust 1.95 MSRV/toolchain ratchet, tighter lint/no-panic/file-policy rails, advisory `ripr`, targeted mutation routing, profile evidence, Python profile helpers, RIPR evidence endpoints and calibration, gRPC profile lint/explain/test, configured-root evidence bundle creation/replay/quarantine, and inline corpus summary/fingerprint/diff parity, the cross-surface evidence parity spec, first-use docs, dirty real-world corpus proof, gRPC serve-mode status sync, the nightly property-test command repair, finite numeric validation, and release-readiness and dry-run receipts. The v1.5.0 readiness proof has been refreshed on `a12c6fdf61396de74f971c27f4887dd7c451543b` after #647. The selected crates.io graph is `hl7v2`, `hl7v2-python`, `hl7v2-server`, and `hl7v2-cli`, with `hl7v2-python` included only as a binding backend. It still needs crates.io publish and tag receipts before any release claim.
- 🟡 **Python binding lane**: `hl7v2-python` is publishable as a governed crates.io binding backend, but it is not part of the primary Rust product graph and has not been uploaded. The public Python distribution is `hl7v2`. The v1.4.0 binding is verified through a maturin wheel build/install/import smoke lane; current `main` also has a manual TestPyPI proof workflow before any production PyPI release. The non-publishing workflow mode passed on `main`; the 2026-05-10 TestPyPI upload attempt built and smoke-tested the wheel but failed with `invalid-publisher` because the TestPyPI Trusted Publisher is not configured. Production PyPI release has not been run. A 2026-05-13 package-state check found no visible `hl7v2` package on TestPyPI or production PyPI.
- ✅ **Binding-backend closeout**: #604 accepted the binding-backend ADR, #605 refreshed the yanked `metrics` lock entry that blocked security checks, #606 added `publish-plan --surface primary|bindings|all-publishable`, #607 framed `hl7v2-python` as the PyO3 backend for the public Python `hl7v2` package, and #608 fixed Python wheel cache behavior. This closeout did not publish `hl7v2-python`, TestPyPI, PyPI, or any v1.5.0 crates.io artifact.
- ✅ **Binding-backend readiness audit**: #610 added the binding-backend release-proof spec, #611 added the binding backend dry-run surface, #612 prepared `hl7v2-python` as publishable backend metadata, #613 defined the future npm/WASM package model, and #614 added a publish-surface classification guard. See [`docs/audits/binding-backend-readiness-2026-05-14.md`](audits/binding-backend-readiness-2026-05-14.md). This audit does not claim a crates.io backend upload, PyPI/TestPyPI upload, npm package, tag, GitHub release, or v1.5.0 publish.
- ⚠️ **Registry history**: crates.io already contains historical `1.2.0` artifacts for several old microcrate names. The current release plan does not publish those names again unless a deliberate deprecation-only compatibility release is chosen.
- ✅ **Tag alignment policy**: the existing `v1.2.0` tag points at an older commit and remains historical. Fresh `v1.2.1`, `v1.3.0`, and `v1.4.0` tags point at their release heads.

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
extension crate backing the Python `hl7v2` package and is publishable as binding
infrastructure only. #610-#614 added the binding-backend release-proof spec,
dry-run surface, publishable metadata, npm/WASM package model, and publish
surface guard. It still needs refreshed release readiness, language install or
import smoke receipts, registry resolution proof, and an explicit release
decision before any crates.io upload claim.
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
nightly property-test command repair, finite numeric validation, and the v1.5.0
release-readiness workflow.

For navigation across current docs, historical receipts, and evidence workflow
guides, start with [the documentation index](README.md). For the current
final source-tree gap audit after the local workbench split, see
[`docs/audits/current-source-tree-evidence-objective-gap-audit.md`](audits/current-source-tree-evidence-objective-gap-audit.md).

| Area | Status | Notes |
|------|--------|-------|
| First-run diagnostics | ✅ Stable | `hl7v2 doctor` verifies CLI version, sample parse, profile loading, JSON output, optional server reachability, and optional Python binding presence. |
| Typed validation evidence | ✅ Stable | `ValidationReport` is shared by library, CLI, server validation, and Python bindings. |
| Profiles as code | ✅ Stable | `profile lint`, `profile test`, and `profile explain` produce machine-readable profile evidence. |
| Corpus observability | ✅ Stable | `corpus summarize`, `corpus fingerprint`, and `corpus diff` produce feed-level evidence for regression and migration review. Current proof includes dirty-corpus coverage for Z-segments, odd MSH fields, MLLP bytes, large OBX messages, malformed delimiters, and safe parse-error output. |
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
Release graph decision: [`docs/audits/v1.5.0-release-graph-decision-2026-05-14.md`](audits/v1.5.0-release-graph-decision-2026-05-14.md).
RIPR calibration: [`docs/audits/ripr-calibration-2026-05-15.md`](audits/ripr-calibration-2026-05-15.md).

- 🟡 **Release candidate**: workspace package versions are prepared as `1.5.0` for the primary Rust product graph.
- ✅ **Rust floor**: MSRV is Rust 1.95 and `rust-toolchain.toml` pins Rust 1.95.0 with `rustfmt` and `clippy`.
- ✅ **Verification rails**: lint policy, Clippy exceptions, no-panic exact identity and no-new-debt baseline, file-policy companion ledgers, advisory `ripr`, and targeted mutation routing are present.
- ✅ **Release readiness workflow**: `.github/workflows/release-readiness.yml` records the non-publishing readiness proof bundle.
- ✅ **Dry-run receipt**: hosted release-readiness dry-run passed on `main` at `b0bb5b5392354273946f36f797f39d741d318fc1`; current-main primary and binding surface dry-runs passed locally at `b4b7962e6f3f9d7ae5d91adf603e6328e3d13297` after #616-#618, at `cc1e3046e2496ea0c10a25239b9d077641d01c36` after #621-#622, at `425ff79b959ef5ceff1bdd072cbe074ff2a7ab04` after #624-#625, at `eb518e948d57bc07e96512ced306fe0db2dc2990` after #627, at `acfeacec0eda6d632d52e61440f2c85fda93d95f` after #629-#630, at `b0018c8760f07d738b3a6b7a11eeeda300786ef2` after #632-#633, at `fe8680c551bc3ce9248063906cbbe90ffe732a70` after #635-#636, at `483fb572a42006c07bd2e857fb7638ec91615b7d` after #638, at `915578b5cab5ab3abde7f806e5ad39c063d9cc82` after #640, at `47ba93201aaa15c95157d341bb5ec4f5f3871741` after #642, at `addbbe3e6962dc7f1629053c8a9c6810c7e37cc1` after #645, and at `a12c6fdf61396de74f971c27f4887dd7c451543b` after #647.
- ✅ **Release graph decision**: v1.5.0 selects `hl7v2`, `hl7v2-python`, `hl7v2-server`, and `hl7v2-cli`, with `hl7v2-python` included only as binding backend infrastructure.
- 🟡 **Publish receipt**: crates.io upload has not been run for v1.5.0.
- 🟡 **Python proof**: the public Python distribution is `hl7v2`, remains separate from the primary Rust product graph, and still requires Trusted Publisher upload/install-back proof before any TestPyPI or PyPI success claim.

## Historical Plans
Old planning documents have been moved to `docs/plans/` for archival reference.

---

**Current published release**: v1.4.0 is tested, package-verified, tagged, and published to crates.io for the primary Rust product graph.

**Current main**: is prepared as the v1.5.0 Rust 1.95 quality-ratchet
candidate. v1.4.0 remains the current published crates.io release until v1.5.0
publish and tag receipts land.
