# HL7v2-rs Development Roadmap

**Current Status**: v1.1.0 (Core features stable)
**Target**: v2.0.0 (Enterprise-ready)
**Last Updated**: 2025-11-13

## Version Strategy

This roadmap balances stability with feature completeness. Each version has clear, achievable goals with backward compatibility guarantees.

---

## v1.1.x (Current - Maintenance Branch)

**Status**: ✅ **STABLE - Bug fixes and documentation only**

### What's Included
- ✅ Event-based streaming parser
- ✅ Full profile validation with inheritance
- ✅ Message generation with realistic data
- ✅ CLI for parse/validate/generate/ack/normalize
- ✅ MLLP protocol support
- ✅ JSON serialization
- ✅ Batch processing

### Focus Areas
- 🐛 Bug fixes and stability improvements
- 📚 Documentation enhancements
- ✅ Test coverage expansion
- 🚀 Performance tuning

### Release Cadence
- Regular patch releases for bug fixes
- Minor version bumps (1.1.x) for documentation/tooling improvements
- No new features in v1.1.x line

### Exit Criteria
- No critical bugs
- 95%+ test coverage
- Production-ready documentation

---

## v1.2.0 (Next Major Release - 4-6 months)

**Status**: 🔄 **IN PROGRESS** (~30% complete)

### Primary Goals
1. Complete streaming parser improvements
2. Add server mode (HTTP/gRPC)
3. Enhance CLI with missing features
4. Improve expression engine

### Feature Breakdown by Component

#### hl7v2-core (Estimated: 6-8 weeks)

**Streaming Parser Completion** (Priority: HIGH)
- ✅ Event-based parsing (DONE)
- 🚧 Backpressure with bounded channels
  - Add `BoundedChannel<Event>` with configurable capacity
  - Implement queue overflow handling
  - Add `--queue-capacity N` CLI flag
- 🚧 Memory bounds enforcement
  - Track cumulative buffer memory
  - Add RSS monitoring test
  - Implement memory pressure signals
- 🚧 Resume parsing across boundaries
  - Track parser state between chunks
  - Support `resume_from(byte_offset)` API
  - Add incremental chunk tests
- ⚠️ Enhanced escape sequence support
  - Highlight escapes: `\H\...\N\`
  - Binary escapes: hex/base64 for non-text fields

**Network Module Implementation** (Priority: CRITICAL)
- ❌ MLLP TCP Server
  - Async TCP listener with Tokio
  - Frame reading/writing
  - Timeout handling (default 30s)
  - Connection pooling
- ❌ TLS Support
  - rustls configuration
  - mTLS with certificate validation
  - Key/cert loading from files
- ❌ ACK Semantics
  - Send AA/AE based on validation
  - Implement `--ack-after=validate|persist` hook
  - Timing policy enforcement

**Effort Estimate**: 40-50 story points | 6-8 weeks | 1-2 engineers

#### hl7v2-prof (Estimated: 4-5 weeks)

**Remote Profile Loading** (Priority: HIGH)
- ❌ HTTP/HTTPS fetching
  - Use `reqwest` with timeout
  - Implement ETag/If-None-Match caching
  - Cache metadata (etag, last-modified)
- ❌ S3 Profile Loading
  - AWS SDK integration
  - Bucket/key resolution
  - SSE-KMS support
- ❌ GCS Profile Loading
  - Google Cloud Storage SDK
  - Service account authentication
- ❌ Local File Caching
  - LRU cache with size limits (default 100MB)
  - Cache directory configuration
  - Manifest for cache metadata

**Expression Engine Hardening** (Priority: MEDIUM)
- 🔄 Expression Compilation
  - Pre-compile regex patterns
  - Validate expressions at profile load time
  - Cache compiled forms
- 🔄 Time-Bound Evaluation
  - Set execution timeout (default 100ms per expression)
  - Prevent infinite loops
  - Add `--expression-timeout MS` flag
- 🔄 Guardrails & Validation
  - Whitelist allowed expression patterns
  - Prevent code injection risks
  - Document safe patterns

**Validation Enhancements** (Priority: LOW)
- ⚠️ Cycle Detection in Profiles
  - Detect circular parent references
  - Report cycle chain in error
  - Add validation in `load_profile_with_inheritance()`

**Effort Estimate**: 25-30 story points | 4-5 weeks | 1 engineer

#### hl7v2-gen (Estimated: 3-4 weeks)

**Corpus Manifest** (Priority: MEDIUM)
- ❌ Manifest Generation
  - Generate `manifest.json` with:
    - Tool version (from Cargo.toml)
    - Generation seed used
    - Template files and SHA-256
    - Profile files and SHA-256
    - Message counts by type
    - Per-file SHA-256 hashes
    - Generation timestamp
- ❌ Manifest Verification
  - `gen --verify-manifest` command
  - Recompute all hashes
  - Detect tampering/changes
  - Report discrepancies

**Statistical Distribution Modeling** (Priority: LOW - v1.3)
- ⚠️ Correlated Distributions
  - Latent variable support
  - Shared random variables between fields
  - Example: BMI → height, weight correlation
- ⚠️ Markov Chains
  - Segment repetition patterns
  - State-based generation
  - Training from real data

**Effort Estimate**: 15-20 story points | 3-4 weeks | 0.5-1 engineer

#### hl7v2-cli (Estimated: 3-4 weeks)

**Server Mode** (Priority: CRITICAL - Depends on network.rs)
- ❌ HTTP Server
  - Axum/Hyper framework
  - Endpoint: `POST /hl7/parse`
  - Endpoint: `POST /hl7/validate`
  - Endpoint: `POST /hl7/ack`
  - Streaming request/response support
  - NDJSON output for multiple messages
- ❌ gRPC Server
  - Tonic framework
  - Define `.proto` files for messages
  - Bi-directional streaming
  - Equivalent endpoints to HTTP
- ❌ Authentication
  - Bearer token validation middleware
  - OIDC integration hooks
  - RBAC support
- ❌ Observability
  - OpenTelemetry metrics
  - Request logging with structured data
  - PHI redaction in logs
  - Health/readiness endpoints

**CLI Enhancements** (Priority: MEDIUM)
- ⚠️ Missing Flags
  - `--report` for validation → JSON file
  - `--canonical-delims` implementation
  - `--envelope` for parse output
- ⚠️ Configuration Files
  - TOML config file support (`hl7v2.toml`)
  - Environment variable overrides
  - Default paths and values
- ⚠️ Batch Operations
  - Process multiple files
  - Parallel processing with thread pool
  - Progress reporting

**Effort Estimate**: 35-45 story points | 5-6 weeks (depends on network.rs) | 1-2 engineers

### v1.2.0 Timeline

```
Week 1-2:    Core Network Module + Streaming Backpressure
Week 3-4:    Remote Profile Loading + TLS
Week 5-6:    Server Mode (HTTP)
Week 7:      Server Mode (gRPC) + Auth
Week 8:      Testing, Documentation, Buffer
```

**Total Effort**: ~120 story points | 8 weeks | 3-4 engineers

### v1.2.0 Exit Criteria
- ✅ Server mode operational (HTTP + gRPC)
- ✅ All CLI flags working as documented
- ✅ Remote profile loading with caching
- ✅ Backpressure and memory bounds enforced
- ✅ 90%+ test coverage
- ✅ Zero critical bugs
- ✅ Performance targets met (≥100k msgs/min)

---

## v1.3.0 (Feature Expansion - 3-4 months after v1.2)

**Status**: 🚧 **PLANNED**

### Primary Goals
1. Language bindings for interoperability
2. Integration tools for common platforms
3. Enhanced analytics and monitoring
4. Performance optimization

### Feature Breakdown

#### Language Bindings (Priority: HIGH)

**C FFI Bindings** (2-3 weeks | 1 engineer)
- ✅ Stable `extern "C"` API
- ✅ Error codes and out-parameters
- ✅ Memory management (`hl7v2_free`)
- ✅ UTF-8 string handling
- ✅ Versioned symbols (`hl7v2_rs_v1_*`)

**Python Bindings (PyO3)** (2-3 weeks | 1 engineer)
- ✅ Wheels for Linux/macOS/Windows
- ✅ manylinux2014 + musllinux targets
- ✅ Python 3.8+ support with `abi3`
- ✅ GIL release for blocking ops
- ✅ Pythonic iterators and generators

**JavaScript/WASM** (2-3 weeks | 1 engineer)
- ✅ `wasm32-unknown-unknown` target
- ✅ Node.js and browser support
- ✅ No FS/Network by default
- ✅ 4 MiB message size limit
- ✅ npm package distribution

**Java Bindings (JNI)** (2-3 weeks | 1 engineer)
- ✅ Shaded JAR with natives per OS
- ✅ UTF-16/UTF-8 conversion
- ✅ Zero JNI local ref leaks
- ✅ Maven/Gradle integration

**Effort**: ~40 story points | 8-10 weeks | 2-3 engineers (parallel)

#### Integration Tools (Priority: MEDIUM)

**Database Integration** (2-3 weeks)
- PostgreSQL connector
- Snowflake connector
- Prepared statements with idempotency keys
- Batch insert optimizations

**Message Queue Integration** (2-3 weeks)
- Kafka producer/consumer
- RabbitMQ integration
- Partition key strategies
- Offset management

**Cloud Storage** (2-3 weeks)
- S3 upload/download
- GCS integration
- Azure Blob Storage
- Multi-part uploads with resumption

**Effort**: ~30 story points | 6-8 weeks | 1-2 engineers

#### Analytics & Observability (Priority: MEDIUM)

**Metrics** (1-2 weeks)
- OpenTelemetry counters/histograms/gauges
- Prometheus export format
- Custom dashboard JSON (Grafana)

**Advanced Analytics** (2-3 weeks)
- Message flow analysis
- Error rate tracking
- Performance analytics
- Compliance reporting

**Effort**: ~20 story points | 3-5 weeks | 1 engineer

### v1.3.0 Timeline
- Weeks 1-4: Python, JavaScript, Java bindings (parallel)
- Weeks 5-6: Database integration
- Weeks 7-8: Message queue integration
- Weeks 9-10: Cloud storage + analytics
- Week 11-12: Testing and documentation

**Total Effort**: ~90 story points | 12 weeks | 4-5 engineers

---

## v2.0.0 (Enterprise Features - 6+ months after v1.3)

**Status**: 🚧 **PLANNED**

### Primary Goals
1. Security & compliance (HIPAA)
2. Advanced deployment features
3. Enterprise analytics
4. GUI interface

### Feature Breakdown

#### Security & Compliance (Priority: CRITICAL)

**TLS/mTLS** (2 weeks)
- Enforce TLS 1.2+ for all network
- Certificate validation
- Certificate rotation support
- Key management integration

**Encryption** (2-3 weeks)
- At-rest encryption (AES-GCM)
- KMS integration (AWS/GCP/Azure)
- Envelope encryption for keys
- Secure key rotation

**Audit Logging** (2-3 weeks)
- Append-only audit logs
- Hash chain integrity (each entry includes prev_hash)
- Tamper detection
- S3 Object Lock / GCS Bucket Lock integration

**RBAC** (2 weeks)
- Role definitions
- Policy-based access control
- OIDC claims mapping
- Least privilege enforcement

**HIPAA Compliance** (2 weeks)
- BAA documentation
- Privacy & security controls
- Audit log requirements
- Access controls

**Effort**: ~40 story points | 10-12 weeks | 2 engineers

#### Deployment & Scaling (Priority: MEDIUM)

**Clustering** (2 weeks)
- Stateless server design
- Service discovery
- Health probes
- Load balancing guidance

**Horizontal Scaling** (2 weeks)
- Idempotent operations
- Distributed state management
- Cache consistency

**High Availability** (2 weeks)
- Zero-downtime deployments
- Rolling restart strategy
- Graceful shutdown (drain connections)
- Failover handling

**Effort**: ~20 story points | 6 weeks | 1 engineer

#### Advanced Analytics (Priority: LOW)

**Performance Analytics** (2 weeks)
- Percentile latency tracking
- Throughput monitoring
- Memory profiling
- Resource utilization

**Compliance Analytics** (2 weeks)
- Validation failure rates
- Error code distribution
- Message type analytics
- Segment/field coverage

**Predictive Analytics** (2 weeks)
- Trend modeling
- Anomaly detection
- Capacity planning

**Effort**: ~20 story points | 6 weeks | 1 engineer

#### GUI Interface (Priority: LOW)

**Tauri Desktop App** (4-6 weeks)
- Parse tree visualization
- Interactive validation
- Error highlighting
- Local profile management
- Corpus browser

**Web Dashboard** (4-6 weeks)
- Real-time metrics
- Message browser
- Audit log viewer
- Configuration UI
- User management

**Effort**: ~30 story points | 8-10 weeks | 2 engineers (optional, lower priority)

### v2.0.0 Timeline
- Months 1-2: Security & compliance (TLS, encryption, audit)
- Months 3: RBAC and HIPAA
- Months 4: Clustering & HA
- Months 5: Advanced analytics
- Month 6: GUI (if resources available)

**Total Effort**: ~110-130 story points | 24+ weeks | 4-5 engineers

---

## Critical Path & Dependencies

### Build Order (Dependency Graph)

```
v1.2.0:
  Network Module (blocking for server mode)
    ↓
  Server Mode HTTP/gRPC (depends on network)
    ↓
  CLI Integration Tests

  Parallel:
    - Remote Profile Loading
    - Backpressure & Memory Bounds
    - Corpus Manifest
```

### Key Dependencies for Future Versions

**v1.3 Blockers**:
- ✅ v1.2 completion (no blockers)
- Language bindings can start in parallel with v1.2 finale

**v2.0 Blockers**:
- ✅ v1.3 features (bindings used for testing)
- Security infrastructure needed for enterprise features

---

## Priority Matrix

### Must Have (Blocking for Release)
1. **Network module** - Required for server mode (v1.2)
2. **Server HTTP/gRPC** - Core v1.2 feature
3. **Profile remote loading** - Core v1.2 feature
4. **Security features** - Required for v2.0

### Should Have (Release Quality)
1. **Corpus manifest** - Data reproducibility
2. **CLI enhancements** - UX improvements
3. **Language bindings** - Interoperability (v1.3)
4. **HA/Clustering** - Production deployment (v2.0)

### Nice to Have (Enhancement)
1. **Analytics dashboard** - Observability
2. **Advanced distributions** - Test data quality
3. **GUI interface** - Better UX
4. **Performance tuning** - Beyond minimum targets

---

## Resource Allocation Recommendations

### Team Size & Roles

**Minimum (v1.2 only)**: 3-4 engineers
- 1 x Core/Network specialist
- 1 x Profile/Validation specialist
- 1 x CLI/Integration specialist
- 0.5 x QA/Testing

**Recommended (Full pipeline)**: 5-6 engineers
- 1 x Core/Network (lead)
- 1 x Profile/Validation
- 1 x CLI/Server (lead)
- 1 x Bindings/Integration
- 1 x QA/Performance
- 0.5 x DevOps/Deployment

### Time Commitment

| Version | Timeline | Engineers | Start | Release |
|---------|----------|-----------|-------|---------|
| v1.1.x  | Ongoing  | 0.5-1     | Now   | Stable  |
| v1.2.0  | 8 weeks  | 3-4       | Week 1| Month 3 |
| v1.3.0  | 12 weeks | 4-5       | Month 2 | Month 6 |
| v2.0.0  | 24 weeks | 4-5       | Month 5 | Month 12 |

---

## Quality Gates Per Release

### All Releases Must Pass
- ✅ 90%+ test coverage
- ✅ Zero critical/high severity bugs
- ✅ Performance targets met
- ✅ Security audit (if public release)
- ✅ Documentation complete

### v1.2.0 Specific
- ✅ Server mode stress tested (1000+ concurrent)
- ✅ Network resilience tested (failures, timeouts, drops)
- ✅ Profile loading with 10k+ remote profiles
- ✅ Memory bounds enforced under load
- ✅ Backward compatibility maintained

### v2.0.0 Specific
- ✅ Security audit by third party
- ✅ HIPAA compliance validation
- ✅ HA failover testing
- ✅ Performance under regional deployment

---

## Success Metrics

### v1.2.0 Success
- Server mode handles 10k+ messages/sec sustained
- Remote profile loading reduces latency by 50%
- Zero lost messages under backpressure
- 99.9% uptime in test environment

### v1.3.0 Success
- Language bindings have >100 GitHub stars
- Bindings cover 80%+ of core API
- Integration samples run without modification
- Adoption by 3+ external projects

### v2.0.0 Success
- HIPAA compliance certification
- Enterprise security audit passes
- HA deployment supports zero-downtime updates
- Analytics dashboard used in production

---

## Known Risks & Mitigation

### Risk: Network Module Complexity
**Impact**: Could delay v1.2 by 4+ weeks
**Mitigation**:
- Start with simple TCP + MLLP only
- Add HTTP/gRPC incrementally
- Consider using existing server framework (Axum already used elsewhere)

### Risk: Performance Degradation with Features
**Impact**: May miss 100k msgs/min target
**Mitigation**:
- Benchmark early and often
- Profile every major feature
- Consider feature flags for non-critical paths

### Risk: Security Vulnerability in Bindings
**Impact**: Could require emergency patch
**Mitigation**:
- Comprehensive FFI testing
- Bound checks on all boundaries
- Security audit before release

### Risk: Breaking Changes in Dependencies
**Impact**: Could require major version bump
**Mitigation**:
- Pin major versions of key deps (Tokio, Axum)
- Regular dependency audits
- Maintain 2-3 version back compatibility

---

## Next Steps (Immediate Actions)

### Week 1
- [ ] Assign network module owner (highest priority)
- [ ] Create GitHub issues for all v1.2 features
- [ ] Set up feature branches per component
- [ ] Create backlog and sprint planning

### Week 2-3
- [ ] Network module kickoff (design review)
- [ ] Profile remote loading API design
- [ ] Server mode architecture design
- [ ] Sprint planning for first 2 weeks

### Month 1
- [ ] Network module 50% complete
- [ ] Remote profile loading started
- [ ] Server mode framework chosen
- [ ] First integration tests running

### Month 2
- [ ] Network module complete
- [ ] Server mode 80% complete
- [ ] Profile loading with caching working
- [ ] Stress testing begins

### Month 3
- [ ] v1.2.0 Release Candidate
- [ ] Security audit begins
- [ ] Documentation complete
- [ ] v1.2.0 Release

---

## Backward Compatibility Policy

### Guarantees
- ✅ Library API: Semantic versioning (no breaking changes in minor versions)
- ✅ CLI: Flags stable after v1.2 (deprecation warnings for removals)
- ✅ File Formats: Profile/template YAML backward compatible
- ✅ Network: MLLP protocol unchanged

### Migration Path
- Deprecated features: Available for 2+ versions with warnings
- Major version bumps: Only for unavoidable API changes
- Long-term support: Commit to maintaining v1.x for 3+ years

---

## Appendix: Estimation Notes

**Story Point Definitions**:
- 1 point: 1-2 hours (simple fix, documentation)
- 2-3 points: 1 day (isolated feature)
- 5-8 points: 2-3 days (feature + tests)
- 13-21 points: 1-2 weeks (complex feature, integration)
- 34+ points: Multiple weeks (epic, coordination needed)

**Velocity Assumptions**:
- Single engineer: 20-25 points/week
- Team of 4: 80-100 points/week (with coordination overhead)
- Add 20-30% buffer for unknowns

---

## Document Control

**Last Updated**: 2025-11-13
**Author**: HL7v2-rs Team
**Review Cycle**: Quarterly (or when major changes)
**Next Review**: 2026-02-13

---

## How to Use This Roadmap

1. **For Sprint Planning**: Use feature breakdown to create 2-week sprint goals
2. **For Resource Planning**: Reference effort estimates and team size
3. **For Stakeholders**: Show release timeline and dependencies
4. **For Contributors**: Link to specific issues in GitHub
5. **For Prioritization**: Use MUST/SHOULD/NICE framework

---

**Questions?** See [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md) for current state,
or [.qoder/quests/](.qoder/quests/) for detailed design documents.
