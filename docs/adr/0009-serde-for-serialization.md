# ADR-0009: Serde for Serialization

**Status**: Accepted

**Date**: 2025-11-19

**Deciders**: Architecture team

**Technical Story**: HL7v2 message processing requires serializing and deserializing data across multiple formats (JSON for API responses, YAML for profile configuration, wire format for HL7v2 itself). A unified serialization framework is needed to avoid maintaining parallel hand-written serializers for every data type.

## Context

The hl7v2-rs workspace contains 28 crates spanning model types, validation rules, configuration profiles, CLI output, and HTTP API responses. All of these need serialization in at least one format:

- **JSON**: API responses from `hl7v2-server`, CLI output in `hl7v2-cli`, debugging and inspection workflows, test fixture files
- **YAML**: Validation profile definitions in `hl7v2-prof` (loaded from `profiles/` directory), template definitions in `hl7v2-template`
- **HL7v2 wire format**: Handled separately by `hl7v2-writer`, but model types still need round-trip capability through structured formats

Key requirements:

1. **Multiple format support** - JSON and YAML at minimum, with the ability to add more formats without modifying model types
2. **Derive-based ergonomics** - With 28 crates and many data types, manual serialization implementations would be a maintenance burden
3. **Performance** - Server mode processes messages at high throughput; serialization must not be a bottleneck
4. **Type safety** - Deserialization should produce typed Rust structs, not unstructured data
5. **Ecosystem compatibility** - Must work with Axum (HTTP framework), configuration loading, and test infrastructure

Currently 19 of 28 workspace crates depend on serde, making it the most widely used dependency in the workspace.

## Decision

We will use **serde** with its derive macros as the standard serialization framework for all data types that need structured serialization. The workspace pins the following versions:

```toml
[workspace.dependencies]
serde = { version = "1.0.228", features = ["derive"] }
serde_json = "1.0.149"
serde_yaml_ng = "0.10.0"
```

All core model types derive `Serialize` and `Deserialize`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Delims {
    pub field: char,
    pub comp: char,
    pub rep: char,
    pub esc: char,
    pub sub: char,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub delims: Delims,
    pub segments: Vec<Segment>,
    pub charsets: Vec<String>,
}
```

**Rationale:**

1. **Ubiquity** - Serde is the de facto standard serialization framework in Rust, used by virtually every production Rust project
2. **Format agnosticism** - A single set of derive attributes supports JSON, YAML, TOML, MessagePack, CBOR, and dozens more formats
3. **Zero-cost derives** - The `#[derive(Serialize, Deserialize)]` macros generate code at compile time with no runtime reflection overhead
4. **Axum integration** - Axum's `Json<T>` extractor requires `T: Serialize`, making serde a natural fit for the HTTP server
5. **Attribute system** - Fine-grained control via `#[serde(default)]`, `#[serde(rename)]`, `#[serde(skip)]`, etc.

## Consequences

### Positive

- **Format flexibility**: Adding a new output format (e.g., MessagePack, CBOR) requires only adding a new format crate dependency, not modifying any model types
- **Consistency**: All 19 crates using serde follow identical serialization patterns, reducing cognitive load
- **Round-trip fidelity**: Deserializing a serialized value produces an identical struct, enabling reliable config loading and test fixtures
- **Ecosystem compatibility**: Works seamlessly with Axum (`Json<T>`), configuration crates, and test assertion libraries
- **Derive ergonomics**: Adding serialization to a new type is a one-line change (`#[derive(Serialize, Deserialize)]`)
- **Mature and stable**: Serde 1.x has been stable since 2017; breaking changes are extremely unlikely

### Negative

- **Compile time impact**: Serde's proc macros add to compile times, especially across 19 crates. Each type with derives generates additional code
- **Binary size**: Monomorphization of generic serde functions for each type increases binary size
- **Complexity for custom formats**: HL7v2 wire format serialization does not use serde (handled by `hl7v2-writer`) because serde's data model does not map cleanly to HL7v2's delimiter-based encoding. This means two serialization systems coexist
- **Derive ordering sensitivity**: Derive macro ordering and feature flags can cause subtle issues if not managed consistently across the workspace

### Neutral

- **Workspace-wide dependency**: Serde becomes a foundational dependency that nearly every crate transitively depends on; updating it requires workspace-wide coordination
- **YAML fork**: We use `serde_yaml_ng` (a maintained fork) rather than the archived `serde_yaml`, which requires monitoring for any future ecosystem shifts

## Alternatives Considered

### Alternative 1: Manual Serialization (impl Display / FromStr)

**Pros:**
- No external dependencies
- Full control over output format
- Potentially smaller binary size

**Cons:**
- Enormous maintenance burden across 28 crates with dozens of types
- Must implement separately for each format (JSON, YAML, etc.)
- Error-prone: hand-written parsers are a common source of bugs
- No derive support; every new field requires updating serialization code manually

**Why not chosen:**
The maintenance cost is prohibitive. With the number of data types in this workspace (Message, Segment, Field, Rep, Comp, Atom, Delims, validation rules, profiles, issues, etc.), manual implementations would require thousands of lines of boilerplate that serde derives generate automatically.

### Alternative 2: Protocol Buffers (protobuf) Only

**Pros:**
- Schema-first design with code generation
- Compact binary format with excellent performance
- Cross-language interoperability
- Strong backward/forward compatibility guarantees

**Cons:**
- Poor fit for human-readable formats (no native JSON/YAML without additional layers)
- Requires `.proto` schema files maintained separately from Rust types
- Code generation step adds build complexity
- Protobuf's type system does not map cleanly to Rust enums or HL7v2's hierarchical structure
- Overkill for configuration files that humans need to read and edit

**Why not chosen:**
HL7v2 profiles are defined in YAML files that humans author and review. Protobuf's strength is binary wire formats, not human-readable configuration. We would still need serde for YAML/JSON, making protobuf an additional dependency rather than a replacement.

### Alternative 3: Custom Derive Macro

**Pros:**
- Tailored exactly to our serialization needs
- Could optimize for HL7v2-specific patterns
- No external dependency

**Cons:**
- Proc macros are among the most complex Rust code to write and maintain
- Would need to reimplement the format-agnostic data model that serde already provides
- No ecosystem compatibility (Axum, config crates, etc. expect serde)
- Years of engineering effort already invested in serde's correctness and performance

**Why not chosen:**
Reinventing serde would be a massive engineering effort with no clear benefit. The HL7v2-specific serialization (wire format) is already handled separately by `hl7v2-writer`.

### Alternative 4: simd-json

**Pros:**
- SIMD-accelerated JSON parsing, significantly faster for large JSON documents
- API-compatible with serde_json for many use cases

**Cons:**
- JSON-only; does not help with YAML or other formats
- Requires SIMD-capable hardware (SSE4.2 / AVX2 / NEON)
- Mutable buffer requirement (`&mut [u8]`) changes API ergonomics
- Less mature than serde_json

**Why not chosen:**
JSON performance is not a bottleneck in our workload. HL7v2 messages are typically small (a few KB). The added complexity and hardware requirements are not justified. If JSON parsing ever becomes a bottleneck, simd-json can be adopted as a drop-in replacement for serde_json without changing any derive attributes.

## Implementation Notes

### Derive Macros on Model Types

All core model types in `hl7v2-model` derive both `Serialize` and `Deserialize`:

```rust
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Delims {
    pub field: char,
    pub comp: char,
    pub rep: char,
    pub esc: char,
    pub sub: char,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub delims: Delims,
    pub segments: Vec<Segment>,
    pub charsets: Vec<String>,
}
```

### JSON Output (serde_json)

Used in `hl7v2-server` for API responses and `hl7v2-cli` for structured output:

```rust
use serde_json;

let message: Message = parse(raw_bytes)?;
let json_output = serde_json::to_string_pretty(&message)?;
```

Axum handlers return `Json<T>` which automatically serializes via serde:

```rust
async fn parse_message(body: Bytes) -> Result<Json<Message>, AppError> {
    let message = parse(&body)?;
    Ok(Json(message))
}
```

### YAML Profile Loading (serde_yaml_ng)

Validation profiles are authored as YAML files and deserialized into Rust structs:

```rust
use serde_yaml_ng;

let profile: Profile = serde_yaml_ng::from_str(&yaml_content)?;
```

### Serde Attributes for Optional Fields

`#[serde(default)]` is used for fields that may be absent in configuration files, allowing backward-compatible additions:

```rust
#[derive(Serialize, Deserialize)]
pub struct ValidationRule {
    pub path: String,
    pub check: CheckType,
    #[serde(default)]
    pub severity: Severity,
    #[serde(default)]
    pub description: Option<String>,
}
```

### Workspace Dependency Management

Serde versions are pinned in the workspace root `Cargo.toml` to ensure all 19 crates use identical versions:

```toml
[workspace.dependencies]
serde = { version = "1.0.228", features = ["derive"] }
serde_json = "1.0.149"
serde_yaml_ng = "0.10.0"
```

Individual crates reference workspace dependencies:

```toml
[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
```

## References

- [Serde Documentation](https://serde.rs/)
- [Serde Derive Macros](https://serde.rs/derive.html)
- [Serde Data Model](https://serde.rs/data-model.html)
- [serde_json Documentation](https://docs.rs/serde_json/)
- [serde_yaml_ng Repository](https://github.com/acatton/serde-yaml-ng)
- [Axum Json Extractor](https://docs.rs/axum/latest/axum/struct.Json.html)
- [Rust Serialization Benchmarks](https://github.com/djkoloski/rust_serialization_benchmark)
