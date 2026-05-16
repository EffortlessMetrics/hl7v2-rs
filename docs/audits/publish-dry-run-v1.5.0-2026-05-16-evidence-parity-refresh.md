# v1.5.0 Evidence Parity Current-Main Readiness Refresh

Date: 2026-05-16

This receipt records a post-release current-main readiness refresh after the
ACK evidence parity train landed through #702. It verifies that the
already-published v1.5.0 package surfaces still classify, package, and validate
after the latest REST, gRPC, and Python ACK parity receipts.

This is not a new publish receipt. v1.5.0 was already published before this
refresh, so the dry-runs correctly observed that the `1.5.0` crates already
exist on crates.io and then verified package contents without uploading.

## Context

| Field | Value |
| --- | --- |
| Branch | `release/v1.5-readiness-current-main` |
| Base commit | `65c3c30ec52c9c5d65b58dae245b62f0bee9d198` |
| Current release | `v1.5.0` |
| Local target dir | `F:\cargo-target\hl7v2-rs-v15-readiness-current-main` |
| Python ABI guard | `PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1` |
| Rust toolchain | `1.95.0` |

The refresh covers the post-release evidence parity train after the
normalization and CLI ACK proof:

- #699: post-ACK-parity v1.5.0 readiness refresh;
- #700: REST `/hl7/ack` parity for `AA`, `AE`, `AR`, `CA`, `CE`, and `CR`,
  MSH-9 `ACK^ADT`, MSA code/control ID, and MLLP framing;
- #701: gRPC `GenerateAck` contract coverage for `ACK^ADT` payloads and parsed
  ACK `MSH`/`MSA` shape on the existing proto-supported ACK codes;
- #702: Python smoke proof for enhanced ACK codes `AE`, `AR`, `CA`, `CE`, and
  `CR` while preserving default `AA` and invalid `ZZ` checks.

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
| `cargo +1.95.0 run -p xtask -- check-python-publish-policy` | Pass; public Python distribution is `hl7v2`, while `hl7v2-python` is a publishable binding backend crate with separate release receipts required. |
| `cargo +1.95.0 run -p xtask -- badges --check` | Pass |
| `cargo +1.95.0 run -p xtask -- impacted-evidence` | Pass; regenerated local ignored impacted-evidence receipt for this checkout. |
| `cargo +1.95.0 run -p xtask -- impacted-evidence --check` | Pass |
| `cargo +1.95.0 run -p xtask -- check-doc-links` | Pass; 187 Markdown files and 485 local links checked. |
| `cargo +1.95.0 run -p xtask -- evidence-schema-check` | Pass; 33 evidence fixtures validated. |
| `cargo +1.95.0 run -p xtask -- check-file-policy` | Pass; 596 tracked/untracked non-ignored files checked, with 39 allowlist entries and 30 companion ledger entries. |
| `git diff --check` | Pass |

## Registry And Release State Check

These checks verify current public state. They do not upload, tag, or release
anything.

| Check | Result |
| --- | --- |
| `cargo +1.95.0 info hl7v2@1.5.0` | Pass; resolved `hl7v2@1.5.0` with `rust-version: 1.95`. |
| `cargo +1.95.0 info hl7v2-python@1.5.0` | Pass; resolved the PyO3 binding backend crate for the Python `hl7v2` package. |
| `cargo +1.95.0 info hl7v2-server@1.5.0` | Pass; resolved `hl7v2-server@1.5.0`. |
| `cargo +1.95.0 info hl7v2-cli@1.5.0` | Pass; resolved `hl7v2-cli@1.5.0`. |
| crates.io API lookup | Pass; all four `1.5.0` crates are visible and not yanked. |
| `git tag --list v1.5.0` | Pass; local tag exists. |
| `gh release view v1.5.0 --json tagName,targetCommitish,name,publishedAt,isDraft,isPrerelease,url` | Pass; GitHub release `v1.5.0 - Rust 1.95 Quality Ratchet` is published, not draft, and not prerelease. |

Observed crates.io registry values:

| Crate | Created at | Yanked | Checksum |
| --- | --- | --- | --- |
| `hl7v2@1.5.0` | `2026-05-15T19:07:46.478132Z` | `false` | `5e28290fe93b3bc611f8cdffd8c1ae542c4b4a38e53399a9ff45a7e3af6fe9d1` |
| `hl7v2-python@1.5.0` | `2026-05-15T19:08:48.057374Z` | `false` | `c58d2449dea1384f6191439b3ebd1a12dd0ecbb99bbf914e305fa6415780dcd5` |
| `hl7v2-server@1.5.0` | `2026-05-15T19:10:03.552399Z` | `false` | `c991ef1adca717d05c1db1919dac0624785322d7f539ef648e9273fbc4f70fdb` |
| `hl7v2-cli@1.5.0` | `2026-05-15T19:11:00.152962Z` | `false` | `6769527259b2a062f72a4e4c0ad087a7668eddba92361b0b8e5c1e910ce534bb` |

## Boundaries

- This receipt did not upload any crate to crates.io.
- This receipt did not create or move a tag.
- This receipt did not create or update a GitHub release.
- This receipt did not publish to TestPyPI or PyPI.
- This receipt did not install back from TestPyPI or PyPI.
- This receipt did not create or publish an npm package.
- `hl7v2-python` remains binding backend infrastructure, not the recommended
  Rust API.
- The gRPC ACK proof remains scoped to the existing proto-supported ACK codes;
  adding commit-accept/error/reject codes to gRPC requires a proto/API/runtime
  change and a separate compatibility decision.
- `ripr` remains advisory static mutation-exposure proof and is not a
  branch-protection gate.

## Follow-Up

The current Rust release remains v1.5.0. Future crates.io uploads from this
post-evidence-parity `main` line require a new version and a fresh release
decision. The public Python `hl7v2` lane remains separate and still needs
TestPyPI Trusted Publisher setup, upload, install-back, import, and smoke
receipts before any TestPyPI or PyPI success claim.
