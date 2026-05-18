# Safe Support Bundle Guide Smoke Receipt

Date: 2026-05-18
Branch: `test/safe-support-bundle-guide-smoke`
Scope: executable source-checkout proof for the operator-facing safe support
bundle guide.

## Purpose

This receipt records the guide-level command that backs
`docs/guides/safe-support-bundle.md` with an executable copy/paste smoke:

```text
cargo +1.95.0 run -p xtask -- check-safe-support-bundle-guide
```

The command proves the local, non-registry support handoff path:

- writes the guide's fail-closed safe-analysis policy to
  `target/hl7v2-safe-support-bundle/safe-analysis.toml`;
- runs redaction preview for `test_data/invalid_message.hl7`;
- creates the operator-facing `support-bundle` packet;
- verifies validation, redaction, bundle summary, manifest, environment,
  README, replay scripts, and replay report artifacts;
- verifies the expected invalid `PID.8` profile failure is reproduced;
- verifies generated reports and shareable bundle artifacts do not contain the
  guide PHI sentinels from the raw fixture.

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
cargo +1.95.0 test -p xtask check_safe_support_bundle --locked
cargo +1.95.0 test -p xtask --locked
cargo +1.95.0 run -p xtask -- check-safe-support-bundle-guide
cargo +1.95.0 run -p xtask -- check-first-use-guides
cargo +1.95.0 run -p xtask -- check-doc-links
cargo +1.95.0 run -p xtask -- check-file-policy
cargo +1.95.0 run -p xtask -- badges --check
cargo +1.95.0 run -p xtask -- impacted-evidence
cargo +1.95.0 run -p xtask -- impacted-evidence --check
git diff --check
```
