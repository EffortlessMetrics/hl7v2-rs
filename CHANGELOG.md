# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added

- Added gRPC enhanced ACK parity for commit ACK codes `CA`, `CE`, and `CR`
  alongside the existing `AA`, `AE`, and `AR` application ACK codes.
- Added shared dirty real-world corpus fixture categories and CLI parity coverage
  for corpus summary, fingerprint, and diff.
- Added REST and gRPC server dirty-corpus parity coverage using the shared dirty
  real-world fixture categories.
- Added local Python wheel dirty-corpus parity coverage using the shared dirty
  real-world fixture categories.
- Added `xtask check-dirty-corpus-parity` to run the shared dirty-corpus
  acceptance proof across Rust core, CLI, REST, and gRPC surfaces.
- Added `xtask check-bundle-replay-parity` to run the shared bundle/replay
  acceptance proof across Rust core, CLI, REST, and gRPC surfaces.
- Added `xtask check-evidence-parity-acceptance` as the aggregate local
  Rust/CLI/REST/gRPC parity acceptance suite, with optional local Python wheel
  smoke.
- Added a shared schema-version parity fixture and manifest guard for
  representative CLI, REST, gRPC, and local Python evidence proof.
- Added `xtask check-schema-version-parity` to run the shared schema-version
  acceptance proof across Rust core, CLI, REST, and gRPC surfaces.
- Added `xtask check-safe-error-phi-parity` to run the shared safe-error and
  PHI sentinel acceptance proof across Rust core, CLI, REST, and gRPC surfaces.
- Added gRPC evidence replay parity with configured-root bundle replay,
  fail-closed missing/unsafe bundle handling, shared replay reports, and opt-in
  v2 replay provenance.
- Added gRPC quarantine output parity through `ValidateRedacted` with
  configured-root writes for failed redacted validation and opt-in v2 quarantine
  provenance.
- Added gRPC inline corpus diff parity, including before/after inline corpus
  deltas and opt-in v2 provenance.
- Added gRPC profile explain parity with shared profile explain reports and
  opt-in v2 provenance.
- Added gRPC profile fixture-test parity with shared profile test reports,
  inline fixture inputs, expected-report subset comparisons, and opt-in v2
  provenance.
- Added gRPC evidence bundle creation parity with configured-root writes,
  hashed public output IDs, unsafe bundle ID rejection, and opt-in v2 bundle
  artifacts.
- Added gRPC profile lint parity with shared profile lint reports and opt-in
  v2 provenance.

### Fixed

- Rejected non-finite float spellings such as `INF`, `Infinity`, and `NaN`
  from HL7 numeric validation.

### Documentation

- Recorded a current-main TestPyPI publishing-mode proof retry for public
  Python package `hl7v2`; wheel smoke passed, while upload remains blocked by
  TestPyPI Trusted Publisher setup.
- Recorded a post-release current-main readiness refresh after gRPC enhanced ACK
  parity landed through #705.
- Recorded a current-main TestPyPI publishing-mode proof attempt for public
  Python package `hl7v2`; wheel smoke passed, while upload remains blocked by
  TestPyPI Trusted Publisher setup.
- Added a repeatable public crates.io install-back smoke script and receipt for
  the v1.5.0 Rust library, CLI, and server first-use paths.
- Recorded a post-release current-main readiness refresh after normalization
  and CLI ACK parity landed through #698.
- Recorded a post-release current-main readiness refresh after shared
  Rust/CLI/server/Python dirty-corpus parity landed through #695.
- Recorded the shared dirty real-world corpus fixture proof for Rust core and
  CLI corpus evidence commands.
- Recorded the server dirty real-world corpus parity proof for REST and gRPC
  corpus evidence commands.
- Recorded the local Python wheel dirty real-world corpus parity proof.
- Recorded a post-release current-main readiness refresh after the focused SRP
  module split train landed through #691.
- Recorded a fresh hosted non-publishing `Python TestPyPI Proof` receipt on
  current `main` and confirmed public Python `hl7v2` is still absent from
  TestPyPI and PyPI.
- Recorded a post-release v1.5.0 objective audit showing that the crates.io,
  tag, GitHub release, and Rust/CLI/server install-back portions are now
  complete while public Python TestPyPI/PyPI proof remains blocked or
  undecided.
- Recorded a current hosted non-publishing `Python TestPyPI Proof` receipt for
  public package `hl7v2` while keeping TestPyPI upload/install-back blocked on
  Trusted Publisher setup.
- Aligned the cross-surface evidence parity docs with the already-tested Python
  ACK local binding proof.
- Aligned the gRPC parity docs with the currently implemented profile,
  bundle/replay, corpus, ACK, and normalize RPC proof surface.
- Refreshed the v1.5.0 release-readiness receipt after the Python ACK and
  gRPC parity documentation syncs landed on `main`.
- Recorded a pre-release package registry state audit showing that v1.5.0 was
  not yet published at that snapshot and that public Python `hl7v2` was not on
  PyPI or TestPyPI.
- Recorded a prompt-to-artifact objective completion audit for the active
  v1.5.0 lane and kept the lane open until publish, Python, and npm receipts
  exist.
- Recorded the v1.5.0 crates.io publish, tag, GitHub release, registry
  resolution, and Rust/CLI/server install-back receipt.
- Recorded the final non-publishing v1.5.0 pre-publish proof for the selected
  crates.io graph before any upload, tag, or GitHub release action.
- Refreshed the v1.5.0 release-readiness receipt after the advisory RIPR
  calibration audit and gRPC profile lint parity landed on `main`.
- Refreshed the v1.5.0 release-readiness receipt after gRPC profile explain
  parity landed on `main`.
- Refreshed the v1.5.0 release-readiness receipt after gRPC quarantine output
  parity landed on `main`.
- Refreshed the v1.5.0 release-readiness receipt after gRPC evidence bundle
  creation parity landed on `main`.
- Confirmed the v1.5.0 release graph decision still selects the primary Rust
  graph plus `hl7v2-python` as binding backend after the latest readiness
  refresh.
- Refreshed the v1.5.0 release-readiness receipt after the gRPC inline corpus
  fingerprint and diff parity work landed on `main`.
- Refreshed the v1.5.0 release-readiness receipt after the nightly property
  command repair and finite numeric validation fix landed on `main`.
- Recorded the first hosted-traffic calibration audit for the advisory `ripr`
  evidence surface and kept the lane non-blocking.

---

## [1.5.0] - 2026-05-15

### Added

- Added gRPC inline corpus fingerprint parity, including optional inline
  profile validation issue-code counts and opt-in v2 provenance.
- Added an advisory `ripr` static mutation-exposure workflow and suppression
  policy, keeping runtime mutation as a targeted backstop rather than a default
  PR tax.
- Added targeted mutation lane routing for high-risk parser, MLLP, profile,
  redaction, bundle/replay, evidence-schema, server, Python, and release
  surfaces.
- Added a Rust 1.95 / v1.5.0 release-readiness workflow and receipt home for
  non-publishing proof before crates.io release.

### Changed

- Raised MSRV from Rust 1.93 to Rust 1.95 and pinned
  `rust-toolchain.toml` to Rust 1.95.0 for local and CI consistency.
- Prepared the Rust package graph as version `1.5.0` for `hl7v2`,
  `hl7v2-server`, and `hl7v2-cli`.
- Prepared `hl7v2-python` as a publishable crates.io binding backend for the
  public Python `hl7v2` package, selected it for the v1.5.0 binding-backend
  release graph, and kept it outside the primary Rust product graph.
- Tightened the compiler, Clippy, no-panic, and file-policy rails for
  high-throughput maintenance without increasing default CI weight.
- Retargeted the public Python distribution metadata and proof workflows to
  `hl7v2` while keeping the internal `hl7v2-python` Rust crate outside the
  primary Rust product graph.

### Fixed

- Burned down selected CLI monitor and server metrics Clippy debt with bounded
  conversions and narrower test assertions.

---

## [1.4.0] - 2026-05-09

### Added

- Added opt-in v2 provenance contracts and producer paths across validation,
  profile, corpus, redaction, bundle, replay, quarantine, and doctor evidence
  artifacts while keeping v1 defaults compatible.
- Added server sidecar hardening after v1.3.0: inline corpus evidence
  endpoints, server replay, bundle artifact schema opt-in, readiness path-leak
  protection, redacted structured evidence logs, evidence metrics, and Docker
  Compose smoke coverage.
- Added Python evidence hardening: validation/corpus/redaction/bundle/replay
  v2 parity, cross-surface PHI sentinel coverage, a manual TestPyPI proof
  workflow, and a Python evidence workflow guide.
- Added `cargo run -p xtask -- evidence-schema-check` as the maintained local
  evidence fixture/schema validation rail.

### Fixed

- Fixed server evidence bundles so replay reproduces messages whose `MSH.9`
  includes a third message-structure component such as `ADT^A01^ADT_A01`.

### Documentation

- Prepared the v1.4.0 package line and publish dry-run receipt for the final
  Rust crates.io graph: `hl7v2`, `hl7v2-server`, and `hl7v2-cli`.
- Verified and refreshed the validation sidecar guide with live server replay
  output and the current inline corpus diff response shape.
- Added current-state and contract-index documentation for the post-v1.3.0
  evidence-contract line.

---

## [1.3.0] - 2026-05-09

### Added

- Released the v1.3.0 Evidence Loop around deterministic
  HL7 interface evidence: first-run diagnostics, typed validation reports,
  profile lint/test/explain, corpus summarize/fingerprint/diff,
  safe-analysis redaction, evidence bundle/replay, and Python binding parity.
- Added schema-backed evidence artifacts and golden fixtures for
  validation reports, profile reports, corpus reports, redaction receipts,
  bundle summaries, and replay reports.
- Added CLI automation semantics for evidence commands: stable exit codes,
  machine-readable stdout, diagnostic stderr, `--output`, `--quiet`, and
  `--no-color` support.
- Added server-side edge-guard workflows: sanitized `--print-config`,
  readiness checks, redacted validation, evidence bundle creation,
  policy-driven ACK/NAK decisions, and quarantine output hooks.
- Added Python binding APIs for parse, JSON export, normalize, validation
  reports, corpus summary/fingerprint/diff, safe-analysis redaction,
  evidence bundle creation, and replay verification.
- Added workflow guides for first-use evidence, vendor upgrade diffs,
  safe support bundles, and validation sidecar deployment.

### Changed

- Evidence bundles now include manifest and README artifacts, and replay
  verifies manifest hashes before comparing regenerated evidence.
- Current release documentation positions v1.3.0 as the published Evidence
  Loop release for the final Rust package graph.

---

## [1.2.1] - 2026-05-08

### Release

- Moved the package line to `1.2.1` so the final Rust publish can use a fresh
  release tag instead of reusing the historical `v1.2.0` tag.
- Published the final Rust package graph to crates.io: `hl7v2`,
  `hl7v2-server`, and `hl7v2-cli`.

### Documentation

- Clarified the post-collapse public package surface: `hl7v2`, `hl7v2-python`,
  `hl7v2-server`, and `hl7v2-cli`.
- Marked old implementation crate READMEs as private deprecated compatibility
  shims and pointed new Rust users to `hl7v2` module paths.
- Added the 2026-05-07 workspace-patched publish dry-run receipt for the final
  four-crate package graph.
- Recorded the direct crates.io dry-run result: `hl7v2` passes before publish,
  while dependent crates correctly wait on `hl7v2` being present in the
  crates.io index.

---

## Historical pre-v1.2.1 recovery notes - 2026-05-03

### Added

**Modernization & Integrity**
- **Rust 2024 Migration**: Entire workspace upgraded to the Rust 2024 edition.
- **MSRV 1.93**: Set minimum supported Rust version to 1.93 to leverage modern language features (`let chains`).
- **Let Chains Adoption**: Extensively refactored nested conditional logic into idiomatic Rust 2024 let chains across all core crates.
- **gRPC Service**: Fully implemented `Hl7Service` providing high-performance RPCs for Parse, Validate, GenerateAck, and Normalize.
- **Message Lifecycle**: New `hl7v2-lifecycle` crate for enterprise message retention, archival state machines, and legal hold management.
- **Statistical Guard**: Enhanced `hl7v2-guard` with statistical baseline anomaly detection and automated warmup learning periods.

**CI & Stability**
- **Deterministic Build**: Integrated `protoc-bin-vendored` for cross-platform gRPC code generation without external dependencies.
- **Network Stabilization**: Refactored E2E tests to use OS-assigned ports and oneshot channel synchronization, eliminating CI flakiness.
- **Security Hardening**: Corrected TruffleHog scan ranges and refined license regression checks to eliminate false positives in documentation.

### Fixed
- Corrected field indices for MSH metadata extraction in gRPC and Lifecycle components.
- Resolved type inference ambiguities in examples caused by recent model refactoring.
- Fixed numerous Clippy warnings including `collapsible-if`, `field-reassign-with-default`, and `io-other-error`.
- Improved gRPC error reporting for invalid profiles and malformed HL7 messages.

---

## [1.2.0] - 2026-03-04

### Added

**Production Readiness & Security**
- **HTTP Server**: Production-ready REST API built with Axum.
- **Observability**: Integrated Prometheus metrics (`/metrics`) and structured JSON tracing.
- **Security**: API Key authentication (`X-API-Key` header) and per-IP rate limiting via `tower-governor`.
- **Swagger UI**: Interactive API documentation served at `/api/docs` via OpenAPI 3.0 spec.
- **Nix Support**: Added `flake.nix` and `.envrc` for reproducible development environments.

**CLI Enhancements**
- **Streaming Parse**: High-performance, memory-efficient parsing for large files via `--streaming` flag.
- **Normalization**: Improved `--canonical-delims` support for standardizing HL7 messages.
- **Profiling**: Real-time performance monitoring and system resource tracking in CLI commands.

**Architecture (SRP Microcrate Refactoring)**
- **New Microcrates**: Extracted logic into 28 specialized crates for better maintainability and reduced dependency trees.
- **Network**: New `hl7v2-network` crate for MLLP over TCP/TLS.
- **Stream**: New `hl7v2-stream` crate for event-based parsing.
- **Validation**: New `hl7v2-validation` crate for rule-based engine.
- **Generation**: Extracted `hl7v2-ack`, `hl7v2-faker`, `hl7v2-template`, and `hl7v2-template-values`.

### Fixed
- Fixed critical infinite loop in streaming parser during partial segment reads.
- Resolved message boundary detection issues in sequential MLLP streams.
- Fixed race conditions in E2E tests caused by TCP port collisions.
- Improved error reporting for HL7 query path out-of-bounds access.
- Corrected numerous clippy lints and formatting issues across the workspace.
- Refactored server authentication to use isolated state for reliable testing.
- Fixed benchmark compatibility with newer Rust versions (criterion deprecations).

### Documentation
- Created comprehensive `docs/API_GUIDE.md` for the REST server.
- Added detailed `README.md` and `CLAUDE.md` to every microcrate.
- Documented key decisions in ADRs 0011-0014 (Security, Observability, Rules).
- Created `RELEASE_PROCESS.md` for project maintainers.

---

## [1.1.0] - 2025-11-13

### Added (v1.1.0 Features)

**Core Parsing (hl7v2-core)**
- Event-based streaming parser with delimiter switching
- MLLP frame wrapping/unwrapping
- Complete escape sequence handling (\F\, \S\, \R\, \E\, \T\)
- JSON serialization to canonical format
- Batch processing (BHS/BTS, FHS/FTS)
- Field path access API with presence semantics
- Performance benchmarks

**Profile Validation (hl7v2-prof)**
- Profile loading from YAML
- Profile inheritance with parent resolution
- Profile merging with conflict resolution
- Constraint validation (required, length, pattern)
- HL7 table value set validation
- Cross-field validation rules
- Advanced data type validation (ST, ID, CX, PN, TS, DT, TM, NM, SI, FT, TX)
- Temporal rules (date/time comparisons)
- Contextual rules (if/then logic)
- Custom validators (phone, email, SSN, birth date, checksums)

**Message Generation (hl7v2-gen)**
- Template-based message generation
- Deterministic seeding for reproducibility
- Realistic data generators:
  - Names (gender-aware)
  - Addresses (US format)
  - Phone numbers
  - Social Security Numbers
  - Medical Record Numbers
  - ICD-10 codes
  - LOINC codes
  - Medications
  - Allergens
  - Blood types
  - Ethnicity/Race
- Value distributions (fixed, lists, ranges, normal)
- Error injection (invalid segments/fields)
- Corpus generation with multi-template support
- Golden hash verification

**CLI Interface (hl7v2-cli)**
- Parse command (with JSON output, MLLP support)
- Normalize command (message normalization)
- Validate command (profile validation)
- ACK command (ACK generation with AA/AE/AR codes)
- Generate command (template-based generation)
- Interactive REPL mode

### Known Limitations (v1.1.0)

- Zero-copy parsing claims overstated (uses Vec internally)
- No backpressure/bounded channels in streaming
- No memory bounds enforcement
- No resume parsing across chunk boundaries
- No highlight escapes (\H\...\N\)
- No remote profile loading
- No server mode HTTP/gRPC
- No configuration file support
- Network module contains stubs only

See [docs/STATUS.md](docs/STATUS.md) for complete status.

### Documentation

- Created [docs/STATUS.md](docs/STATUS.md) - Transparent feature status
- Created [ROADMAP.md](ROADMAP.md) - v1.2.0-v2.0.0 roadmap and sprint-level planning
- Created [CONTRIBUTING.md](CONTRIBUTING.md) - Contributor guide
- Created [DEVELOPMENT.md](DEVELOPMENT.md) - Developer setup guide
- Created [TESTING.md](TESTING.md) - Testing procedures
- Updated README.md with accurate feature descriptions

---

## [1.0.0] - 2025-01-01 (Hypothetical)

### Initial Release

- Core HL7 v2 parsing
- Basic MLLP support
- Message normalization
- Simple JSON serialization
- Basic validation rules
- CLI interface (parse, validate, normalize)

---

## Future Releases

- Continue server sidecar hardening, Python distribution proof, profile
  conformance quality, and compatibility-shim policy cleanup as separate
  release trains.

---

## Contributing

For information about contributing changes, see [CONTRIBUTING.md](CONTRIBUTING.md).

---

## Versioning

This project follows [Semantic Versioning](https://semver.org/):

- **PATCH** (1.0.x): Bug fixes, documentation, internal improvements
- **MINOR** (1.x.0): New features, backward compatible
- **MAJOR** (x.0.0): Breaking changes, major redesigns

---

## Compatibility

### Rust Version Support

- **MSRV** (Minimum Supported Rust Version): 1.95
- **Stable**: Latest stable Rust recommended

### HL7 Versions Supported

- HL7 v2.3
- HL7 v2.4
- HL7 v2.5
- HL7 v2.5.1
- HL7 v2.7
- HL7 v2.8
- HL7 v2.9

---

## Release Notes

### v1.1.0 Release Notes

**Highlights**:
- Complete core parsing implementation
- Comprehensive profile validation
- Realistic message generation
- CLI interface for common operations

**Performance**:
- Parse: ≥100k messages/minute
- Memory: Proportional to message size
- Latency: Sub-millisecond

**Quality**:
- 87%+ test coverage
- Zero unsafe code in public APIs
- Comprehensive error handling

See [docs/STATUS.md](docs/STATUS.md) for complete feature list.

---

## Links

- [GitHub Repository](https://github.com/EffortlessMetrics/hl7v2-rs)
- [Documentation](README.md)
- [Implementation Status](docs/STATUS.md)
- [Development Roadmap](ROADMAP.md)
- [Contributing Guide](CONTRIBUTING.md)

---

## License

This project is licensed under the GNU Affero General Public License, version 3 or later
(**AGPL-3.0-or-later**). See [LICENSE](LICENSE).
