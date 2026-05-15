# ADR-0008: Tonic for gRPC Server

**Status**: Accepted; implemented incrementally

**Date**: 2025-11-19

**Updated**: 2026-05-15

**Deciders**: Architecture team

**Technical Story**: Healthcare integration engines and high-throughput clinical
systems often prefer gRPC for typed, binary message exchange. The gRPC surface
complements the Axum HTTP API and gives integration operators a typed transport
for parser, validation, redaction, inline corpus evidence, ACK, normalization,
and health checks.

## Context

When this ADR was first written, `hl7v2-server` already provided an HTTP REST
API and the CLI exposed a `--mode grpc` option, but the gRPC path returned a
stub error. A Protocol Buffers service definition existed, while no `tonic`,
`prost`, generated protobuf bindings, or runtime server wiring had landed.

Current implementation note: `hl7v2-server` now uses Tonic, Prost, and
`tonic-build`; the protobuf contract lives at
`api/proto/hl7v2/v1/hl7v2.proto`; generated bindings are compiled into the
server crate; and `hl7v2 serve --mode grpc` starts the Tonic service through
the same runtime crate used by the CLI serve command.

The current `HL7Service` protobuf contract includes:

1. `Parse` -- Parse a single HL7v2 message.
2. `ParseStream` -- Parse a stream of request messages into response messages.
3. `Validate` -- Validate a message against a profile.
4. `ValidateRedacted` -- Redact and validate with opt-in evidence fields.
5. `CorpusSummarize` -- Summarize inline corpus messages.
6. `CorpusFingerprint` -- Fingerprint inline corpus messages.
7. `GenerateAck` -- Generate an ACK/NAK response.
8. `Normalize` -- Normalize message delimiters and structure.
9. `HealthCheck` -- Return service health status.

The gRPC surface remains intentionally narrower than the full HTTP/CLI/Python
evidence sidecar. Profile lint/test/explain, corpus diff, bundle, replay, and
quarantine behavior remain outside the protobuf contract unless a later ADR or
PR promotes those operations. The current corpus summary and fingerprint RPCs
cover inline caller-supplied messages only; they do not read request filesystem
paths.

## Decision

Use **Tonic** as the gRPC framework for `hl7v2-rs`.

Tonic is the standard gRPC implementation for the Tokio ecosystem and builds on
`hyper`, `tower`, and `prost`. It fits the existing async runtime and server
stack selected in ADR-0003 and ADR-0007.

## Consequences

### Positive

- **Tokio-native runtime**: gRPC uses the same async foundation as the HTTP
  server.
- **Typed transport**: Protobuf-generated request and response types give
  client and server implementations an explicit contract.
- **Streaming path**: `ParseStream` provides a typed route for streaming-style
  parse workflows.
- **Polyglot clients**: The `.proto` contract can generate clients in other
  runtimes.
- **Shared service state**: `Hl7ServiceImpl` maps gRPC requests back to the
  canonical `hl7v2` parser, validation, redaction, ACK, normalization, and
  corpus evidence code.

### Negative

- **Build complexity**: Protobuf generation adds `protoc` and `tonic-build`
  expectations to the server build.
- **Contract synchronization**: gRPC message types must stay aligned with the
  Rust model and evidence contracts.
- **Debugging friction**: Binary protobuf payloads are less convenient for ad
  hoc troubleshooting than JSON.
- **Scope split**: Some evidence operations remain HTTP/CLI/Python only until a
  focused contract expansion promotes them to protobuf RPCs.

### Neutral

- **Complementary to HTTP**: gRPC does not replace the Axum HTTP API.
- **Beta runtime surface**: The service is implemented and tested, but it still
  has intentionally narrower evidence coverage than HTTP/CLI/Python.
- **Versioned proto package**: `hl7v2.v1` leaves room for future incompatible
  protobuf contracts.

## Alternatives Considered

### grpcio

`grpcio` wraps the official C gRPC implementation. It is battle-tested, but it
requires a larger native toolchain, is not Tokio-native, and complicates the
project's reproducible build posture. Tonic avoids those C dependency costs.

### Custom HTTP/2 Implementation

A custom transport would provide maximum control but would require reimplementing
HTTP/2 framing, flow control, gRPC semantics, and client tooling. That effort
would not buy useful risk reduction compared with Tonic.

### HTTP-only

The HTTP API remains the easiest surface for casual use and evidence sidecar
workflows. It does not cover typed gRPC clients or streaming-oriented
integration-engine paths, so gRPC is additive rather than a replacement.

### Connect RPC

Connect would offer browser- and HTTP/1.1-friendly options, but the Rust
ecosystem did not have a mature implementation when this decision was made.
Tonic gives the repo a production-quality Rust path now.

## Current Implementation

- **Proto file**: `api/proto/hl7v2/v1/hl7v2.proto`.
- **Server implementation**: `crates/hl7v2-server/src/grpc.rs` implements the
  generated `Hl7Service` trait.
- **Runtime entry**: `hl7v2 serve --mode grpc` starts the Tonic server through
  `crates/hl7v2-cli/src/serve.rs`.
- **Server wiring**: `crates/hl7v2-server/src/server.rs` exposes `serve_grpc`
  and configures the generated service.
- **Scope**: gRPC currently covers parse, stream parse, validate,
  validate-redacted, corpus summarize, corpus fingerprint, ACK generation,
  normalize, and health.

## Remaining Work

- Decide whether profile lint/test/explain, corpus diff, bundle, replay, and
  quarantine should become protobuf RPCs or remain HTTP/CLI/Python evidence
  surfaces.
- Keep gRPC auth, rate-limit, and deployment behavior aligned with the server
  support-tier claims.
- Add network-level integration tests with `tonic::transport::Channel` clients
  where they buy coverage beyond the current service contract tests.
- Benchmark gRPC and HTTP for parse and validation workloads.
- Document the narrower gRPC evidence scope in user-facing deployment guidance.

## References

- [Tonic](https://github.com/hyperium/tonic) -- Tokio-native gRPC framework for Rust.
- [Prost](https://github.com/tokio-rs/prost) -- Protocol Buffers implementation for Rust.
- [gRPC](https://grpc.io/) -- High-performance RPC framework.
- [Protocol Buffers](https://protobuf.dev/) -- Language-neutral serialization format.
- [Tower](https://github.com/tower-rs/tower) -- Middleware framework shared by Axum and Tonic.
- `api/proto/hl7v2/v1/hl7v2.proto` -- Service definition.
- `crates/hl7v2-server/src/grpc.rs` -- Tonic service implementation.
- `crates/hl7v2-cli/src/serve.rs` -- `hl7v2 serve --mode grpc` runtime entry.
- ADR-0003: Use Tokio for Async Runtime -- Foundation for async I/O.
- ADR-0007: Axum for HTTP Server -- Complementary HTTP server decision.
