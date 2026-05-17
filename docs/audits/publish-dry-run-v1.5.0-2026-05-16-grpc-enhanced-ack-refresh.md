# v1.5.0 gRPC Enhanced ACK Current-Main Readiness Refresh

Date: 2026-05-16

This receipt records a post-release current-main readiness refresh after #705
landed gRPC `GenerateAck` parity for all six supported ACK codes: `AA`, `AE`,
`AR`, `CA`, `CE`, and `CR`.

This is not a new publish receipt. v1.5.0 was already published before this
refresh, so the dry-runs correctly observed that the `1.5.0` crates already
exist on crates.io and then verified package contents without uploading.

## Context

| Field | Value |
| --- | --- |
| Branch | `release/refresh-v1.5.0-readiness-after-grpc-ack` |
| Base commit | `a8ac4cf5b9ab9e33a1a0aa61287901e00b13ab04` |
| Current release | `v1.5.0` |
| Local target dir | `F:\cargo-target\hl7v2-rs-readiness-2026-05-16` |
| Python ABI guard | `PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1` for the binding dry-run |
| Rust toolchain | `1.95.0` |

The refresh covers the post-release gRPC ACK train after the evidence parity
refresh:

- #705: gRPC `GenerateAck` accepts and maps `AA`, `AE`, `AR`, `CA`, `CE`, and
  `CR` through the canonical protobuf/API/runtime surface.

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
| `cargo +1.95.0 run -p xtask -- badges --check` | Pass |
| `cargo +1.95.0 run -p xtask -- impacted-evidence` | Pass; regenerated the local ignored impacted-evidence receipt for this checkout. |
| `cargo +1.95.0 run -p xtask -- impacted-evidence --check` | Pass |
| `cargo +1.95.0 run -p xtask -- check-doc-links` | Pass; 191 Markdown files and 502 local links checked. |
| `cargo +1.95.0 run -p xtask -- evidence-schema-check` | Pass; 33 evidence fixtures validated. |
| `cargo +1.95.0 run -p xtask -- check-file-policy` | Pass; 601 tracked/untracked non-ignored files checked, with 40 allowlist entries and 32 companion ledger entries. |
| `git diff --check` | Pass |

## Registry And Release State Check

These checks verify current public state. They do not upload, tag, or release
anything.

| Check | Result |
| --- | --- |
| `cargo +1.95.0 info hl7v2@1.5.0` from a temporary directory outside the workspace | Pass; resolved `hl7v2@1.5.0` on crates.io with `rust-version: 1.95`. |
| `cargo +1.95.0 info hl7v2-python@1.5.0` from a temporary directory outside the workspace | Pass; resolved the PyO3 binding backend crate for the Python `hl7v2` package on crates.io. |
| `cargo +1.95.0 info hl7v2-server@1.5.0` from a temporary directory outside the workspace | Pass; resolved `hl7v2-server@1.5.0` on crates.io. |
| `cargo +1.95.0 info hl7v2-cli@1.5.0` from a temporary directory outside the workspace | Pass; resolved `hl7v2-cli@1.5.0` on crates.io. |
| `git tag -l v1.5.0` | Pass; local tag exists. |
| `git rev-list -n 1 v1.5.0` | Pass; tag resolves to `04760587b83e2b4aaf410814b46ad1818c881371`. |
| `gh release view v1.5.0 --json tagName,name,publishedAt,isDraft,isPrerelease,url` | Pass; GitHub release `v1.5.0 - Rust 1.95 Quality Ratchet` is published, not draft, and not prerelease. |

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
- `hl7v2-python` remains binding backend infrastructure, not the recommended
  Rust API.
- `ripr` remains advisory static mutation-exposure proof and is not a
  branch-protection gate.

## Follow-Up

The current Rust release remains v1.5.0. Future crates.io uploads from this
post-gRPC-ACK `main` line require a new version and a fresh release decision.
The public Python `hl7v2` lane remains separate and still needs TestPyPI
Trusted Publisher setup, upload, install-back, import, and smoke receipts before
any TestPyPI or PyPI success claim.
