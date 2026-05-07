# Publish Dry-Run Receipt - 2026-05-07

## Context

This receipt records package verification after the foundation implementation
carriers were collapsed into `hl7v2` at merge commit `3482fd4`.

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

## Interpretation

Current status is **package-verified**, not published.

The workspace-patched dry run proves the package contents and verification build
against the current unpublished workspace dependency chain. It is not a claim
that crates are available in the public crates.io index, and it does not replace
the final dependency-ordered `cargo publish --dry-run` immediately before a real
publish.

The real publish sequence has not been executed.
