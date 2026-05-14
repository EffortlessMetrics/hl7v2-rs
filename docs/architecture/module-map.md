# HL7v2 Module Map

This document records the current post-1.2.1 shape after the public crate
surface collapse described by [ADR-015](../adr/0015-collapse-public-crate-surface.md).

## Current State

As of 2026-05-08, the implementation modules have been collapsed into the
canonical `hl7v2` Rust library crate. Local compatibility shim crate folders
were retired after the v1.2.1 release. `cargo run -p xtask -- publish-plan`
defaults to the primary Rust product graph:

1. `hl7v2`
2. `hl7v2-server`
3. `hl7v2-cli`

`hl7v2-python` remains in the workspace as a separate PyO3 binding backend
crate. Current metadata makes it publishable as binding infrastructure; ADR
[HL7V2-ADR-0003](../adr/HL7V2-ADR-0003-publishable-binding-backend-crates.md)
keeps it separate from the primary Rust product graph until release receipts
make any backend upload explicit.
`cargo run -p xtask -- publish-plan --surface bindings` reports that graph
separately from the primary Rust release plan.

Historical old microcrate package names may exist on crates.io, but those names
are compatibility artifacts. New Rust code should depend on `hl7v2` and use
module paths under that crate.

## Boundary Rule

Crates are product and distribution surfaces. SRP implementation units are
modules unless they require a separate release, package, binary, runtime
service, or foreign-language binding boundary.

## Crate Classes

| Class | Examples | Audience | Publishable |
| --- | --- | --- | --- |
| Primary Rust product | `hl7v2`, `hl7v2-server`, `hl7v2-cli` | Rust users and operators | Yes |
| Language package | PyPI `hl7v2`, future npm `@effortlessmetrics/hl7v2` | Python and TypeScript users | Yes, through that language registry |
| Binding backend crate | `hl7v2-python`, future `hl7v2-wasm`, future `hl7v2-node` | Packagers and binding maintainers | Yes, when governed by binding-backend release policy |
| Internal/dev crate | `hl7v2-e2e-tests`, `hl7v2-test-utils`, `hl7v2-bench`, `xtask`, root examples | Repository implementation | No |

Binding backend crates are real language-boundary APIs. They are not the
recommended Rust API for normal users.

## Workspace Crates

| Crate | Role | Publish policy |
| --- | --- | --- |
| `hl7v2` | Canonical Rust library crate and implementation home. | Public crates.io package. |
| `hl7v2-server` | HTTP/gRPC runtime service with Axum, Tonic, metrics, auth, CORS, and deployment config. | Public crates.io package. |
| `hl7v2-cli` | Command-line binary distribution. | Public crates.io package. |
| `hl7v2-python` | PyO3 binding backend for the public Python `hl7v2` package. Rust users should depend on `hl7v2`. | Publishable binding backend crate; not part of the primary Rust product graph and no upload claim exists without a receipt. |
| `hl7v2-e2e-tests` | End-to-end tests. | `publish = false`. |
| `hl7v2-test-utils` | Shared test utilities. | `publish = false`; later candidate for `tests/support`. |
| `hl7v2-bench` | Benchmark harness. | `publish = false`; later candidate for root `benches/`. |
| root `hl7v2-examples` package | Examples. | `publish = false`. |
| `xtask` | Workspace automation. | `publish = false`. |

## Former Package To Current Module Map

| Former package | Current location | Notes |
| --- | --- | --- |
| `hl7v2-core` | Removed locally | `hl7v2` is the only canonical Rust facade. |
| `hl7v2-model` | `hl7v2::model` | Foundation data structures. |
| `hl7v2-escape` | `hl7v2::escape` | HL7 v2 escape and unescape helpers. |
| `hl7v2-mllp` | `hl7v2::transport::mllp` | Core MLLP framing. |
| `hl7v2-network` | `hl7v2::transport::network` | Async MLLP client/server behind `network`. |
| `hl7v2-path` | `hl7v2::query::path` | Path parsing belongs with query. |
| `hl7v2-query` | `hl7v2::query` | Field/query access and presence semantics. |
| `hl7v2-json` | `hl7v2::writer::json` | JSON serialization lives with writer. |
| `hl7v2-parser` | `hl7v2::parser` | Also exposes top-level `parse`, `parse_mllp`, `parse_batch`, and `parse_file_batch`. |
| `hl7v2-writer` | `hl7v2::writer` | Also exposes top-level `write`, `write_mllp`, and JSON helpers. |
| `hl7v2-normalize` | `hl7v2::normalize` | Top-level `normalize` remains stable. |
| `hl7v2-batch` | `hl7v2::batch` | Batch file parsing/writing. |
| `hl7v2-stream` | `hl7v2::stream` | Event parser behind `stream`. |
| `hl7v2-prof` | `hl7v2::conformance::profile` | Profile loading, inheritance, cache, and profile-backed validation. |
| `hl7v2-validation` | `hl7v2::conformance::validation` | Issues, severity, validators, and validation engine helpers. |
| `hl7v2-datatype` | `hl7v2::conformance::datatype` | Primitive/composite data type validation. |
| `hl7v2-datetime` | `hl7v2::conformance::datatype::datetime` | Datetime parsing is part of datatype validation. |
| `hl7v2-ack` | `hl7v2::ack` | First-class runtime HL7 ACK behavior. |
| `hl7v2-faker` | `hl7v2::synthetic::faker` | Test/synthetic data generation behind `synthetic`. |
| `hl7v2-corpus` | `hl7v2::synthetic::corpus` | Corpus manifest and lock behavior behind `synthetic`. |
| `hl7v2-template` | `hl7v2::synthetic::template` | Template model/rendering behind `synthetic`. |
| `hl7v2-template-values` | `hl7v2::synthetic::values` | Value distributions/sources behind `synthetic`. |
| `hl7v2-gen` | `hl7v2::synthetic::generate` | Generation facade behind `synthetic`; ACK remains in `hl7v2::ack`. |
| `hl7v2-redact` | `hl7v2::redact` | Redaction rules and policies. |
| `hl7v2-lifecycle` | `hl7v2::lifecycle` | Retention/archive lifecycle. |
| `hl7v2-guard` | `hl7v2::experimental::guard` | Experimental guard/anomaly detection. |

## Feature Flags

| Feature | Modules |
| --- | --- |
| `default` | Normal parse/inspect/validate/ACK/normalize/serialize workflow. |
| `json` | JSON writer helpers. |
| `profile` | Profile loading and validation. |
| `ack` | ACK generation. |
| `normalize` | Normalization helpers. |
| `batch` | Batch parsing/writing. |
| `stream` | Streaming parser APIs. |
| `network` | Async MLLP networking. |
| `synthetic` | Faker, corpus, templates, values, and generation. |
| `redact` | Redaction rules/policies. |
| `lifecycle` | Retention/archive lifecycle. |
| `experimental-guard` | Experimental guard/anomaly detection. |

## Import Rules

Public consumers should use `hl7v2`:

```rust
use hl7v2::{ack, get, load_profile_checked, parse, validate, write};
```

Module-specific tests and implementers can use module paths:

```rust
use hl7v2::conformance::datatype::parse_hl7_ts;
```

Inside `crates/hl7v2`, implementation code should use crate-internal module
paths such as `crate::model`, `crate::parser`, and `crate::writer`.

## Historical Migration

The collapse sequence was completed in stages: facade inversion, first-party
consumer migration, module collapse by dependency layer, workspace membership
cleanup, canonical facade tests, and local shim-folder deletion. The historical
planning details remain in ADR-015 and dated audit documents; this file now
describes the active repository shape.

## Rules For Future Changes

- Do not reintroduce implementation microcrates for ordinary SRP boundaries.
- Do not collapse `hl7v2-server`, `hl7v2-cli`, or `hl7v2-python` into `hl7v2`.
- Do not keep `hl7v2-core` as a second facade.
- Add new workspace crates only for product, runtime, binary, language binding,
  test, benchmark, or tool boundaries.
- Binding backend crates must stay thin over `hl7v2`; do not use them to split
  parser, model, redaction, transport, or evidence implementation back into
  public Rust microcrates.
