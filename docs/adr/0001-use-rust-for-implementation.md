# ADR-0001: Use Rust for Implementation

**Status**: Accepted

**Date**: 2025-11-19

**Deciders**: Architecture team

**Technical Story**: Selection of implementation language for an HL7v2 message processing library and server targeting healthcare integration environments that handle Protected Health Information (PHI).

## Context

We are building `hl7v2-rs`, a comprehensive HL7v2 message processing system covering parsing, validation, serialization, network transport (MLLP over TCP/TLS), and an HTTP API server. HL7v2 is the dominant messaging standard in healthcare integration, used for patient admissions, lab results, pharmacy orders, and clinical observations.

The system has several hard requirements driven by the healthcare domain:

1. **Safety** -- Messages contain Protected Health Information (PHI) subject to HIPAA and equivalent regulations. Memory safety bugs (buffer overflows, use-after-free) could lead to data corruption or unauthorized PHI disclosure.
2. **Performance** -- High-throughput environments such as hospital ADT feeds, lab result routing, and health information exchanges process millions of messages per day. The parser and validation engine must handle sustained load without garbage-collection pauses.
3. **Correctness** -- HL7v2 messages have complex structure (segments, fields, repetitions, components, sub-components) with configurable delimiters. Incorrect parsing can cause clinical data to be misrouted or misinterpreted.
4. **Concurrency** -- The server must handle many simultaneous MLLP connections, HTTP API requests, and background validation tasks.
5. **Modularity** -- Different deployment scenarios need different subsets of functionality (e.g., parsing-only library vs. full server), requiring a modular architecture.
6. **Cross-platform** -- Must run on Linux (production), macOS (development), and Windows (development/testing).

## Decision

We will use **Rust** (edition 2024, MSRV 1.92) as the sole implementation language for all crates in the workspace, licensed under AGPL-3.0-or-later.

**Rationale:**

1. **Memory safety without garbage collection** -- Rust's ownership model and borrow checker eliminate entire classes of memory bugs (buffer overflows, dangling pointers, data races) at compile time, without introducing GC pauses. This is critical for a system processing PHI where memory corruption could expose sensitive data.
2. **Zero-cost abstractions** -- Traits, generics, and iterators compile to the same machine code as hand-written equivalents, enabling high-throughput message processing without runtime overhead.
3. **Strong type system** -- Algebraic data types (`enum`), pattern matching, and `Result<T, E>` error handling catch structural and logical errors at compile time. Each crate defines its own error type via `thiserror`, preserving error context through `#[source]` chains.
4. **Cargo workspace** -- Cargo's native workspace support enables the microcrate architecture (28 crates organized in layers: microcrates, mid-level crates, application crates, testing crates) with centralized dependency management via `[workspace.dependencies]`.
5. **Ecosystem maturity** -- Production-grade crates exist for every layer of the stack:
   - Async runtime: `tokio 1.50.0`
   - HTTP server: `axum 0.8.8`
   - Serialization: `serde 1.0.228`, `serde_json 1.0.149`
   - Error handling: `thiserror 2.0.18`
   - Metrics: `metrics 0.24.3`, `metrics-exporter-prometheus 0.18.1`
   - Rate limiting: `tower_governor 0.8.0`
   - Date/time: `chrono 0.4.44`
   - Cryptography: `sha2 0.10.9`
6. **Concurrency safety** -- The `Send`/`Sync` trait system prevents data races at compile time, making concurrent MLLP connection handling and async HTTP serving safe by construction.
7. **Cross-platform support** -- Rust compiles natively to Linux, macOS, and Windows with the same codebase.

## Consequences

### Positive

- **Eliminates memory safety vulnerabilities** -- No buffer overflows, use-after-free, or data races in safe Rust code; critical for PHI handling.
- **Predictable latency** -- No GC pauses; deterministic memory management through RAII and ownership.
- **Compile-time correctness** -- Strong typing catches field-access errors, delimiter mismatches, and protocol violations before runtime.
- **Modular architecture** -- Cargo workspace supports 28 crates with clear dependency layers (`microcrates -> core -> prof/gen -> cli/server`), enabling consumers to depend on only what they need.
- **Single binary deployment** -- Statically linked binaries simplify deployment in containerized healthcare environments.
- **Performance ceiling** -- Zero-cost abstractions and no runtime overhead allow scaling to high message volumes without language-level bottlenecks.

### Negative

- **Steeper learning curve** -- Rust's ownership model and lifetimes require investment from new contributors. The borrow checker can slow initial development velocity.
- **Longer compile times** -- 28 crates with heavy use of generics and procedural macros increase full-rebuild times. Incremental compilation and `cargo check` mitigate this for development workflows.
- **Smaller talent pool** -- Fewer Rust developers compared to Java or Python in the healthcare integration space, which is historically dominated by Java (HAPI) and C# (Nhapi).
- **Ecosystem gaps** -- No equivalent to HAPI's comprehensive HL7v2 message definitions; message structure, segment definitions, and data type validation had to be implemented from scratch.

### Neutral

- **No runtime reflection** -- Rust lacks runtime reflection, so schema-driven features (profile validation, template generation) use declarative YAML files and code generation rather than annotation-based approaches common in Java.
- **Async complexity** -- Async Rust adds complexity (pinning, `Send` bounds) but is necessary for the concurrent server architecture and is well-supported by the Tokio ecosystem.
- **AGPL-3.0-or-later license** -- Copyleft license ensures derivative works remain open source, which may limit adoption by proprietary vendors but aligns with the project's goals.

## Alternatives Considered

### Alternative 1: Java

**Pros:**
- Dominant language in healthcare integration; extensive institutional knowledge.
- HAPI library provides mature HL7v2 parsing, validation, and message definitions out of the box.
- Large talent pool familiar with healthcare messaging.
- JVM provides memory safety (no buffer overflows) and cross-platform support.

**Cons:**
- Garbage collection pauses can cause latency spikes in high-throughput message processing.
- Higher memory footprint per connection due to JVM overhead.
- HAPI library is monolithic and tightly coupled; difficult to use selectively.
- Verbose language compared to Rust for data transformation pipelines.

**Why not chosen:**
GC pauses are unacceptable for latency-sensitive MLLP processing. JVM memory overhead limits connection density. HAPI's monolithic architecture conflicts with the microcrate approach. While Java has the strongest HL7v2 ecosystem, building from scratch in Rust provides better architectural control.

### Alternative 2: Python

**Pros:**
- Rapid prototyping and iteration speed.
- Extensive data processing libraries (pandas, etc.).
- Easy to learn; large contributor pool.
- `python-hl7` library exists for basic parsing.

**Cons:**
- GIL prevents true parallelism; concurrency requires multiprocessing or async.
- Orders of magnitude slower for byte-level message parsing.
- Dynamic typing means structural errors surface at runtime, not compile time.
- Memory usage is high for large batch processing.
- `python-hl7` is minimal and unmaintained.

**Why not chosen:**
Performance is insufficient for high-throughput message processing. Dynamic typing is a liability for a system where parsing correctness is safety-critical. The GIL makes concurrent connection handling awkward.

### Alternative 3: Go

**Pros:**
- Strong concurrency model (goroutines, channels) well-suited to network servers.
- Fast compilation and simple deployment (single binary).
- Growing adoption in healthcare infrastructure (e.g., Google Health).
- Simpler learning curve than Rust.

**Cons:**
- GC pauses, though shorter than Java's, still introduce latency variability.
- Less expressive type system; no sum types (`enum`), no generics until recently.
- Error handling is verbose and error-prone (`if err != nil` patterns).
- No equivalent to Cargo workspaces for fine-grained modular architecture.

**Why not chosen:**
Go's type system is too weak for modeling HL7v2's complex nested structure (message > segment > field > repetition > component > sub-component) with compile-time safety. Lack of sum types means parser events and error types cannot be expressed as cleanly. GC pauses, while mild, are still present.

### Alternative 4: C++

**Pros:**
- Maximum performance; zero-overhead abstractions.
- Mature ecosystem for systems programming.
- Can interface with existing C-based healthcare libraries.

**Cons:**
- No memory safety guarantees; buffer overflows and use-after-free are common vulnerability classes.
- Manual memory management is error-prone in complex parsing code.
- Build system fragmentation (CMake, Bazel, Meson) vs. Cargo's integrated approach.
- Header-only libraries and long compile times.
- Undefined behavior is pervasive and difficult to audit.

**Why not chosen:**
Memory safety is non-negotiable for a system processing PHI. C++ offers equivalent performance to Rust but without safety guarantees. The lack of a standard package manager makes the 28-crate modular architecture impractical.

## Implementation Notes

### Workspace Structure

The workspace is organized in `Cargo.toml` with 28 member crates following a layered dependency architecture:

```toml
[workspace]
members = [
    # Microcrates (SRP-focused)
    "crates/hl7v2-model",       # Core data types: Message, Segment, Field, Rep, Comp, Atom, Delims
    "crates/hl7v2-escape",      # HL7v2 escape sequences (\F\, \S\, \R\, \E\, \T\)
    "crates/hl7v2-mllp",        # MLLP framing (VT...FS CR)
    "crates/hl7v2-parser",      # Message parsing, delimiter discovery from MSH
    "crates/hl7v2-writer",      # Serialization to HL7 wire format and JSON
    "crates/hl7v2-json",        # JSON serialization/deserialization
    "crates/hl7v2-normalize",   # Message normalization and delimiter transformation
    "crates/hl7v2-datetime",    # Date/time parsing and validation
    "crates/hl7v2-datatype",    # Data type validation (CX, PN, TS, etc.)
    "crates/hl7v2-path",        # Field path parsing/resolution (e.g., PID.5[1].1)
    "crates/hl7v2-query",       # Fast path-based data extraction
    "crates/hl7v2-batch",       # Batch message handling (FHS/BHS/BTS/FTS)
    "crates/hl7v2-network",     # Async TCP/TLS MLLP client and server
    "crates/hl7v2-stream",      # Event-based streaming parser
    "crates/hl7v2-validation",  # Rule-based message validation engine
    "crates/hl7v2-ack",         # Automatic ACK generation
    "crates/hl7v2-faker",       # Synthetic data generation
    "crates/hl7v2-template",    # Template-based message generation
    "crates/hl7v2-template-values", # Values and generators for templates
    "crates/hl7v2-corpus",      # Pre-defined HL7 sample messages
    # Mid-level crates
    "crates/hl7v2-core",        # Facade re-exporting all microcrates
    "crates/hl7v2-prof",        # Profile-based validation (YAML profiles)
    "crates/hl7v2-gen",         # Synthetic message generation facade
    "crates/hl7v2-bench",       # Benchmarks
    # Application crates
    "crates/hl7v2-cli",         # CLI binary (hl7v2)
    "crates/hl7v2-server",      # Axum HTTP API server
    # Testing
    "crates/hl7v2-test-utils",  # Shared testing utilities
    "crates/hl7v2-e2e-tests",   # Integration tests
]
resolver = "2"
```

### Edition and Toolchain

```toml
[workspace.package]
edition = "2024"
rust-version = "1.92"
license = "AGPL-3.0-or-later"
```

Edition 2024 enables the latest language features. MSRV 1.92 ensures compatibility with recent stable toolchains while allowing use of newer features like improved `async` support.

### Error Handling Pattern

Each crate defines its own error type using `thiserror`, with `#[source]` chains for context preservation:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Invalid MSH segment: {details}")]
    InvalidMsh { details: String },

    #[error("Invalid field format in {segment_id}.{field_index}")]
    InvalidFieldFormat {
        segment_id: String,
        field_index: usize,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}
```

### Dependency Management

All shared dependency versions are centralized in the workspace root via `[workspace.dependencies]`, ensuring version consistency across all 28 crates:

```toml
[workspace.dependencies]
tokio = "1.50.0"
axum = "0.8.8"
serde = { version = "1.0.228", features = ["derive"] }
thiserror = "2.0.18"
```

Individual crates inherit these versions:

```toml
[dependencies]
serde = { workspace = true }
thiserror = { workspace = true }
```

### CI and Quality Gates

The project uses an `xtask` pattern for CI-parity local checks:

```bash
# Auto-fix formatting and lints (runs on pre-commit)
cargo run -p xtask -- lint-fix

# Strict CI-parity gate (runs on pre-push)
cargo run -p xtask -- gate --check
```

## References

- [The Rust Programming Language](https://doc.rust-lang.org/book/)
- [Rust Edition Guide - Edition 2024](https://doc.rust-lang.org/edition-guide/)
- [Cargo Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [HL7 Version 2 Standard](https://www.hl7.org/implement/standards/product_brief.cfm?product_id=185)
- [HAPI - Java HL7v2 Library](https://hapifhir.github.io/hapi-hl7v2/)
- [Tokio - Async Runtime for Rust](https://tokio.rs/)
- [Axum - Web Framework for Rust](https://github.com/tokio-rs/axum)
- [thiserror - Derive Error for Rust](https://github.com/dtolnay/thiserror)
