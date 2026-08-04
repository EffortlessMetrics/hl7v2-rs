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
| `tracing` | Optional tracing spans for parser entry points |
| `metrics` | Optional parser latency, size, item-count, and error metrics |
| `observability` | Enables both `tracing` and `metrics` |

### Parser observability

Enable `observability` to emit parser metrics and debug spans. Metric labels are
bounded to the parser operation (`message`, `mllp`, `batch`, or `file_batch`),
the parse outcome (`success` or `error`), and the fixed error kind vocabulary.
Message contents are never emitted as labels or span fields.

| Metric | Type | Labels | Notes |
|--------|------|--------|-------|
| `hl7v2_parser_parse_total` | Counter | `operation`, `outcome` | Every public parser entry-point attempt |
| `hl7v2_parser_parse_duration_seconds` | Histogram | `operation` | Successful and failed attempts |
| `hl7v2_parser_parse_input_bytes` | Histogram | `operation` | Raw input size, including MLLP framing when applicable |
| `hl7v2_parser_parse_items` | Histogram | `operation` | Successful message, batch-message, or file-batch counts |
| `hl7v2_parser_parse_errors_total` | Counter | `operation`, `kind` | Failed attempts with a fixed, non-sensitive error kind |

The crate uses the `metrics` facade; applications must install their preferred
recorder. The `tracing` feature adds the same entry-point boundaries as debug
spans and skips the raw input bytes.

## API layout

Common operations are available at the crate root:

```rust
use hl7v2::{ack, get, normalize, parse, validate, write};
```

For repeated queries, parse a path once and reuse it:

```rust
use hl7v2::{get_located, parse, parse_located_path};

let msg = parse(b"MSH|^~\\&|App||Fac||20250128||ORU^R01|123|P|2.5.1\rOBX|1|ST|NOTE||Alpha\r").unwrap();
let path = parse_located_path("OBX[1]-5").unwrap();
assert_eq!(get_located(&msg, &path), Some("Alpha"));
```

Implementer APIs are grouped under modules such as `hl7v2::model`,
`hl7v2::parser`, `hl7v2::writer`, `hl7v2::query`, `hl7v2::transport`,
`hl7v2::conformance`, and `hl7v2::synthetic`.

## Evidence APIs

The crate also exposes the evidence-loop building blocks used by the CLI,
server, and Python binding:

- typed validation reports through `hl7v2::ValidationReport`
- profile lint/test/explain report types under `hl7v2::conformance`
- corpus summary, fingerprint, and diff helpers under `hl7v2::synthetic::corpus`
- safe-analysis redaction helpers under `hl7v2::redact`
- evidence bundle and replay helpers under `hl7v2::evidence`

These APIs are the source of truth for the machine-readable artifacts documented
in the workspace evidence schemas and guides.

## License

AGPL-3.0-or-later
