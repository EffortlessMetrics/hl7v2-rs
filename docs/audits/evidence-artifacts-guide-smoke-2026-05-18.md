# Evidence Artifacts Guide Smoke Receipt

Date: 2026-05-18
Branch: `test/evidence-artifacts-guide-smoke`
Scope: executable source-checkout proof for the operator-facing evidence
artifact interpretation guide.

## Purpose

This receipt records the guide-level command that backs
`docs/guides/evidence-artifacts-for-operators.md` with an executable smoke:

```text
cargo +1.95.0 run -p xtask -- check-evidence-artifacts-guide
```

The command proves the local, non-registry artifact reader path:

- generates a doctor report and verifies the version/checks reader fields;
- generates profile lint, explain, and fixture-test reports for
  `profiles/generic.yaml`;
- generates a validation report for `test_data/invalid_message.hl7` and checks
  the expected `PID.8` validation issue;
- generates corpus summary, fingerprint, and diff reports from the shared
  dirty real-world fixtures, including the current direct fixture counts
  for the before/after diff directories;
- generates redaction preview evidence and checks retained-field receipt data;
- creates a `support-bundle` packet and verifies bundle summary, manifest,
  environment, field-path, redaction receipt, replay scripts, README, and
  replay report artifacts;
- verifies generated shareable artifacts do not contain the guide PHI
  sentinels from the raw fixture.

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
cargo +1.95.0 test -p xtask check_evidence_artifacts_guide --locked
cargo +1.95.0 test -p xtask --locked
cargo +1.95.0 run -p xtask -- check-evidence-artifacts-guide
cargo +1.95.0 run -p xtask -- check-doc-links
cargo +1.95.0 run -p xtask -- check-file-policy
cargo +1.95.0 run -p xtask -- check-no-panic-family
cargo +1.95.0 run -p xtask -- check-lint-policy
cargo +1.95.0 run -p xtask -- check-evidence-parity
cargo +1.95.0 run -p xtask -- evidence-schema-check
cargo +1.95.0 run -p xtask -- badges --check
cargo +1.95.0 run -p xtask -- impacted-evidence
cargo +1.95.0 run -p xtask -- impacted-evidence --check
python -c "import pathlib,tomllib; tomllib.loads(pathlib.Path('.hl7v2/goals/active.toml').read_text())"
git diff --check
```
