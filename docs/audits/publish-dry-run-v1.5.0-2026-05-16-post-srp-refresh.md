# v1.5.0 Post-SRP Current-Main Readiness Refresh

Date: 2026-05-16

This receipt records a post-release current-main readiness refresh after the
SRP refactor wave landed through #691. It verifies that the release and package
surfaces still classify, package, and validate from `main` after the parser,
redaction, corpus, evidence helper, profile, CLI, server, and xtask module
splits.

This is not a new publish receipt. v1.5.0 was already published before this
refresh, so the dry-runs correctly observed that the `1.5.0` crates already
exist on crates.io and then verified package contents without uploading.

## Context

| Field | Value |
| --- | --- |
| Branch | `main` |
| Commit | `4cf501ddc0f7fc3d027b3ce2459e899fe4aa7092` |
| Current release | `v1.5.0` |
| Local target dir | `F:\cargo-target\hl7v2-rs-post-srp-readiness` |
| Python ABI guard | `PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1` |
| Cargo incremental | `0` |

The refresh covers the post-release SRP refactor train:

- #684: evidence helpers split into focused modules;
- #685: redaction split into SRP modules;
- #686: parser split into SRP submodules;
- #687: CLI main split into focused modules;
- #688: profile module split into SRP submodules;
- #690: corpus utilities split into SRP submodules;
- #691: CLI corpus commands split into SRP submodules.

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
| `cargo +1.95.0 run -p xtask -- impacted-evidence --check` | Pass |
| `cargo +1.95.0 run -p xtask -- evidence-schema-check` | Pass; 33 evidence fixtures validated. |
| `cargo +1.95.0 run -p xtask -- check-doc-links` | Pass |

## Boundaries

- This receipt did not upload any crate to crates.io.
- This receipt did not create or move a tag.
- This receipt did not create or update a GitHub release.
- This receipt did not publish to TestPyPI or PyPI.
- This receipt did not install back from TestPyPI or PyPI.
- This receipt did not create or publish an npm package.
- `hl7v2-python` remains binding backend infrastructure, not the recommended
  Rust API.

## Follow-Up

The current Rust release remains v1.5.0. Future crates.io uploads from this
post-SRP `main` line require a new version and a fresh release decision. The
public Python `hl7v2` lane remains separate and still needs TestPyPI Trusted
Publisher setup, upload, install-back, import, and smoke receipts before any
TestPyPI or PyPI success claim.
