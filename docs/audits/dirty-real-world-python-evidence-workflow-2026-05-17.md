# Dirty Real-World Python Evidence Workflow Proof

Date: 2026-05-17
Branch: `test/python-dirty-evidence-workflow`
Result: Passed locally

This receipt extends the local Python `hl7v2` wheel proof from dirty-corpus
summary/fingerprint/diff into the same dirty validate, redact, bundle, and
replay workflow already covered for CLI, REST, and gRPC.

This is a local wheel proof only. It is not a TestPyPI or PyPI upload,
install-back, or registry availability receipt.

## Fixture

The proof uses the shared dirty real-world Z-segment fixture:

```text
test_data/dirty-real-world/after/z-segment.hl7
```

The workflow validates the message with the shared dirty ADT profile, redacts
it with the shared dirty safe-analysis policy, creates a schema-versioned
evidence bundle, replays that bundle, and verifies that synthetic PHI markers
are absent from the reports and bundle artifacts.

## Proof

The Python smoke verifies:

- `hl7v2.validate` accepts the dirty Z-segment fixture and emits v2 provenance;
- `hl7v2.redact` hashes `PID.3`, drops patient name and birth date fields, and
  retains `ZPV` triage context;
- `hl7v2.bundle` writes the expected redacted evidence artifacts with v2
  provenance;
- `hl7v2.replay` verifies the bundle manifest hashes and reproduces validation;
- bundle and replay evidence does not expose `MRN-Z`, `Example^Zed`,
  `19700101`, or local scratch paths.

## Validation

| Command | Result |
| --- | --- |
| `cargo +1.95.0 fmt --all -- --check` | pass |
| `cargo +1.95.0 test -p xtask dirty --locked` | pass |
| `PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo +1.95.0 run -p xtask -- gate --check --only clippy` | pass |
| `cargo +1.95.0 run -p xtask -- python-local-wheel-proof --root F:\cargo-target\hl7v2-rs-python-dirty-evidence-proof-workflow-2026-05-17` | pass |
| `cargo +1.95.0 run -p xtask -- check-dirty-corpus-parity --include-python` with the proof venv first on `PATH` | pass |
| `cargo +1.95.0 run -p xtask -- check-evidence-parity` | pass |
| `cargo +1.95.0 run -p xtask -- check-doc-links` | pass |
| `cargo +1.95.0 run -p xtask -- check-file-policy` | pass |
| `git diff --check` | pass |

The local proof command built and installed `hl7v2-1.5.0` into a scratch
virtual environment and ran:

```text
tests/python_smoke/smoke.py
tests/python_smoke/evidence_workflow_guide.py
tests/python_smoke/dirty_evidence_workflow.py
```

## Non-Claims

- No TestPyPI upload occurred.
- No TestPyPI install-back occurred.
- No production PyPI upload occurred.
- No production PyPI install-back occurred.
- No token fallback was used.
- No `skip-existing` workaround was used.
- This does not close issue
  [#563](https://github.com/EffortlessMetrics/hl7v2-rs/issues/563).
