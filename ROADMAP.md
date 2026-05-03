# HL7v2-rs Development Roadmap

**Current Status**: v1.3.0 (Enterprise Readiness)
**Target**: v1.4.0 (Cloud-native & Interop) / v2.0.0 (Global Scale)
**Last Updated**: 2026-05-03

## Version Strategy

This roadmap balances stability with feature completeness. Each version has clear, achievable goals with backward compatibility guarantees.

---

## Now/Next/Later
**Now** (Current Sprint/Immediate): 
- Finalize WASM target for browser-based processing
- Java (JNI) bindings development

**Next** (Upcoming): 
- Native Cloud Storage connectors (S3/GCS)
- Advanced analytics dashboard

**Later** (Future Consideration): 
- Real-time visualization REPL
- Automatic profile generation from sample messages

---

## v1.3.x (Current - Enterprise Readiness) - ✅ 80% COMPLETE

**Status**: 🚀 **CORE ENTERPRISE FEATURES**

### Key Features
- ✅ **Full gRPC Service**: Native high-performance protobuf interface (Tonic) for all core operations.
- ✅ **Message Lifecycle**: Enterprise-grade archival state machines and legal hold management.
- ✅ **Rust 2024 Migration**: Full adoption of modern Rust standards and 1.93 toolchain.
- ✅ **Statistical Guard**: Baseline learning for message flow anomaly detection.
- ✅ **Python Bindings**: Official support via PyO3 for high-performance Python integration.
- 🚧 **WASM Target**: Browser-based parsing and validation (In Progress).
- 🚧 **Java Bindings**: JNI-based integration (In Progress).

---

## v1.4.0 (Cloud & Interop - Next 3-6 months)

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