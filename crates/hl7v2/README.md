# hl7v2

HL7 v2 message parser and processor for Rust.

This is the canonical entry point for the
[`hl7v2-rs`](https://github.com/EffortlessMetrics/hl7v2-rs) workspace.

## Quick start

```rust
use hl7v2::{parse, get};

let msg = parse(b"MSH|^~\\&|App||Fac||20250128||ADT^A01|123|P|2.5.1\rPID|1||PAT123||Doe^John\r").unwrap();
assert_eq!(get(&msg, "PID.5.1"), Some("Doe"));
```

## Features

| Feature | Description |
|---------|-------------|
| `stream` | Streaming/event-based parser |
| `network` | Async MLLP client/server (TCP/TLS) |

## Microcrates

For finer-grained dependencies, use the individual crates directly:

| Crate | Purpose |
|-------|---------|
| `hl7v2-model` | Core data types |
| `hl7v2-parser` | Message parsing |
| `hl7v2-writer` | Message serialization |
| `hl7v2-escape` | Escape sequence handling |
| `hl7v2-mllp` | MLLP framing |
| `hl7v2-normalize` | Message normalization |

## License

AGPL-3.0-or-later
