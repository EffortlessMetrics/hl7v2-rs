# HL7v2 Crate Collapse Module Map

This document is the working map for collapsing implementation microcrates into modules under the canonical `hl7v2` Rust library crate. It implements the policy from [ADR-015](../adr/0015-collapse-public-crate-surface.md).

## Current State

As of 2026-05-07, the implementation modules have been collapsed into
`hl7v2`, and `cargo run -p xtask -- publish-plan` resolves the final public
package graph:

- `hl7v2`
- `hl7v2-python`
- `hl7v2-server`
- `hl7v2-cli`

The old implementation package names remain in the workspace as private
deprecated compatibility shims and test harnesses. They are retained only to
prove old import paths while the compatibility policy is still active.

## Boundary Rule

Crates are product and distribution surfaces. SRP implementation units are modules unless they require a separate release, package, binary, runtime service, or foreign-language binding boundary.

## Target Public Crates

| Crate | Role | Migration rule |
| --- | --- | --- |
| `hl7v2` | Canonical Rust library crate | Owns the core HL7 API and implementation modules. |
| `hl7v2-server` | HTTP/gRPC runtime service | Depends on `hl7v2` with runtime features; keeps Axum, Tonic, metrics, auth, CORS, and deployment config. |
| `hl7v2-cli` | Binary distribution | Depends on `hl7v2` with CLI-needed features. |
| `hl7v2-python` | Python binding package | Depends on `hl7v2`; keeps PyO3 isolated. |

## Target Internal Crates

| Crate | Role | Publish policy |
| --- | --- | --- |
| `hl7v2-e2e-tests` | End-to-end tests | `publish = false` |
| `hl7v2-test-utils` | Shared test utilities | `publish = false`; later candidate for `tests/support` |
| `hl7v2-bench` | Benchmark harness if retained | `publish = false`; prefer root `benches/` if practical |
| `xtask` | Workspace automation | `publish = false` |

## Current Crate to Target Module Map

| Current crate | Target location | Public status | Notes |
| --- | --- | --- | --- |
| `hl7v2` | `hl7v2` crate root | Public | Becomes the real facade and implementation crate. |
| `hl7v2-core` | temporary compatibility shim or removed | Temporary | Do not keep as a second facade. |
| `hl7v2-model` | `hl7v2::model` | Module | Foundation data structures. |
| `hl7v2-escape` | `hl7v2::escape` | Module | Small standalone escaping helpers. |
| `hl7v2-mllp` | `hl7v2::transport::mllp` | Module | Core MLLP framing; not async networking. |
| `hl7v2-network` | `hl7v2::transport::network` | Feature-gated module | Async MLLP client/server behind `network`. |
| `hl7v2-path` | `hl7v2::query::path` | Module | Path parsing belongs with query. |
| `hl7v2-query` | `hl7v2::query` | Module | Field/query access and presence semantics. |
| `hl7v2-json` | `hl7v2::writer::json` | Module | JSON serialization folds into writer unless a later PR proves otherwise. |
| `hl7v2-parser` | `hl7v2::parser` | Module | Also exposes top-level `parse`, `parse_mllp`, `parse_batch`, `parse_file_batch`. |
| `hl7v2-writer` | `hl7v2::writer` | Module | Also exposes top-level `write`, `write_mllp`, and JSON helpers. |
| `hl7v2-normalize` | `hl7v2::normalize` | Module | Top-level `normalize` remains stable. |
| `hl7v2-batch` | `hl7v2::batch` | Collapsed as leaf feature module | Batch file parsing/writing; `hl7v2-batch` remains a compatibility shim. |
| `hl7v2-stream` | `hl7v2::stream` | Feature-gated module | Event parser behind `stream`. |
| `hl7v2-prof` | `hl7v2::conformance::profile` | Module | Profile loading, inheritance, cache, and profile-backed validation. |
| `hl7v2-validation` | `hl7v2::conformance::validation` | Module | Issues, severity, validators, and validation engine helpers. |
| `hl7v2-datatype` | `hl7v2::conformance::datatype` | Collapsed as leaf conformance module | Primitive/composite data type validation; `hl7v2-datatype` remains a compatibility shim. |
| `hl7v2-datetime` | `hl7v2::conformance::datatype::datetime` | Collapsed as leaf conformance module | Datetime parsing is part of datatype validation; `hl7v2-datetime` remains a compatibility shim. |
| `hl7v2-ack` | `hl7v2::ack` | Collapsed as ACK feature module | First-class runtime HL7 behavior; `hl7v2-ack` remains a compatibility shim. |
| `hl7v2-faker` | `hl7v2::synthetic::faker` | Collapsed as synthetic feature module | Test/synthetic data generation behind `synthetic`; `hl7v2-faker` remains a compatibility shim. |
| `hl7v2-corpus` | `hl7v2::synthetic::corpus` | Collapsed as synthetic feature module | Corpus manifest and lock behavior behind `synthetic`; `hl7v2-corpus` remains a compatibility shim. |
| `hl7v2-template` | `hl7v2::synthetic::template` | Collapsed as synthetic feature module | Template model/rendering behind `synthetic`; `hl7v2-template` remains a compatibility shim. |
| `hl7v2-template-values` | `hl7v2::synthetic::values` | Collapsed as synthetic feature module | Value distributions/sources behind `synthetic`; `hl7v2-template-values` remains a compatibility shim. |
| `hl7v2-gen` | `hl7v2::synthetic::generate` | Collapsed as synthetic feature module | Generation facade behind `synthetic`; ACK remains in `hl7v2::ack`; `hl7v2-gen` remains a compatibility shim. |
| `hl7v2-redact` | `hl7v2::redact` | Feature-gated module | Collapsed as a leaf feature module; `hl7v2-redact` is a compatibility shim. |
| `hl7v2-lifecycle` | `hl7v2::lifecycle` | Collapsed as leaf feature module | `hl7v2-lifecycle` remains a compatibility shim. |
| `hl7v2-guard` | `hl7v2::experimental::guard` | Collapsed as leaf experimental feature module | `hl7v2-guard` remains a compatibility shim; do not present guard as stable until semantics are proven. |
| `hl7v2-server` | `crates/hl7v2-server` | Public crate | Keep external. |
| `hl7v2-cli` | `crates/hl7v2-cli` | Public crate | Keep external. |
| `hl7v2-python` | `crates/hl7v2-python` | Public crate | Keep external. |
| `hl7v2-bench` | root `benches/` or private crate | Internal | Prefer root benches; keep private crate only if needed. |
| `hl7v2-e2e-tests` | `crates/hl7v2-e2e-tests` | Internal | Keep or later move to workspace tests. |
| `hl7v2-test-utils` | `crates/hl7v2-test-utils` or `tests/support` | Internal | Keep until shared helpers can move safely. |
| `xtask` | `xtask` | Internal | Keep. |

## Target Feature Flags

| Feature | Dependencies | Modules |
| --- | --- | --- |
| `default` | `std`, `serde`, `json`, `profile`, `ack`, `normalize` | Normal parse/inspect/validate/ACK/normalize/serialize workflow. |
| `std` | none | Standard-library support. |
| `serde` | `serde` | Serialization derives and models. |
| `json` | `serde`, `serde_json` | JSON writer helpers. |
| `profile` | `serde`, `serde_yaml`, `regex` | Profile loading and validation. |
| `ack` | none | ACK generation. |
| `normalize` | none | Normalization helpers. |
| `batch` | none unless later proven needed | Batch parsing/writing. |
| `stream` | `tokio` if async stream support remains in the same feature | Streaming parser APIs. |
| `network` | `stream`, `tokio`, `tokio-util`, `futures`, `bytes` | Async MLLP networking. |
| `synthetic` | `ack`, `serde`, `chrono`, `rand`, `rand_distr`, `serde_json`, `serde_yaml`, `sha2`, `uuid` | Faker, corpus, templates, values, generation. |
| `redact` | none | Redaction rules/policies. |
| `lifecycle` | `chrono`, `serde`, `sha2` | Retention/archive lifecycle. |
| `experimental-guard` | `serde` | Experimental guard/anomaly detection. |

## Dependency-Order Migration

Move from the dependency floor upward:

1. `path`
2. `model`
3. `escape` and MLLP framing
4. `query`
5. `parser`
6. `writer`, `json`, and `normalize`
7. `validation`, `datatype`, and `datetime`
8. `profile`
9. `ack`
10. `synthetic`
11. `batch`, `stream`, and `network`
12. `redact`, `lifecycle`, and `experimental::guard`
13. server, CLI, Python, examples, e2e tests, and test-utils imports
14. workspace member removal
15. compatibility shim removal or freeze

`path` is intentionally split out first because it has no implementation-crate
dependencies. Moving `model`, `escape`, or MLLP while parser/writer/query still
depend on those crates would force compatibility shims to depend back on
`hl7v2` and create Cargo cycles.

Leaf feature crates with no remaining implementation-crate dependents can also
collapse before the foundation block is fully resolved. Those PRs must be
narrow, keep compatibility shims, and state why they do not introduce cycles.
`hl7v2-redact`, `hl7v2-guard`, `hl7v2-lifecycle`, `hl7v2-datatype`,
`hl7v2-datetime`, and `hl7v2-batch` used that path: their implementations now
live under `hl7v2`, and the old crates are compatibility shims.

The synthetic crates collapsed as one cluster instead of one crate per PR.
`hl7v2-faker`, `hl7v2-corpus`, `hl7v2-template-values`, `hl7v2-template`, and
`hl7v2-gen` depend on each other enough that collapsing only one would make its
shim depend back on `hl7v2` while another synthetic implementation crate still
depended on the shim. Moving the cluster together preserves behavior without
introducing Cargo cycles.

## Import Conversion Rules

Moved implementation code should use crate-internal paths:

```rust
use crate::model::{Message, Segment};
use crate::parser::parse;
use crate::writer::write;
```

Application, wrapper, and public API tests should use the public product crate:

```rust
use hl7v2::{ack, get, load_profile_checked, parse, validate, write};
```

Module-specific tests can use module paths when they are testing that module directly:

```rust
use hl7v2::conformance::datatype::parse_hl7_ts;
```

## PR Sequence

| PR lane | Branch | Scope |
| --- | --- | --- |
| 1 | `docs/crate-surface-collapse-adr` | ADR and module map only. |
| 2 | `refactor/hl7v2-canonical-facade` | Make `hl7v2` re-export the stable API directly from current crates; no file moves. |
| 3 | `refactor/collapse-path-module` | Move `hl7v2-path` into `hl7v2::query::path`; keep a temporary shim. |
| 4 | `refactor/collapse-foundation-modules` | Move `model`, `escape`, and MLLP framing once their downstream users can be handled without cycles. |
| 5 | `refactor/collapse-query-modules` | Move query access/presence after path is already internal. |
| 6 | `refactor/collapse-parse-write-modules` | Move parser, writer, JSON, and normalize. |
| 7 | `refactor/collapse-conformance-modules` | Move validation, datatype, datetime, and profile. |
| 8 | `refactor/collapse-ack-synthetic-modules` | Move ACK and synthetic modules. |
| 9 | `refactor/collapse-transport-processing-modules` | Move batch, stream, and network. |
| 10 | `refactor/collapse-operational-modules` | Move redact, lifecycle, and experimental guard. |
| 11 | `refactor/update-app-crates-to-hl7v2` | Make server, CLI, Python, examples, e2e tests, and test-utils import through `hl7v2`. |
| 12 | `refactor/remove-microcrate-workspace-members` | Shrink `[workspace].members`. |
| 13 | `refactor/remove-compat-shims` | Remove or freeze compatibility shims. |

## Per-PR Rules

- One layer per PR.
- Preserve behavior.
- Do not weaken tests.
- Do not collapse `hl7v2-server`, `hl7v2-cli`, or `hl7v2-python` into `hl7v2`.
- Do not keep `hl7v2-core` as a second facade.
- Do not merge #380 as part of this migration.
- State whether a PR adds, keeps, or removes compatibility shims.
- Run focused tests for the moved layer plus a workspace check before merge.
