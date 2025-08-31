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

```bash
# TODO: Add installation instructions
```

## Usage

```bash
# Parse an HL7 message and output JSON
hl7v2 parse <input.hl7> --json > output.json

# Normalize an HL7 message
hl7v2 norm <input.hl7> > output.hl7

# Validate against a profile
hl7v2 val <input.hl7> --profile profiles/oru_r01.yaml

# Generate an ACK
hl7v2 ack <input.hl7> --code AA > ack.hl7

# Generate synthetic messages
hl7v2 gen --profile profiles/oru_r01.yaml --seed 1337 --count 100 --out corpus/
```

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
