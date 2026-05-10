# HL7v2-rs Implementation Status

This document provides a transparent view of which features are fully implemented, partially implemented, or planned.

> **Last Updated**: 2026-05-09
> **Project Status**: v1.4.0 is published to crates.io for the final Rust package graph: `hl7v2`, `hl7v2-server`, and `hl7v2-cli`.

## Core Components

| Crate | Status | Coverage | Notes |
|-------|--------|----------|-------|
| `hl7v2` | ✅ 100% | 92% | Canonical Rust library crate for parsing, writing, validation, transport framing, ACK, normalization, and generation. Foundation model, escape, and MLLP implementations now live here. |
| `hl7v2-server` | ✅ 100% | 80% | HTTP REST API with metrics, auth, ACK, normalization, redacted validation, configured-root bundle/replay, inline corpus evidence, readiness, quarantine, and redacted structured logs. |
| `hl7v2-cli` | ✅ 100% | 75% | Full-featured CLI with streaming support. |
| `hl7v2-python` | 🟡 Experimental | Smoke | PyO3 binding package held out of the crates.io Rust publish graph; validated through the Python/maturin wheel smoke lane before any PyPI release. |
| retired old package names | ✅ Retired locally | N/A | Old microcrate package names, including `hl7v2-model`, `hl7v2-escape`, and `hl7v2-mllp`, are no longer local workspace crates. Some historical old-name `1.2.0` artifacts already exist on crates.io and should not be treated as the current product surface. |

## Published Feature Set (v1.4.0)

### 🚀 Connectivity
- ✅ **MLLP Over TCP**: Fully implemented async client and server.
- ✅ **TLS Support**: Secure framing using `rustls`.
- ✅ **HTTP REST API**: Axum-based JSON endpoints for parse, validate, ACK, and normalize.
- 🟡 **gRPC Service**: v1.4.0 unary RPCs have contract tests. Current `main` also implements `ParseStream` as one request message into one response message and `ValidateRedacted` with opt-in v2 validation and redaction receipt evidence.

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
- ✅ **Publish order**: `cargo run -p xtask -- publish-plan` resolves the final Rust package graph: `hl7v2`, `hl7v2-server`, and `hl7v2-cli`.
- ✅ **Published Rust graph**: `hl7v2`, `hl7v2-server`, and `hl7v2-cli` v1.4.0 are published and visible in the crates.io index. See `docs/audits/publish-v1.4.0-2026-05-09.md`.
- ✅ **Dry-run publish**: Workspace-patched dry-run verification and dependency-ordered direct dry-runs were completed before upload. See `docs/audits/publish-dry-run-v1.4.0-2026-05-09.md`.
- 🟡 **Python binding lane**: `hl7v2-python` is `publish = false` for crates.io. The v1.4.0 binding is verified through a maturin wheel build/install/import smoke lane; current `main` also has a manual TestPyPI proof workflow before any production PyPI release. The non-publishing workflow mode passed on `main`; the TestPyPI upload/install-back mode has not been run.
- ⚠️ **Registry history**: crates.io already contains historical `1.2.0` artifacts for several old microcrate names. The current release plan does not publish those names again unless a deliberate deprecation-only compatibility release is chosen.
- ✅ **Tag alignment policy**: the existing `v1.2.0` tag points at an older commit and remains historical. Fresh `v1.2.1`, `v1.3.0`, and `v1.4.0` tags point at their release heads.

## Evidence Contracts Release And Current Main

v1.4.0 is the Evidence Contracts and Server Sidecar release line around
deterministic HL7 interface evidence. It is tagged, released on GitHub, and
uploaded to crates.io for the final Rust package graph.

Current `main` contains opt-in v2 provenance producers, maintained schema
validation through `xtask evidence-schema-check`, server replay and inline
corpus endpoints, redacted structured evidence logs, Docker sidecar smoke
coverage, broader PHI sentinel tests, Python/TestPyPI proof, and the server
bundle replay message-type fix.

| Area | Status | Notes |
|------|--------|-------|
| First-run diagnostics | ✅ Stable | `hl7v2 doctor` verifies CLI version, sample parse, profile loading, JSON output, optional server reachability, and optional Python binding presence. |
| Typed validation evidence | ✅ Stable | `ValidationReport` is shared by library, CLI, server validation, and Python bindings. |
| Profiles as code | ✅ Stable | `profile lint`, `profile test`, and `profile explain` produce machine-readable profile evidence. |
| Corpus observability | ✅ Stable | `corpus summarize`, `corpus fingerprint`, and `corpus diff` produce feed-level evidence for regression and migration review. |
| Safe support packets | ✅ Stable | `redact`, `bundle`, and `replay` produce redacted evidence bundles with manifest checks and replay verification. |
| Evidence contracts | ✅ Stable | v1.4.0 ships opt-in v2 provenance schemas/producers and an `xtask evidence-schema-check` gate. |
| CLI automation contract | ✅ Stable | Evidence commands use stable exit codes, primary stdout, diagnostic stderr, and output-file/quiet/no-color flags. |
| Server edge guard | ✅ Stable | v1.4.0 ships `/hl7/replay`, inline-message corpus endpoints that do not read request filesystem paths, bundle artifact schema opt-in, redacted structured evidence logs with hashed message-control and bundle identifiers, evidence metrics, Docker smoke coverage, and the bundle replay message-type fix. |
| Python evidence lane | 🟡 Separate lane | Python wheel proof and minimum API parity cover parse, JSON export, normalize, validation, corpus, redaction, bundle, and replay. v1.4.0 adds v2 parity, PHI sentinel coverage, Python evidence docs, and a manual TestPyPI proof workflow. Python remains outside the crates.io Rust publish graph. |

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

- ✅ **Publish plan**: `cargo run -p xtask -- publish-plan` resolves `hl7v2`, `hl7v2-server`, and `hl7v2-cli`.
- ✅ **Evidence schemas**: `cargo run -p xtask -- evidence-schema-check` passes on the v1.4.0 package line.
- ✅ **API contracts**: local OpenAPI lint, proto lint, and packaged proto/OpenAPI drift tests pass on the v1.4.0 package line.
- ✅ **Full gate**: `cargo run -p xtask -- gate --check` passes on the v1.4.0 package line.
- ✅ **Dry-runs**: direct `hl7v2` dry-run and workspace-patched full-graph dry-run pass. Direct dependent dry-runs correctly wait for `hl7v2` v1.4.0 to exist in the crates.io index during the real publish sequence.
- ✅ **Python proof**: the maturin wheel build/install/import smoke proof passes for the v1.4.0 Python lane package without publishing `hl7v2-python` to crates.io.
- ✅ **Release notes and tag**: `v1.4.0` is tagged and the GitHub release is published.

## Historical Plans
Old planning documents have been moved to `docs/plans/` for archival reference.

---

**Current published release**: v1.4.0 is tested, package-verified, tagged, and published to crates.io for the final Rust package graph.

**Current main**: tracks the v1.4.0 Evidence Contracts and Server Sidecar
release line.
