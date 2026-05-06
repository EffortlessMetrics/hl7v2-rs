# Publish Dry-Run Receipt - 2026-05-06

## Context

This receipt records the current crates.io publish-readiness boundary for `hl7v2-rs` after merge commit `6de37e0`.

Commands run locally on Windows:

```powershell
cargo run -p xtask -- publish-plan
$env:PYO3_USE_ABI3_FORWARD_COMPATIBILITY='1'
cargo publish --dry-run -p <crate> --locked
```

`PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1` was set before the dry-run loop because the local machine uses a newer Python toolchain than the PyO3 version in this workspace expects by default.

## Publish Order

`cargo run -p xtask -- publish-plan` returned 30 publishable crates:

| Order | Crate | Direct dry-run result |
| --- | --- | --- |
| 1 | `hl7v2-datetime` | Pass |
| 2 | `hl7v2-datatype` | Pass |
| 3 | `hl7v2-faker` | Pass |
| 4 | `hl7v2-model` | Pass |
| 5 | `hl7v2-escape` | Pass |
| 6 | `hl7v2-json` | Pass |
| 7 | `hl7v2-mllp` | Pass |
| 8 | `hl7v2-path` | Pass |
| 9 | `hl7v2-query` | Pass |
| 10 | `hl7v2-parser` | Pass |
| 11 | `hl7v2-batch` | Pass |
| 12 | `hl7v2-stream` | Pass |
| 13 | `hl7v2-writer` | Pass |
| 14 | `hl7v2-network` | Pass |
| 15 | `hl7v2-normalize` | Pass |
| 16 | `hl7v2-core` | Pass |
| 17 | `hl7v2` | Blocked by registry dependency state |
| 18 | `hl7v2-ack` | Not run after stop |
| 19 | `hl7v2-corpus` | Not run after stop |
| 20 | `hl7v2-guard` | Not run after stop |
| 21 | `hl7v2-lifecycle` | Not run after stop |
| 22 | `hl7v2-python` | Not run after stop |
| 23 | `hl7v2-redact` | Not run after stop |
| 24 | `hl7v2-template-values` | Not run after stop |
| 25 | `hl7v2-template` | Not run after stop |
| 26 | `hl7v2-gen` | Not run after stop |
| 27 | `hl7v2-validation` | Not run after stop |
| 28 | `hl7v2-prof` | Not run after stop |
| 29 | `hl7v2-server` | Not run after stop |
| 30 | `hl7v2-cli` | Not run after stop |

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

The stop at `hl7v2` is expected for a direct dry-run before publishing the dependency chain: during packaging, Cargo resolves registry dependencies from crates.io, and `hl7v2-core` is not available there yet. This means higher-level crates are not proven publish-ready by direct dry-run from the current registry state.

Do not claim full crates.io publish readiness until one of these is true:

1. The real publish sequence has published dependencies through `hl7v2-core`, then direct dry-runs or publishes continue from `hl7v2`.
2. A local-registry simulation proves the higher-level crates against packaged dependencies in publish order.

## Current Label

Current status is **partial publish readiness**:

- green main workflows: proven
- runtime and contract behavior: proven for the named HTTP/gRPC/schema surfaces
- direct dry-run publish: proven through `hl7v2-core`
- full workspace publish readiness: not yet proven
