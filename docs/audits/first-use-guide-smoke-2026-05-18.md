# First-Use Guide Smoke Receipt

Date: 2026-05-18
Branch: `test/first-use-guide-smoke`
Scope: executable source-checkout proof for the documented first-use receipt
path.

## Purpose

This receipt records the new guide-level command that backs the user-facing
full evidence receipt path with executable checks:

```text
cargo +1.95.0 run -p xtask -- check-first-use-guides
```

The command proves the local, non-registry path:

- runs the literal CLI receipt recipe from
  `docs/guides/full-evidence-receipt-path.md` into
  `target/hl7v2-receipt`;
- verifies validation, redaction, support-bundle, replay, and PHI-sentinel
  safety artifacts;
- runs the Rust user-journey acceptance test;
- runs the CLI support-bundle user-journey acceptance test.

## Optional Proofs

The command has two opt-in surfaces:

```text
--include-python
--include-public-crates
```

`--include-python` is only for an environment where the local Python `hl7v2`
wheel is already installed. `--include-public-crates` refreshes crates.io
install-back proof for the released Rust crates.

## Non-Claims

- This receipt does not upload to TestPyPI or PyPI.
- This receipt does not prove `pip install hl7v2` from a public Python
  registry.
- This receipt does not publish or prove an npm package.
- This receipt does not create a new crates.io, tag, or GitHub release claim.
- This receipt does not promote `hl7v2-python` as the recommended Rust API.

## Validation

```text
cargo +1.95.0 fmt --all -- --check
cargo +1.95.0 clippy -p xtask --all-targets --locked -- -D warnings
cargo +1.95.0 test -p xtask check_first_use_guides --locked
cargo +1.95.0 test -p xtask --locked
cargo +1.95.0 run -p xtask -- check-first-use-guides
cargo +1.95.0 run -p xtask -- check-doc-links
cargo +1.95.0 run -p xtask -- check-file-policy
cargo +1.95.0 run -p xtask -- badges --check
cargo +1.95.0 run -p xtask -- impacted-evidence
cargo +1.95.0 run -p xtask -- impacted-evidence --check
git diff --check
```
