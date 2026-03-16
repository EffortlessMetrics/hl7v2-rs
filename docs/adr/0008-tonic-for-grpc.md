# ADR-0008: Tonic for gRPC Server

**Status**: Proposed

**Date**: 2025-11-19

**Deciders**: Architecture team

**Technical Story**: Some healthcare integration engines and high-throughput clinical systems prefer gRPC for message exchange due to its binary protocol efficiency, streaming support, and strong typing via Protocol Buffers. A gRPC interface would complement the existing Axum HTTP API and provide bidirectional streaming for batch processing scenarios.

## Context

The hl7v2-rs system currently provides an HTTP REST API via the `hl7v2-server` crate (Axum-based, fully functional) for parsing, validation, ACK generation, and health checks. The CLI supports a `--mode grpc` flag (`crates/hl7v2-cli/src/serve.rs`), but the gRPC server implementation returns an error:

```rust
async fn run_grpc_server(bind_address: &str) -> Result<(), Box<dyn std::error::Error>> {
    warn!("gRPC server mode is not yet implemented");
    Err("gRPC server mode is not yet implemented. Use --mode http for now.".into())
}
```

A Protocol Buffers service definition has been authored at `api/proto/hl7v2.proto` (324 lines, package `hl7v2.v1`) defining the `HL7Service` with six RPCs:

1. **`Parse`** -- Parse a single HL7v2 message (unary)
2. **`ParseStream`** -- Parse multiple messages (bidirectional streaming)
3. **`Validate`** -- Validate a message against a profile (unary)
4. **`GenerateAck`** -- Generate an ACK/NAK response (unary)
5. **`Normalize`** -- Normalize message delimiters and structure (unary)
6. **`HealthCheck`** -- Service health status (unary)

The proto file includes detailed message types for delimiters, segments, fields, components, repetitions, validation issues, and metadata -- a complete mapping of the hl7v2-rs data model to Protocol Buffers.

However, **no `tonic` dependency exists in any `Cargo.toml`** in the workspace, **no generated Rust protobuf code exists**, and the `ServerMode::Grpc` enum variant is a stub. gRPC support is planned future work.

Key considerations driving this proposal:

- **Integration engine compatibility**: Enterprise healthcare integration engines (Mirth Connect, Rhapsody, InterSystems HealthShare) increasingly support gRPC alongside traditional MLLP and HTTP.
- **Streaming for batch processing**: The `ParseStream` RPC enables bidirectional streaming, which is valuable for processing large batch files (FHS/BHS-wrapped message batches) without loading everything into memory.
- **Performance**: gRPC's binary encoding (protobuf) and HTTP/2 multiplexing offer lower latency and higher throughput than JSON over HTTP/1.1 for high-volume scenarios.
- **Existing Tokio foundation**: ADR-0003 established Tokio as the async runtime, and ADR-0007 chose Axum for HTTP. Tonic is built on the same Tokio/Hyper/Tower stack, ensuring compatibility.

## Decision

We propose using **Tonic** as the gRPC framework for hl7v2-rs when gRPC support is implemented. Tonic is the de facto standard gRPC implementation for the Tokio ecosystem, built on `hyper`, `tower`, and `prost` (for protobuf code generation).

The service contract has been designed (`api/proto/hl7v2.proto`) and the CLI infrastructure has been stubbed (`ServerMode::Grpc`), but **no Tonic dependency has been added** and **no implementation work has begun**.

**Rationale:**

1. **Tokio-native**: Tonic is built on the same `hyper`/`tower` stack as Axum, sharing the connection pool, middleware tower layers, and async runtime.
2. **Code generation**: `tonic-build` generates idiomatic Rust server and client code from `.proto` files at compile time via `build.rs`.
3. **Streaming support**: First-class support for unary, server-streaming, client-streaming, and bidirectional streaming RPCs -- essential for the `ParseStream` use case.
4. **Tower middleware**: Tonic services are `tower::Service` implementations, meaning the same middleware (tracing, rate limiting, authentication) used in the Axum server can be reused.
5. **TLS**: Built-in `rustls` integration for TLS, consistent with the existing MLLP TLS support in `hl7v2-network`.
6. **Production adoption**: Used by Linkerd, Materialize, Neon, and other production Rust systems.

## Consequences

### Positive

- **Middleware reuse**: Tower layers (tracing, metrics, rate limiting, authentication) can be shared between Axum HTTP and Tonic gRPC servers, reducing code duplication.
- **Streaming efficiency**: Bidirectional streaming via `ParseStream` enables processing large HL7v2 batch files as a stream of messages, with backpressure handled by HTTP/2 flow control.
- **Strong typing**: Protobuf-generated types ensure that clients and servers agree on the message format at compile time, eliminating serialization mismatches.
- **Polyglot clients**: The `.proto` file can generate client libraries in Go, Java, Python, C#, and other languages, broadening the ecosystem of systems that can integrate with hl7v2-rs.
- **Performance**: Binary protobuf encoding is significantly more compact and faster to serialize/deserialize than JSON, which matters for high-throughput HL7v2 pipelines.
- **Unified server**: Tonic and Axum can run on the same `hyper` server, enabling a single process to serve both HTTP and gRPC on different ports (or even the same port with content-type routing).

### Negative

- **Build complexity**: `tonic-build` requires `protoc` (the Protocol Buffer compiler) to be installed at build time, adding a build dependency that must be managed in CI and the Nix dev shell.
- **Compile time**: Protobuf code generation and compilation adds to build times, especially for the initial build.
- **Proto maintenance**: Changes to the data model must be reflected in both the Rust types and the `.proto` file, creating a synchronization burden.
- **HTTP/2 requirement**: gRPC requires HTTP/2, which complicates debugging (cannot use `curl` easily) and may not pass through all proxies and load balancers.
- **Binary protocol opacity**: Unlike JSON, protobuf messages are not human-readable, making ad-hoc debugging harder without specialized tools like `grpcurl` or `grpcui`.

### Neutral

- **Complementary to HTTP**: gRPC would not replace the Axum HTTP API; both would coexist. HTTP remains the simpler option for casual use, web UIs, and environments that cannot use gRPC.
- **Feature flag**: gRPC support would likely be behind a Cargo feature flag to keep the default binary lean for users who only need HTTP.
- **Proto versioning**: The `hl7v2.v1` package namespace allows future backward-incompatible changes via `hl7v2.v2` without breaking existing clients.

## Alternatives Considered

### Alternative 1: grpcio (C-based gRPC)

**Pros:**
- Wraps Google's official C gRPC library (grpc-core)
- Battle-tested C implementation used across many languages
- Feature-complete gRPC implementation

**Cons:**
- Requires C/C++ build toolchain (cmake, gcc/clang) -- complicates cross-compilation and Nix builds
- Large native dependency footprint
- Not Tokio-native; uses its own event loop or completion queue, requiring compatibility shims
- Slower Rust compilation due to C FFI bindings
- Less idiomatic Rust API compared to Tonic

**Why not chosen:**
The C build toolchain requirement conflicts with the project's Nix-based reproducible build strategy (ADR-0005). Tonic's pure-Rust implementation avoids C dependencies entirely and integrates natively with the existing Tokio runtime.

### Alternative 2: Custom HTTP/2 Implementation

**Pros:**
- Full control over protocol behavior
- No protobuf dependency; could use a custom binary format
- Minimal external dependencies

**Cons:**
- Enormous implementation effort; HTTP/2 framing, flow control, and HPACK are complex
- No ecosystem tooling (no `grpcurl`, no client generators)
- No interoperability with existing gRPC clients
- Ongoing maintenance burden for protocol compliance

**Why not chosen:**
Reimplementing gRPC from scratch provides no benefits over Tonic and would require months of effort for an inferior result. The proto file has already been designed for standard gRPC interoperability.

### Alternative 3: HTTP-only (No gRPC)

**Pros:**
- Simpler architecture; one server framework (Axum) to maintain
- JSON is human-readable and debuggable with standard tools
- No protobuf build dependency
- HTTP/1.1 compatibility with all proxies and load balancers

**Cons:**
- No streaming support for batch processing (HTTP/1.1 request/response model)
- Higher serialization overhead (JSON vs. protobuf) for high-throughput scenarios
- Limited to REST semantics; some operations (streaming parse) do not map cleanly
- Missing integration path for gRPC-native healthcare systems

**Why not chosen:**
The existing HTTP server covers most use cases adequately, but the `ParseStream` streaming RPC and high-throughput binary encoding are compelling for healthcare integration engines that process millions of messages daily. gRPC would be additive, not a replacement.

### Alternative 4: Connect (connect-rpc)

**Pros:**
- Compatible with both gRPC and HTTP/1.1 (via Connect protocol)
- Simpler than gRPC for browser clients
- Works through HTTP/1.1 proxies that block HTTP/2

**Cons:**
- No mature Rust implementation (connect-rpc is primarily Go and TypeScript)
- Smaller ecosystem than gRPC
- Less tooling support
- Would require building or porting a Rust Connect implementation

**Why not chosen:**
No production-quality Rust implementation exists. Tonic provides the same functionality for gRPC-native clients, and the existing Axum server already handles HTTP/1.1 use cases.

## Implementation Notes

### Current State

- **Proto file**: `api/proto/hl7v2.proto` (324 lines) -- complete service definition with six RPCs and full message type hierarchy
- **CLI stub**: `crates/hl7v2-cli/src/serve.rs` -- `ServerMode::Grpc` variant exists, `run_grpc_server` returns an error
- **No dependencies**: No `tonic`, `tonic-build`, or `prost` entries in any `Cargo.toml`
- **No generated code**: No `*.rs` files generated from the proto definition

### Proposed Implementation Path

#### Step 1: Add Dependencies

```toml
# Root Cargo.toml [workspace.dependencies]
tonic = "0.12"
tonic-build = "0.12"
prost = "0.13"
prost-types = "0.13"
```

#### Step 2: Create Build Script

```rust
// crates/hl7v2-server/build.rs (or a new hl7v2-grpc crate)
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir("src/gen")
        .compile_protos(
            &["../../api/proto/hl7v2.proto"],
            &["../../api/proto"],
        )?;
    Ok(())
}
```

#### Step 3: Implement Service Trait

```rust
// Hypothetical implementation -- NOT yet written
use tonic::{Request, Response, Status};

pub struct Hl7ServiceImpl {
    // Shared state with HTTP server (parser, validator, etc.)
}

#[tonic::async_trait]
impl hl7_service_server::Hl7Service for Hl7ServiceImpl {
    async fn parse(
        &self,
        request: Request<ParseRequest>,
    ) -> Result<Response<ParseResponse>, Status> {
        let req = request.into_inner();
        let message = hl7v2_parser::parse(&req.message)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;
        // Convert to proto types and return
        todo!("Implement proto type conversion")
    }

    type ParseStreamStream = ReceiverStream<Result<ParseResponse, Status>>;

    async fn parse_stream(
        &self,
        request: Request<tonic::Streaming<ParseRequest>>,
    ) -> Result<Response<Self::ParseStreamStream>, Status> {
        // Bidirectional streaming implementation
        todo!("Implement streaming parse")
    }

    // ... other RPCs
}
```

#### Step 4: Wire into CLI

```rust
// Replace the stub in crates/hl7v2-cli/src/serve.rs
async fn run_grpc_server(bind_address: &str) -> Result<(), Box<dyn std::error::Error>> {
    let addr = bind_address.parse()?;
    let service = Hl7ServiceImpl::new();

    info!("Starting gRPC server on {}", bind_address);

    tonic::transport::Server::builder()
        .add_service(hl7_service_server::Hl7ServiceServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
```

### Proto-to-Rust Type Mapping

The proto message types map to existing hl7v2-rs types:

| Proto Type | Rust Type (hl7v2-model) |
|---|---|
| `Message` | `hl7v2_model::Message` |
| `Segment` | `hl7v2_model::Segment` |
| `Field` | `hl7v2_model::Field` |
| `Repetition` | `hl7v2_model::Rep` |
| `Component` | `hl7v2_model::Comp` |
| `Delimiters` | `hl7v2_model::Delims` |
| `Field.Presence` | `hl7v2_model::Atom` variants |

Conversion traits (`From<Message> for proto::Message` and vice versa) would be implemented in the gRPC crate.

### Dual-Protocol Server

Tonic and Axum can share the same Hyper server:

```rust
// Hypothetical dual-protocol setup -- NOT yet implemented
use axum::Router;
use tonic::transport::Server as TonicServer;

// Option 1: Separate ports
// HTTP on :8080, gRPC on :9090

// Option 2: Same port with content-type multiplexing
// Uses tower to route based on content-type header
```

### Next Steps (Not Yet Started)

- [ ] Add `tonic`, `tonic-build`, and `prost` to workspace dependencies
- [ ] Add `protoc` to Nix dev shell (`flake.nix`)
- [ ] Create `build.rs` for proto code generation
- [ ] Implement `Hl7Service` trait with conversion from hl7v2-model types
- [ ] Implement `ParseStream` bidirectional streaming
- [ ] Replace `run_grpc_server` stub in CLI
- [ ] Add Tower middleware (tracing, auth) to gRPC server
- [ ] Write integration tests with `tonic::transport::Channel` client
- [ ] Benchmark gRPC vs. HTTP throughput for parse and validate operations
- [ ] Document gRPC API in project documentation

## References

- [Tonic](https://github.com/hyperium/tonic) -- Tokio-native gRPC framework for Rust
- [Prost](https://github.com/tokio-rs/prost) -- Protocol Buffers implementation for Rust
- [gRPC](https://grpc.io/) -- High-performance RPC framework
- [Protocol Buffers](https://protobuf.dev/) -- Language-neutral serialization format
- [Tower](https://github.com/tower-rs/tower) -- Middleware framework shared by Axum and Tonic
- `api/proto/hl7v2.proto` -- Service definition (324 lines, 6 RPCs)
- `crates/hl7v2-cli/src/serve.rs` -- gRPC server stub (lines 101-110)
- ADR-0003: Use Tokio for Async Runtime -- Foundation for async I/O
- ADR-0007: Axum for HTTP Server -- Complementary HTTP server decision
