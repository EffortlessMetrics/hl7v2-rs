# hl7v2-redact

PHI (Protected Health Information) redaction for HL7 v2 messages.

## Overview

This crate provides functionality to redact sensitive patient information from HL7 v2 messages, making them safe for logging, testing, and non-production environments while maintaining HIPAA compliance.

## Features

- **Multiple redaction strategies**: Hash, mask, replace, remove, or truncate PHI fields
- **Pre-configured rules**: Common PHI rules and HIPAA Safe Harbor rules
- **Audit logging**: Track all redaction operations for compliance
- **Custom rules**: Define your own redaction logic

## Usage

```rust
use hl7v2_redact::{RedactionEngine, RedactionRule, RedactionStrategy};
use hl7v2_parser::parse;

let message = parse(hl7_bytes)?;

// Apply common PHI redaction rules
let engine = RedactionEngine::common_phi_rules();
let result = engine.redact(&message)?;

// The redacted message is now safe for logging
println!("Redacted: {:?}", result.message_bytes);

// Review the audit log
for entry in &result.audit_log {
    println!("Redacted {} using {:?}", entry.path, entry.strategy);
}
```

## Redaction Strategies

- `Replace(String)`: Replace with a fixed value
- `Hash`: SHA-256 hash of original value
- `Mask`: Mask with asterisks (e.g., `J**n`)
- `Remove`: Empty the field
- `Truncate(usize)`: Keep first N chars, mask rest
- `Custom(fn(&str) -> String)`: Custom transformation

## Pre-configured Rule Sets

### Common PHI Rules
Redacts standard PHI fields like patient ID, name, DOB, address, phone, SSN.

### HIPAA Safe Harbor
Implements the 18 identifiers specified in HIPAA Safe Harbor method for de-identification.

## License

AGPL-3.0-or-later
