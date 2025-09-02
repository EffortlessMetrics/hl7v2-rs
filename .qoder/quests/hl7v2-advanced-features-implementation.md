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
