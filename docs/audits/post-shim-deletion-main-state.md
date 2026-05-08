# Post-Shim-Deletion Main State Audit

Date: 2026-05-08

## Purpose

This receipt verifies the repository state after the v1.2.1 release and the
post-release retirement of local implementation shim crate folders.

The expected current shape is:

- Rust product packages: `hl7v2`, `hl7v2-server`, `hl7v2-cli`
- Separate Python binding lane: `hl7v2-python`
- Internal development/test/tool packages: `hl7v2-bench`, `hl7v2-e2e-tests`,
  `hl7v2-test-utils`, `xtask`, and the root `hl7v2-examples` package
- Retired old implementation package names: absent from local repo topology

## Package Topology

`cargo metadata --format-version 1 --no-deps` reports the following workspace
packages:

| Package | Path | Publish status | Classification |
| --- | --- | --- | --- |
| `hl7v2` | `crates/hl7v2` | publishable | Rust product crate |
| `hl7v2-server` | `crates/hl7v2-server` | publishable | Rust product crate |
| `hl7v2-cli` | `crates/hl7v2-cli` | publishable | Rust product crate |
| `hl7v2-python` | `crates/hl7v2-python` | `publish = false` | Python/maturin binding lane |
| `hl7v2-bench` | `crates/hl7v2-bench` | `publish = false` | Internal benchmark harness |
| `hl7v2-e2e-tests` | `crates/hl7v2-e2e-tests` | `publish = false` | Internal end-to-end tests |
| `hl7v2-test-utils` | `crates/hl7v2-test-utils` | `publish = false` | Internal test utilities |
| `xtask` | `xtask` | `publish = false` | Internal automation |
| `hl7v2-examples` | repository root | `publish = false` | Examples package |

The `crates/` directory contains only:

```text
hl7v2
hl7v2-bench
hl7v2-cli
hl7v2-e2e-tests
hl7v2-python
hl7v2-server
hl7v2-test-utils
```

Old implementation microcrate folders such as `hl7v2-model`, `hl7v2-parser`,
`hl7v2-core`, `hl7v2-mllp`, `hl7v2-ack`, and related retired names are no
longer present as local crate directories or workspace members.

Current-facing references to old package names are limited to:

- architecture/history mapping that points old names to current `hl7v2` modules,
- retirement audit history,
- `xtask` tests that assert retired names are excluded from the publish graph,
- source comments or `#[expect]` reasons that identify moved implementation debt.

Those references do not define active local package surfaces.

## Verification

The local machine has Python 3.14, while the current PyO3 version supports up to
Python 3.13 without the forward-compatibility override. Workspace all-features
checks that include `hl7v2-python` were therefore run with
`PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1`.

| Command | Result | Evidence |
| --- | --- | --- |
| `cargo +1.93.0 metadata --format-version 1 --no-deps` | Pass | Metadata resolved the active workspace packages listed above. |
| `cargo +1.93.0 run -p xtask -- publish-plan` | Pass | Publish order is `hl7v2`, `hl7v2-server`, `hl7v2-cli`. |
| `cargo +1.93.0 run -p xtask -- check-file-policy` | Pass | 332 tracked files checked; 30 allowlist entries. |
| `cargo +1.93.0 run -p xtask -- check-lint-policy` | Pass | 4 baseline-inheriting packages and 4 staged packages verified. |
| `PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo +1.93.0 check --workspace --all-features --all-targets` | Pass | Full workspace all-target check completed. |
| `PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo +1.93.0 test --workspace --all-features` | Pass | Full workspace test suite completed. |
| `PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo +1.93.0 test --doc --workspace` | Pass | Doctests passed; `hl7v2-python` reports the expected `cdylib` doctest warning. |
| `PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo +1.93.0 check --examples` | Pass | Root examples compile through the canonical `hl7v2` facade. |

## Result

The local repository topology is structurally honest after shim deletion:

- the crates.io Rust product graph is still `hl7v2 -> hl7v2-server -> hl7v2-cli`,
- `hl7v2-python` remains outside the Rust publish graph,
- internal harnesses and test utilities are private,
- old implementation microcrate folders have been retired from the active repo,
- no retired implementation package is part of `workspace.members` or
  `xtask publish-plan`.

## Remaining Separate Lanes

This audit does not attempt to solve follow-up product or maintenance work.
Open follow-up lanes remain:

- API contract warning cleanup,
- GitHub Actions runtime modernization,
- Python/maturin package proof,
- moved-module lint debt reduction,
- product-usefulness CLI/reporting features.
