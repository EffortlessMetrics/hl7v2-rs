# Post-1.2.1 Crate Retirement Audit

> Completion note: PR #447 deleted the retired local shim crate directories
> after workspace membership cleanup and facade coverage migration. This
> document is retained as the dated retirement plan and evidence trail.

## Context

`hl7v2-rs` v1.2.1 has been published to crates.io for the final Rust package
graph:

1. `hl7v2`
2. `hl7v2-server`
3. `hl7v2-cli`

`hl7v2-python` remains `publish = false` and belongs to the separate
Python/maturin packaging lane.

The implementation microcrate folders were useful during demicrocrating because
they preserved old import paths, kept compatibility tests available while files
moved, and gave publish tooling a stable transition graph. They are now
post-release scaffolding. Keeping them as active workspace members increases CI
surface, lint-policy staging, and contributor confusion.

This audit records the current local crate folders and classifies the intended
retirement action before any workspace membership or directory deletion PR.

## Current Inventory

| Folder or package | Category | Action |
| --- | --- | --- |
| `crates/hl7v2` | Public product | Keep. Canonical Rust library crate and implementation home. |
| `crates/hl7v2-server` | Public product | Keep. HTTP/gRPC service crate. |
| `crates/hl7v2-cli` | Public product | Keep. Command-line binary crate. |
| `crates/hl7v2-python` | Python lane | Keep with `publish = false`; validate and release with Python packaging tooling. |
| `crates/hl7v2-e2e-tests` | Internal test/tool | Keep for now; later candidate for workspace-level tests if that reduces complexity. |
| `crates/hl7v2-test-utils` | Internal test/tool | Keep for now; later candidate for `tests/support`. |
| `crates/hl7v2-bench` | Internal test/tool | Keep for now with `publish = false`; later candidate for root `benches/`. |
| root `hl7v2-examples` package | Internal test/tool | Keep. The root package owns examples and remains `publish = false`. |
| `xtask` | Internal test/tool | Keep. Workspace automation. |
| `crates/hl7v2-ack` | Deprecated shim | Retire. Implementation lives in `hl7v2::ack`. |
| `crates/hl7v2-batch` | Deprecated shim | Retire. Implementation lives in `hl7v2::batch`. |
| `crates/hl7v2-core` | Deprecated shim | Retire. `hl7v2` is the only canonical Rust facade. |
| `crates/hl7v2-corpus` | Deprecated shim | Retire. Implementation lives in `hl7v2::synthetic::corpus`. |
| `crates/hl7v2-datatype` | Deprecated shim | Retire. Implementation lives in `hl7v2::conformance::datatype`. |
| `crates/hl7v2-datetime` | Deprecated shim | Retire. Implementation lives in `hl7v2::conformance::datatype::datetime`. |
| `crates/hl7v2-escape` | Deprecated shim | Retire. Implementation lives in `hl7v2::escape`. |
| `crates/hl7v2-faker` | Deprecated shim | Retire. Implementation lives in `hl7v2::synthetic::faker`. |
| `crates/hl7v2-gen` | Deprecated shim | Retire. Implementation lives in `hl7v2::synthetic::generate`. |
| `crates/hl7v2-guard` | Deprecated shim | Retire. Implementation lives in `hl7v2::experimental::guard`. |
| `crates/hl7v2-json` | Deprecated shim | Retire. Implementation lives in `hl7v2::writer::json`. |
| `crates/hl7v2-lifecycle` | Deprecated shim | Retire. Implementation lives in `hl7v2::lifecycle`. |
| `crates/hl7v2-mllp` | Deprecated shim | Retire. Implementation lives in `hl7v2::transport::mllp`. |
| `crates/hl7v2-model` | Deprecated shim | Retire. Implementation lives in `hl7v2::model`. |
| `crates/hl7v2-network` | Deprecated shim | Retire. Implementation lives in `hl7v2::transport::network`. |
| `crates/hl7v2-normalize` | Deprecated shim | Retire. Implementation lives in `hl7v2::normalize`. |
| `crates/hl7v2-parser` | Deprecated shim | Retire. Implementation lives in `hl7v2::parser`. |
| `crates/hl7v2-path` | Deprecated shim | Retire. Implementation lives in `hl7v2::query::path`. |
| `crates/hl7v2-prof` | Deprecated shim | Retire. Implementation lives in `hl7v2::conformance::profile`. |
| `crates/hl7v2-query` | Deprecated shim | Retire. Implementation lives in `hl7v2::query`. |
| `crates/hl7v2-redact` | Deprecated shim | Retire. Implementation lives in `hl7v2::redact`. |
| `crates/hl7v2-stream` | Deprecated shim | Retire. Implementation lives in `hl7v2::stream`. |
| `crates/hl7v2-template` | Deprecated shim | Retire. Implementation lives in `hl7v2::synthetic::template`. |
| `crates/hl7v2-template-values` | Deprecated shim | Retire. Implementation lives in `hl7v2::synthetic::values`. |
| `crates/hl7v2-validation` | Deprecated shim | Retire. Implementation lives in `hl7v2::conformance::validation`. |
| `crates/hl7v2-writer` | Deprecated shim | Retire. Implementation lives in `hl7v2::writer`. |

## Historical Documentation

Keep historical architecture and planning records, but do not let them describe
the active repo as if the old microcrate topology were still the target.

| Document family | Category | Action |
| --- | --- | --- |
| `docs/MICROCRATE_ANALYSIS.md` | Historical docs | Keep as historical context. It already notes that the microcrate extraction plan is no longer the target architecture. |
| `docs/adr/0015-collapse-public-crate-surface.md` | Historical and current policy | Keep. It remains the architecture decision behind the post-1.2.1 cleanup. |
| `docs/architecture/module-map.md` | Current-to-historical bridge | Keep for now, then update after shim folders leave the workspace. |
| `docs/TESTING_ANALYSIS.md` and `docs/TESTING_ARCHITECTURE.md` | Historical docs with current notes | Keep, but verify that any current-facing references point users to `hl7v2` modules. |
| `docs/audits/publish-dry-run-*.md` and release receipts | Historical docs | Keep as dated evidence. Do not rewrite old receipts into current-state docs. |

## Retirement Sequence

Retire the shims in layers:

1. Remove deprecated shim crates from `[workspace].members` without deleting
   directories.
2. Migrate useful compatibility tests into canonical `hl7v2` tests that import
   from `hl7v2`, not old package names.
3. Delete retired shim crate directories after workspace membership and test
   migration prove no active path relies on them.
4. Update current-facing docs to remove local shim-folder language and mark
   old topology documents as historical where needed.

This order gives each PR a clear failure boundary:

- Workspace failures after step 1 indicate a dependency graph assumption.
- Test failures after step 2 indicate a behavior or coverage migration issue.
- Build or path failures after step 3 indicate a hidden local path assumption.
- Documentation failures after step 4 indicate stale public story, not code
  behavior.

## Non-Goals

- Do not publish old microcrate names as `1.2.1` deprecation releases unless a
  separate support decision requires it.
- Do not delete internal test/tool crates in the same lane as shim retirement.
- Do not mix Python wheel packaging, Factory Droid automation, or runtime API
  changes into shim retirement PRs.
- Do not rewrite historical audits or ADRs to pretend the old topology never
  existed.

## Expected Active Workspace After Membership Cleanup

The next non-doc PR should reduce active workspace participation to:

```toml
[workspace]
members = [
    "crates/hl7v2",
    "crates/hl7v2-server",
    "crates/hl7v2-cli",
    "crates/hl7v2-python",
    "crates/hl7v2-e2e-tests",
    "crates/hl7v2-test-utils",
    "crates/hl7v2-bench",
    "xtask",
]
```

The root `hl7v2-examples` package remains implicit because the workspace root
also has a `[package]` section and `publish = false`.

`cargo +1.93.0 run -p xtask -- publish-plan` must continue to resolve:

```text
1. hl7v2
2. hl7v2-server
3. hl7v2-cli
```
