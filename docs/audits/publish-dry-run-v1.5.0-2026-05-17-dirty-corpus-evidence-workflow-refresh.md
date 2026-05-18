# v1.5.0 Dirty-Corpus Evidence Workflow Readiness Refresh

Date: 2026-05-17

This receipt records a post-release current-main readiness refresh at commit
`adda4353aae604972670db82415bd5dac1a6373a` after the dirty real-world evidence
workflow parity updates landed through #734.

This is not a new publish receipt. v1.5.0 was already published before this
refresh, so the dry-runs correctly observed that the `1.5.0` crates already
exist on crates.io and then verified package contents without uploading.

## Context

| Field | Value |
| --- | --- |
| Branch | `release/v1.5.x-readiness-dirty-evidence` |
| Base commit | `adda4353aae604972670db82415bd5dac1a6373a` |
| Current release | `v1.5.0` |
| Local target dir | `F:\cargo-target\hl7v2-rs-readiness-dirty-evidence-2026-05-17` |
| Python ABI guard | `PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1` |
| Cargo incremental | `0` |
| Rust toolchain | `1.95.0` |

The refresh covers the post-release dirty-corpus evidence workflow updates:

- #733 added a REST dirty real-world `validate-redacted -> bundle -> replay`
  workflow over the shared Z-segment fixture.
- #734 added a gRPC dirty real-world
  `ValidateRedacted -> CreateEvidenceBundle -> ReplayEvidenceBundle` workflow
  over the same fixture.
- Both workflows are routed through
  `cargo +1.95.0 run -p xtask -- check-dirty-corpus-parity`.

## Publish Surface Results

| Command | Result | Notes |
| --- | --- | --- |
| `cargo +1.95.0 run -p xtask -- publish-plan --surface primary` | Pass | Primary Rust product graph remains `hl7v2`, `hl7v2-server`, `hl7v2-cli`. |
| `cargo +1.95.0 run -p xtask -- publish-plan --surface bindings` | Pass | Binding backend graph remains `hl7v2-python`. |
| `cargo +1.95.0 run -p xtask -- publish-plan --surface all-publishable` | Pass | All-publishable graph remains `hl7v2`, `hl7v2-python`, `hl7v2-server`, `hl7v2-cli`. |
| `cargo +1.95.0 run -p xtask -- publish-dry-run --surface primary --workspace-patches --allow-dirty` | Pass | `hl7v2`, `hl7v2-server`, and `hl7v2-cli` packaged and verified. Cargo warned that each `1.5.0` crate already exists, which is expected after the v1.5.0 release. |
| `PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo +1.95.0 run -p xtask -- publish-dry-run --surface bindings --workspace-patches --allow-dirty` | Pass | `hl7v2-python` package listed and dry-run verified as the binding backend. Cargo warned that `hl7v2-python@1.5.0` already exists, which is expected after the v1.5.0 release. |

The binding backend package list remained narrow:

```text
.cargo_vcs_info.json
Cargo.lock
Cargo.toml
Cargo.toml.orig
README.md
src/lib.rs
```

## Policy And Evidence Results

| Command | Result |
| --- | --- |
| `cargo +1.95.0 run -p xtask -- check-python-publish-policy` | Pass; public Python distribution is `hl7v2`, while `hl7v2-python` is a publishable binding backend crate with separate release receipts required. |
| `cargo +1.95.0 run -p xtask -- badges --check` | Pass; badge endpoints are current. |
| `cargo +1.95.0 run -p xtask -- impacted-evidence` | Pass; regenerated the local ignored impacted-evidence receipt for this checkout. |
| `cargo +1.95.0 run -p xtask -- impacted-evidence --check` | Pass; impacted evidence is current. |
| `cargo +1.95.0 run -p xtask -- check-doc-links` | Pass; 196 Markdown files and 551 local links checked. |
| `cargo +1.95.0 run -p xtask -- evidence-schema-check` | Pass; 33 evidence fixtures validated. |
| `cargo +1.95.0 run -p xtask -- check-file-policy` | Pass; 611 tracked/untracked non-ignored files checked, with 40 allowlist entries and 32 companion ledger entries. |
| `cargo +1.95.0 run -p xtask -- check-dirty-corpus-parity` | Pass; Rust, CLI, REST, and gRPC dirty corpus proof passed. Python local-wheel smoke was skipped because no local wheel was installed for this refresh. |
| `cargo +1.95.0 run -p xtask -- check-evidence-parity` | Pass; 6 surfaces, 11 contracts, and registry non-claim boundaries checked. |
| `cargo +1.95.0 run -p xtask -- check-evidence-parity-acceptance` | Pass; safe-error/PHI, schema-version, dirty-corpus, and bundle/replay parity acceptance passed for Rust, CLI, REST, and gRPC. Python local-wheel smoke was skipped because no local wheel was installed for this refresh. |

## Registry And Release State Check

These checks verify current public state. They do not upload, tag, or release
anything.

| Check | Result |
| --- | --- |
| `cargo +1.95.0 info --registry crates-io hl7v2@1.5.0` | Pass; resolved `hl7v2@1.5.0` on crates.io with `rust-version: 1.95`. |
| `cargo +1.95.0 info --registry crates-io hl7v2-python@1.5.0` | Pass; resolved the PyO3 binding backend crate for the Python `hl7v2` package on crates.io. |
| `cargo +1.95.0 info --registry crates-io hl7v2-server@1.5.0` | Pass; resolved `hl7v2-server@1.5.0` on crates.io. |
| `cargo +1.95.0 info --registry crates-io hl7v2-cli@1.5.0` | Pass; resolved `hl7v2-cli@1.5.0` on crates.io. |
| `git tag -l v1.5.0` | Pass; local tag exists. |
| `git rev-list -n 1 v1.5.0` | Pass; local tag resolves to release commit `04760587b83e2b4aaf410814b46ad1818c881371`. |
| `git ls-remote --tags origin v1.5.0` | Pass; remote tag object exists as `b0b8ace2be687ad326a68590be9b61d74470c063`. |
| `gh release view v1.5.0 --json tagName,name,publishedAt,isDraft,isPrerelease,url,targetCommitish` | Pass; GitHub release `v1.5.0 - Rust 1.95 Quality Ratchet` is published, not draft, and not prerelease. |
| `curl -I https://test.pypi.org/pypi/hl7v2/json` | Pass as boundary check; returned `404 Not Found`, so no TestPyPI package success is visible. |
| `curl -I https://pypi.org/pypi/hl7v2/json` | Pass as boundary check; returned `404 Not Found`, so no production PyPI package success is visible. |
| `curl -I https://registry.npmjs.org/@effortlessmetrics%2Fhl7v2` | Pass as boundary check; returned `404 Not Found`, so no npm package success is visible. |
| `gh issue view 563 --json number,title,state,url,labels` | Pass; issue #563 remains open for TestPyPI Trusted Publisher setup. |

Observed crates.io registry URLs:

| Crate | URL |
| --- | --- |
| `hl7v2@1.5.0` | <https://crates.io/crates/hl7v2/1.5.0> |
| `hl7v2-python@1.5.0` | <https://crates.io/crates/hl7v2-python/1.5.0> |
| `hl7v2-server@1.5.0` | <https://crates.io/crates/hl7v2-server/1.5.0> |
| `hl7v2-cli@1.5.0` | <https://crates.io/crates/hl7v2-cli/1.5.0> |

## Boundaries

- This receipt did not upload any crate to crates.io.
- This receipt did not create or move a tag.
- This receipt did not create or update a GitHub release.
- This receipt did not publish to TestPyPI or PyPI.
- This receipt did not install back from TestPyPI or PyPI.
- This receipt did not create or publish an npm package.
- This receipt does not claim a Python dirty validate/redact/bundle/replay
  workflow; the new workflow proof is REST and gRPC only.
- `hl7v2-python` remains binding backend infrastructure, not the recommended
  Rust API.
- `ripr` remains advisory static mutation-exposure proof and is not a
  branch-protection gate.

## Follow-Up

The current Rust release remains v1.5.0. Future crates.io uploads from this
post-release `main` line require a new version and a fresh release decision.
The public Python `hl7v2` lane remains separate and still needs TestPyPI
Trusted Publisher setup, upload, install-back, import, and smoke receipts
before any TestPyPI or PyPI success claim.
