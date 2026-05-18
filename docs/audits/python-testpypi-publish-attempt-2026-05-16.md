# Python TestPyPI Publishing Attempt Refresh

Date: 2026-05-16

This receipt records a guarded `Python TestPyPI Proof` workflow run with
`publish_to_testpypi=true` after the public Python distribution was retargeted
to `hl7v2`, the Rust v1.5.0 release was published, and the post-#705 readiness
receipt landed.

It proves the hosted wheel build and local wheel smoke still pass on current
`main`. The TestPyPI upload/install-back proof remains blocked by missing
Trusted Publishing configuration on TestPyPI.

## Run

| Field | Value |
| --- | --- |
| Workflow | `Python TestPyPI Proof` |
| Workflow file | `.github/workflows/python-testpypi.yml` |
| Trigger | `workflow_dispatch` |
| Input | `publish_to_testpypi=true` |
| Branch | `main` |
| Commit | `bbb10eae8b351753f8741905fa28d64e521fed03` |
| Run ID | `25972812666` |
| Run URL | <https://github.com/EffortlessMetrics/hl7v2-rs/actions/runs/25972812666> |
| Result | `failure` |

## Verified Jobs

| Job | Result | Evidence |
| --- | --- | --- |
| `Build and smoke wheel` | `success` | Built the wheel, installed it into a fresh virtual environment, ran `tests/python_smoke/smoke.py`, ran `tests/python_smoke/evidence_workflow_guide.py`, and uploaded the wheel artifact. |
| `Publish to TestPyPI` | `failure` | `pypa/gh-action-pypi-publish@v1.14.0` failed during Trusted Publishing token exchange with `invalid-publisher`. |
| `Install from TestPyPI and smoke` | `skipped` | Expected after the publish job failed; no install-back proof was completed. |

Observed job IDs:

```text
Build and smoke wheel: 76347700976
Publish to TestPyPI: 76347757507
Install from TestPyPI and smoke: 76347774391
```

The run uploaded a short-retention wheel artifact:

| Field | Value |
| --- | --- |
| Artifact name | `python-testpypi-wheel` |
| Artifact ID | `7036674892` |
| Artifact digest | `sha256:b6a5aa74d50e79e84f8560ca63d9210f09841aedf5280e5b9133c93ff6570089` |
| Artifact URL | <https://github.com/EffortlessMetrics/hl7v2-rs/actions/runs/25972812666/artifacts/7036674892> |
| Expiration | `2026-05-23T21:00:57Z` |

## Trusted Publisher Claims

The publish action reported these claims for the failed exchange:

| Claim | Value |
| --- | --- |
| `sub` | `repo:EffortlessMetrics/hl7v2-rs:environment:testpypi` |
| `repository` | `EffortlessMetrics/hl7v2-rs` |
| `repository_owner` | `EffortlessMetrics` |
| `repository_owner_id` | `164865351` |
| `workflow_ref` | `EffortlessMetrics/hl7v2-rs/.github/workflows/python-testpypi.yml@refs/heads/main` |
| `job_workflow_ref` | `EffortlessMetrics/hl7v2-rs/.github/workflows/python-testpypi.yml@refs/heads/main` |
| `ref` | `refs/heads/main` |
| `environment` | `testpypi` |

The workflow and repository claims match the intended setup documented in
[`docs/guides/python-testpypi-release-proof.md`](../guides/python-testpypi-release-proof.md).
The remaining action is to configure the corresponding TestPyPI pending
publisher for project `hl7v2`.

## Registry State

Live registry checks during this receipt still found no public Python package:

| Registry | URL | Result |
| --- | --- | --- |
| TestPyPI | <https://test.pypi.org/pypi/hl7v2/json> | `404 Not Found` at 2026-05-16T21:04:15Z |
| PyPI | <https://pypi.org/pypi/hl7v2/json> | `404 Not Found` at 2026-05-16T20:55:16Z |

## Boundaries

- No TestPyPI package was published by this run.
- No install-back from TestPyPI was completed by this run.
- No production PyPI publishing was attempted.
- No install-back from production PyPI was completed.
- No token fallback was used.
- `skip-existing` remained `false`.
- The crates.io `hl7v2-python` v1.5.0 upload remains only a binding-backend
  crates.io receipt; it is not a public Python package receipt.

## Next Step

Configure the TestPyPI Trusted Publisher for project `hl7v2` with the fields
below. The external setup work is tracked in
[#563](https://github.com/EffortlessMetrics/hl7v2-rs/issues/563).

| TestPyPI field | Value |
| --- | --- |
| Project name | `hl7v2` |
| Owner | `EffortlessMetrics` |
| Repository name | `hl7v2-rs` |
| Workflow filename | `python-testpypi.yml` |
| Environment name | `testpypi` |

After that setup is in place, rerun the `Python TestPyPI Proof` workflow from
`main` with `publish_to_testpypi=true` and require both publish and install-back
jobs to pass before claiming TestPyPI proof complete.
