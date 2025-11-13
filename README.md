# hl7v2-rs

Modern Rust HL7v2 Processor

A fast, safe, and deterministic HL7 v2 parser, validator, and generator written in Rust.

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

### Parse HL7 Messages

```bash
# Parse an HL7 message and output canonical JSON
hl7v2 parse <input.hl7> --json > output.json

# Parse multiple messages with streaming (large files)
hl7v2 parse <large_file.hl7> --streaming --json > output.jsonl
```

### Validate Messages

```bash
# Validate against a profile (supports profile inheritance)
hl7v2 val <input.hl7> --profile profiles/oru_r01.yaml

# Generate validation report with detailed errors
hl7v2 val <input.hl7> --profile profiles/oru_r01.yaml --report validation_errors.json
```

### Normalize Messages

```bash
# Normalize an HL7 message
hl7v2 norm <input.hl7> > output.hl7

# Normalize with canonical delimiters
hl7v2 norm <input.hl7> --canonical-delims > output.hl7
```

### Generate Messages

```bash
# Generate synthetic HL7 messages with deterministic seeding
hl7v2 gen --profile profiles/oru_r01.yaml --seed 1337 --count 100 --out corpus/

# Generate with advanced statistical distributions
hl7v2 gen --profile profiles/oru_r01.yaml --seed 1337 --count 100 --out corpus/ --distributions
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
- **Streaming parser**: Memory-efficient processing of large HL7 messages
- **Zero-copy semantics**: Minimize memory allocations and copies
- **Escape sequence handling**: Proper support for all HL7 v2 escape sequences
- **MLLP transport**: Built-in support for Minimal Lower Layer Protocol framing
- **Batch processing**: Handle FHS/BHS/BTS/FTS batch structures
- **Deterministic outputs**: Consistent, reproducible message parsing

### Profile Validation (hl7v2-prof)

- **Profile inheritance**: Compose profiles from parent profiles with conflict resolution
- **Advanced validation rules**:
  - Constraint validation (required fields, patterns, lengths)
  - Value set validation against HL7 tables
  - Cross-field conditional rules
  - Custom expression-based rules
  - Temporal and contextual validation
- **Dynamic profile loading**: Load profiles from local files or remote sources
- **Flexible rule composition**: Merge and override validation rules

### Synthetic Message Generation (hl7v2-gen)

- **Template-based generation**: Define templates for reproducible test data
- **Realistic data**: Generate names, addresses, phone numbers, medical identifiers
- **Statistical distributions**: Support for uniform, normal, and categorical distributions
- **Deterministic seeding**: Same seed produces identical output
- **Error injection**: Generate intentionally malformed messages for testing
- **Corpus management**: Create diverse test datasets with metadata tracking

### CLI Interface (hl7v2-cli)

- **Unified command interface**: Single tool for all HL7 processing tasks
- **Multiple input/output formats**: Support for raw, JSON, NDJSON formats
- **Batch operations**: Process multiple files efficiently
- **Configuration files**: Optional TOML config for production deployments

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

- **Parsing**: ≥100k messages/minute (small messages on NVMe)
- **Memory**: Streaming parser uses constant memory regardless of input size
- **Determinism**: 100% reproducible outputs with identical seeds
- **Latency**: Sub-millisecond parsing for typical HL7 messages

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
