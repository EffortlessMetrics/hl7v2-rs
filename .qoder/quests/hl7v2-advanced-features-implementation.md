# HL7v2-rs Advanced Features Implementation Design

## 1. Overview

This document outlines the design for implementing advanced features in the HL7v2-rs project across multiple release phases. The implementation will enhance the existing core parsing, profile validation, and message generation capabilities with streaming processing, dynamic profile loading, server mode, language bindings, integration tools, security features, and advanced analytics.

This design follows a normative approach with specific implementation requirements to ensure consistency, security, and performance across all components.

## 2. Repository Type Detection

The HL7v2-rs project is a **Backend Framework/Library** implemented in Rust. It consists of multiple crates:
- `hl7v2-core`: Core parsing and data model
- `hl7v2-prof`: Profile validation
- `hl7v2-gen`: Message generation
- `hl7v2-cli`: Command-line interface

## 3. Architecture

The project follows a modular architecture with separate crates for different functionalities:

```
graph TD
    A[hl7v2-cli] --> B[hl7v2-core]
    A --> C[hl7v2-prof]
    A --> D[hl7v2-gen]
    C --> B
    D --> B
```

## 4. Core Advanced Features (v1.2.0)

### 4.1 Streaming Parser for Large Messages

**Design Approach:**
- Implement an event-based SAX-like parser that yields `Event` enum variants
- Use zero-copy techniques to borrow slices from the underlying buffer
- Support incremental parsing with partial buffer consumption
- Handle delimiter discovery and reconfiguration per message
- Implement backpressure and memory bounds

**API Design:**
```
pub struct Delimiters { field: u8, comp: u8, rep: u8, esc: u8, sub: u8 }

pub enum Event<'a> {
    StartMessage{ delims: Delimiters },
    Segment { id: &'a [u8] },
    Field { num: u16, raw: &'a [u8] },
    // ... other event types
}

pub struct StreamParser<D> {
    // delim table and internal state
}

impl<D: BufRead> StreamParser<D> {
    pub fn new(reader: D, delims: Delimiters) -> Self { /* ... */ }
    pub fn next_event<'a>(&'a mut self) -> Result<Option<Event<'a>>, Error> { /* ... */ }
}
```

**Normative Requirements:**
- Start in "pre-MSH" mode: use default `|^~\&` until first `MSH`
- On `MSH`, read field separator = byte at pos 4; encoding chars = field 2 (4 chars). Switch delimiters **for that message only**
- Batch iterators forward current delimiters to each message parse
- Support bounded channels per worker; default 1024 messages; `--queue-capacity N`
- On full: block (server) or apply 429 (HTTP) and MLLP backpressure (no read)

### 4.2 Incremental Parsing

**Design Approach:**
- Modify the parser to accept partial buffers
- Maintain internal state to resume parsing on the next chunk
- Support both `Read` and `AsyncRead` traits
- Handle partial TS (time stamp) comparison semantics with precision-aware comparisons

### 4.3 Advanced Memory Management

**Design Approach:**
- Implement memory pools for scratch buffers per thread
- Use `SmallVec` for short segments/components to avoid heap allocation
- Ensure constant memory usage regardless of input size

### 4.4 Zero-Copy Optimizations

**Design Approach:**
- Fields/segments are borrowed slices of the underlying buffer
- Consumers can copy if needed, but zero-copy is the default
- Validate borrow validity through comprehensive testing
- Implement highlight escape `\H\..\N\` handling with range tracking
- Add hex/base64 escape handling for binary data in non-text fields

## 5. Profile Advanced Features (v1.2.0)

### 5.1 Dynamic Profile Loading

**Design Approach:**
- Implement a memory-bounded LRU cache keyed by `(name, version)`
- Support profile loading from multiple sources (local files, HTTP, cloud storage)

### 5.2 Profile Validation

**Design Approach:**
- Add JSON Schema validation for profiles using `schemars`
- Fail fast on load with precise path errors
- Implement table precedence and versioning with normative order
- Add expression engine guardrails with pre-compilation and time-bound evaluation

### 5.3 Profile Merging

**Design Approach:**
- Extend existing profile inheritance mechanism
- Support merging of constraints, rules, and value sets
- Implement inheritance conflict resolution with child precedence
- Detect and handle cycles with `E_Profile_Cycle` error

**Normative Requirements:**
- Table precedence order: `custom_valueset > profile_alias > official_table(version=message_version)`
- For identical `(path, constraint_type)` in parent and child:
  - If both set `severity`, **child severity wins**
  - If either sets `description`, **child description wins**
- Cycles produce `E_Profile_Cycle` with full chain in `human`
- CLI: `--dump-resolved-profile` to produce fully merged YAML

### 5.4 Remote Profile Fetching

**Design Approach:**
- Implement fetchers for `file://`, `http(s)://`, `s3://`, `gs://`
- Add ETag/If-None-Match caching
- Support truststore (TLS) and signature verification

## 6. Generator Advanced Features (v1.2.0)

### 6.1 Fuzz Testing Capabilities

**Design Approach:**
- Integrate `proptest` for generating random valid/invalid messages
- Implement shrinking on failure with invariants enforced

### 6.2 Statistical Distribution Modeling

**Design Approach:**
- Add field-level distributions: `uniform`, `normal`, `categorical`
- Implement correlated fields via shared latent variables
- Add segment repetition models (Markov chains)
- Implement correlated distributions with latent variable mechanism

**API Design:**
```
latents:
  - name: bmi
    normal: { mean: 27, std: 5 }
fields:
  - path: "OBX(1).5"
    from: latent "bmi" transform: "round(1)"
  - path: "OBX(2).5"
    from: latent "bmi" transform: "map_bmi_to_hrisk"
```

### 6.3 Integration with External Data Sources

**Design Approach:**
- Create pluggable data providers: CSV, SQLite, FHIR server
- Support in-memory dictionaries

### 6.4 Advanced Corpus Management

**Design Approach:**
- Implement manifest describing seed, counts, message types
- Add support for train/val/test splits with stratification
- Implement corpus manifest with reproducibility guarantees

**Normative Requirements:**
- `manifest.json` must include tool version, seed, templates' SHA-256, profiles' SHA-256, counts, message type breakdown, and per-file SHA-256
- Add `gen --verify-manifest` to recompute hashes and validate

## 7. CLI Advanced Features (v1.2.0)

### 7.1 Server Mode for Continuous Processing

**Design Approach:**
- Implement long-running process with HTTP/gRPC endpoints
- Add concurrency limits, backpressure, structured logs, graceful shutdown
- Implement HTTP framing and payload formats support
- Add Authn/Authz hooks

**Normative Requirements:**
- Support: `binary/mllp` over TCP (separate port), `application/hl7-v2` (raw with \r separators), and `application/x-ndjson` of JSON messages
- Streaming uploads: chunked transfer; server parses incrementally and streams NDJSON responses with validation results per message
- Provide middleware points for Authn (Bearer/OIDC validate → principal in context) and RBAC (method + path + principal → allow/deny)
- No PHI in logs: redact values; keep structural context (segment/field) only unless `--log-phidata` set for test

### 7.2 Web API Interface

**Design Approach:**
- Use Axum/Hyper for HTTP and optional Tonic for gRPC
- Support streaming request/response for large payloads
- Add auth via bearer/OIDC

### 7.3 Plugin System

**Design Approach:**
- Load validators/transforms as dynamic libs behind FFI boundary
- Implement sandbox with configurable allowlist

### 7.4 GUI Interface

**Design Approach:**
- Use Tauri for desktop UI
- Visualize parse trees, highlight errors, run validation locally

## 8. Language Bindings Implementation (v1.3.0)

### 8.1 C ABI for FFI

**Design Approach:**
- Implement stable `extern "C"` functions
- Use error codes and out-params with caller freeing memory

**Normative Requirements:**
- Versioned symbols: `hl7v2_rs_v1_*`
- Single `hl7v2_free(void*)`
- All strings **UTF-8**; lengths explicitly passed; never null-terminated

### 8.2 Python Bindings (PyO3)

**Design Approach:**
- Generate wheels for macOS/Linux/Windows
- Release GIL for parse/validate/gen operations

**Normative Requirements:**
- Build manylinux2014 + musllinux; `pyo3` with `abi3` for Python ≥3.8
- Release GIL around parse/validate; ensure iterators are Pythonic generators

### 8.3 JavaScript Bindings (wasm)

**Design Approach:**
- Target `wasm32-unknown-unknown`
- Provide wrappers for both Node and browser

**Normative Requirements:**
- Node and browser entry points
- Fetch/FS disabled by default; accept buffers only
- Limit message size (e.g., 4 MiB) with friendly error `S_Config`

### 8.4 Java Bindings

**Design Approach:**
- Use JNI for minimal API mapping
- Create shaded JAR with native libs per OS

**Normative Requirements:**
- Shaded JAR with natives per OS
- Handle UTF-16/UTF-8 conversions explicitly
- Zero JNI local ref leaks (spot-checked via tests)

## 9. Integration Tools Implementation (v1.3.0)

### 9.1 Database Connectors

**Design Approach:**
- Implement Postgres and Snowflake connectors
- Use prepared statements and idempotency keys

**Normative Requirements:**
- At-least-once delivery with idempotency keys (`MSH-10` default)
- Optional exactly-once if backend supports transactions (Kafka→Postgres with transaction fencing)

### 9.2 Message Queue Integration

**Design Approach:**
- Integrate with Kafka (rdkafka) and RabbitMQ (lapin)
- Support partition keys based on message control ID or MRN

**Normative Requirements:**
- Partition key: `MSH-10` fallback to `PID.3`. Configurable.
- Consumer group offset commits *after* successful validation/persist depending on `--ack-after`.

### 9.3 Cloud Service Integration

**Design Approach:**
- Implement S3/GCS/Azure Blob stores with resumable uploads
- Support signed URLs and server-side encryption

**Normative Requirements:**
- Multi-part uploads; resumable; server-side encryption flags
- SSE-KMS key id pass-through

### 9.4 Monitoring/Observability Integration

**Design Approach:**
- Integrate OpenTelemetry metrics & traces
- Provide default dashboard JSON for Grafana

**Normative Requirements:**
- Metrics naming (OpenTelemetry):
  - Counters: `hl7_messages_parsed_total`, `hl7_segments_total`, `hl7_validation_errors_total{code}`, `hl7_mllp_frames_total{dir=rx|tx}`, `hl7_bytes_total`
  - Histograms: `hl7_parse_duration_ms`, `hl7_validate_duration_ms`, `hl7_ack_duration_ms`
  - Gauges: `hl7_inflight_requests`
- Sample Grafana JSON ships with repo

## 10. Documentation & Examples Implementation (v1.3.0)

### 10.1 Comprehensive User Guide

**Design Approach:**
- Create install, CLI, profiles, tables, and server mode documentation
- Use mdbook or Docusaurus for site generation

### 10.2 API Documentation

**Design Approach:**
- Enhance rustdoc with examples for each crate

### 10.3 Tutorial Series

**Design Approach:**
- Create "Build a validator," "Generate a corpus," "Wire Kafka ingest" tutorials

### 10.4 Example Projects

**Design Approach:**
- Develop ready-to-run projects for parse→validate→ack→export workflows

## 11. Security & Compliance Features (v2.0.0)

### 11.1 HIPAA Compliance Features

**Design Approach:**
- Implement TLS 1.2+ with optional mTLS
- Add file encryption (AES-GCM) with KMS envelopes

### 11.2 Audit Logging

**Design Approach:**
- Create append-only, tamper-evident logs with hash chains
- Include user, action, subject, and outcome information

**Normative Requirements:**
- Hash chain: each entry includes `prev_hash` and `sha256(entry)`
- Rotation and immutability: write-once files or external append-only store (S3 Object Lock/GCS Bucket Lock)

### 11.3 Encryption Support

**Design Approach:**
- Implement encryption in flight and at rest
- Support AWS/GCP/Azure KMS integration

### 11.4 Access Control

**Design Approach:**
- Implement RBAC with least privilege
- Support policies file or OIDC claims mapping

## Security Implementation Details

**Threat Model & Defaults:**
- Default deny on plugins (no FS/network)
- TLS required for server in "prod" mode; otherwise WARN at startup
- PHI in memory only; redaction in logs by default

## 12. Scalability & Performance Features (v2.0.0)

### 12.1 Horizontal Scaling Support

**Design Approach:**
- Ensure stateless server operations
- Implement idempotent operations and sticky routing

### 12.2 Distributed Processing

**Design Approach:**
- Implement queue workers with checkpointing offsets
- Support exactly-once semantics where backend allows

### 12.3 High Availability Features

**Design Approach:**
- Add health/readiness probes
- Implement zero-downtime rolling updates

### 12.4 Load Balancing

**Design Approach:**
- Implement concurrency tokens and adaptive throttling
- Add backpressure handling

## 13. Advanced Analytics Features (v2.0.0)

### 13.1 Message Flow Analysis

**Design Approach:**
- Track per-type volumes, error rates, and latency histograms

### 13.2 Performance Analytics

**Design Approach:**
- Monitor system performance metrics
- Implement alerting on performance degradation

### 13.3 Compliance Reporting

**Design Approach:**
- Track validation failures by segment/field
- Monitor table mismatch rates

### 13.4 Predictive Analytics

**Design Approach:**
- Implement trend modeling for volumes
- Add optional anomaly detection (z-score based)

## Performance & SLO Validation

**Perf CI Gates:**
- Benchmarks run on fixed instance size; deltas >±10% fail CI unless approved tag present
- Produce `perf.json` artifact with medians, p95

**Memory Bound Tests:**
- Stream parse 10GB corpus; assert RSS < 64 MiB (v1.2 target)
- Leak tests via `cargo-valgrind`/`address-sanitizer` in nightly CI

## 14. Implementation Roadmap

### Phase 1: Core Implementation (Months 1-2)
- Streaming/incremental/zero-copy parser
- Basic dynamic profile loading
- Server mode with HTTP endpoints

### Phase 2: Feature Completeness (Months 3-4)
- Advanced profile features (validation, merging, remote fetching)
- Generator enhancements (fuzz testing, statistical modeling)
- Plugin system and web API

### Phase 3: Advanced Features (Months 5-6)
- Language bindings (C, Python, JavaScript, Java)
- Integration tools (DB, MQ, Cloud, Observability)
- Comprehensive documentation and examples

### Phase 4: Ecosystem Integration (Months 7-8)
- GUI interface implementation
- Advanced security features
- Scalability enhancements

### Phase 5: Enterprise Features (Months 9-12)
- Full compliance features
- Advanced analytics
- Performance optimization

## Implementation Clarifications

- **Path grammar:** Document `SEG.F(~rep).C.S`. Disallow whitespace; case-sensitive segment IDs; numeric components 1-based
- **JSON shape stability:** Publish a JSON schema for CLI/Server outputs (parse, validate, ack)
- **Config file:** Support `hl7v2.toml` with env overrides to reduce CLI verbosity in prod
- **SemVer policy:** Crate public APIs follow SemVer; CLI flags are stable after v1.2; new flags behind `--unstable-*`

## 15. Technical Success Metrics

- Parse ≥ 100k msgs/min on NVMe (short ORU/ADT messages)
- Memory usage < 128 MiB steady-state
- 100% deterministic outputs with same seed/profile
- 0 unsafe code in public API

## 16. Quality Success Metrics

- 100% test coverage for core functionality
- Zero critical/high severity bugs in production
- 100% compatibility with HL7 v2.3-v2.9

## 17. Adoption Success Metrics

- Profiles: advanced rules + HL7 tables usable from CLI and Server
- Generator: realistic corpora + golden verification for downstream teams

## Error Handling and Observability

All components will follow a consistent error taxonomy with:
- Stable, documented error codes with machine-readable context
- OpenTelemetry metrics and logging
- No panics on user input

**Error Code Shape and Stability:**
Every error JSON includes:
```
{ "code":"P_SegmentId", "segment":"PID", "field":3, "comp":1, "rep":1,
  "byte_offset":123, "human":"...", "advice":"...", "trace_id":"..." }
```

**Acceptance:** Snapshot test for error JSON schema; changelog guards breaking changes.

## 18. Error Handling and Observability

All components will follow a consistent error taxonomy with:
- Stable, documented error codes with machine-readable context
- OpenTelemetry metrics and logging
- No panics on user input

## 19. Security Considerations

- Memory safety through Rust's ownership model
- No `unsafe` in public APIs
- TLS 1.2+ for all network communications
- Audit logging for all operations
- RBAC for access control

## 20. Performance Requirements

- Streaming first approach for all file/network paths
- Bounded memory usage for all operations
- Throughput SLO: ≥100k short ADT/ORU msgs/min on NVMe
- Steady-state RSS <128 MiB

## Cross-cutting Normative Additions

### Empty vs NULL vs Missing

HL7 v2 distinguishes *empty field* (no characters), *missing* (field not present), and *NULL* (`""`). Validation and defaults differ.

**Normative Specifications:**

* *Missing* = field index out of range → **absent**.
* *Empty* = present but zero-length → **absent** for "required" checks **unless** profile marks `treat_empty_as_present: true`.
* *NULL* = literal `""` (two quotes) → **present null** which **suppresses defaults** and should **fail** a required constraint unless `allow_null: true`.

**API:**
```rust
pub enum Presence<'a> { Missing, Empty, Null, Value(&'a str) }
pub fn get_presence(msg: &Message, path: &str) -> Presence;
```

**Acceptance:** Tests covering required/conditional rules and table checks for each presence kind.

### MSH-18 Character Set Handling

Real feeds flip encodings mid-stream; you'll see `UNICODE UTF-8`, `8859/1`, etc.

**Normative Specifications:**

* If `--charset=auto`, prefer MSH-18 per message; else use CLI-forced charset.
* Unsupported MSH-18 → `S_Encoding` warning (lenient) or error (strict).

**Acceptance:** Corpus with mixed UTF-8/ISO-8859-1; auto mode decodes both; strict mode fails unknown.

### TS Comparison Semantics

TS precision varies (YYYY, YYYYMM, ...). Direct compare is ambiguous.

**Normative Specifications:**

* Parsing returns `(dt: DateTime<FixedOffset>, precision: enum {Year, Month, Day, Second, Subsecond})`.
* `before(a,b)` compares to **lowest common precision**. Example: `2000` vs `2000-01-01` → treat `a` as `2000-01-01 00:00:00`.

**Acceptance:** Tests for each precision pairing.

### MLLP & ACK Semantics

#### MLLP Frame Semantics (Normative)

* Start: `VT (0x0B)`, end: `FS (0x1C) CR (0x0D)`.
* **Read timeout** default 30s; partial frame timeout → `T_MLLP_Timeout`.
* **Strict** mode rejects nested `VT` or missing `FS CR`; **lenient** attempts recovery (scan to next `VT` with warning).
* Optional TLS (rustls); mutual TLS via `--mtls` with CA bundle path.

#### ACK Generation

* **Policy:** Only emit application ACK (AA/AE/AR) *after* validation completes. Offer `--ack-after=validate|persist` hook for integrators.
* **Mapping:** Validation errors map to `MSA-1=AE` and populate ERR segments with field/location and codes.
* **Acceptance:** Loopback tests (client/server) exercising happy path + AE/AR; verify timing relative to persistence callback.

## Consolidated Acceptance Deltas

This design addresses all the requirements outlined in the specification:

* Add **Presence semantics** (Missing/Empty/Null) and update validator behavior.
* Add **MSH-18 charset policy** and per-message delimiter switching in streaming.
* Fix **TS comparison** rules for partial precision.
* Flesh out **MLLP** framing, timeouts, TLS, ACK timing policy.
* Lock down **table precedence** and **expression guardrails**.
* Specify **metrics names**, **error JSON schema**, and **no-PHI logging** default.
* Define **bindings packaging targets** (manylinux/musllinux/wasm) and ABI rules.
* Codify **DB/Kafka** idempotency & persistence/ACK ordering.
* Add **perf CI gates** and **memory bound tests**.

If you want, I can turn these into **red-line edits** against your document or generate **issue templates** (title, rationale, acceptance, test pointers) so the team can pick them up sprint-by-sprint.

## Implementation Issue Templates

To facilitate sprint-by-sprint implementation of the advanced features, the following issue templates are provided to ensure consistent tracking and implementation:

### Template 1: Core Advanced Features Implementation

**Title:** [Core v1.2.0] Implement {feature_name} for Streaming Parser

**Rationale:** 
Implement the {feature_name} functionality as specified in the advanced features design document to enhance the streaming parser capabilities for handling large HL7 messages efficiently.

**Requirements:**
- [ ] Implement {feature_name} according to the design specification
- [ ] Ensure zero-copy optimizations where applicable
- [ ] Add comprehensive unit tests covering normal and edge cases
- [ ] Add performance benchmarks to validate memory bounds
- [ ] Update documentation with API usage examples

**Acceptance Criteria:**
- [ ] Streaming parser correctly handles {feature_name} for various message sizes
- [ ] Memory usage stays within specified bounds (RSS < 64 MiB for 10GB corpus)
- [ ] All existing functionality remains unaffected
- [ ] Performance benchmarks show improvement or maintainment of existing performance
- [ ] Code passes all CI checks (clippy, fmt, tests)

**Test Pointers:**
- Reference tests in `crates/hl7v2-core/src/tests.rs`
- Add new test cases in `crates/hl7v2-core/tests/` for integration testing
- Benchmark tests in `crates/hl7v2-core/benches/`

### Template 2: Profile Advanced Features Implementation

**Title:** [Profile v1.2.0] Implement {feature_name} for Dynamic Profile Loading

**Rationale:**
Implement the {feature_name} functionality to enhance profile validation and dynamic loading capabilities, allowing for more flexible and robust HL7 message validation.

**Requirements:**
- [ ] Implement {feature_name} according to design specification
- [ ] Ensure backward compatibility with existing profile formats
- [ ] Add support for remote profile fetching with caching
- [ ] Implement proper error handling and logging
- [ ] Add comprehensive unit and integration tests

**Acceptance Criteria:**
- [ ] Dynamic profile loading works with local and remote sources
- [ ] Profile merging follows specified precedence rules
- [ ] Error handling is consistent with existing error taxonomy
- [ ] Performance is within acceptable bounds
- [ ] All existing profile functionality remains unaffected

**Test Pointers:**
- Reference tests in `crates/hl7v2-prof/src/tests.rs`
- Add new test cases for profile merging and inheritance
- Create test profiles with various validation rules

### Template 3: Generator Advanced Features Implementation

**Title:** [Generator v1.2.0] Implement {feature_name} for Statistical Modeling

**Rationale:**
Implement the {feature_name} functionality to enhance the message generation capabilities with advanced statistical modeling for more realistic test data.

**Requirements:**
- [ ] Implement {feature_name} according to design specification
- [ ] Ensure deterministic outputs with proper seeding
- [ ] Add support for correlated distributions
- [ ] Implement corpus manifest with reproducibility guarantees
- [ ] Add comprehensive unit and integration tests

**Acceptance Criteria:**
- [ ] Statistical modeling produces expected distributions
- [ ] Correlated fields maintain specified correlations
- [ ] Corpus manifest includes all required metadata
- [ ] Generated messages are valid HL7 v2.x
- [ ] Deterministic outputs are consistent with same seed

**Test Pointers:**
- Reference tests in `crates/hl7v2-gen/src/tests.rs`
- Add statistical validation tests
- Create test templates with various distribution configurations

### Template 4: CLI Advanced Features Implementation

**Title:** [CLI v1.2.0] Implement {feature_name} for Server Mode

**Rationale:**
Implement the {feature_name} functionality to provide server mode capabilities for continuous processing of HL7 messages with proper authentication and observability.

**Requirements:**
- [ ] Implement {feature_name} according to design specification
- [ ] Add HTTP/gRPC endpoints with proper authentication
- [ ] Implement backpressure handling and bounded channels
- [ ] Add comprehensive logging with PHI protection
- [ ] Add integration tests for server functionality

**Acceptance Criteria:**
- [ ] Server mode starts and stops gracefully
- [ ] HTTP/gRPC endpoints respond correctly
- [ ] Authentication and authorization work as specified
- [ ] Backpressure prevents memory exhaustion
- [ ] Logs contain appropriate information without PHI

**Test Pointers:**
- Reference tests in `crates/hl7v2-cli/src/main.rs`
- Add integration tests for server endpoints
- Create test scenarios with various load conditions

### Template 5: Language Bindings Implementation

**Title:** [Bindings v1.3.0] Implement {language} Bindings with {feature}

**Rationale:**
Implement {language} bindings with {feature} to enable integration with systems using that language, expanding the reach of the HL7v2-rs library.

**Requirements:**
- [ ] Implement {language} bindings according to design specification
- [ ] Ensure ABI stability and versioning
- [ ] Add comprehensive documentation and examples
- [ ] Implement proper error handling and memory management
- [ ] Add platform-specific build and packaging

**Acceptance Criteria:**
- [ ] Bindings work correctly on target platforms
- [ ] API is consistent with Rust library functionality
- [ ] Memory is properly managed with no leaks
- [ ] Examples demonstrate all major functionality
- [ ] Package can be installed and used by developers

**Test Pointers:**
- Create language-specific test projects
- Add integration tests for all major API functions
- Validate packaging and distribution mechanisms

### Template 6: Integration Tools Implementation

**Title:** [Integration v1.3.0] Implement {tool} Integration with {feature}

**Rationale:**
Implement {tool} integration with {feature} to enable seamless connection with databases, message queues, and cloud services for HL7 message processing pipelines.

**Requirements:**
- [ ] Implement {tool} integration according to design specification
- [ ] Ensure at-least-once delivery with idempotency
- [ ] Add proper error handling and retry mechanisms
- [ ] Implement monitoring and observability integration
- [ ] Add comprehensive integration tests

**Acceptance Criteria:**
- [ ] Integration works correctly with {tool}
- [ ] Idempotency prevents duplicate processing
- [ ] Error handling is robust and informative
- [ ] Monitoring metrics are properly reported
- [ ] Performance is within acceptable bounds

**Test Pointers:**
- Set up test instances of {tool} for integration testing
- Create test scenarios with various failure conditions
- Validate monitoring and observability integration

### Template 7: Security & Compliance Implementation

**Title:** [Security v2.0.0] Implement {feature} for {compliance_standard} Compliance

**Rationale:**
Implement {feature} to ensure {compliance_standard} compliance and enhance the security posture of the HL7v2-rs system for enterprise healthcare environments.

**Requirements:**
- [ ] Implement {feature} according to design specification
- [ ] Ensure PHI protection in memory and logs
- [ ] Add proper authentication and authorization
- [ ] Implement audit logging with tamper evidence
- [ ] Add comprehensive security tests

**Acceptance Criteria:**
- [ ] {feature} works correctly and securely
- [ ] PHI is properly protected in all scenarios
- [ ] Authentication and authorization are enforced
- [ ] Audit logs provide tamper evidence
- [ ] Security tests pass without vulnerabilities

**Test Pointers:**
- Create security test scenarios with various attack vectors
- Validate PHI protection mechanisms
- Test audit logging and tamper detection

### Template 8: Performance & Analytics Implementation

**Title:** [Performance v2.0.0] Implement {feature} for {analytics_type} Analytics

**Rationale:**
Implement {feature} to provide {analytics_type} analytics capabilities for monitoring and optimizing HL7 message processing performance in production environments.

**Requirements:**
- [ ] Implement {feature} according to design specification
- [ ] Ensure metrics naming consistency with OpenTelemetry standards
- [ ] Add proper performance monitoring and alerting
- [ ] Implement analytics data collection and processing
- [ ] Add comprehensive performance tests

**Acceptance Criteria:**
- [ ] {feature} provides accurate analytics data
- [ ] Metrics follow OpenTelemetry naming conventions
- [ ] Performance monitoring works correctly
- [ ] Analytics data is properly collected and processed
- [ ] Performance tests validate system capabilities

**Test Pointers:**
- Create performance test scenarios with various loads
- Validate metrics collection and reporting
- Test analytics data processing and visualization

## Conclusion

This comprehensive design document provides a detailed roadmap for implementing advanced features in the HL7v2-rs project. By following the phased approach outlined in this document, the project will evolve from a solid foundation of core HL7 v2 parsing and validation capabilities to a full-featured, enterprise-grade solution with streaming processing, dynamic profile loading, server mode, language bindings, integration tools, security features, and advanced analytics.

The implementation approach emphasizes:

1. **Library-First Design**: Ensuring all functionality is available through robust library APIs with a thin CLI wrapper
2. **Performance and Efficiency**: Meeting stringent performance requirements while maintaining memory efficiency
3. **Security and Compliance**: Implementing comprehensive security measures and compliance features
4. **Interoperability**: Providing multiple integration points through language bindings and standard protocols
5. **Observability**: Including comprehensive monitoring, logging, and tracing capabilities
6. **Quality Assurance**: Maintaining high test coverage and following established quality gates

With the issue templates and implementation guidance provided in this document, development teams can systematically implement each feature while ensuring consistency, quality, and alignment with the overall project goals. The phased approach allows for regular validation and course correction, ensuring successful delivery of all advanced features within the targeted timeline.

The HL7v2-rs project will become a modern, robust, and efficient solution for HL7 v2 message processing, enabling healthcare organizations to handle their data exchange requirements with confidence in performance, security, and reliability.

## Next Steps for Implementation

Based on the phased development roadmap, the implementation should proceed as follows:

### Implementation Approach

The implementation should follow these principles:

1. **Library-First Design**: All functionality should be implemented in the appropriate library crate (`hl7v2-core`, `hl7v2-prof`, `hl7v2-gen`) with thin CLI wrappers in `hl7v2-cli`
2. **Deterministic Outputs**: Ensure all operations produce deterministic results when given the same inputs and seeds
3. **Memory Safety**: Leverage Rust's memory safety guarantees with zero `unsafe` code in public APIs
4. **Performance Optimization**: Follow the performance requirements with parsing ≥ 100k msgs/min on NVMe and memory usage < 128 MiB steady-state
5. **Comprehensive Testing**: Maintain 100% test coverage for core functionality with unit, integration, and property-based tests
6. **Backward Compatibility**: Ensure new features do not break existing functionality
7. **Error Handling**: Follow the established error taxonomy with specific error variants and context information

### Phase 1: Core Implementation (Months 1-2)
Focus on implementing the streaming/incremental/zero-copy parser with all the enhanced features:
- Delimiter discovery and reconfiguration per message
- Backpressure and memory bounds implementation
- Highlight escape and hex/base64 escape handling
- TS comparison semantics for partial precision timestamps

**Key Implementation Tasks:**
- Implement `StreamParser` struct in `hl7v2-core` with incremental parsing capabilities
- Add `Event` enum variants for all HL7 message elements
- Implement delimiter switching logic based on MSH segment detection
- Add backpressure mechanisms with bounded channels
- Implement escape sequence processing for highlights and binary data
- Add precision-aware timestamp comparison functions
- Create comprehensive benchmarks for streaming performance
- Add memory usage tests for large message processing

### Phase 2: Feature Completeness (Months 3-4)
Complete the advanced profile features and generator enhancements:
- Table precedence and versioning
- Expression engine guardrails
- Inheritance conflict resolution
- Correlated distributions with latent variables
- Corpus manifest reproducibility

**Key Implementation Tasks:**
- Enhance profile loading in `hl7v2-prof` with dynamic loading capabilities
- Implement table precedence resolution with version fallback logic
- Add expression engine with pre-compilation and time-bound evaluation
- Implement profile merging with conflict resolution rules
- Add latent variable support in `hl7v2-gen` for correlated distributions
- Implement corpus manifest with SHA-256 hashing for reproducibility
- Add remote profile fetching with caching mechanisms
- Create integration tests for complex profile scenarios

### Phase 3: Advanced Features (Months 5-6)
Implement language bindings and integration tools:
- C ABI stability with versioned symbols
- Python wheels (manylinux2014 + musllinux)
- WASM support (Node and browser)
- Java bindings (shaded JAR with natives)
- DB connectors with idempotency
- Message queue integration (Kafka/RabbitMQ)
- Cloud service integration (S3/GCS/Azure)

**Key Implementation Tasks:**
- Create C FFI wrapper crate with stable ABI and versioned symbols
- Implement Python bindings using PyO3 with proper GIL management
- Add WASM target with browser and Node.js support
- Create Java bindings with JNI and shaded JAR distribution
- Implement database connectors with idempotency key support
- Add message queue integration with partition key configuration
- Create cloud storage integration with resumable uploads
- Add comprehensive integration tests for all bindings

### Phase 4: Ecosystem Integration (Months 7-8)
Focus on CLI enhancements and security features:
- HTTP framing and payload formats
- Authn/Authz hooks with middleware
- PHI logging protection
- Threat model implementation
- Audit logging with hash chains

**Key Implementation Tasks:**
- Implement server mode in `hl7v2-cli` with Axum/Hyper HTTP server
- Add gRPC support with Tonic for high-performance RPC
- Implement authentication middleware with Bearer/OIDC support
- Add authorization middleware with RBAC enforcement
- Implement PHI redaction in logs with structural context preservation
- Create threat model documentation with mitigation strategies
- Add audit logging with hash chain integrity protection
- Create comprehensive security tests and penetration testing procedures

### Phase 5: Enterprise Features (Months 9-12)
Complete performance validation and advanced analytics:
- Perf CI gates with benchmark validation
- Memory bound tests with RSS assertions
- MLLP frame semantics implementation
- ACK generation policy enforcement
- Comprehensive analytics dashboard

**Key Implementation Tasks:**
- Implement performance CI gates with Criterion benchmarks
- Add memory usage tests with valgrind/address-sanitizer
- Implement MLLP frame handling with proper timeouts and TLS support
- Add ACK generation with validation timing policies
- Create analytics dashboard with Grafana JSON templates
- Implement advanced flow analysis and compliance reporting
- Add predictive analytics with trend modeling
- Create comprehensive documentation and user guides

Each phase should be tracked using the issue templates provided above, with specific issues created for each feature implementation. Regular sprint reviews should ensure progress is on track and any blockers are addressed promptly.

### Development Practices

To ensure successful implementation, the following development practices should be followed:

1. **Test-Driven Development**: Write tests before implementing features
2. **Continuous Integration**: All changes must pass CI checks before merging
3. **Code Reviews**: All code changes must be reviewed by at least one other team member
4. **Documentation**: All public APIs must be documented with examples
5. **Benchmarking**: Performance-critical code must include benchmarks
6. **Security Audits**: Security-sensitive code should be audited regularly
7. **Release Management**: Follow semantic versioning with clear release notes

### Quality Gates

Each phase must pass the following quality gates before proceeding:

1. **Code Quality**: All code must pass clippy and rustfmt checks
2. **Test Coverage**: Minimum 100% coverage for core functionality
3. **Performance**: Meet or exceed specified performance requirements
4. **Security**: No critical or high severity vulnerabilities
5. **Documentation**: All features must be documented with examples
6. **Compatibility**: Full compatibility with HL7 v2.3-v2.9

## Development Environment and Tooling

### Required Tools

1. **Rust Toolchain**: 
   - Minimum Supported Rust Version (MSRV): 1.89
   - Install via rustup: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
   - Components: `rustc`, `cargo`, `rustfmt`, `clippy`

2. **Development Environment**:
   - IDE/editor with Rust support (VS Code with rust-analyzer, IntelliJ IDEA with Rust plugin, etc.)
   - Terminal/Shell for running commands

3. **Testing Tools**:
   - Criterion.rs for benchmarking
   - cargo-valgrind or address-sanitizer for memory leak detection
   - k6 for load testing server components

4. **Documentation Tools**:
   - mdbook or similar for documentation generation
   - Graphviz for architecture diagrams

### Project Structure

The HL7v2-rs project follows a workspace structure with four main crates:

```
hl7v2-rs/
├── crates/
│   ├── hl7v2-core/     # Core parsing and data model
│   ├── hl7v2-prof/     # Profile validation
│   ├── hl7v2-gen/      # Message generation
│   └── hl7v2-cli/      # Command-line interface
├── profiles/           # Sample profiles
├── schemas/            # JSON schemas
├── test_data/          # Test data files
└── Cargo.toml          # Workspace configuration
```

### Build and Test Commands

1. **Building**:
   - Build all crates: `cargo build`
   - Build release: `cargo build --release`
   - Build specific crate: `cargo build -p hl7v2-core`

2. **Testing**:
   - Run all tests: `cargo test`
   - Run tests for specific crate: `cargo test -p hl7v2-core`
   - Run benchmarks: `cargo bench`
   - Run Clippy: `cargo clippy`
   - Format code: `cargo fmt`

3. **Running**:
   - Run CLI: `cargo run -- [args]`
   - Run specific example: `cargo run --example [example_name]`

### Development Workflow

1. **Feature Development**:
   - Create feature branch from main
   - Implement feature with tests
   - Run all checks (test, clippy, fmt, bench)
   - Create pull request with description and testing evidence
   - Address review comments
   - Merge after approval

2. **Testing Strategy**:
   - Unit tests for individual functions
   - Integration tests for component interactions
   - Property-based tests for complex logic
   - Performance tests for critical paths
   - Security tests for sensitive functionality

3. **Release Process**:
   - Update version numbers according to SemVer
   - Update CHANGELOG.md with changes
   - Create Git tag
   - Publish to crates.io
   - Update documentation

## Monitoring and Observability

### Metrics Collection

All components should emit OpenTelemetry metrics with the following naming conventions:

- **Counters**: `hl7_messages_parsed_total`, `hl7_segments_total`, `hl7_validation_errors_total{code}`, `hl7_mllp_frames_total{dir=rx|tx}`, `hl7_bytes_total`
- **Histograms**: `hl7_parse_duration_ms`, `hl7_validate_duration_ms`, `hl7_ack_duration_ms`
- **Gauges**: `hl7_inflight_requests`

### Logging

- Structured logging with context information
- PHI redaction by default unless explicitly enabled
- Log levels: TRACE, DEBUG, INFO, WARN, ERROR
- JSON format for machine parsing

### Tracing

- Distributed tracing with trace IDs
- Span creation for major operations
- Context propagation across components

## Security Considerations

### Data Protection

- PHI handling in accordance with HIPAA requirements
- Encryption in transit (TLS 1.2+) and at rest (AES-GCM)
- Secure key management with KMS integration
- Audit logging with tamper evidence

### Access Control

- Role-based access control (RBAC)
- Authentication with OIDC/bearer tokens
- Authorization enforcement at API boundaries
- Principle of least privilege

### Secure Coding Practices

- Zero unsafe code in public APIs
- Input validation and sanitization
- Dependency vulnerability scanning
- Regular security audits

## Performance Optimization

### Memory Management

- Zero-copy parsing where possible
- Memory pools for scratch buffers
- Bounded memory usage for streaming operations
- Leak detection in CI pipeline

### CPU Optimization

- Efficient string operations
- Minimal allocations in hot paths
- Vector operation optimization
- Criterion-based performance regression testing

### I/O Optimization

- Buffered I/O operations
- Async I/O for concurrent operations
- Backpressure mechanisms
- Efficient serialization/deserialization
