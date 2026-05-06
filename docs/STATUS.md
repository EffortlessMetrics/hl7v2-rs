# HL7v2-rs Implementation Status

This document provides a transparent view of which features are fully implemented, partially implemented, or planned.

> **Last Updated**: 2026-05-06
> **Project Status**: v1.2.0 current release; current `main` is tested, but full publish readiness is not yet proven.

## Core Components

| Crate | Status | Coverage | Notes |
|-------|--------|----------|-------|
| `hl7v2-core` | ✅ 100% | 92% | Canonical parser, writer, and MLLP framing. |
| `hl7v2-model` | ✅ 100% | 100% | Core AST and domain models. |
| `hl7v2-parser` | ✅ 100% | 95% | High-performance NOM-based parser. |
| `hl7v2-writer` | ✅ 100% | 94% | Optimized HL7 serialization. |
| `hl7v2-stream` | ✅ 100% | 88% | Event-based streaming parser for large messages. |
| `hl7v2-query` | ✅ 100% | 90% | Path-based field extraction (MSH.3.1). |
| `hl7v2-prof` | ✅ 100% | 85% | Conformance profile engine (Modularized). |
| `hl7v2-validation` | ✅ 100% | 82% | Rule-based message validation. |
| `hl7v2-gen` | ✅ 100% | 80% | Synthetic data and ACK generation. |
| `hl7v2-server` | ✅ 100% | 80% | HTTP REST API with metrics, auth, ACK, and normalization routes. |
| `hl7v2-cli` | ✅ 100% | 75% | Full-featured CLI with streaming support. |

## Feature Set (v1.2.0)

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

- ✅ **Main workflows**: CI, Coverage, Security, and API Contracts were green on merge commit `6de37e0`.
- ✅ **Publish order**: `cargo run -p xtask -- publish-plan` resolves 30 publishable crates.
- 🟡 **Dry-run publish**: Direct `cargo publish --dry-run --locked` is proven through `hl7v2-core`; it stops at `hl7v2` because `hl7v2-core` is not yet present on crates.io. See `docs/audits/publish-dry-run-2026-05-06.md`.

## Historical Plans
Old planning documents have been moved to `docs/plans/` for archival reference.

---

**Release v1.2.0 is tagged. Current main is tested; full crates.io publish readiness still needs the publish sequence or a local-registry simulation for higher-level crates.**
