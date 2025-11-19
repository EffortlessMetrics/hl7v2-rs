# Architecture Decision Records (ADRs)

This directory contains Architecture Decision Records (ADRs) documenting significant architectural decisions in the hl7v2-rs project.

## What is an ADR?

An ADR captures an important architectural decision made along with its context and consequences.

## Format

We use the format popularized by Michael Nygard:

- **Title**: Short noun phrase
- **Status**: Proposed, Accepted, Deprecated, Superseded
- **Context**: What forces are at play?
- **Decision**: What is the change that we're proposing/doing?
- **Consequences**: What becomes easier or harder?

## Index

| ADR | Title | Status | Date |
|-----|-------|--------|------|
| [0001](0001-use-rust-for-implementation.md) | Use Rust for Implementation | Accepted | 2025-11-19 |
| [0002](0002-adopt-event-based-streaming-parser.md) | Adopt Event-Based Streaming Parser | Accepted | 2025-11-19 |
| [0003](0003-use-tokio-for-async-runtime.md) | Use Tokio for Async Runtime | Accepted | 2025-11-19 |
| [0004](0004-schema-driven-validation.md) | Schema-Driven Validation | Accepted | 2025-11-19 |
| [0005](0005-nix-for-reproducible-builds.md) | Nix for Reproducible Builds | Accepted | 2025-11-19 |
| [0006](0006-opa-for-policy-enforcement.md) | OPA for Policy Enforcement | Accepted | 2025-11-19 |
| [0007](0007-axum-for-http-server.md) | Axum for HTTP Server | Accepted | 2025-11-19 |
| [0008](0008-tonic-for-grpc.md) | Tonic for gRPC Server | Accepted | 2025-11-19 |
| [0009](0009-serde-for-serialization.md) | Serde for Serialization | Accepted | 2025-11-19 |
| [0010](0010-thiserror-for-error-handling.md) | Thiserror for Error Handling | Accepted | 2025-11-19 |

## Creating a New ADR

```bash
# Use the next sequential number
cp docs/adr/template.md docs/adr/NNNN-title-with-dashes.md

# Edit the new file with your decision
vim docs/adr/NNNN-title-with-dashes.md

# Update this README index
```

## Superseding an ADR

When a decision is superseded:
1. Mark the old ADR status as "Superseded by ADR-NNNN"
2. Create a new ADR with the new decision
3. Link them together

## References

- [Michael Nygard's ADR format](https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions)
- [ADR GitHub organization](https://adr.github.io/)
