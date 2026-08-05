# hl7v2-rs

[![CI](https://github.com/EffortlessMetrics/hl7v2-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/EffortlessMetrics/hl7v2-rs/actions/workflows/ci.yml)
[![Codecov](https://codecov.io/gh/EffortlessMetrics/hl7v2-rs/branch/main/graph/badge.svg)](https://codecov.io/gh/EffortlessMetrics/hl7v2-rs)
[![MSRV](https://img.shields.io/badge/MSRV-1.95-blue.svg)](https://doc.rust-lang.org/cargo/reference/manifest.html#the-rust-version-field)
[![License: AGPL-3.0-or-later](https://img.shields.io/badge/license-AGPL--3.0--or--later-blue.svg)](LICENSE)
[![ripr](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/EffortlessMetrics/hl7v2-rs/main/badges/ripr.json)](docs/VERIFICATION.md)
[![ripr+](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/EffortlessMetrics/hl7v2-rs/main/badges/ripr-plus.json)](docs/VERIFICATION.md)

Modern Rust HL7v2 Processor

A fast, safe, and deterministic HL7 v2 parser, validator, and generator written in Rust.

> **Status**: v1.5.0 is published to crates.io for `hl7v2`, `hl7v2-python`, `hl7v2-server`, and `hl7v2-cli`. `hl7v2-python` is binding backend infrastructure for the public Python `hl7v2` package, not the recommended Rust API. Public Python TestPyPI/PyPI proof remains a separate lane. For a detailed breakdown of features, see [docs/STATUS.md](docs/STATUS.md).

## Feature Status

| Layer | Status | Evidence |
| --- | --- | --- |
| **Parser / Core** | Stable | Main CI, workspace tests, and contract checks green after the `hl7v2` facade and foundation-module collapse |
| **Writer / Normalize** | Stable | Writer tests plus HTTP/gRPC normalization contract coverage |
| **MLLP / Network** | Stable | MLLP parse/framing tests and CI matrix coverage |
| **REST Server** | Stable | Runtime and OpenAPI agree for parse, validate, redacted validation, bundle/replay, ACK, normalize, inline corpus evidence, readiness, and redacted structured logs |
| **gRPC Service** | Beta | `hl7v2-cli serve --mode grpc` starts the Tonic service; contract tests cover Parse, Validate, ProfileLint, ProfileExplain, ProfileTest, ValidateRedacted, CreateEvidenceBundle, ReplayEvidenceBundle, CorpusSummarize, CorpusFingerprint, CorpusDiff, GenerateAck, Normalize, HealthCheck, and ParseStream as one request message into one response message |
| **Lifecycle** | Beta | Domain tests exist, but lifecycle is not part of the current HTTP/gRPC contract gate |
| **Guard / Anomaly** | Experimental | Statistical baseline fixtures exist; not a stable runtime contract |
| **Profile Cache** | L1-only | In-memory verified; Postgres L2 pending |
| **Python Bindings** | Experimental | Separate maturin lane with wheel build/install/import smoke coverage; backend crate `hl7v2-python` is published as binding infrastructure, while public Python `hl7v2` still needs TestPyPI/PyPI proof |
| **Publish Readiness** | Published | v1.5.0 is published to crates.io for `hl7v2`, `hl7v2-python`, `hl7v2-server`, and `hl7v2-cli`; see the publish receipt in `docs/audits/` |
| **Evidence Loop** | Stable | v1.5.0 is the published Rust 1.95 quality-ratchet release; it keeps the evidence-contract surface and adds tighter policy rails, advisory ripr, targeted mutation, and release-readiness proof |

## Features

- Parse, normalize, and validate HL7 v2.x messages
- Canonical JSON view with round-trip preservation
- Conformance profile validation
- Deterministic synthetic message generation
- No AI dependencies
- Lockable corpora (synthetic + optional real)

## Crates

The primary Rust product surface is intentionally small:

| Crate | Role |
| --- | --- |
| `hl7v2` | Canonical Rust library crate. Normal Rust users should depend on this crate. |
| `hl7v2-server` | HTTP/gRPC runtime service with Axum, Tonic, metrics, auth, and deployment behavior. |
| `hl7v2-cli` | Command-line binary distribution. |
| `hl7v2-python` | PyO3 binding backend for the public `hl7v2` Python distribution. It is not the recommended Rust API and is not included in the primary Rust product graph. |

Binding backend crates such as `hl7v2-python`, future `hl7v2-wasm`, and future
`hl7v2-node` may become publishable implementation surfaces for language
packages. They are not a reason to split parser/model/redaction/MLLP behavior
back into public Rust microcrates. Future TypeScript users should install
`@effortlessmetrics/hl7v2`, not `hl7v2-rs`; see
[`HL7V2-SPEC-0005`](docs/specs/HL7V2-SPEC-0005-npm-wasm-binding-package-model.md).

Implementation boundaries live as modules under `hl7v2`, including
`hl7v2::model`, `hl7v2::parser`, `hl7v2::writer`, `hl7v2::query`,
`hl7v2::transport`, `hl7v2::conformance`, `hl7v2::synthetic`,
`hl7v2::lifecycle`, and `hl7v2::experimental`.

Old implementation microcrate package names may exist historically on
crates.io, but their local shim folders have been retired from this repository.
Those names are compatibility artifacts, not the product surface for new code.

## Installation

### From source

```bash
git clone https://github.com/EffortlessMetrics/hl7v2-rs.git
cd hl7v2-rs
cargo install --path crates/hl7v2-cli
```

### From crates.io

```bash
cargo add hl7v2
cargo install hl7v2-cli
cargo install hl7v2-server
```

## Quick Start

If you are choosing an install/runtime surface first, start with
[First Use By Surface](docs/guides/first-use-by-surface.md). It routes Rust,
CLI, server, and Python users to the first useful receipt without requiring
the workspace topology.

For a task-focused walkthrough from local diagnostics to validation reports,
corpus fingerprint/diff output, and replayable redacted bundles, start with the
[First 10 Minutes guide](docs/guides/first-10-minutes.md).
For the single message-to-replay path across current surfaces, use the
[Full Evidence Receipt Path](docs/guides/full-evidence-receipt-path.md).
To interpret validation reports, redaction receipts, corpus fingerprints,
bundles, replay output, and PHI-sharing posture, use
[Evidence Artifacts For Operators](docs/guides/evidence-artifacts-for-operators.md).
For migration and vendor-change review, see the
[Vendor Upgrade Diff guide](docs/guides/vendor-upgrade-diff.md).
For support escalation without raw message PHI, see the
[Safe Support Bundle guide](docs/guides/safe-support-bundle.md).
For sidecar deployment, see the
[Deploy Validation Sidecar guide](docs/guides/deploy-validation-sidecar.md).
For the full documentation map, including current sources of truth and
historical receipts, see [docs/README.md](docs/README.md).

### HTTP/REST API Server

The fastest way to get started is with the HTTP API server:

```bash
# Start the server
hl7v2-server

# Or with custom configuration
BIND_ADDRESS=0.0.0.0:8080 hl7v2-server

# Inspect sanitized effective configuration and exit
hl7v2-server --print-config
```

From a source checkout, use `cargo run -q -p hl7v2-server --` before the
server arguments instead.

**Parse a message via HTTP:**
```bash
curl -X POST http://localhost:8080/hl7/parse \
  -H "X-API-Key: your-secret-key" \
  -H "Content-Type: application/json" \
  -d '{
    "message": "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20231119120000||ADT^A01|MSG001|P|2.5\rPID|1||MRN123||Doe^John||19800101|M"
  }'
```

**Validate a message against a profile:**
```bash
curl -X POST http://localhost:8080/hl7/validate \
  -H "X-API-Key: your-secret-key" \
  -H "Content-Type: application/json" \
  -d '{
    "message": "MSH|^~\\&|...",
    "profile": "..."
  }'
```

**Validate after safe-analysis redaction:**
```bash
curl -X POST http://localhost:8080/hl7/validate-redacted \
  -H "X-API-Key: your-secret-key" \
  -H "Content-Type: application/json" \
  -d '{
    "message": "MSH|^~\\&|...",
    "profile": "...",
    "redaction_policy": "[[rules]]\npath = \"PID.3\"\naction = \"hash\"\nreason = \"hash patient identifier\"\n"
  }'
```

If `[quarantine]` is enabled in `HL7V2_CONFIG`, failed
`/hl7/validate-redacted` requests write configured quarantine artifacts under
the server-controlled quarantine root and include a root-relative `quarantine`
summary in the response.

**Generate a policy-driven ACK/NAK:**
```bash
curl -X POST http://localhost:8080/hl7/ack-policy \
  -H "X-API-Key: your-secret-key" \
  -H "Content-Type: application/json" \
  -d '{
    "message": "MSH|^~\\&|...",
    "profile": "..."
  }'
```

**Create a redacted evidence bundle:**
```bash
# Requires HL7V2_BUNDLE_OUTPUT_ROOT to point at an existing writable directory.
curl -X POST http://localhost:8080/hl7/bundle \
  -H "X-API-Key: your-secret-key" \
  -H "Content-Type: application/json" \
  -d '{
    "bundle_id": "case-001",
    "message": "MSH|^~\\&|...",
    "profile": "...",
    "redaction_policy": "[[rules]]\npath = \"PID.3\"\naction = \"hash\"\nreason = \"hash patient identifier\"\n"
  }'
```

**Check server health:**
```bash
curl http://localhost:8080/health
curl http://localhost:8080/ready
curl http://localhost:8080/metrics  # Prometheus metrics
```

See the [OpenAPI specification](api/openapi/hl7v2-api-v1.yaml) for complete API documentation.
Server corpus endpoints take inline message arrays and safe labels; they do not
read filesystem paths supplied in request bodies.
Server evidence workflow logs are structured and PHI-conscious. Set
`RUST_LOG_FORMAT=json` for JSON records; logged identifiers such as message
control IDs and bundle IDs are hashed, and raw HL7, profile YAML, redaction
policy TOML, and configured filesystem roots are not logged by default.

### CLI Tools

### Parse HL7 Messages

```bash
# Parse an HL7 message and output canonical JSON
hl7v2-cli parse <input.hl7> --json > output.json

# Parse MLLP-framed messages
hl7v2-cli parse <input.mllp> --mllp --json > output.json
```

### Validate Messages

```bash
# Validate against a profile (supports profile inheritance)
hl7v2-cli val <input.hl7> --profile profiles/oru_r01.yaml

# Emit a machine-readable validation report
hl7v2-cli val <input.hl7> --profile profiles/oru_r01.yaml --report json

# Lint a profile before using it as an interface contract
hl7v2-cli profile lint profiles/oru_r01.yaml --report json

# Explain the loaded profile contract
hl7v2-cli profile explain profiles/oru_r01.yaml --format json

# Test profile fixtures as executable interface contracts
hl7v2-cli profile test profiles/oru_r01.yaml fixtures/oru_r01/ --report json
```

### Normalize Messages

```bash
# Normalize an HL7 message
hl7v2-cli norm <input.hl7> > output.hl7
```

### Redact Messages

```bash
# Redact PHI while retaining deterministic analysis evidence
hl7v2-cli redact <input.hl7> --policy safe-analysis.toml --format json
```

### Bundle Evidence

```bash
# Create a PHI-safe evidence packet for support or replay
hl7v2-cli support-bundle failing.hl7 --profile profiles/oru_r01.yaml --redact-policy safe-analysis.toml --out issue-bundle/

# Re-run the redacted bundle and verify the stored validation report reproduces
hl7v2-cli replay issue-bundle/ --format json
```

`hl7v2-cli bundle` remains available as the compatibility command.

### Generate Messages

```bash
# Generate synthetic HL7 messages with deterministic seeding
hl7v2-cli gen --profile profiles/oru_r01.yaml --seed 1337 --count 100 --out corpus/

# Generate with different template
hl7v2-cli gen --template templates/adt_a01.yaml --seed 42 --count 50 --out test_data/
```

### Summarize Corpora

```bash
# Summarize a directory or single-file corpus of HL7 messages
hl7v2-cli corpus summarize corpus/

# Emit a machine-readable corpus summary
hl7v2-cli corpus summarize corpus/ --format json

# Create a deterministic feed fingerprint
hl7v2-cli corpus fingerprint corpus/ --format json

# Include validation issue-code counts in the fingerprint
hl7v2-cli corpus fingerprint corpus/ --profile profiles/oru_r01.yaml --format json

# Compare before/after corpora for feed drift
hl7v2-cli corpus diff feeds/before feeds/after --profile profiles/oru_r01.yaml --format json
```

### Acknowledgment Generation

```bash
# Generate an application ACK (AA - Application Accept)
hl7v2-cli ack <input.hl7> --code AA > ack.hl7

# Generate an application error ACK (AE - Application Error)
hl7v2-cli ack <input.hl7> --code AE > error_ack.hl7
```

## Key Features

### Core Parsing (`hl7v2`)

- **Fast, safe parsing**: Written in Rust with zero unsafe code in public APIs
- **Event-based streaming parser**: Process HL7 messages as a sequence of events
- **Escape sequence handling**: Full support for HL7 v2 escape sequences (\F\, \S\, \R\, \E\, \T\)
- **MLLP transport**: Complete MLLP frame parsing and generation
- **Batch processing**: Full support for FHS/BHS/BTS/FTS batch and file batch structures
- **JSON serialization**: Convert messages to canonical JSON format
- **Field path access**: Query message fields with path notation (e.g., "PID.5[1].1")

### Profile Validation (`hl7v2::conformance`)

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
- **Profile linting**: Check profile YAML for ignored keys, malformed paths, invalid regexes, and dangling rule references
- **Flexible rule composition**: Merge profiles with child precedence

### Synthetic Message Generation (`hl7v2::synthetic`)

- **Template-based generation**: Define message templates with variable value sources
- **Realistic data generators**: Names (gender-aware), addresses, phone numbers, SSNs, MRNs, ICD-10, LOINC codes
- **Value distributions**: Fixed values, value lists, numeric ranges, dates, normal distributions
- **Deterministic seeding**: Same seed + template = identical output for regression testing
- **Error injection**: Generate invalid messages with segmentation/format errors for testing
- **Corpus tools**: Generate collections with golden hash verification for test data reproducibility

### CLI Interface (`hl7v2-cli`)

- **Unified command interface**: parse, normalize, validate, lint profiles, acknowledge, generate, summarize corpora
- **Input/output formats**: Raw HL7, JSON, MLLP framing
- **First-use diagnostics**: `hl7v2-cli doctor` checks a sample, profile, and optional server URL
- **File I/O**: Read from files or stdin, write to files or stdout

### HTTP/REST API Server (`hl7v2-server`)

- **RESTful API**: Parse, validate, redact, bundle, replay, acknowledge, normalize, and inspect inline-message corpus evidence over HTTP without request-supplied filesystem paths
- **Health & Readiness**: Production-ready health checks
- **Prometheus metrics**: Request counts, latencies, error rates
- **Redacted structured logs**: Evidence workflow logs hash message control IDs and bundle IDs while avoiding raw HL7, profile YAML, redaction policy TOML, and configured filesystem roots by default
- **Concurrency limiting**: Built-in backpressure (100 concurrent requests default)
- **CORS support**: Configurable allowed origins, with permissive local default
- **Compression**: Gzip compression for responses
- **OpenAPI 3.1 spec**: Complete API documentation at `api/openapi/hl7v2-api-v1.yaml`
- **Docker ready**: Containerized deployment with Nix-built images
- **Kubernetes ready**: Version-tagged and digest-template manifests in `infrastructure/k8s/`

See [DEPLOYMENT.md](DEPLOYMENT.md) for production deployment guide.

## Architecture

The project uses one canonical Rust library crate with SRP modules inside it,
plus separate runtime and binding wrappers:

```text
hl7v2
  model, parser, writer, query, transport, conformance, synthetic,
  batch, stream, ack, normalize, redact, lifecycle, experimental

hl7v2-server
  HTTP/gRPC service, metrics, auth, CORS, deployment/runtime config

hl7v2-cli
  command-line binary

hl7v2-python
  PyO3 binding backend; builds the public hl7v2 Python distribution
```

The workspace also contains internal support surfaces used to develop and
verify the public crates:

- `hl7v2-bench`: Criterion benchmarks for parsing, transport, queries, JSON,
  and related performance work.
- `hl7v2-e2e-tests`: black-box end-to-end coverage for workspace behavior.
- `hl7v2-test-utils`: shared test fixtures and helpers.
- `xtask`: repository-local setup, validation, policy, and evidence commands.

Retired compatibility crate names should not gain new behavior. See
[ADR-015](docs/adr/0015-collapse-public-crate-surface.md) and
[the module map](docs/architecture/module-map.md) for the migration policy.

## Python Binding Lane

The public Python distribution is `hl7v2` and imports as `hl7v2`. It is built
from the `hl7v2-python` maturin backend crate. Rust users should depend on
`hl7v2`; Python users should install `hl7v2` from PyPI once the Python release
lane is proven.

```bash
python -m pip install "maturin==1.13.1"
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 maturin build --release --out dist
python -m pip install dist/*.whl
python tests/python_smoke/smoke.py
```

The current binding proof covers wheel build, install, import, version
metadata, parse, segment count, JSON conversion, validation report parity,
corpus summary/fingerprint/diff dict outputs, safe-analysis redaction, evidence
bundle creation, and replay verification. Validation, corpus, redaction,
bundle, and replay APIs also support the same opt-in v2 evidence shapes used by
the CLI/server contracts where those surfaces expose v2 output.

TestPyPI proof is manual-first through the `Python TestPyPI Proof` workflow and
does not change the primary Rust product graph. See
[`docs/guides/python-testpypi-release-proof.md`](docs/guides/python-testpypi-release-proof.md).

## Performance Characteristics

- Parsing throughput: ≥100k small messages/minute on NVMe (typical ADT/ORU ~200 bytes)
- Large messages: ≥10k messages/minute for ~2 KB messages in batch mode
- Memory usage: bounded; no unbounded growth in the streaming parser for typical workloads
- Determinism: 100% reproducible generation with the same seed + template
- Latency: sub-millisecond parsing for typical messages on a modern CPU

For exact benchmark numbers and hardware, see [docs/STATUS.md](docs/STATUS.md).

## Memory Efficiency

The parser uses a "zero-allocation where possible" approach rather than true zero-copy:

- **Small messages**: Parsed in-place with minimal allocations
- **Large messages**: Use `hl7v2::stream` for bounded memory usage
- **Trade-off**: Safety and ergonomics are prioritized over raw performance

### Why not true zero-copy?

The standard parser (`hl7v2::parser`) uses `Vec<u8>` internally for owned data, which provides:
- Safe lifetime management without complex borrow checker patterns
- Ergonomic API that doesn't require managing input lifetimes
- Ability to modify and re-serialize messages

For production use with large HL7 messages or memory-constrained environments, use the streaming parser with configured memory bounds:

```rust
use std::io::{BufReader, Cursor};
use hl7v2::stream::StreamParserBuilder;

let hl7_bytes = b"MSH|^~\\&|App||Fac||20250128||ADT^A01|123|P|2.5.1\r";
let reader = BufReader::new(Cursor::new(hl7_bytes));
let parser = StreamParserBuilder::new()
    .max_message_size(1024 * 1024)
    .build(reader);
```

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

This project is licensed under the GNU Affero General Public License, version 3 or later
(**AGPL-3.0-or-later**). See [LICENSE](LICENSE).

## Contributing

By submitting a contribution (pull request, patch, issue comment containing code, etc.),
you agree to the terms in [CLA.md](CLA.md) and you license your contribution under
**AGPL-3.0-or-later**.
