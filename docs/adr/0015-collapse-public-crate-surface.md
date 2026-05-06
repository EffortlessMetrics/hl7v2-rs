# ADR-015: Collapse Public Crate Surface

**Date**: 2026-05-06
**Status**: Accepted
**Deciders**: Project Team
**Technical Story**: Crate-surface cleanup after contract/runtime/publish-proof hardening

## Context

The current workspace exposes many SRP microcrates as independent package identities. That split made earlier refactors easier to stage, but it now makes the product surface harder to explain and harder to publish coherently.

The current facade is also inverted:

- `hl7v2` is the name users expect, but it only re-exports `hl7v2-core`.
- `hl7v2-core` re-exports the implementation microcrates and still points new users toward depending on microcrates directly.
- The workspace has package-verified publish readiness, but that proof currently has to reason about a long dependency-ordered package chain instead of one normal library crate plus runtime/binding wrappers.

After the contract workflow, HTTP runtime, gRPC behavior, and publish packaging repairs, the remaining architecture problem is not another recovery pass. The public package boundary should now tell the truth: Rust users depend on `hl7v2`; operators run `hl7v2-server`; shell users install `hl7v2-cli`; Python users install the Python binding package.

This ADR supersedes the public-crate-surface direction in `docs/MICROCRATE_ANALYSIS.md`. That document remains useful historical context for how the current microcrate shape was produced, but it is no longer the target architecture.

## Decision

Crates are product and distribution surfaces. SRP implementation units are modules unless they require a separate release, package, binary, runtime service, or foreign-language binding boundary.

The target public package surface is:

| Crate | Role |
| --- | --- |
| `hl7v2` | Canonical Rust library facade and implementation crate. |
| `hl7v2-server` | HTTP/gRPC runtime service with Axum, Tonic, metrics, auth, CORS, and deployment behavior. |
| `hl7v2-cli` | Binary distribution for shell users. |
| `hl7v2-python` | PyO3/Python binding package. |

The target internal package surface is:

| Crate | Role |
| --- | --- |
| `hl7v2-e2e-tests` | Workspace-only end-to-end tests. |
| `hl7v2-test-utils` | Workspace-only test helpers, or later `tests/support`. |
| `hl7v2-bench` | Workspace-only benchmark harness if Criterion setup still needs a crate; otherwise move to root `benches/`. |
| `xtask` | Workspace-only automation tool. |

All other current `hl7v2-*` crates should collapse into modules under `crates/hl7v2/src/`, or temporarily remain as compatibility shims if there is a deliberate compatibility reason.

## Target Module Layout

The canonical Rust library crate should grow toward this module tree:

```text
crates/hl7v2/src/
  lib.rs
  model/
  escape.rs
  transport/
    mllp.rs
    network/
  parser/
  writer/
  query/
  normalize.rs
  batch/
  stream/
  ack/
  conformance/
    profile/
    validation/
    datatype/
  synthetic/
    faker/
    corpus/
    template/
    values/
    generate.rs
  redact/
  lifecycle/
  experimental/
    guard/
```

`docs/architecture/module-map.md` is the detailed current-crate to target-module map for the migration.

## Public API Target

Normal Rust users should be able to use the common API from one import path:

```rust
use hl7v2::{ack, get, normalize, parse, validate, write};
```

The top-level `hl7v2` API should expose stable convenience items:

```rust
pub use ack::{ack, ack_with_error, AckCode};
pub use conformance::profile::{load_profile, load_profile_checked, validate, Profile};
pub use conformance::validation::{Issue, Severity};
pub use escape::{escape_text, needs_escaping, needs_unescaping, unescape_text};
pub use model::{Atom, Batch, Comp, Delims, Error, Field, FileBatch, Message, Presence, Rep, Segment};
pub use normalize::normalize;
pub use parser::{parse, parse_batch, parse_file_batch, parse_mllp};
pub use query::{get, get_presence, CompiledPath, Path};
pub use transport::mllp::{find_complete_mllp_message, is_mllp_framed, unwrap_mllp, unwrap_mllp_owned, wrap_mllp, MllpFrameIterator, MLLP_END_1, MLLP_END_2, MLLP_START};
pub use writer::{to_json, to_json_string, to_json_string_pretty, write, write_batch, write_file_batch, write_mllp};
```

Implementers can still use module paths:

```rust
hl7v2::model
hl7v2::parser
hl7v2::writer
hl7v2::query
hl7v2::transport
hl7v2::conformance
hl7v2::synthetic
hl7v2::lifecycle
```

## Feature Policy

Feature flags replace public microcrate selection for dependency control.

| Feature | Modules |
| --- | --- |
| default | model, escape, MLLP framing, parser, writer, query, json, normalize, ack, profile validation |
| `batch` | batch parsing and writing |
| `stream` | event stream parser |
| `network` | async MLLP client/server |
| `synthetic` | template, faker, corpus, and generation helpers |
| `redact` | PHI redaction |
| `lifecycle` | retention and archive lifecycle model |
| `experimental-guard` | statistical guard and anomaly detection |

Core HL7 library expectations should not be hidden behind narrow microcrate dependencies. A default `hl7v2` user should be able to parse, inspect, validate, ACK, normalize, and serialize.

Runtime-heavy dependency groups should remain feature-gated. `hl7v2-server`, `hl7v2-cli`, and `hl7v2-python` choose the `hl7v2` features they need.

## Compatibility Shim Policy

Compatibility shims are temporary and deliberate.

Use a shim only when one of these is true:

- the crate has already been published externally,
- the crate is known to have external users,
- the migration needs a short-lived internal bridge to keep a PR reviewable.

Shim crates should be small:

```rust
//! Deprecated compatibility crate.
//!
//! Use `hl7v2::model` instead.

pub use hl7v2::model::*;
```

Shim manifests should use `publish = false` unless the project deliberately chooses to publish deprecation-only compatibility crates. Shims must not become a second facade layer. `hl7v2-core` should either become a deprecated compatibility shim over `hl7v2` or be removed if there is no external publication/use reason.

## Publish Policy

Target public packages:

```text
hl7v2
hl7v2-server
hl7v2-cli
hl7v2-python
```

Target private workspace packages:

```text
hl7v2-e2e-tests
hl7v2-test-utils
hl7v2-bench, if retained
xtask
```

Collapsed implementation crates should be removed from `[workspace].members` once their modules are moved and all app/test imports go through `hl7v2`.

## Migration Sequence

Do not move all crates at once. Collapse in dependency order with one proof-backed PR per layer.

1. ADR and module map. No Rust code moves.
2. Make `hl7v2` the canonical facade. No implementation moves.
3. Collapse foundation modules: model, escape, MLLP framing, path, query.
4. Collapse parse/write/normalize/json modules.
5. Collapse conformance modules: validation, datatype, datetime, profile.
6. Collapse ACK and synthetic modules.
7. Collapse batch, stream, and network modules.
8. Collapse operational modules: redact, lifecycle, experimental guard.
9. Update server, CLI, Python, examples, e2e tests, and test utilities to import through `hl7v2`.
10. Remove collapsed microcrates from workspace membership.
11. Remove or freeze compatibility shims according to the shim policy.

Each movement PR must preserve public behavior, convert moved-code imports from `hl7v2_*` crates to `crate::...` modules, and keep application crates importing through `hl7v2`.

## Consequences

### Positive

- Users get one canonical Rust library crate.
- Publish readiness becomes easier to reason about.
- SRP boundaries remain visible as modules without multiplying public packages.
- Runtime and binding wrappers keep their real packaging boundaries.
- Future docs can point to one product API instead of many implementation crates.

### Negative

- The migration is mechanically large and must be staged carefully.
- Compatibility shims may be needed for a transition period.
- Feature gates must be designed carefully so default users keep expected HL7 behavior without inheriting runtime-heavy dependencies.

### Neutral

- Historical microcrate docs remain useful background, but the new architecture policy points in the opposite direction.
- Some tests may move from crate-level tests into module-level or public API tests as the workspace shrinks.

## Alternatives Considered

### Keep the current microcrate surface

**Pros:**
- Lowest short-term churn.
- Preserves current dependency granularity.

**Cons:**
- Keeps the inverted facade.
- Makes normal user documentation harder.
- Makes publish sequencing and compatibility policy harder than the product needs.

**Why not chosen:** The current shape exposes implementation seams as product packages and no longer matches the desired public surface.

### Keep `hl7v2-core` as the main facade

**Pros:**
- Smaller immediate code change.
- Preserves the current facade crate name.

**Cons:**
- Keeps two Rust library facades.
- Makes the obvious crate name, `hl7v2`, a wrapper around a wrapper.

**Why not chosen:** `hl7v2` should be the product crate.

### Collapse server, CLI, or Python into `hl7v2`

**Pros:**
- Fewer workspace members.

**Cons:**
- Pulls runtime, binary, and PyO3 packaging concerns into the normal Rust library crate.
- Makes dependency and release boundaries less honest.

**Why not chosen:** Server, CLI, and Python are real product/package boundaries.

## Implementation Notes

- Start with `refactor/hl7v2-canonical-facade`: make `hl7v2` depend directly on current implementation crates and expose the target top-level API without moving files.
- Collapse modules from the dependency floor upward: model first, then escape/MLLP, then query/path, then parser/writer/normalize, then conformance, then generation, then runtime-adjacent modules.
- Keep `hl7v2-server`, `hl7v2-cli`, and `hl7v2-python` independent public packages.
- Keep #380 and unrelated automation/control-plane PRs out of this migration.

## References

- `docs/architecture/module-map.md`
- `docs/MICROCRATE_ANALYSIS.md`
- `docs/audits/publish-dry-run-2026-05-06.md`
