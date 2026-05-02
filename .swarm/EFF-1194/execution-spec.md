# EFF-1194: gRPC Server Implementation - Execution Specification

## Overview

This specification documents the implementation requirements for gRPC server support in the hl7v2-rs toolkit, addressing the gap between the complete proto file design and the current stub implementation.

**Issue**: [EFF-1194](/EFF/issues/EFF-1194)  
**GitHub Issue**: [#303](https://github.com/EffortlessMetrics/hl7v2-rs/issues/303)  
**Branch**: `feature/grpc-server-impl`  
**PR**: [#311](https://github.com/EffortlessMetrics/hl7v2-rs/pull/311)  
**Status**: In Progress  
**Priority**: High  

---

## Current State

### Verified Gap

| Component | Status | Evidence |
|-----------|--------|----------|
| Proto file | ✅ Complete | `api/proto/hl7v2.proto` - 323 lines, 6 RPCs |
| ADR-0008 | ✅ Accepted | Tonic selected as gRPC framework |
| CLI stub | ⚠️ Implemented | `crates/hl7v2-cli/src/serve.rs:102-110` - returns hardcoded error |
| Tonic deps | ❌ Missing | No tonic/tonic-build/prost in workspace |
| Codegen | ❌ Missing | No build.rs or generated code |
| Service impl | ❌ Missing | No HL7Service trait implementation |

### ADR Reference

- **ADR-0008**: Tonic for gRPC Server - Status: **Accepted**
- Architecture: Tonic (Tokio-native, Tower middleware compatible)
- Proto path: `api/proto/hl7v2.proto`

---

## Requirements

### Functional Requirements

1. **Parse RPC** - Parse HL7 v2 messages via gRPC
   - Input: Raw HL7 message bytes
   - Output: Structured message with metadata
   - Support MLLP-framed messages

2. **ParseStream RPC** - Bidirectional streaming for batch processing
   - Stream-in: Multiple parse requests
   - Stream-out: Corresponding parse responses
   - Handle backpressure via HTTP/2 flow control

3. **Validate RPC** - Profile validation endpoint
   - Input: Message bytes + profile identifier
   - Output: Validation result with errors/warnings

4. **GenerateAck RPC** - ACK message generation
   - Input: Original message + ACK code (AA/AE/AR)
   - Output: Generated ACK message

5. **Normalize RPC** - Message normalization
   - Input: Message bytes + normalization options
   - Output: Normalized message

6. **HealthCheck RPC** - Service health status
   - Standard gRPC health check protocol
   - Include version and uptime

### Non-Functional Requirements

1. **Performance**: gRPC throughput should match or exceed HTTP API
2. **Compatibility**: Share Tower middleware with existing Axum HTTP server
3. **Port Strategy**: Separate gRPC port (9090) from HTTP port (8080)
4. **Build**: protoc must be available in Nix dev shell and CI
5. **Feature Flag**: gRPC behind `--features grpc` for lean default builds

---

## Architecture

### Workspace Structure

```
crates/
├── hl7v2-server/          # Existing HTTP server (Axum)
│   └── src/
│       └── grpc/          # NEW: gRPC module (conditionally compiled)
├── hl7v2-cli/
│   └── src/serve.rs       # MODIFIED: Replace run_grpc_server() stub
└── hl7v2-grpc/            # OPTIONAL: Separate crate for gRPC types
```

### Dependency Strategy

```toml
# Root Cargo.toml [workspace.dependencies]
tonic = "0.12"
tonix-build = "0.12"
prost = "0.13"
prost-types = "0.13"

# crates/hl7v2-server/Cargo.toml
[features]
default = []
grpc = ["dep:tonic", "dep:prost", "tonic-build"]
```

### Proto Codegen

```rust
// crates/hl7v2-server/build.rs
fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "grpc")]
    {
        tonic_build::configure()
            .build_server(true)
            .build_client(true)
            .out_dir("src/grpc/gen")
            .compile_protos(
                &["../../api/proto/hl7v2.proto"],
                &["../../api/proto"],
            )?;
    }
    Ok(())
}
```

---

## Implementation Phases

### Phase 1: Infrastructure (2-3 hours)

- [ ] Add tonic/prost dependencies to workspace Cargo.toml
- [ ] Add tonic-build to workspace dependencies
- [ ] Update Nix flake.nix to include protoc
- [ ] Create build.rs in hl7v2-server crate
- [ ] Add `grpc` feature flag to hl7v2-server/Cargo.toml
- [ ] Verify proto code generation builds successfully

**Acceptance**: `cargo build --features grpc` compiles without errors

### Phase 2: Core RPCs (4-6 hours)

- [ ] Implement proto-to-model type conversions
- [ ] Implement Hl7Service trait for Parse RPC
- [ ] Implement Validate RPC
- [ ] Implement GenerateAck RPC
- [ ] Implement Normalize RPC
- [ ] Implement HealthCheck RPC

**Acceptance**: All unary RPCs return valid responses for valid inputs

### Phase 3: Streaming (3-4 hours)

- [ ] Implement ParseStream bidirectional streaming
- [ ] Handle request-response correlation
- [ ] Implement backpressure handling
- [ ] Add streaming error handling

**Acceptance**: Can stream 1000+ messages with backpressure

### Phase 4: Integration (2-3 hours)

- [ ] Replace run_grpc_server() stub in CLI
- [ ] Add Tower middleware (tracing, metrics)
- [ ] Configure gRPC port (default 9090)
- [ ] Add graceful shutdown handling

**Acceptance**: `hl7v2 serve --mode grpc` starts functional server

### Phase 5: Testing & Polish (3-4 hours)

- [ ] Unit tests for each RPC handler
- [ ] Integration tests with tonic client
- [ ] Proto compatibility verification
- [ ] Documentation updates
- [ ] Benchmark comparison with HTTP API

**Acceptance**: Test coverage >80%, benchmarks complete

---

## Test Strategy

### Unit Tests

- Each RPC handler tested in isolation
- Type conversion tests (proto ↔ model)
- Error handling tests

### Integration Tests

- End-to-end gRPC client/server tests
- Streaming load tests
- Middleware integration tests

### Compatibility Tests

- Proto backward compatibility checks
- Cross-language client verification (Go, Java)

---

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| protoc build dependency | High | Update Nix shell, document in DEVELOPMENT.md |
| Compile time increase | Medium | Feature flag keeps default builds lean |
| Proto drift from model | Medium | Conversion tests, CI checks |
| HTTP/2 proxy issues | Low | Document limitations, provide HTTP fallback |

---

## Open Questions

1. Should we create a separate `hl7v2-grpc` crate or keep in `hl7v2-server`?
2. Should gRPC be enabled by default once stable?
3. Do we need TLS termination for gRPC (separate from mTLS)?

---

## Deliverables

1. **Code**: Complete gRPC implementation in branch
2. **Tests**: Unit and integration test suite
3. **Docs**: Updated API documentation
4. **CI**: Updated workflows with protoc

---

## Next Owner

**Spec Verifier** - Review and approve this execution specification before implementation proceeds.
