# Python TestPyPI Non-Publishing Proof Refresh

Date: 2026-05-15 local / 2026-05-16 UTC

This receipt records a current-main manual `Python TestPyPI Proof` workflow run
in non-publishing mode after the v1.5.0 crates.io release and the local Python
wheel proof landed. It proves the hosted wheel build, fresh-venv install, and
Python evidence smoke checks for the public Python package identity `hl7v2`.

It does not publish to TestPyPI or PyPI.

## Run

| Field | Value |
| --- | --- |
| Workflow | `Python TestPyPI Proof` |
| Workflow file | `.github/workflows/python-testpypi.yml` |
| Trigger | `workflow_dispatch` |
| Input | `publish_to_testpypi=false` |
| Branch | `main` |
| Commit | `204b50507bbfe5d5d960e405c35218489a7f205c` |
| Run ID | `25948637228` |
| Run URL | <https://github.com/EffortlessMetrics/hl7v2-rs/actions/runs/25948637228> |
| Result | `success` |

## Verified Jobs

| Job | Result | Evidence |
| --- | --- | --- |
| `Build and smoke wheel` | `success` | Checked out `main`, resolved package version `1.5.0`, built the wheel, installed it into a fresh virtual environment, ran `tests/python_smoke/smoke.py`, ran `tests/python_smoke/evidence_workflow_guide.py`, and uploaded the wheel artifact. |
| `Publish to TestPyPI` | `skipped` | Expected because `publish_to_testpypi=false`. |
| `Install from TestPyPI and smoke` | `skipped` | Expected because no TestPyPI upload was requested. |

Observed job IDs:

```text
Build and smoke wheel: 76281948112
Publish to TestPyPI: 76282105529
Install from TestPyPI and smoke: 76282105553
```

The run installed the built wheel in the hosted fresh virtual environment and
reported:

```text
Processing ./dist/hl7v2-1.5.0-cp313-cp313-manylinux_2_34_x86_64.whl
Successfully installed hl7v2-1.5.0
hl7v2 smoke ok version=1.5.0 segments=2
python evidence workflow guide ok version=1.5.0
```

The run uploaded a short-retention wheel artifact:

| Field | Value |
| --- | --- |
| Artifact name | `python-testpypi-wheel` |
| Artifact ID | `7029158879` |
| Artifact digest | `sha256:bfa778257655cd893de070277a215c10760e2e668e38b0cec487efdb8584fba6` |
| Artifact URL | <https://github.com/EffortlessMetrics/hl7v2-rs/actions/runs/25948637228/artifacts/7029158879> |
| Expiration | 2026-05-23T01:04:01Z |

## Boundaries

- This proof did not upload to TestPyPI.
- This proof did not install back from TestPyPI.
- This proof did not publish to production PyPI.
- This proof did not use token fallback.
- This proof did not use `skip-existing`.
- This proof does not make `hl7v2-python` the recommended Rust API.
- The crates.io `hl7v2-python` v1.5.0 upload remains only a binding-backend
  crates.io receipt; it is not a public Python package receipt.

## Remaining Gap

Issue #563 remains the external blocker for TestPyPI Trusted Publisher setup for
project `hl7v2`. The Python lane can claim TestPyPI success only after the
manual workflow runs with `publish_to_testpypi=true`, uploads
`hl7v2==1.5.0` to TestPyPI, installs it back from
`https://test.pypi.org/simple/`, imports `hl7v2`, and runs both Python smoke
scripts successfully.
