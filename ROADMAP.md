# HL7v2-rs Development Roadmap

**Current Status**: v1.2.0 (Production-ready release)
**Target**: v1.3.0 (Enterprise Features) / v2.0.0 (Cloud-native)
**Last Updated**: 2026-05-01

## Version Strategy

This roadmap balances stability with feature completeness. Each version has clear, achievable goals with backward compatibility guarantees.

---

## Now/Next/Later
**Now** (Current Sprint/Immediate): 
- Complete gRPC service implementation
- Enhance ML models for anomaly detection

**Next** (Upcoming): 
- WASM target for browser-based processing
- Java (JNI) bindings

**Later** (Future Consideration): 
- Native Cloud Storage connectors (S3/GCS)
- Advanced analytics dashboard

---

## v1.2.x (Recent Improvements) - ✅ 100% COMPLETE

**Status**: 🚀 **ENHANCED STABILITY & ECOSYSTEM**

### Key Features
- ✅ **Python Bindings**: Official support via PyO3 for high-performance Python integration.
- ✅ **HIPAA Readiness**: Built-in PII/PHI redaction crate for HIPAA compliance.
- ✅ **Anomaly Detection**: Statistical/ML-based guardrails for real-time flow monitoring.
- ✅ **Enhanced Performance**: Optimized escaping fast-paths and stack-allocated streaming buffers.
- ✅ **Persistent Caching**: PostgreSQL-backed two-tier cache for conformance profiles.
- ✅ **gRPC Skeleton**: Initial architectural foundation for high-performance RPCs.

---

## v1.3.0 (Enterprise Expansion - Next 3 months)

**Status**: 🚧 **ACTIVE DEVELOPMENT**

### Primary Goals
1. **Full gRPC Service**: Native high-performance protobuf interface (Tonic).
2. **WASM Target**: Browser-based parsing and validation.
3. **Java Bindings**: JNI-based integration for Enterprise systems.
4. **Custom Field Rules**: Support for embedded scripts (Lua or Rhai) in profiles.

### Component Targets
- **`hl7v2-server`**: Complete tonic-based gRPC endpoints and bi-directional streaming.
- **`hl7v2-prof`**: Implement dynamic rule evaluation using a safe script engine.
- **`hl7v2-wasm`**: Create a specialized target for web-based health apps.

---

## v2.0.0 (Cloud-Native & Advanced Compliance - Next 6-12 months)

**Status**: 🚧 **LONG-TERM VISION**

### Primary Goals
1. **Advanced Analytics**: Real-time message flow monitoring dashboard with Grafana integration.
2. **Cloud Storage**: Native connectors for S3, GCS, and Azure Blob.
3. **Audit Trails**: Distributed ledger or secure audit logs for every message lifecycle event.
4. **Enterprise Connectors**: Direct persistence to Snowflake and Databricks.

---

## Success Metrics (v1.2.x)
- **Performance**: Verified 150k+ messages/min on standard hardware.
- **Reliability**: 100% test pass rate across Unit, Integration, E2E, and Property-based suites (120+ tests).
- **Security**: Zero known vulnerabilities in `cargo audit` (pyo3, rand, rustls-webpki updated).

---

## Release v1.2.0 with v1.2.x enhancements is officially ready for deployment.