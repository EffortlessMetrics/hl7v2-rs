# Python TestPyPI Non-Publishing Proof

Date: 2026-05-09

This receipt records the manual `Python TestPyPI Proof` workflow run in
non-publishing mode. It proves the hosted wheel build, fresh-venv install, and
Python evidence smoke checks for the separate `hl7v2-python` lane. It does not
publish to TestPyPI or PyPI.

## Run

| Field | Value |
| --- | --- |
| Workflow | `Python TestPyPI Proof` |
| Workflow file | `.github/workflows/python-testpypi.yml` |
| Trigger | `workflow_dispatch` |
| Input | `publish_to_testpypi=false` |
| Branch | `main` |
| Commit | `171264eddb4786caeab8a99c3c4b8a52294ec53a` |
| Run ID | `25613226162` |
| Run URL | <https://github.com/EffortlessMetrics/hl7v2-rs/actions/runs/25613226162> |
| Result | `success` |

## Verified Jobs

| Job | Result | Evidence |
| --- | --- | --- |
| `Build and smoke wheel` | `success` | Built the wheel, installed it into a fresh virtual environment, ran `tests/python_smoke/smoke.py`, ran `tests/python_smoke/evidence_workflow_guide.py`, and uploaded the wheel artifact. |
| `Publish to TestPyPI` | `skipped` | Expected because `publish_to_testpypi=false`. |
| `Install from TestPyPI and smoke` | `skipped` | Expected because no TestPyPI upload was requested. |

Observed completed job ID:

```text
Build and smoke wheel: 75186830790
```

## Boundaries

- `hl7v2-python` remains `publish = false` for crates.io.
- This proof did not upload to TestPyPI.
- This proof did not install back from TestPyPI.
- This proof did not publish to production PyPI.
- A full TestPyPI distribution proof still requires a separate manual workflow
  run with `publish_to_testpypi=true` after the TestPyPI trusted-publisher
  setup is confirmed.

## Current Status

The hosted non-publishing Python distribution proof is complete for current
`main`. The remaining Python distribution question is whether and when to run
the publishing TestPyPI proof and then decide on production PyPI.
