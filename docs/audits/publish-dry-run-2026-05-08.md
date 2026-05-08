# Publish Dry-Run Receipt - 2026-05-08

This dry-run receipt was followed by the actual `v1.2.1` crates.io publish.
See `docs/audits/publish-2026-05-08.md` for the upload receipt.

## Context

This receipt records the release-line correction from `1.2.0` to `1.2.1`.
The existing `v1.2.0` tag points at historical commit `1782d9a`, so the
current release line moves forward instead of reusing or moving that tag.

The Rust crates.io publish graph remains:

1. `hl7v2`
2. `hl7v2-server`
3. `hl7v2-cli`

`hl7v2-python` remains `publish = false` and belongs to the separate
Python/maturin binding lane.

## Commands

Run locally on Windows:

```powershell
cargo +1.93.0 metadata --format-version 1 --no-deps
git diff --check
cargo +1.93.0 fmt --all -- --check
cargo +1.93.0 run -p xtask -- publish-plan
cargo +1.93.0 run -p xtask -- publish-dry-run --from hl7v2 --workspace-patches
cargo +1.93.0 publish -p hl7v2 --dry-run
cargo +1.93.0 publish -p hl7v2-server --dry-run
cargo +1.93.0 publish -p hl7v2-cli --dry-run
npx @stoplight/spectral-cli lint api/openapi/hl7v2-api-v1.yaml --ruleset .spectral.yml
npx -y @bufbuild/buf lint api/proto
```

These commands were rerun on a clean version-alignment branch after committing
the release-prep changes. Run the same dependency-ordered dry-runs again on
clean `main` immediately before upload.

## Results

`cargo +1.93.0 run -p xtask -- publish-plan` returned:

```text
1. hl7v2
2. hl7v2-server
3. hl7v2-cli
```

The workspace-patched dry run passed for all three Rust crates:

```text
Dry-running hl7v2...
Dry-running hl7v2-server...
Dry-running hl7v2-cli...
Publish dry-run checks passed!
```

Package verification completed for:

| Crate | Version | Package size | Result |
| --- | --- | ---: | --- |
| `hl7v2` | `1.2.1` | 732.1 KiB | Pass |
| `hl7v2-server` | `1.2.1` | 287.3 KiB | Pass |
| `hl7v2-cli` | `1.2.1` | 276.3 KiB | Pass |

`cargo +1.93.0 publish -p hl7v2 --dry-run` passed directly against the
crates.io index:

```text
Packaging hl7v2 v1.2.1
Packaged 56 files, 732.1KiB (140.0KiB compressed)
Verifying hl7v2 v1.2.1
Uploading hl7v2 v1.2.1
warning: aborting upload due to dry run
```

The dependent crates reached packaging, then stopped at dependency resolution
because `hl7v2 v1.2.1` is not published yet:

```text
no matching package named `hl7v2` found
location searched: crates.io index
required by package `hl7v2-server v1.2.1`
```

`hl7v2-cli` reported the same registry-resolution blocker. This is the
expected dependency-ordered pre-publish result, not a package-content failure.

OpenAPI lint passed with the existing non-blocking contact warning:

```text
1 problem (0 errors, 1 warning, 0 infos, 0 hints)
```

Buf proto lint passed through `npx -y @bufbuild/buf lint api/proto`.

## Status

This receipt established **package verification** for the `1.2.1` Rust publish
graph. The follow-up publish receipt records the actual upload.
