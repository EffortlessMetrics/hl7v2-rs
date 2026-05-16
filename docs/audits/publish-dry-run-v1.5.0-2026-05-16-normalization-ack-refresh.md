# v1.5.0 Normalization And ACK Parity Current-Main Readiness Refresh

Date: 2026-05-16

This receipt records a post-release current-main readiness refresh after the
normalization and CLI ACK parity train landed through #698. It verifies that
the already-published v1.5.0 package surfaces still classify, package, and
validate after the latest cross-surface parity tests.

This is not a new publish receipt. v1.5.0 was already published before this
refresh, so the dry-runs correctly observed that the `1.5.0` crates already
exist on crates.io and then verified package contents without uploading.

## Context

| Field | Value |
| --- | --- |
| Branch | `docs/refresh-v1.5-readiness-after-ack-parity` |
| Base commit | `c888405000384f391b24864e3e572dd0e9b6ba6a` |
| Current release | `v1.5.0` |
| Local target dir | `F:\cargo-target\hl7v2-rs-v15-readiness-ack-parity` |
| Python ABI guard | `PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1` for binding dry-run |
| Cargo incremental | `0` |

The refresh covers the post-release parity train after the dirty-corpus proof:

- #696: post-release dirty-corpus readiness receipt on current `main`;
- #697: normalization parity across CLI and local Python helper surfaces;
- #698: CLI ACK parity for command success, ACK code/control ID preservation,
  MSH-9 ACK message type, and MLLP frame terminator behavior.

## Publish Surface Results

| Command | Result | Notes |
| --- | --- | --- |
| `cargo +1.95.0 run -p xtask -- publish-plan --surface primary` | Pass | Primary Rust product graph remains `hl7v2`, `hl7v2-server`, `hl7v2-cli`. |
| `cargo +1.95.0 run -p xtask -- publish-plan --surface bindings` | Pass | Binding backend graph remains `hl7v2-python`. |
| `cargo +1.95.0 run -p xtask -- publish-plan --surface all-publishable` | Pass | All-publishable graph remains `hl7v2`, `hl7v2-python`, `hl7v2-server`, `hl7v2-cli`. |
| `cargo +1.95.0 run -p xtask -- publish-dry-run --surface primary --workspace-patches --allow-dirty` | Pass | `hl7v2`, `hl7v2-server`, and `hl7v2-cli` packaged and verified. Cargo warned that each `1.5.0` crate already exists, which is expected after the v1.5.0 release. |
| `cargo +1.95.0 run -p xtask -- publish-dry-run --surface bindings --workspace-patches --allow-dirty` | Pass | `hl7v2-python` package listed and dry-run verified as the binding backend. Cargo warned that `hl7v2-python@1.5.0` already exists, which is expected after the v1.5.0 release. |

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
| `cargo +1.95.0 run -p xtask -- check-python-publish-policy` | Pass |
| `cargo +1.95.0 run -p xtask -- badges --check` | Pass |
| `cargo +1.95.0 run -p xtask -- impacted-evidence` | Pass; regenerated local ignored impacted-evidence receipt for this checkout. |
| `cargo +1.95.0 run -p xtask -- impacted-evidence --check` | Pass |
| `cargo +1.95.0 run -p xtask -- check-doc-links` | Pass; 187 Markdown files and 485 local links checked. |
| `cargo +1.95.0 run -p xtask -- evidence-schema-check` | Pass; 33 evidence fixtures validated. |
| `cargo +1.95.0 run -p xtask -- check-file-policy` | Pass; 596 tracked/untracked non-ignored files checked, with 39 allowlist entries and 30 companion ledger entries. |
| `git diff --check` | Pass |

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
post-ACK-parity `main` line require a new version and a fresh release decision.
The public Python `hl7v2` lane remains separate and still needs TestPyPI Trusted
Publisher setup, upload, install-back, import, and smoke receipts before any
TestPyPI or PyPI success claim.
