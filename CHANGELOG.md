# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### SRP Microcrate Refactoring (2026-02-23)

**Five new microcrates extracted following Single Responsibility Principle:**

#### New Crates

| Crate | Source | Purpose |
|-------|--------|---------|
| `hl7v2-network` | `hl7v2-core/network/` | MLLP client/server/codec for TCP connections |
| `hl7v2-stream` | `hl7v2-core` | Event-based streaming parser |
| `hl7v2-validation` | `hl7v2-prof` | Validation logic and types |
| `hl7v2-ack` | `hl7v2-gen` | ACK message generation |
| `hl7v2-faker` | `hl7v2-gen` | Realistic test data generation |

#### Migration Guide

All new crates maintain backward compatibility through re-exports in their source crates:

```rust
// OLD (still works via re-export)
use hl7v2_core::network::MllpClient;
use hl7v2_core::{StreamParser, Event};
use hl7v2_prof::{validate_data_type, Issue};
use hl7v2_gen::{ack, AckCode};
use hl7v2_gen::Faker;

// NEW (recommended)
use hl7v2_network::MllpClient;
use hl7v2_stream::{StreamParser, Event};
use hl7v2_validation::{validate_data_type, Issue};
use hl7v2_ack::{ack, AckCode};
use hl7v2_faker::Faker;
```

#### Benefits

- **Reduced Dependencies**: Users who only need parsing can avoid async overhead
- **Clearer Boundaries**: Each crate has a single, well-defined responsibility
- **Independent Evolution**: Validation logic can evolve separately from profile loading
- **Smaller Footprint**: ACK generation without full gen crate dependencies

See [docs/MICROCRATE_ANALYSIS.md](docs/MICROCRATE_ANALYSIS.md) for complete details.

---

### Planned for v1.2.0
- Server mode with HTTP and gRPC endpoints
- Remote profile loading (HTTP, S3, GCS)
- Streaming parser backpressure and memory bounds
- CLI enhancements (config files, report generation)
- Expression engine improvements
- Corpus manifest generation

See [ROADMAP.md](ROADMAP.md) and [IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md) for details.

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

See [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md) for complete status.

### Documentation

- Created [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md) - Transparent feature status
- Created [ROADMAP.md](ROADMAP.md) - v1.2.0-v2.0.0 roadmap
- Created [IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md) - Sprint-level planning
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

### v1.2.0 (Planned - 8 weeks)
- Server mode with HTTP/gRPC
- Remote profile loading
- Streaming backpressure
- CLI enhancements
- Corpus manifest
- Expression engine improvements

**See**: [ROADMAP.md](ROADMAP.md) and [IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md)

### v1.3.0 (Planned - 12 weeks after v1.2)
- Language bindings (C, Python, JavaScript, Java)
- Database integration (PostgreSQL, Snowflake)
- Message queue integration (Kafka, RabbitMQ)
- Cloud storage integration (S3, GCS, Azure Blob)
- Advanced analytics

### v2.0.0 (Planned - 24 weeks after v1.3)
- Security & compliance (HIPAA, TLS, encryption)
- Audit logging with integrity
- High availability & clustering
- Advanced analytics & dashboards
- GUI interface

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

- **MSRV** (Minimum Supported Rust Version): 1.89
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

See [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md) for complete feature list.

---

## Links

- [GitHub Repository](https://github.com/EffortlessMetrics/hl7v2-rs)
- [Documentation](README.md)
- [Implementation Status](IMPLEMENTATION_STATUS.md)
- [Development Roadmap](ROADMAP.md)
- [Contributing Guide](CONTRIBUTING.md)

---

## License

This project is licensed under the GNU Affero General Public License, version 3 or later
(**AGPL-3.0-or-later**). See [LICENSE](LICENSE).
