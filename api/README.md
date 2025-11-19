# API Contracts

This directory contains the API contracts for hl7v2-rs services.

## Contract-First Development

We follow a **contract-first** approach:

1. **Define the contract** - OpenAPI/Proto specs define the API
2. **Generate code** - Tooling generates server stubs and client libraries
3. **Implement handlers** - Business logic implements the contract
4. **Validate compliance** - Tests ensure implementation matches contract

## Contracts

### OpenAPI (REST/HTTP)

**File**: `openapi/hl7v2-api-v1.yaml`

**Endpoints**:
- `GET /health` - Health check
- `GET /ready` - Readiness check
- `GET /metrics` - Prometheus metrics
- `POST /hl7/parse` - Parse HL7 message
- `POST /hl7/validate` - Validate against profile
- `POST /hl7/ack` - Generate ACK
- `POST /hl7/normalize` - Normalize message

**Features**:
- JWT Bearer token authentication
- Request/response schemas
- Error codes and descriptions
- Rate limiting documentation
- Trace ID correlation

**Tools**:
```bash
# Validate OpenAPI spec
npx @stoplight/spectral-cli lint api/openapi/hl7v2-api-v1.yaml

# Generate Rust server stubs (future)
openapi-generator generate -i api/openapi/hl7v2-api-v1.yaml \
  -g rust-server -o generated/

# Generate documentation
npx @redocly/cli build-docs api/openapi/hl7v2-api-v1.yaml \
  -o docs/api.html
```

### gRPC (Protocol Buffers)

**File**: `proto/hl7v2.proto`

**Services**:
- `HL7Service` - Main service with all operations

**RPCs**:
- `Parse(ParseRequest) -> ParseResponse`
- `ParseStream(stream ParseRequest) -> stream ParseResponse`
- `Validate(ValidateRequest) -> ValidateResponse`
- `GenerateAck(AckRequest) -> AckResponse`
- `Normalize(NormalizeRequest) -> NormalizeResponse`
- `HealthCheck(HealthCheckRequest) -> HealthCheckResponse`

**Features**:
- Streaming support for bulk processing
- Rich type definitions
- Forward/backward compatibility
- Code generation for multiple languages

**Tools**:
```bash
# Compile proto (included in build.rs)
cargo build  # Tonic build script compiles automatically

# Generate docs
protoc --doc_out=docs --doc_opt=html,grpc-api.html \
  api/proto/hl7v2.proto

# Validate with buf
buf lint api/proto/
```

## Using the Contracts

### Rust Server Implementation

The contracts drive the implementation:

```rust
// OpenAPI -> Axum handlers
async fn parse_handler(
    Json(req): Json<ParseRequest>,
) -> Result<Json<ParseResponse>, Error> {
    // Implementation
}

// gRPC -> Tonic service
#[tonic::async_trait]
impl HL7Service for HL7ServiceImpl {
    async fn parse(
        &self,
        request: Request<ParseRequest>,
    ) -> Result<Response<ParseResponse>, Status> {
        // Implementation
    }
}
```

### Client Generation

Clients can be generated in any language:

**Go**:
```bash
protoc --go_out=. --go-grpc_out=. api/proto/hl7v2.proto
```

**Python**:
```bash
python -m grpc_tools.protoc -I. --python_out=. --grpc_python_out=. \
  api/proto/hl7v2.proto
```

**JavaScript**:
```bash
protoc --js_out=import_style=commonjs,binary:. \
  --grpc-web_out=import_style=commonjs,mode=grpcwebtext:. \
  api/proto/hl7v2.proto
```

## Contract Testing

Ensure implementation matches contracts:

```bash
# OpenAPI contract tests (Dredd)
dredd api/openapi/hl7v2-api-v1.yaml http://localhost:8080

# gRPC reflection tests
grpcurl -plaintext localhost:9090 list
grpcurl -plaintext localhost:9090 describe hl7v2.v1.HL7Service
```

## Versioning

Contracts are versioned:
- **OpenAPI**: `hl7v2-api-v1.yaml`, `hl7v2-api-v2.yaml`
- **Proto**: `package hl7v2.v1`, `package hl7v2.v2`

Breaking changes require new versions. Non-breaking changes can be added to existing versions.

## Documentation

Generated documentation:

- **OpenAPI**: [ReDoc](https://redocly.github.io/redoc/)
- **gRPC**: [protoc-gen-doc](https://github.com/pseudomuto/protoc-gen-doc)

## References

- [OpenAPI Specification](https://swagger.io/specification/)
- [Protocol Buffers](https://developers.google.com/protocol-buffers)
- [gRPC](https://grpc.io/)
- [Tonic (Rust gRPC)](https://github.com/hyperium/tonic)
- [Contract Testing](https://martinfowler.com/bliki/ContractTest.html)
