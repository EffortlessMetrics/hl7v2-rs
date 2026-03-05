# HL7v2-rs Development Roadmap

**Current Status**: v1.2.0 (Production-ready release)
**Target**: v1.3.0 (Enterprise Features) / v2.0.0 (Cloud-native)
**Last Updated**: 2026-03-05

## Version Strategy

This roadmap balances stability with feature completeness. Each version has clear, achievable goals with backward compatibility guarantees.

---

## v1.2.0 (Current Release) - ✅ 100% COMPLETE

**Status**: 🚀 **STABLE - PRODUCTION READY**

### Key Features
- ✅ **Microcrate Architecture**: 28 specialized crates for minimal dependency trees.
- ✅ **MLLP Network Stack**: Async TCP/TLS client and server with high throughput.
- ✅ **Streaming Parser**: Event-based parsing for multi-gigabyte HL7 files.
- ✅ **HTTP/REST Server**: Production-ready API with Axum, OpenAPI, and Swagger UI.
- ✅ **Security**: API Key authentication and per-IP rate limiting.
- ✅ **Observability**: Prometheus metrics and structured JSON logging.
- ✅ **Validation Engine**: Conformance profiles (YAML) with inheritance support.
- ✅ **CLI**: Comprehensive tool for parsing, validation, and synthetic data generation.

---

## v1.3.0 (Enterprise Expansion - Next 3-6 months)

**Status**: 🚧 **PLANNING**

### Primary Goals
1. **gRPC Support**: Native high-performance protobuf interface.
2. **Language Bindings**: Official support for Python (PyO3) and Java (JNI).
3. **WASM Target**: Browser-based parsing and validation.
4. **Custom Field Rules**: Support for embedded scripts (Lua or Rhai) in profiles.

### Component Targets
- **`hl7v2-server`**: Add tonic-based gRPC endpoints and bi-directional streaming.
- **`hl7v2-prof`**: Implement dynamic rule evaluation using a safe script engine.
- **`hl7v2-wasm`**: Create a specialized target for web-based health apps.

---

## v2.0.0 (Cloud-Native & Compliance - Next 12 months)

**Status**: 🚧 **LONG-TERM VISION**

### Primary Goals
1. **HIPAA Readiness**: Built-in PII/PHI redaction and audit trails.
2. **Advanced Analytics**: Real-time message flow monitoring dashboard.
3. **Cloud Storage**: Native connectors for S3, GCS, and Azure Blob.
4. **Database Integration**: Direct persistence to PostgreSQL and Snowflake.

---

## Success Metrics (v1.2.0)
- **Performance**: Verified 100k+ messages/min on standard hardware.
- **Reliability**: 100% test pass rate across Unit, Integration, E2E, and Property-based suites.
- **Security**: Zero open vulnerabilities in `cargo audit`.

---

**Release v1.2.0 is officially ready for deployment.**
