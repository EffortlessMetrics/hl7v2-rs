# User Journey Acceptance Proof

Date: 2026-05-15
Branch: `docs/user-journey-acceptance-proof`
Scope: current first-use and evidence-workflow acceptance surface after the
v1.5.0 crates.io release, CLI/Rust journey tests, and operator guidance landed.

This receipt records the user-facing paths that prove a normal operator can get
from install or local build to a useful, shareable evidence artifact without
understanding the repo topology.

## Acceptance Surfaces

| Surface | User job | Proof |
| --- | --- | --- |
| Rust library | Embed parse, validation, redaction, bundle, and replay evidence in an application. | `cargo +1.95.0 test -p hl7v2 --test user_journey --all-features --locked` |
| CLI | Validate, redact, bundle, replay, and inspect shareable support evidence. | `cargo +1.95.0 test -p hl7v2-cli --test integration_tests journey_cli_validate_redact_bundle_replay_produces_shareable_receipts --locked` |
| Server REST sidecar | Run a validation sidecar and prove redacted validation, bundle, replay, and corpus diff over HTTP. | `tests/server_smoke/smoke.py` against a running sidecar; covered by the server smoke workflow. |
| Python local wheel | Import `hl7v2`, run evidence helpers, validate schemas, redact, bundle, and replay through the installed wheel. | `tests/python_smoke/smoke.py` and `tests/python_smoke/evidence_workflow_guide.py` after local wheel install. |

## What The Journey Proves

- the Rust and CLI paths produce validation reports, redaction receipts, evidence
  bundles, and replay reports from one realistic HL7 message;
- shareable artifacts do not contain the configured PHI leak sentinels;
- replay detects tampering instead of treating a bundle as trusted by existence;
- Python and server first-use checks have explicit smoke scripts tied to their
  install/runtime setup instead of relying only on unit tests;
- first-use documentation points users to the same product classes recorded in
  the package-boundary model: Rust product crates, language packages, binding
  backend crates, and internal/dev crates.

## Current Validation

Run on this branch:

```text
cargo +1.95.0 test -p hl7v2 --test user_journey --all-features --locked
cargo +1.95.0 test -p hl7v2-cli --test integration_tests journey_cli_validate_redact_bundle_replay_produces_shareable_receipts --locked
cargo +1.95.0 run -p xtask -- check-doc-links
cargo +1.95.0 run -p xtask -- badges --check
cargo +1.95.0 run -p xtask -- impacted-evidence --check
git diff --check
```

## Non-Claims

- This receipt does not upload to TestPyPI or PyPI.
- This receipt does not prove `pip install hl7v2` from a public Python registry.
- This receipt does not create or publish an npm package.
- This receipt does not change the v1.5.0 crates.io release.
- This receipt does not promote `hl7v2-python` as the recommended Rust API.

## Remaining Gap

The public Python package remains blocked on TestPyPI Trusted Publisher setup
for project `hl7v2`. After issue #563 is resolved, the Python TestPyPI workflow
must upload, install back, import `hl7v2`, run `smoke.py`, and run
`evidence_workflow_guide.py` before any TestPyPI success claim.
