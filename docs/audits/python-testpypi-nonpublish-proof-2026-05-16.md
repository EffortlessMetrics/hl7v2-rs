# Python TestPyPI Non-Publishing Proof Refresh

Date: 2026-05-16

This receipt records a current-main hosted `Python TestPyPI Proof` workflow run
in non-publishing mode after the post-release SRP refactor wave landed through
#691. It proves the hosted wheel build, fresh-venv local wheel install, import
smoke, and Python evidence workflow smoke for the public Python package
identity `hl7v2`.

It does not publish to TestPyPI or PyPI.

## Run

| Field | Value |
| --- | --- |
| Workflow | `Python TestPyPI Proof` |
| Workflow file | `.github/workflows/python-testpypi.yml` |
| Trigger | `workflow_dispatch` |
| Input | `publish_to_testpypi=false` |
| Branch | `main` |
| Commit | `4cf501ddc0f7fc3d027b3ce2459e899fe4aa7092` |
| Run ID | `25958692952` |
| Run URL | <https://github.com/EffortlessMetrics/hl7v2-rs/actions/runs/25958692952> |
| Result | `success` |

## Verified Jobs

| Job | Result | Evidence |
| --- | --- | --- |
| `Build and smoke wheel` | `success` | Checked out `main`, resolved package version `1.5.0`, built the wheel, installed it into a fresh virtual environment, ran `tests/python_smoke/smoke.py`, ran `tests/python_smoke/evidence_workflow_guide.py`, and uploaded the wheel artifact. |
| `Publish to TestPyPI` | `skipped` | Expected because `publish_to_testpypi=false`. |
| `Install from TestPyPI and smoke` | `skipped` | Expected because no TestPyPI upload was requested. |

Observed job IDs:

```text
Build and smoke wheel: 76310046651
Publish to TestPyPI: 76310107871
Install from TestPyPI and smoke: 76310107947
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
| Artifact ID | `7032389168` |
| Artifact digest | `sha256:83e65989796b1e2a4745eeb2c50106af63787f7e6d90cd3c84efcc0aa3223a8b` |
| Artifact URL | <https://github.com/EffortlessMetrics/hl7v2-rs/actions/runs/25958692952/artifacts/7032389168> |
| Expiration | 2026-05-23T09:41:23Z |

## Registry State

Live registry checks during this receipt still found no public Python package:

| Registry | URL | Result |
| --- | --- | --- |
| TestPyPI | <https://test.pypi.org/simple/hl7v2/> | `404 Not Found` at 2026-05-16T09:45:35Z |
| PyPI | <https://pypi.org/simple/hl7v2/> | `404 Not Found` at 2026-05-16T09:45:35Z |

## Boundaries

- This proof did not upload to TestPyPI.
- This proof did not install back from TestPyPI.
- This proof did not publish to production PyPI.
- This proof did not install back from production PyPI.
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
