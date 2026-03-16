# ADR-0010: Thiserror for Error Handling

**Status**: Accepted

**Date**: 2025-11-19

**Deciders**: Architecture team

**Technical Story**: A workspace with 28 crates needs a consistent, ergonomic error handling strategy. Library crates require strongly typed error enums for domain-specific failures, while application crates (CLI, server) need flexible ad-hoc error handling at the boundary layer.

## Context

The hl7v2-rs workspace contains 28 crates organized in layers: microcrates, mid-level facades, and application binaries. Error handling requirements differ by layer:

- **Microcrates** (e.g., `hl7v2-model`, `hl7v2-parser`, `hl7v2-path`): Need precise, typed error enums so callers can match on specific failure modes (invalid segment ID, parse error at a specific field, malformed path expression)
- **Mid-level crates** (e.g., `hl7v2-core`, `hl7v2-prof`): Need to wrap and propagate errors from multiple microcrates while preserving the error chain
- **Application crates** (`hl7v2-cli`, `hl7v2-server`): Need to collect errors from any layer and present them to users (HTTP responses, CLI output) without exhaustively matching every variant

Key requirements:

1. **Per-crate error types** - Each crate defines its own error enum reflecting its domain
2. **Error chaining** - Errors must carry `#[source]` context so `anyhow`/tracing can display the full causal chain
3. **Display impl generation** - Human-readable error messages should be derived, not hand-written
4. **Compatibility with `?` operator** - Errors must implement `std::error::Error` for ergonomic propagation
5. **Minimal boilerplate** - With 12+ crates defining error types, the pattern must be concise

Currently 12 of 28 workspace crates use thiserror for their error types. The application boundary crates additionally use anyhow for flexible error handling.

## Decision

We will use **thiserror** for all library-level error types and **anyhow** at application boundaries:

```toml
[workspace.dependencies]
thiserror = "2.0.18"
anyhow = "1.0.102"
```

Each crate defines its own error enum using `#[derive(thiserror::Error)]`:

```rust
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum Error {
    #[error("Invalid segment ID")]
    InvalidSegmentId,

    #[error("Parse error at segment {segment_id} field {field_index}: {source}")]
    ParseError {
        segment_id: String,
        field_index: usize,
        #[source]
        source: Box<Error>,
    },

    #[error("Framing error: {0}")]
    Framing(String),
}
```

Application crates (`hl7v2-cli`, `hl7v2-server`) use anyhow for top-level error handling:

```rust
use anyhow::Result;

fn main() -> Result<()> {
    let message = parse(&input)?;  // thiserror Error auto-converts to anyhow
    Ok(())
}
```

**Rationale:**

1. **Derive ergonomics** - `#[derive(thiserror::Error)]` generates `Display` and `Error` impls from a single attribute, eliminating boilerplate
2. **Source chaining** - `#[source]` attribute automatically implements `Error::source()`, enabling full causal chain inspection
3. **Library/application split** - thiserror for typed errors in libraries, anyhow for ergonomic error collection at boundaries; this is the idiomatic Rust pattern
4. **Compile-time safety** - Typed error enums ensure callers handle all failure modes; the compiler catches missing match arms
5. **Ecosystem standard** - thiserror + anyhow is the most widely adopted error handling pattern in the Rust ecosystem

## Consequences

### Positive

- **Consistency**: All 12 crates using thiserror follow an identical error definition pattern, making it easy to navigate error types across the workspace
- **Ergonomic propagation**: The `?` operator works seamlessly because thiserror generates proper `std::error::Error` implementations
- **Readable error messages**: `#[error("...")]` format strings produce clear, context-rich messages (e.g., "Parse error at segment PID field 5: invalid component")
- **Error chain preservation**: `#[source]` attributes ensure the full causal chain is available for logging and debugging, from application boundary down to the originating microcrate
- **Match exhaustiveness**: Typed error enums let callers selectively handle specific failure modes (e.g., retry on network errors, fail fast on parse errors)
- **Zero runtime cost**: thiserror is a proc macro; all code generation happens at compile time with no runtime overhead

### Negative

- **Compile time**: Proc macro expansion adds to compile times, though the impact is smaller than serde since error types are fewer and simpler
- **Boilerplate for conversions**: Cross-crate error wrapping requires `From` implementations (either manual or via thiserror's `#[from]` attribute), which can become verbose in crates that aggregate errors from many dependencies
- **Enum growth**: Error enums can accumulate many variants over time, making exhaustive matching increasingly burdensome for callers
- **Two error systems**: Having both thiserror (libraries) and anyhow (applications) means developers must understand when to use which

### Neutral

- **Crate boundary design**: The decision to give each crate its own error type means errors must be explicitly converted at crate boundaries, which is both a safety benefit and a friction point
- **Box\<Error\> for recursion**: Recursive error types (where an error variant wraps the same error type) require `Box<Error>` to avoid infinite size, as seen in `hl7v2-model`'s `ParseError` variant

## Alternatives Considered

### Alternative 1: Manual Error Trait Implementations

**Pros:**
- No external dependencies
- Full control over Display formatting and Error trait behavior
- Can optimize for specific use cases

**Cons:**
- Enormous boilerplate: each error enum requires hand-written `impl fmt::Display` and `impl std::error::Error` with `source()` method
- Error-prone: easy to forget to implement `source()` or format fields incorrectly
- Maintenance burden multiplied across 12+ crates
- No compile-time verification that Display messages reference valid fields

**Why not chosen:**
The boilerplate cost is unjustifiable. A typical error enum with 10 variants requires ~50 lines of hand-written Display/Error impl that thiserror generates from ~10 lines of attributes. Across 12 crates, this would add hundreds of lines of mechanical code.

### Alternative 2: eyre

**Pros:**
- Drop-in replacement for anyhow with customizable error reporting
- `color-eyre` provides beautiful colored backtraces and span traces
- Good integration with tracing

**Cons:**
- eyre replaces anyhow at the application boundary but does not replace thiserror for library error types
- Adds hook-based global state for report customization, which complicates testing
- Smaller ecosystem adoption than anyhow
- We would still need thiserror (or equivalent) for typed errors

**Why not chosen:**
eyre solves a different problem (application-level error reporting) and does not replace the need for typed library errors. Its global hook mechanism adds complexity without clear benefit for our use case. If richer error reports are needed in the future, eyre can be adopted alongside thiserror without changing library error types.

### Alternative 3: snafu

**Pros:**
- Context selectors provide ergonomic error wrapping at call sites
- Encourages adding context at the point of failure
- Built-in backtrace support
- Can generate both the error type and the context selector

**Cons:**
- Heavier API surface with context selectors, `Snafu` derive, and `ensure!` macro
- Less familiar to most Rust developers compared to thiserror
- Different ergonomic philosophy: snafu emphasizes context at the call site, thiserror emphasizes the error type definition
- Smaller ecosystem adoption

**Why not chosen:**
thiserror's simpler model (derive on the error type, explicit `From` conversions) is more widely understood and requires less project-specific knowledge. snafu's context selector pattern, while powerful, adds conceptual overhead that is not justified for our error handling needs.

### Alternative 4: Custom Error Derive Macro

**Pros:**
- Could be tailored to HL7v2-specific error patterns (e.g., always include segment ID and field index)
- No external dependency

**Cons:**
- Proc macros are complex to write, test, and maintain
- Would need to replicate thiserror's Display generation, source chaining, and From conversion features
- No ecosystem familiarity; new contributors must learn a custom system
- Ongoing maintenance burden as requirements evolve

**Why not chosen:**
thiserror already provides exactly the derive functionality needed. Building a custom macro would be reimplementing thiserror with a narrower feature set and no ecosystem support.

## Implementation Notes

### Per-Crate Error Enums

Each crate defines an `Error` enum in its `error.rs` (or inline in `lib.rs`). The enum captures domain-specific failure modes:

```rust
// crates/hl7v2-model/src/error.rs
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum Error {
    #[error("Invalid segment ID")]
    InvalidSegmentId,

    #[error("Parse error at segment {segment_id} field {field_index}: {source}")]
    ParseError {
        segment_id: String,
        field_index: usize,
        #[source]
        source: Box<Error>,
    },

    #[error("Framing error: {0}")]
    Framing(String),
}
```

### Error Chaining with #[source]

The `#[source]` attribute generates the `Error::source()` implementation, enabling tools like anyhow and tracing to walk the full causal chain:

```rust
// crates/hl7v2-path/src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum PathError {
    #[error("Invalid path expression: {path}")]
    InvalidPath { path: String },

    #[error("Field index out of range: {index}")]
    FieldOutOfRange { index: usize },
}
```

### Cross-Crate Error Conversion with #[from]

When a crate wraps errors from its dependencies, `#[from]` generates the `From` impl:

```rust
// crates/hl7v2-core/src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("Model error: {0}")]
    Model(#[from] hl7v2_model::Error),

    #[error("Parse error: {0}")]
    Parse(#[from] hl7v2_parser::Error),

    #[error("Path error: {0}")]
    Path(#[from] hl7v2_path::PathError),
}
```

### Application Boundary with anyhow

CLI and server crates use anyhow to collect errors from any library crate without exhaustive matching:

```rust
// crates/hl7v2-cli/src/main.rs
use anyhow::{Context, Result};

fn process_file(path: &str) -> Result<()> {
    let content = std::fs::read(path)
        .with_context(|| format!("Failed to read file: {path}"))?;

    let message = hl7v2_core::parse(&content)
        .context("Failed to parse HL7v2 message")?;

    Ok(())
}
```

### Crates Using thiserror

The following 12 crates define error types with thiserror:

| Crate | Error Type | Typical Variants |
|-------|-----------|-----------------|
| `hl7v2-model` | `Error` | InvalidSegmentId, ParseError, Framing |
| `hl7v2-datetime` | `DateTimeError` | InvalidFormat, OutOfRange |
| `hl7v2-datatype` | `DatatypeError` | InvalidFormat, MissingField |
| `hl7v2-path` | `PathError` | InvalidPath, FieldOutOfRange |
| `hl7v2-batch` | `BatchError` | InvalidHeader, MissingTrailer |
| `hl7v2-core` | `CoreError` | Model, Parse, Path (wraps sub-crate errors) |
| `hl7v2-prof` | `ProfileError` | InvalidProfile, ValidationFailed |
| `hl7v2-stream` | `StreamError` | BufferOverflow, InvalidFrame |
| `hl7v2-mllp` | `MllpError` | FramingError, IncompleteMessage |
| `hl7v2-corpus` | `CorpusError` | NotFound, ParseFailed |
| `hl7v2-server` | `AppError` | Internal, BadRequest, NotFound |
| `hl7v2-test-utils` | `TestError` | FixtureNotFound, SetupFailed |

### Workspace Dependency Management

Both thiserror and anyhow are pinned in the workspace root:

```toml
[workspace.dependencies]
thiserror = "2.0.18"
anyhow = "1.0.102"
```

## References

- [thiserror Documentation](https://docs.rs/thiserror/)
- [thiserror Repository](https://github.com/dtolnay/thiserror)
- [anyhow Documentation](https://docs.rs/anyhow/)
- [anyhow Repository](https://github.com/dtolnay/anyhow)
- [Rust Error Handling Best Practices](https://nick.groenen.me/posts/rust-error-handling/)
- [thiserror vs anyhow: Library vs Application Error Handling](https://www.lpalmieri.com/posts/error-handling-rust/)
- [Rust API Guidelines: Error Types](https://rust-lang.github.io/api-guidelines/interoperability.html#c-good-err)
