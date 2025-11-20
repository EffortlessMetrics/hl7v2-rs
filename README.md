# hl7v2-rs

Modern Rust HL7v2 Processor

A fast, safe, and deterministic HL7 v2 parser, validator, and generator written in Rust.

> **Note**: This project is in active development. For a detailed breakdown of implemented vs. planned features, see [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md).

## Features

- Parse, normalize, and validate HL7 v2.x messages
- Canonical JSON view with round-trip preservation
- Conformance profile validation
- Deterministic synthetic message generation
- No AI dependencies
- Lockable corpora (synthetic + optional real)

## Crates

- `hl7v2-core`: Core parsing and data model
- `hl7v2-prof`: Profile validation
- `hl7v2-gen`: Message generation
- `hl7v2-cli`: Command-line interface
- `hl7v2-server`: HTTP/REST API server

## Installation

### From source

```bash
git clone https://github.com/EffortlessMetrics/hl7v2-rs.git
cd hl7v2-rs
cargo install --path crates/hl7v2-cli
```

### From crates.io (when published)

```bash
cargo install hl7v2-cli
```

## Quick Start

### HTTP/REST API Server

The fastest way to get started is with the HTTP API server:

```bash
# Start the server
cargo run --bin hl7v2-server

# Or with custom configuration
HL7V2_HOST=0.0.0.0 HL7V2_PORT=8080 cargo run --bin hl7v2-server
```

**Parse a message via HTTP:**
```bash
curl -X POST http://localhost:8080/hl7/parse \
  -H "Content-Type: application/json" \
  -d '{
    "message": "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20231119120000||ADT^A01|MSG001|P|2.5\rPID|1||MRN123||Doe^John||19800101|M",
    "mllp_framed": false,
    "options": {
      "include_json": true
    }
  }'
```

**Validate a message against a profile:**
```bash
curl -X POST http://localhost:8080/hl7/validate \
  -H "Content-Type": application/json" \
  -d '{
    "message": "MSH|^~\\&|...",
    "profile": "..."  # YAML profile content
  }'
```

**Check server health:**
```bash
curl http://localhost:8080/health
curl http://localhost:8080/ready
curl http://localhost:8080/metrics  # Prometheus metrics
```

See the [OpenAPI specification](schemas/openapi/hl7v2-api.yaml) for complete API documentation.

### CLI Tools

### Parse HL7 Messages

```bash
# Parse an HL7 message and output canonical JSON
hl7v2 parse <input.hl7> --json > output.json

# Parse MLLP-framed messages
hl7v2 parse <input.mllp> --mllp --json > output.json
```

### Validate Messages

```bash
# Validate against a profile (supports profile inheritance)
hl7v2 val <input.hl7> --profile profiles/oru_r01.yaml

# (Planned) Emit a JSON validation report
# hl7v2 val <input.hl7> --profile profiles/oru_r01.yaml --report validation_errors.json
# See IMPLEMENTATION_STATUS.md for current CLI flag support.
```

### Normalize Messages

```bash
# Normalize an HL7 message
hl7v2 norm <input.hl7> > output.hl7
```

### Generate Messages

```bash
# Generate synthetic HL7 messages with deterministic seeding
hl7v2 gen --profile profiles/oru_r01.yaml --seed 1337 --count 100 --out corpus/

# Generate with different template
hl7v2 gen --template templates/adt_a01.yaml --seed 42 --count 50 --out test_data/
```

### Acknowledgment Generation

```bash
# Generate an application ACK (AA - Application Accept)
hl7v2 ack <input.hl7> --code AA > ack.hl7

# Generate an application error ACK (AE - Application Error)
hl7v2 ack <input.hl7> --code AE > error_ack.hl7
```

## Key Features

### Core Parsing (hl7v2-core)

- **Fast, safe parsing**: Written in Rust with zero unsafe code in public APIs
- **Event-based streaming parser**: Process HL7 messages as a sequence of events
- **Escape sequence handling**: Full support for HL7 v2 escape sequences (\F\, \S\, \R\, \E\, \T\)
- **MLLP transport**: Complete MLLP frame parsing and generation
- **Batch processing**: Full support for FHS/BHS/BTS/FTS batch and file batch structures
- **JSON serialization**: Convert messages to canonical JSON format
- **Field path access**: Query message fields with path notation (e.g., "PID.5[1].1")

### Profile Validation (hl7v2-prof)

- **Profile inheritance**: Load and compose profiles with parent resolution and merging
- **Comprehensive validation rules**:
  - Constraint validation (required fields, patterns, lengths)
  - HL7 table value set validation with custom tables
  - Cross-field conditional rules (requires, prohibits, validates)
  - Advanced data type validation (CX, PN, TS, DT, TM, etc.)
  - Custom validation patterns (regex, checksums, formats)
  - Temporal rules for date/time comparisons
  - Contextual rules with if/then logic
- **Local profile loading**: Load YAML-based profiles from files
- **Flexible rule composition**: Merge profiles with child precedence

### Synthetic Message Generation (hl7v2-gen)

- **Template-based generation**: Define message templates with variable value sources
- **Realistic data generators**: Names (gender-aware), addresses, phone numbers, SSNs, MRNs, ICD-10, LOINC codes
- **Value distributions**: Fixed values, value lists, numeric ranges, dates, normal distributions
- **Deterministic seeding**: Same seed + template = identical output for regression testing
- **Error injection**: Generate invalid messages with segmentation/format errors for testing
- **Corpus tools**: Generate collections with golden hash verification for test data reproducibility

### CLI Interface (hl7v2-cli)

- **Unified command interface**: parse, normalize, validate, acknowledge, generate
- **Input/output formats**: Raw HL7, JSON, MLLP framing
- **Interactive mode**: Command-line REPL for exploratory use
- **File I/O**: Read from files or stdin, write to files or stdout

### HTTP/REST API Server (hl7v2-server)

- **RESTful API**: Parse and validate HL7 messages over HTTP
- **Health & Readiness**: Production-ready health checks
- **Prometheus metrics**: Request counts, latencies, error rates
- **Concurrency limiting**: Built-in backpressure (100 concurrent requests default)
- **CORS support**: Cross-origin requests enabled
- **Compression**: Gzip compression for responses
- **OpenAPI 3.0 spec**: Complete API documentation at `schemas/openapi/hl7v2-api.yaml`
- **Docker ready**: Containerized deployment with Nix-built images
- **Kubernetes ready**: Helm charts and manifests in `infrastructure/k8s/`

See [DEPLOYMENT.md](DEPLOYMENT.md) for production deployment guide.

## Architecture

The project uses a modular crate-based architecture for flexible integration:

```
┌─────────────────┐
│   hl7v2-cli     │  Command-line interface
├─────────────────┤
│                 │
│  hl7v2-core   │  Core parsing, validation, serialization
│  hl7v2-prof   │  Profile validation with inheritance
│  hl7v2-gen    │  Synthetic message generation
└─────────────────┘
```

Each crate is independently usable as a library, enabling integration into other Rust projects.

## Performance Characteristics

- Parsing throughput: ≥100k small messages/minute on NVMe (typical ADT/ORU ~200 bytes)
- Large messages: ≥10k messages/minute for ~2 KB messages in batch mode
- Memory usage: bounded; no unbounded growth in the streaming parser for typical workloads
- Determinism: 100% reproducible generation with the same seed + template
- Latency: sub-millisecond parsing for typical messages on a modern CPU

For exact benchmark numbers and hardware, see [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md).

## HL7 Standards Compliance

- **Version Support**: HL7 v2.3 through v2.9
- **Encoding Rules**: Full support for standard HL7 delimiters and escape sequences
- **Message Types**: Support for all common message types (ADT, ORU, ORM, RGV, etc.)
- **Segment Handling**: Complete segment parsing and validation
- **Field Types**: Support for all HL7 v2 field data types

## Use Cases

- **Healthcare Data Integration**: Parse and validate messages from clinical systems
- **Message Transformation**: Convert between HL7 and JSON for API integration
- **Data Quality Testing**: Generate synthetic test corpora for system validation
- **Compliance Validation**: Ensure messages meet organizational standards
- **Message Monitoring**: Validate and process messages in production pipelines

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
