# EFF-1194: gRPC Server Implementation - Implementation Notes

## Design Decisions

### Decision 1: Feature Flag Strategy

**Decision**: gRPC support will be behind a `grpc` feature flag in the `hl7v2-server` crate.

**Rationale**:
- Keeps default builds lean for users who only need HTTP
- protoc build dependency is optional
- Compile time impact is isolated

**Implementation**:
```toml
# crates/hl7v2-server/Cargo.toml
[features]
default = []
grpc = ["dep:tonic", "dep:prost", "dep:tonic-build"]
```

---

### Decision 2: Crate Organization

**Decision**: Keep gRPC code in `hl7v2-server` crate rather than creating a new `hl7v2-grpc` crate.

**Rationale**:
- gRPC and HTTP servers share significant logic (middleware, state management)
- Single server crate aligns with "server modes" concept in CLI
- Reduces cross-crate dependency complexity

**Structure**:
```
crates/hl7v2-server/src/
├── lib.rs              # Main exports
├── http/               # Existing Axum HTTP server
│   └── ...
└── grpc/               # NEW: gRPC module
    ├── mod.rs          # Feature-gated module
    ├── gen/            # Generated proto code
    │   └── hl7v2.v1.rs
    ├── service.rs      # Hl7Service trait impl
    ├── convert.rs      # Proto↔Model conversions
    └── server.rs       # gRPC server setup
```

---

### Decision 3: Code Generation Strategy

**Decision**: Use `tonic-build` in a `build.rs` script with conditional compilation.

**Rationale**:
- Standard tonic approach
- Generates both server and client code
- Allows customization of output directory

**Implementation**:
```rust
// crates/hl7v2-server/build.rs
#[cfg(feature = "grpc")]
fn build_proto() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir("src/grpc/gen")
        .compile_protos(
            &["../../api/proto/hl7v2.proto"],
            &["../../api/proto"],
        )?;
    Ok(())
}
```

**Note**: Generated files should be:
- Committed to git for reproducibility
- Regenerated when proto changes
- Ignored by rustfmt via `#[rustfmt::skip]`

---

### Decision 4: Port Strategy

**Decision**: gRPC server runs on a separate port from HTTP (default 9090 vs 8080).

**Rationale**:
- Clear separation of concerns
- Independent scaling/configuration
- HTTP/2 on gRPC port, HTTP/1.1 on HTTP port
- Avoids content-type routing complexity

**Configuration**:
```rust
// Default ports
const DEFAULT_HTTP_PORT: u16 = 8080;
const DEFAULT_GRPC_PORT: u16 = 9090;
```

---

### Decision 5: Middleware Sharing

**Decision**: Use Tower middleware layers shared between Axum HTTP and Tonic gRPC.

**Rationale**:
- Consistent observability (tracing, metrics)
- Consistent cross-cutting concerns (auth, rate limiting)
- Tower is the standard middleware interface for both

**Implementation**:
```rust
// Shared middleware stack
let middleware = ServiceBuilder::new()
    .layer(TraceLayer::new_for_grpc())
    .layer(RateLimitLayer::new(100, Duration::from_secs(1)))
    .into_inner();

// Applied to both HTTP and gRPC
```

---

## Technical Implementation Details

### Proto-to-Model Conversions

The proto types need bidirectional conversion with `hl7v2-model` types:

| Proto | Model | Conversion |
|-------|-------|------------|
| `Message` | `hl7v2_model::Message` | Full structure mapping |
| `Segment` | `hl7v2_model::Segment` | Fields as Vec |
| `Field` | `hl7v2_model::Field` | Atom-based presence |
| `Delimiters` | `hl7v2_model::Delims` | Simple field mapping |
| `ValidationIssue` | `hl7v2_validation::Issue` | Severity mapping |

**Conversion Pattern**:
```rust
// From model to proto
impl From<hl7v2_model::Message> for proto::Message {
    fn from(msg: hl7v2_model::Message) -> Self {
        // ... conversion logic
    }
}

// From proto to model (may fail)
impl TryFrom<proto::Message> for hl7v2_model::Message {
    type Error = ConversionError;
    fn try_from(msg: proto::Message) -> Result<Self, Self::Error> {
        // ... conversion logic
    }
}
```

---

### Hl7Service Trait Implementation

```rust
use tonic::{Request, Response, Status};

pub struct Hl7ServiceImpl {
    parser: Arc<MessageParser>,
    validator: Arc<MessageValidator>,
    // Shared state
}

#[tonic::async_trait]
impl hl7_service_server::Hl7Service for Hl7ServiceImpl {
    async fn parse(
        &self,
        request: Request<ParseRequest>,
    ) -> Result<Response<ParseResponse>, Status> {
        let req = request.into_inner();
        
        // Parse message bytes
        let message = self.parser.parse(&req.message)
            .map_err(|e| Status::invalid_argument(format!("Parse failed: {}", e)))?;
        
        // Convert to proto and respond
        let response = ParseResponse {
            success: true,
            message: Some(message.into()),
            errors: vec![],
            metadata: extract_metadata(&message),
        };
        
        Ok(Response::new(response))
    }
    
    type ParseStreamStream = ReceiverStream<Result<ParseResponse, Status>>;
    
    async fn parse_stream(
        &self,
        request: Request<Streaming<ParseRequest>>,
    ) -> Result<Response<Self::ParseStreamStream>, Status> {
        let mut stream = request.into_inner();
        let (tx, rx) = mpsc::channel(128);
        
        tokio::spawn(async move {
            while let Some(req) = stream.message().await? {
                let response = self.process_parse(req).await;
                tx.send(response).await?;
            }
            Ok::<_, Status>(())
        });
        
        Ok(Response::new(ReceiverStream::new(rx)))
    }
    
    // ... other RPC implementations
}
```

---

### CLI Integration

Replace the stub in `crates/hl7v2-cli/src/serve.rs`:

```rust
#[cfg(feature = "grpc")]
async fn run_grpc_server(bind_address: &str) -> Result<(), Box<dyn std::error::Error>> {
    use hl7v2_server::grpc::{Hl7ServiceImpl, hl7_service_server};
    
    let addr = bind_address.parse()?;
    let service = Hl7ServiceImpl::new();
    
    info!("Starting gRPC server on {}", bind_address);
    
    tonic::transport::Server::builder()
        .add_service(hl7_service_server::Hl7ServiceServer::new(service))
        .serve(addr)
        .await?;
    
    Ok(())
}

#[cfg(not(feature = "grpc"))]
async fn run_grpc_server(_bind_address: &str) -> Result<(), Box<dyn std::error::Error>> {
    Err("gRPC support not enabled. Build with --features grpc".into())
}
```

---

## Build Infrastructure

### Nix flake.nix Update

Add protoc to dev shell:

```nix
# flake.nix - devShell
packages = with pkgs; [
  # ... existing packages
  protobuf    # protoc compiler
];
```

### CI Update

Add protoc to GitHub Actions:

```yaml
- name: Install protoc
  uses: arduino/setup-protoc@v2
  with:
    version: "23.x"
```

---

## Options Considered

### Option A: Separate hl7v2-grpc Crate (Rejected)

**Pros**:
- Cleaner separation of concerns
- Could publish separately

**Cons**:
- Duplicated server logic
- More complex dependency graph
- Slower builds

**Why rejected**: Not worth the overhead for the shared functionality.

---

### Option B: Same Port with Content-Type Routing (Rejected)

**Pros**:
- Single port exposure
- Unified server entry point

**Cons**:
- Complex routing logic
- HTTP/2 vs HTTP/1.1 complications
- Harder to debug

**Why rejected**: Separation of ports is clearer operationally.

---

### Option C: Prost-only (no Tonic) (Rejected)

**Pros**:
- Smaller dependency tree
- More control over transport

**Cons**:
- Reinventing gRPC protocol
- No streaming support
- No ecosystem tooling

**Why rejected**: Tonic is the standard; don't reinvent.

---

## Risk Mitigations

| Risk | Mitigation |
|------|------------|
| protoc not in build env | Fail build with clear error message; document requirement |
| Proto file changes | CI check that generated code is up-to-date |
| Performance regression | Benchmark suite comparing gRPC vs HTTP throughput |
| Binary size increase | Feature flag keeps default builds lean |

---

## Testing Strategy

### Unit Tests

Located in `crates/hl7v2-server/src/grpc/`

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_parse_rpc() {
        let service = Hl7ServiceImpl::new();
        let request = Request::new(ParseRequest {
            message: b"MSH|...".to_vec(),
            mllp_framed: false,
            options: None,
        });
        
        let response = service.parse(request).await.unwrap();
        assert!(response.into_inner().success);
    }
}
```

### Integration Tests

Located in `crates/hl7v2-server/tests/`

```rust
#[tokio::test]
async fn test_grpc_health_check() {
    let addr = start_test_server().await;
    let client = HealthClient::connect(format!("http://{}", addr)).await.unwrap();
    
    let response = client.check(HealthCheckRequest {}).await.unwrap();
    assert_eq!(response.into_inner().status, ServingStatus::Serving as i32);
}
```

---

## References

- **ADR-0008**: Tonic for gRPC Server (`docs/adr/0008-tonic-for-grpc.md`)
- **Proto file**: `api/proto/hl7v2.proto`
- **CLI stub**: `crates/hl7v2-cli/src/serve.rs` lines 102-110
- **Tonic docs**: https://docs.rs/tonic/latest/tonic/
