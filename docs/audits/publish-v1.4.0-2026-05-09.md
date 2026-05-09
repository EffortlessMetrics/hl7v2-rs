# v1.4.0 Publish Receipt

Date: 2026-05-09

This receipt records the actual crates.io publication for `hl7v2-rs` v1.4.0,
the Evidence Contracts and Server Sidecar release.

The published Rust package graph is:

1. `hl7v2`
2. `hl7v2-server`
3. `hl7v2-cli`

`hl7v2-python` remains `publish = false` for crates.io and stays on the
separate Python/maturin binding lane.

## Release Head

```text
34941f4 release: prepare v1.4.0 dry-run (#542)
```

The `v1.4.0` tag points at this release head and was pushed to GitHub. The
GitHub release is:

```text
https://github.com/EffortlessMetrics/hl7v2-rs/releases/tag/v1.4.0
```

## Pre-Publish Verification

Release-head verification was recorded in:

```text
docs/audits/publish-dry-run-v1.4.0-2026-05-09.md
```

Hosted checks on the release PR were green before upload:

```text
CI
API Contracts
Python Wheels
Security
CodeRabbit
GitGuardian
```

Final direct dry-runs were run in dependency order:

```powershell
cargo +1.93.0 publish -p hl7v2 --dry-run --locked
cargo +1.93.0 publish -p hl7v2-server --dry-run --locked
cargo +1.93.0 publish -p hl7v2-cli --dry-run --locked
```

The dependent direct dry-runs were rerun after each dependency became visible
in the crates.io index.

## Publish Commands

```powershell
cargo +1.93.0 publish -p hl7v2
cargo +1.93.0 publish -p hl7v2-server
cargo +1.93.0 publish -p hl7v2-cli
```

## Results

Cargo reported successful upload and index availability for:

| Crate | Version | Result |
| --- | --- | --- |
| `hl7v2` | `1.4.0` | Published |
| `hl7v2-server` | `1.4.0` | Published |
| `hl7v2-cli` | `1.4.0` | Published |

Registry verification:

```text
hl7v2 = "1.4.0"
hl7v2-server = "1.4.0"
hl7v2-cli = "1.4.0"
```

`cargo +1.93.0 info` resolved each package from the public crates.io index at
version `1.4.0`.

## Status

The v1.4.0 final Rust package graph is published to crates.io. Historical old
implementation package names remain compatibility artifacts and are not the
product surface for new code. `hl7v2-python` remains outside the Rust crates.io
publish graph.
