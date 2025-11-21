# HL7v2-rs Implementation Plan

**Version**: v1.2.0 Completion Plan
**Effective Date**: 2025-11-19
**Target Release**: Q1 2026

> **Note**: This plan aligns with `SYSTEMATIC_BUILD_PLAN.md` and reflects completed work.

---

## Table of Contents

1. [Current Progress](#current-progress)
2. [Remaining Work (Sprint Breakdown)](#remaining-work-sprint-breakdown)
3. [Detailed Task Lists](#detailed-task-lists)
4. [Dependencies & Critical Path](#dependencies--critical-path)
5. [Testing Strategy](#testing-strategy)
6. [Release Checklist](#release-checklist)

---

## Current Progress

- **Phase 1: Foundation & Infrastructure** (✅ Complete)
  - Schema-driven design
  - BDD framework
  - Infrastructure-as-Code (Docker/K8s)
  - Policy-as-Code (OPA)

- **Phase 2: Core Features** (🔄 In Progress)
  - Network Module: ✅ MLLP Codec/Server/Client implemented
  - HTTP Server: ✅ Axum server, Endpoints, Auth implemented
  - Backpressure: 🚧 Planned
  - gRPC: 🚧 Planned

---

## Remaining Work (Sprint Breakdown)

### Sprint 1 (Weeks 1-2): Streaming & Reliability

**Goal**: Complete streaming parser improvements and harden network

**Tasks**:
1. **Streaming Backpressure** (Priority: HIGH)
   - [ ] Implement bounded channels in `StreamParser`
   - [ ] Add memory usage tracking (RSS)
   - [ ] Add backpressure handling in HTTP/MLLP servers
   - [ ] Benchmark under load (10GB corpus)
   - **Acceptance**: Memory stays <64MB for large files

2. **Resume Parsing** (Priority: MEDIUM)
   - [ ] Track parser state between chunks
   - [ ] Implement `resume_from(offset)`
   - [ ] Add tests for chunk boundaries
   - **Acceptance**: Can resume interrupted parsing

3. **Network Hardening** (Priority: MEDIUM)
   - [ ] Add connection pooling to MLLP client
   - [ ] Add TLS support to MLLP server/client (rustls)
   - [ ] Add comprehensive timeout handling
   - **Acceptance**: Robust against network failures

### Sprint 2 (Weeks 3-4): Remote Profiles & Advanced Features

**Goal**: Enable distributed operation

**Tasks**:
1. **Remote Profile Loading** (Priority: HIGH)
   - [ ] Implement HTTP/S3/GCS profile fetchers
   - [ ] Add LRU caching for profiles
   - [ ] Implement ETag support
   - **Acceptance**: Can validate against remote profiles

2. **gRPC Server** (Priority: MEDIUM)
   - [ ] Define `.proto` files
   - [ ] Implement Tonic server
   - [ ] Mirror HTTP endpoints
   - **Acceptance**: gRPC client can parse/validate

3. **Corpus Manifest** (Priority: LOW)
   - [ ] Generate `manifest.json`
   - [ ] Add verification command
   - **Acceptance**: Reproducible corpora

---

### Sprint 3 (Week 5): CLI Polish & Observability

**Goal**: Complete CLI and ensure production readiness

**Tasks**:
1. **CLI Enhancements** (Priority: MEDIUM)
   - [ ] Implement `--report` flag
   - [ ] Add TOML config file support
   - [ ] Fix `--canonical-delims`
   - **Acceptance**: CLI feature complete

2. **Observability** (Priority: HIGH)
   - [ ] Finalize OpenTelemetry tracing
   - [ ] Create Grafana dashboards
   - [ ] Validate PHI redaction in logs
   - **Acceptance**: Full visibility in production

**Exit Criteria (v1.2.0)**:
- [ ] All critical features implemented
- [ ] 90% Test coverage
- [ ] Documentation complete
- [ ] Performance targets met

---

### Sprint 8 (Week 8): Final Polish & Release Prep

**Goal**: Release v1.2.0

**Tasks**:
1. **Bug Fixes & Optimization** (2-3 days)
   - [ ] Fix issues found in testing
   - [ ] Performance optimization pass
   - [ ] Memory optimization
   - [ ] Final cleanup
   - **Acceptance**: Zero critical/high bugs

2. **Version & Changelog** (0.5 days)
   - [ ] Update version in Cargo.toml
   - [ ] Generate changelog
   - [ ] Document breaking changes (if any)
   - [ ] Update feature flags

3. **Release Testing** (1-2 days)
   - [ ] Final integration test run
   - [ ] Performance validation
   - [ ] Smoke tests on deployment
   - [ ] Security audit
   - **Acceptance**: All tests pass

4. **Release** (0.5 days)
   - [ ] Tag release in Git
   - [ ] Publish to crates.io
   - [ ] Update documentation
   - [ ] Create GitHub release notes
   - [ ] Announce release

**Effort**: 4-6 story points | 1 week | 1-2 engineers

---

## Detailed Task Lists

### Network Module (Mostly Complete)

**File**: `crates/hl7v2-core/src/network/`

**Remaining Tasks**:
- [ ] Implement TLS support (rustls)
- [ ] Add connection pooling
- [ ] Stress test MLLP codec

### Profile Module Enhancements

**File**: `crates/hl7v2-prof/src/lib.rs`

**Tasks**:
- [ ] Add `remote_loader` module for HTTP/S3/GCS fetching
- [ ] Implement `ProfileCache` with LRU eviction
- [ ] Add ETag-based cache invalidation
- [ ] Improve expression engine with pre-compilation
- [ ] Add cycle detection in profile inheritance
- [ ] Write comprehensive tests

**Key Functions to Implement**:
```rust
pub async fn load_remote_profile(url: &str, cache: &ProfileCache)
    -> Result<Profile>;

pub struct ProfileCache {
    pub fn new(max_size_mb: usize) -> Self;
    pub async fn get(&self, key: &str) -> Option<Profile>;
    pub async fn set(&self, key: String, profile: Profile) -> Result<()>;
}

pub fn detect_profile_cycles(profile: &Profile) -> Result<()>;
```

### CLI Module Enhancements

**File**: `crates/hl7v2-cli/src/main.rs`

**Tasks**:
- [ ] Add `--report FILE` flag to validation command
- [ ] Implement `--canonical-delims` in normalize command
- [ ] Add `--envelope FILE` to parse command
- [ ] Add TOML config file support

**Example New Commands**:
```bash
hl7v2 parse input.hl7 --report validation.json
hl7v2 val input.hl7 --profile p.yaml --report errors.json
```

### Generation Module Enhancements

**File**: `crates/hl7v2-gen/src/lib.rs`

**Tasks**:
- [ ] Add manifest.json generation
- [ ] Track template/profile SHA-256s
- [ ] Implement `verify_manifest()` function
- [ ] Add manifest to corpus output
- [ ] Write tests

**Key Functions**:
```rust
#[derive(Serialize)]
pub struct Manifest {
    pub tool_version: String,
    pub seed: u64,
    pub templates: Vec<(String, String)>, // (path, sha256)
    pub profiles: Vec<(String, String)>,  // (path, sha256)
    pub message_count: usize,
    pub generation_timestamp: String,
    pub per_file_hashes: Vec<(String, String)>, // (filename, sha256)
}

pub fn generate_manifest(/* ... */) -> Result<Manifest>;
pub fn verify_manifest(manifest: &Manifest) -> Result<bool>;
```

---

## Dependencies & Critical Path

### Dependency Graph

```
Start
  ├─ Network Module (blocking)
  │  ├─ MLLP Frame Handler (2-3 days)
  │  ├─ TCP Connection Handler (2-3 days)
  │  └─ Tests (2-3 days)
  │
  ├─ Server Mode HTTP (depends on Network)
  │  ├─ Axum Setup (2 days)
  │  ├─ Endpoints (6-8 days)
  │  └─ Tests (2-3 days)
  │
  ├─ Remote Profile Loading (parallel)
  │  ├─ HTTP Fetcher (1-2 days)
  │  ├─ S3/GCS Support (2 days)
  │  ├─ LRU Cache (2 days)
  │  └─ Tests (2 days)
  │
  └─ Server Mode gRPC (depends on HTTP Server)
     ├─ Tonic Setup (1-2 days)
     ├─ Proto Definitions (1 day)
     ├─ Implementation (2 days)
     └─ Tests (1-2 days)

  All Parallel:
  ├─ CLI Enhancements (3-4 days)
  ├─ Streaming Improvements (3-4 days)
  ├─ Authentication (3-4 days)
  └─ Corpus Manifest (2-3 days)

  Integration & Release:
  └─ Final Testing & Release (1-2 weeks)
```

### Critical Path (Longest Route)

```
Network Module → Server HTTP → Authentication → Release
Estimated: ~6-7 weeks (blocking for v1.2.0)
```

### Non-Blocking Items (Can Start Immediately)

- Remote profile loading (design parallel with network)
- Streaming improvements
- CLI enhancements
- Corpus manifest
- Documentation

---

## Testing Strategy

### Unit Testing (Mandatory)

**Coverage Targets**: 90%+ of core logic

**Test Categories**:
1. **Network Module Tests** (40+ tests)
   - Frame handling (happy path + edge cases)
   - Connection setup/teardown
   - Timeout handling
   - Error scenarios

2. **Server Endpoint Tests** (30+ tests)
   - Parse endpoint (valid/invalid input)
   - Validate endpoint (passing/failing profiles)
   - ACK endpoint
   - Error responses

3. **Authentication Tests** (20+ tests)
   - Valid tokens
   - Invalid tokens
   - Expired tokens
   - Missing tokens
   - RBAC enforcement

4. **Profile Tests** (20+ tests)
   - Remote loading (HTTP/S3/GCS)
   - Caching behavior
   - Cache invalidation
   - Cycle detection

### Integration Testing (Mandatory)

**Scenarios**:
1. **E2E Parse → Validate → ACK** (5 tests)
2. **Server Under Load** (5 tests)
   - 100 concurrent connections
   - 1000 messages/sec sustained
   - Large messages (10KB+)
3. **Remote Profile Loading** (5 tests)
   - Load from multiple sources
   - Cache hits/misses
   - Network failures
4. **MLLP Protocol** (5 tests)
   - Frame boundaries
   - Multiple messages per frame
   - Malformed frames

### Performance Testing (Mandatory)

**Benchmarks** (run before/after optimization):
1. Parse latency (p50, p95, p99)
2. Validate latency
3. ACK generation latency
4. Memory usage (steady-state, peak)
5. Throughput (messages/sec)

**Targets**:
- Parse: <5ms p95 (typical message)
- Validate: <10ms p95
- Server: ≥1000 RPS sustained
- Memory: <500MB steady-state

### Security Testing (Mandatory)

**Tests**:
1. Authentication bypass attempts
2. SQL injection (if applicable)
3. Large input handling (DoS prevention)
4. PHI leakage in logs
5. TLS/mTLS validation

---

## Release Checklist

### Pre-Release (Week before)

- [ ] All tests passing (unit + integration)
- [ ] Code review completed (>80% reviewed)
- [ ] Documentation updated
- [ ] Performance targets validated
- [ ] Security audit complete
- [ ] Changelog prepared
- [ ] Version bumped in Cargo.toml

### Release Day

- [ ] Final test run on release branch
- [ ] Create release tag
- [ ] Build artifacts (binaries for all platforms)
- [ ] Publish to crates.io
- [ ] Create GitHub release with notes
- [ ] Update documentation site
- [ ] Announce release (Discord, Twitter, etc.)

### Post-Release (Week after)

- [ ] Monitor for critical issues
- [ ] Prepare patch releases as needed
- [ ] Gather feedback
- [ ] Plan v1.3.0

---

## Resource Allocation

### Recommended Team Structure

**Team A: Network & Server (Lead: Network Specialist)**
- 1 senior engineer (network/async)
- 1 mid engineer (server integration)
- Focus: Network module, HTTP/gRPC, authentication

**Team B: Features (Lead: Library Specialist)**
- 1 mid engineer (profiles/generation)
- 1 junior engineer (CLI/testing)
- Focus: Remote profiles, streaming, corpus manifest

**Team C: QA & DevOps**
- 1 QA engineer (testing infrastructure)
- 0.5 DevOps engineer (release automation)
- Focus: Tests, performance, deployment

**Total**: 4-5 engineers | 8-10 weeks | ~120 story points

### Weekly Standup Format

**Monday**: Sprint planning, blockers from previous week
**Wednesday**: Mid-week checkpoint (progress update)
**Friday**: Sprint wrap-up, next week prep

---

## Risk Management

### High-Risk Items & Mitigation

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|-----------|
| Network module complexity | +2-4 weeks | Medium | Start with MVP (MLLP only), add HTTP incrementally |
| Performance regression | Missing target | Medium | Benchmark early, profile often, feature flags |
| TLS/Security issues | Blockers | Low | Third-party audit, fuzzing, code review |
| Dependency conflicts | Build issues | Low | Pin versions, regular audits, test on CI |

### Escalation Path

**Blocker Found**: Escalate immediately (same day)
1. Document blocker in issue
2. Notify team lead
3. Plan workaround/mitigation
4. Communicate to stakeholders

**Schedule Slip >1 week**: Escalate to management
1. Assess impact on release date
2. Consider de-scoping features
3. Allocate additional resources if needed

---

## Definition of Done

A task is **DONE** when:

1. ✅ Code is written and reviewed (2+ approvals for core changes)
2. ✅ Unit tests written (90%+ coverage of new code)
3. ✅ Integration tests passing
4. ✅ Documentation updated (code examples, API docs)
5. ✅ No new clippy warnings
6. ✅ Performance benchmarked (no regressions)
7. ✅ Security reviewed (if applicable)
8. ✅ Merged to main branch
9. ✅ Related issues closed

---

## Success Criteria for v1.2.0

**Must Have (Non-Negotiable)**:
- [ ] Server mode running (HTTP + gRPC)
- [ ] All 4 HTTP endpoints working
- [ ] Authentication/RBAC functional
- [ ] Remote profile loading working
- [ ] ≥90% test coverage
- [ ] Zero critical bugs
- [ ] Performance targets met (≥1000 RPS)
- [ ] Documentation complete

**Should Have (High Value)**:
- [ ] CLI fully documented
- [ ] Corpus manifest working
- [ ] Streaming optimizations complete
- [ ] Performance baseline established
- [ ] MLLP over TCP working

**Nice to Have**:
- [ ] Horizontal scaling support
- [ ] Advanced analytics
- [ ] GUI mockups

---

## Next Immediate Actions

### This Week
1. [ ] Assign network module owner (CRITICAL)
2. [ ] Create GitHub issues for all v1.2 tasks
3. [ ] Set up feature branches
4. [ ] Schedule architecture review
5. [ ] Create sprint 1 board in project management tool

### Next 2 Weeks
1. [ ] Network module design review
2. [ ] Sprint 1 kickoff
3. [ ] Start MLLP frame handler implementation
4. [ ] Complete test infrastructure setup
5. [ ] First working MLLP frame tests

### Month 1 Checkpoint
1. [ ] Network module 50%+ done
2. [ ] First server endpoints running
3. [ ] Remote profile design finalized
4. [ ] Streaming backpressure functional

---

## Document Control

**Version**: 1.0
**Last Updated**: 2025-11-13
**Author**: HL7v2-rs Team
**Review Cycle**: Bi-weekly (during development)
**Next Review**: Sprint 3 (week 5)

---

## References

- [ROADMAP.md](ROADMAP.md) - High-level timeline and vision
- [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md) - Current feature status
- [.qoder/quests/hl7v2-advanced-features-implementation.md](.qoder/quests/hl7v2-advanced-features-implementation.md) - Detailed design
- GitHub Issues - Per-task tracking
- CI/CD Logs - Build and test status

---

**Questions?** Contact the team lead or open a GitHub discussion.
