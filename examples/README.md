# HL7 v2 Rust Examples

This directory contains comprehensive usage examples for the hl7v2-rs library. Each example demonstrates specific functionality and best practices for working with HL7 v2 messages.

## Running Examples

All examples can be run using Cargo:

```bash
# Run a specific example
cargo run --example <example_name>

# For example:
cargo run --example parsing_basics
```

## Available Examples

### 1. [`parsing_basics.rs`](./parsing_basics.rs)

**Topics covered:**
- Parse a simple HL7 message from string
- Access segments and fields using path-based queries
- Handle parsing errors properly

**Dependencies:** `hl7v2-parser`, `hl7v2-core`

```bash
cargo run --example parsing_basics
```

### 2. [`message_building.rs`](./message_building.rs)

**Topics covered:**
- Build a message programmatically using the data model
- Add segments and fields
- Set custom delimiters
- Serialize the message to bytes

**Dependencies:** `hl7v2-core`

```bash
cargo run --example message_building
```

### 3. [`mllp_client.rs`](./mllp_client.rs)

**Topics covered:**
- Connect to MLLP server
- Send message and receive ACK
- Handle timeouts and errors
- Configure connection parameters

**Dependencies:** `hl7v2-mllp`, `hl7v2-network`, `hl7v2-parser`

```bash
# Note: Requires a running MLLP server
# Start a test server first:
cargo run --package hl7v2-cli -- serve --port 2575

# Then run the client:
cargo run --example mllp_client
```

### 4. [`validation_basics.rs`](./validation_basics.rs)

**Topics covered:**
- Load a profile from YAML
- Validate a message against profile
- Handle validation errors
- Work with validation results

**Dependencies:** `hl7v2-validation`, `hl7v2-prof`

```bash
cargo run --example validation_basics
```

### 5. [`batch_processing.rs`](./batch_processing.rs)

**Topics covered:**
- Read batch file with multiple messages
- Process each message individually
- Write results
- Handle batch headers/trailers (FHS/BHS/FTS/BTS)

**Dependencies:** `hl7v2-batch`, `hl7v2-parser`

```bash
cargo run --example batch_processing
```

### 6. [`streaming_parser.rs`](./streaming_parser.rs)

**Topics covered:**
- Parse large files incrementally
- Handle backpressure with async streaming
- Process messages as they're parsed
- Memory-efficient processing

**Dependencies:** `hl7v2-stream`, `hl7v2-parser`

```bash
cargo run --example streaming_parser
```

### 7. [`template_generation.rs`](./template_generation.rs)

**Topics covered:**
- Load template from YAML
- Generate message with dynamic values
- Use various value sources (fixed, random, UUID, etc.)
- Batch generation with deterministic seeds

**Dependencies:** `hl7v2-template`, `hl7v2-template-values`

```bash
cargo run --example template_generation
```

### 8. [`ack_generation.rs`](./ack_generation.rs)

**Topics covered:**
- Generate ACK for received message
- Handle accept/reject scenarios
- Include error details in rejection ACKs
- All ACK code types (AA, AE, AR, CA, CE, CR)

**Dependencies:** `hl7v2-ack`, `hl7v2-parser`

```bash
cargo run --example ack_generation
```

## Example Categories

### Basic Usage
- `parsing_basics.rs` - Start here if you're new to the library
- `message_building.rs` - Learn how to construct messages

### Networking
- `mllp_client.rs` - MLLP client for TCP communication

### Validation
- `validation_basics.rs` - Profile-based message validation

### Advanced Processing
- `batch_processing.rs` - Handle multiple messages in batches
- `streaming_parser.rs` - Memory-efficient processing for large files

### Message Generation
- `template_generation.rs` - Generate synthetic messages from templates
- `ack_generation.rs` - Create acknowledgment messages

## Common Patterns

### Error Handling

All examples follow proper error handling practices. The recommended pattern is:

```rust
use hl7v2_core::{parse, Error};

fn process_message(bytes: &[u8]) -> Result<String, Error> {
    let message = parse(bytes)?;
    
    // Process message...
    
    Ok("Success".to_string())
}
```

### Path-Based Field Access

```rust
use hl7v2_core::{parse, get};

let message = parse(hl7_bytes)?;
let patient_name = get(&message, "PID.5.1");  // Family name
let mrn = get(&message, "PID.3.1");           // Patient ID
```

### ACK Generation

```rust
use hl7v2_core::parse;
use hl7v2_ack::{ack, AckCode};

let original = parse(hl7_bytes)?;
let ack_message = ack(&original, AckCode::AA)?;
```

## Profile Examples

The `profiles/` directory contains example validation profiles:

- `minimal.yaml` - Basic profile with required field checks
- `ADT_A01.yaml` - Profile for ADT^A01 messages
- `ADT_A04.yaml` - Profile for ADT^A04 messages
- `ORU_R01.yaml` - Profile for ORU^R01 (lab result) messages

## Test Data

The `../test_data/` directory contains sample HL7 messages for testing:

- `test_message.hl7` - Simple test message
- `valid_message.hl7` - Valid ADT message
- `invalid_message.hl7` - Message with errors for testing validation

## Feature Flags

Some examples require specific features to be enabled:

```toml
# Cargo.toml
[dependencies]
hl7v2-core = { version = "0.1.0", features = ["network", "stream"] }
```

- `network` - Enables MLLP client/server functionality
- `stream` - Enables streaming parser

## Troubleshooting

### "Cannot find example" error

Make sure you're running from the workspace root:

```bash
cd /path/to/hl7v2-rs
cargo run --example parsing_basics
```

### MLLP client connection refused

Start a test server first:

```bash
cargo run --package hl7v2-cli -- serve --port 2575
```

### Feature not found

Enable required features in your `Cargo.toml`:

```toml
hl7v2-core = { version = "0.1.0", features = ["network"] }
```

## Contributing

When adding new examples:

1. Follow the existing code structure
2. Include comprehensive comments
3. Handle errors properly (no `unwrap()` in production code)
4. Add the example to this README
5. Update the workspace `Cargo.toml` with the example entry

## License

These examples are part of the hl7v2-rs project and are licensed under the same terms (AGPL-3.0-or-later).
