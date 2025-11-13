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

# Parse MLLP-framed messages
hl7v2 parse <input.mllp> --mllp --json > output.json
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

# Generate with different template
hl7v2 gen --template templates/adm_a01.yaml --seed 42 --count 50 --out test_data/
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

- **Parsing throughput**: ≥100k small messages/minute on NVMe (typical ORU/ADT ~200 bytes)
- **Memory usage**: Proportional to message size; batch operations use bounded memory
- **Determinism**: 100% reproducible message generation with same seed
- **Latency**: Sub-millisecond for typical messages

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
