# hl7v2

HL7 v2 message parser and processor for Rust.

This is the canonical entry point for the
[`hl7v2-rs`](https://github.com/EffortlessMetrics/hl7v2-rs) workspace.
Use this crate for normal Rust library integration.

## Quick start

```rust
use hl7v2::{parse, get};

let msg = parse(b"MSH|^~\\&|App||Fac||20250128||ADT^A01|123|P|2.5.1\rPID|1||PAT123||Doe^John\r").unwrap();
assert_eq!(get(&msg, "PID.5.1"), Some("Doe"));
```

## Features

| Feature | Description |
|---------|-------------|
| `json` | JSON serialization helpers |
| `profile` | Profile loading and conformance validation |
| `ack` | ACK message generation |
| `normalize` | Message normalization |
| `batch` | Batch parsing and writing helpers |
| `stream` | Streaming/event-based parser |
| `network` | Async MLLP client/server (TCP/TLS) |
| `synthetic` | Template, faker, corpus, and generation APIs |
| `redact` | Redaction helpers |
| `lifecycle` | Lifecycle and archive metadata helpers |
| `experimental-guard` | Experimental guard/anomaly detection APIs |

## API layout

Common operations are available at the crate root:

```rust
use hl7v2::{ack, get, normalize, parse, validate, write};
```

Implementer APIs are grouped under modules such as `hl7v2::model`,
`hl7v2::parser`, `hl7v2::writer`, `hl7v2::query`, `hl7v2::transport`,
`hl7v2::conformance`, and `hl7v2::synthetic`.

## License

AGPL-3.0-or-later
