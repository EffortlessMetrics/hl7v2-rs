# hl7v2-python

Python bindings for the hl7v2-rs library via PyO3.

## Overview

This crate provides Python bindings for parsing, validating, normalizing, and generating HL7 v2 messages using the Rust hl7v2-rs library.

## Features

- **Parse**: Parse HL7 v2 messages from strings
- **Validate**: Validate messages against HL7 version specifications
- **Normalize**: Normalize messages with canonical delimiters
- **Generate**: Generate HL7 strings from Message objects
- **Batch Processing**: Parse HL7 batch messages containing multiple messages

## Python API

```python
import hl7v2

# Parse an HL7 message
message = hl7v2.parse("MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r")

# Access message properties
print(message.message_type())  # "ADT"
print(message.trigger_event())  # "A01"
print(message.version())  # "2.5.1"
print(message.segment_count())  # 2

# Get field values
print(message.get("PID.5.1"))  # "Doe" (patient last name)
print(message.get("PID.5.2"))  # "John" (patient first name)

# Validate the message
is_valid = hl7v2.validate(message, "2.5.1")

# Normalize the message
normalized = hl7v2.normalize(message)

# Generate HL7 string
hl7_string = hl7v2.generate(message)

# Convert to JSON
json_str = message.to_json()

# Parse a batch
messages = hl7v2.parse_batch(batch_string)
```

## Building

### Development Build

```bash
cargo build -p hl7v2-python
```

### Python Wheel (requires maturin)

```bash
cd crates/hl7v2-python
maturin build --release
```

## Testing

```bash
cargo test -p hl7v2-python
```

## Requirements

- Python 3.8+
- Rust 1.92+

## License

AGPL-3.0-or-later
