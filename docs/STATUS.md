# HL7v2-rs Implementation Status

This document provides a transparent view of which features are fully implemented, partially implemented, or planned.

> **Last Updated**: 2026-05-08
> **Project Status**: v1.2.1 package line; current `main` is tested and package-verified, but the final Rust crates.io publish sequence has not been executed.

## Core Components

| Crate | Status | Coverage | Notes |
|-------|--------|----------|-------|
| `hl7v2` | ✅ 100% | 92% | Canonical Rust library crate for parsing, writing, validation, transport framing, ACK, normalization, and generation. Foundation model, escape, and MLLP implementations now live here. |
| `hl7v2-server` | ✅ 100% | 80% | HTTP REST API with metrics, auth, ACK, and normalization routes. |
| `hl7v2-cli` | ✅ 100% | 75% | Full-featured CLI with streaming support. |
| `hl7v2-python` | 🟡 Experimental | N/A | PyO3 binding package held out of the crates.io Rust publish graph; validate through the Python/maturin lane before release. |
| compatibility shims | ✅ Package-frozen | N/A | In the current workspace, old microcrate package names, including `hl7v2-model`, `hl7v2-escape`, and `hl7v2-mllp`, are private deprecated shims unless explicitly retained for compatibility. Some historical old-name `1.2.0` artifacts already exist on crates.io and should not be treated as the current product surface. |

## Feature Set (v1.2.1)

### 🚀 Connectivity
- ✅ **MLLP Over TCP**: Fully implemented async client and server.
- ✅ **TLS Support**: Secure framing using `rustls`.
- ✅ **HTTP REST API**: Axum-based JSON endpoints for parse, validate, ACK, and normalize.
- 🟡 **gRPC Service**: Unary RPCs have contract tests; `ParseStream` is explicitly unsupported.

### 🛡️ Security & Observability
- ✅ **API Authentication**: Constant-time API Key validation.
- ✅ **Rate Limiting**: Per-IP throttling to prevent DoS.
- ✅ **Prometheus Metrics**: Throughput, latency, and error tracking.
- ✅ **Audit Ready**: Structured JSON logging.

### 🧪 Quality Assurance
- ✅ **BDD Tests**: Real validation scenarios verified with Cucumber.
- ✅ **E2E Tests**: Subprocess CLI and network integration tests.
- ✅ **Property Testing**: Robust parsing and escaping edge-case coverage.
- ✅ **Security Workflow**: Dependency audit, cargo-deny, Semgrep, Trivy, and secret scanning are green on current `main`.

## Release and Publish Readiness

- ✅ **Main workflows**: CI, Coverage, Security, Extended, Benchmarks, and API Contracts are green on the 2026-05-07 release-readiness head after manual API Contracts and Coverage dispatches.
- ✅ **Publish order**: `cargo run -p xtask -- publish-plan` resolves the final Rust package graph: `hl7v2`, `hl7v2-server`, and `hl7v2-cli`.
- ✅ **Dry-run publish**: Workspace-patched dry-run verification proves the current Rust package graph while the dependency chain is still unpublished. Direct crates.io dry-run passes for `hl7v2`; dependent crates must be dry-run again after `hl7v2` is published and available in the crates.io index. See `docs/audits/publish-dry-run-2026-05-08.md`.
- 🟡 **Python binding lane**: `hl7v2-python` is `publish = false` for crates.io and should be verified with Python packaging tooling before PyPI or wheel release.
- ⚠️ **Registry history**: crates.io already contains historical `1.2.0` artifacts for several old microcrate names. The current release plan does not publish those names again unless a deliberate deprecation-only compatibility release is chosen.
- ✅ **Tag alignment policy**: the existing `v1.2.0` tag points at an older commit and remains historical. The current release line moves forward as `v1.2.1`; create a fresh `v1.2.1` tag on the release head after final dry-runs and before upload.

## Historical Plans
Old planning documents have been moved to `docs/plans/` for archival reference.

---

**Current code is tested and package-verified; publishing the final Rust package graph must still run dependency-ordered final dry-runs, create the fresh `v1.2.1` release tag, and wait for each published dependency to appear in the crates.io index before publishing dependents.**
