# Publish Dry-Run Receipt - 2026-05-06

## Context

This receipt records the current crates.io publish-readiness boundary for `hl7v2-rs` after merge commit `6de37e0`.

Commands run locally on Windows:

```powershell
cargo run -p xtask -- publish-plan
$env:PYO3_USE_ABI3_FORWARD_COMPATIBILITY='1'
cargo publish --dry-run -p <crate> --locked
cargo run -p xtask -- publish-dry-run --from hl7v2 --workspace-patches
```

`PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1` was set before the dry-run loop because the local machine uses a newer Python toolchain than the PyO3 version in this workspace expects by default.

## Publish Order

`cargo run -p xtask -- publish-plan` returned 30 publishable crates:

| Order | Crate | Direct registry-state dry-run | Workspace-patched dry-run |
| --- | --- | --- | --- |
| 1 | `hl7v2-datetime` | Pass | Covered by direct dry-run |
| 2 | `hl7v2-datatype` | Pass | Covered by direct dry-run |
| 3 | `hl7v2-faker` | Pass | Covered by direct dry-run |
| 4 | `hl7v2-model` | Pass | Covered by direct dry-run |
| 5 | `hl7v2-escape` | Pass | Covered by direct dry-run |
| 6 | `hl7v2-json` | Pass | Covered by direct dry-run |
| 7 | `hl7v2-mllp` | Pass | Covered by direct dry-run |
| 8 | `hl7v2-path` | Pass | Covered by direct dry-run |
| 9 | `hl7v2-query` | Pass | Covered by direct dry-run |
| 10 | `hl7v2-parser` | Pass | Covered by direct dry-run |
| 11 | `hl7v2-batch` | Pass | Covered by direct dry-run |
| 12 | `hl7v2-stream` | Pass | Covered by direct dry-run |
| 13 | `hl7v2-writer` | Pass | Covered by direct dry-run |
| 14 | `hl7v2-network` | Pass | Covered by direct dry-run |
| 15 | `hl7v2-normalize` | Pass | Covered by direct dry-run |
| 16 | `hl7v2-core` | Pass | Covered by direct dry-run |
| 17 | `hl7v2` | Blocked by registry dependency state | Pass |
| 18 | `hl7v2-ack` | Not run after direct stop | Pass |
| 19 | `hl7v2-corpus` | Not run after direct stop | Pass |
| 20 | `hl7v2-guard` | Not run after direct stop | Pass |
| 21 | `hl7v2-lifecycle` | Not run after direct stop | Pass |
| 22 | `hl7v2-python` | Not run after direct stop | Pass |
| 23 | `hl7v2-redact` | Not run after direct stop | Pass |
| 24 | `hl7v2-template-values` | Not run after direct stop | Pass |
| 25 | `hl7v2-template` | Not run after direct stop | Pass |
| 26 | `hl7v2-gen` | Not run after direct stop | Pass |
| 27 | `hl7v2-validation` | Not run after direct stop | Pass |
| 28 | `hl7v2-prof` | Not run after direct stop | Pass |
| 29 | `hl7v2-server` | Not run after direct stop | Pass |
| 30 | `hl7v2-cli` | Not run after direct stop | Pass |

## Stop Condition

The direct dry-run stopped at `hl7v2`:

```text
error: failed to prepare local package for uploading

Caused by:
  no matching package named `hl7v2-core` found
  location searched: crates.io index
  required by package `hl7v2 v1.2.0`
```

## Interpretation

Crates 1-16 package and verify with `cargo publish --dry-run --locked`.

The stop at `hl7v2` is expected for a direct dry-run before publishing the dependency chain: during packaging, Cargo resolves registry dependencies from crates.io, and `hl7v2-core` is not available there yet. This means higher-level crates cannot be direct-dry-run proven from the current public registry state until the dependency chain is actually published.

The workspace-patched dry-run is a simulation command, not a crates.io upload and not a claim that dependencies already exist in the public registry. It still runs `cargo publish --dry-run --locked`; it supplies unpublished internal crates through generated Cargo `[patch.crates-io]` entries so higher-level packages verify against the current workspace dependency chain.

The simulation found and fixed concrete package verification defects:

- `hl7v2-lifecycle`, `hl7v2-python`, and `hl7v2-redact` had unused dev-dependencies on unpublished `hl7v2-test-utils`.
- `hl7v2-server` generated gRPC bindings from `api/proto` outside the package tarball.
- `hl7v2-server` embedded OpenAPI YAML from `api/openapi` outside the package tarball.

After those fixes, `cargo run -p xtask -- publish-dry-run --from hl7v2 --workspace-patches` verifies crates 17-30 in publish order.

## Current Label

Current status is **package-verified publish readiness**:

- green main workflows: proven
- runtime and contract behavior: proven for the named HTTP/gRPC/schema surfaces
- direct dry-run publish: proven through `hl7v2-core`
- higher-level package verification: proven by workspace-patched `cargo publish --dry-run`
- real crates.io publish sequence: not executed
