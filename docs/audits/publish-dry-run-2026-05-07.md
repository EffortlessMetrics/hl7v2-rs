# Publish Dry-Run Receipt - 2026-05-07

## Context

This receipt records package verification after the foundation implementation
carriers were collapsed into `hl7v2` at merge commit `3482fd4`, then refreshed
after the release-readiness docs sync at `aafa0b0`.

The old foundation package names are now private deprecated compatibility shims:

- `hl7v2-model`
- `hl7v2-escape`
- `hl7v2-mllp`

They are not part of the crates.io publish plan.

## Commands

Run locally on Windows:

```powershell
cargo +1.93.0 run -p xtask -- publish-plan
$env:PYO3_USE_ABI3_FORWARD_COMPATIBILITY='1'
cargo +1.93.0 run -p xtask -- publish-dry-run --from hl7v2 --workspace-patches --allow-dirty
cargo +1.93.0 publish -p hl7v2 --dry-run
$env:PYO3_USE_ABI3_FORWARD_COMPATIBILITY='1'; cargo +1.93.0 publish -p hl7v2-python --dry-run
cargo +1.93.0 publish -p hl7v2-server --dry-run
cargo +1.93.0 publish -p hl7v2-cli --dry-run
```

`PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1` is required on this machine because the
local Python toolchain is newer than the PyO3 version in this workspace expects
by default.

## Publish Order

`cargo +1.93.0 run -p xtask -- publish-plan` returned the final public package
graph:

| Order | Crate |
| --- | --- |
| 1 | `hl7v2` |
| 2 | `hl7v2-python` |
| 3 | `hl7v2-server` |
| 4 | `hl7v2-cli` |

## Dry-Run Result

The workspace-patched dry run passed for all four packages:

```text
Dry-running hl7v2...
Dry-running hl7v2-python...
Dry-running hl7v2-server...
Dry-running hl7v2-cli...
Publish dry-run checks passed!
```

Package verification completed for:

| Crate | Package size | Result |
| --- | ---: | --- |
| `hl7v2` | 732.2 KiB | Pass |
| `hl7v2-python` | 69.1 KiB | Pass |
| `hl7v2-server` | 287.4 KiB | Pass |
| `hl7v2-cli` | 276.3 KiB | Pass |

## Direct crates.io Dry-Run Check

`cargo +1.93.0 publish -p hl7v2 --dry-run` passed directly against the crates.io
index:

```text
Packaging hl7v2 v1.2.0
Packaged 56 files, 732.2KiB (140.0KiB compressed)
Verifying hl7v2 v1.2.0
Finished `dev` profile [unoptimized + debuginfo] target(s)
Uploading hl7v2 v1.2.0
warning: aborting upload due to dry run
```

The dependent crates reached packaging, then stopped at dependency resolution
because `hl7v2 v1.2.0` is not published yet:

```text
error: failed to prepare local package for uploading

Caused by:
  no matching package named `hl7v2` found
  location searched: crates.io index
  required by package `hl7v2-python v1.2.0`
```

`hl7v2-server` and `hl7v2-cli` reported the same registry-resolution blocker.
This is the expected pre-publish result for dependency-ordered crates.io
packages, not a package-content failure. Their direct `cargo publish --dry-run`
checks must be rerun after `hl7v2 v1.2.0` is published and visible in the
crates.io index.

## Interpretation

Current status is **package-verified**, not published.

The workspace-patched dry run proves the package contents and verification build
against the current unpublished workspace dependency chain. It is not a claim
that crates are available in the public crates.io index, and it does not replace
the dependency-ordered direct `cargo publish --dry-run` checks during a real
publish. The first direct dry-run now passes for `hl7v2`; direct dry-runs for
dependent crates are gated on `hl7v2` being published first.

The real publish sequence has not been executed.
