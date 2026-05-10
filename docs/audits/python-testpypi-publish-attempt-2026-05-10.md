# Python TestPyPI Publishing Attempt

Date: 2026-05-10

This receipt records the first guarded `Python TestPyPI Proof` workflow run with
`publish_to_testpypi=true`. It proves the hosted wheel build and local wheel
smoke still pass on current `main`, but the TestPyPI upload/install-back proof
is blocked by missing Trusted Publishing configuration on TestPyPI.

## Run

| Field | Value |
| --- | --- |
| Workflow | `Python TestPyPI Proof` |
| Workflow file | `.github/workflows/python-testpypi.yml` |
| Trigger | `workflow_dispatch` |
| Input | `publish_to_testpypi=true` |
| Branch | `main` |
| Commit | `57e90731452c7f9e63fb21b5c3f40724051c1c89` |
| Run ID | `25628094425` |
| Run URL | <https://github.com/EffortlessMetrics/hl7v2-rs/actions/runs/25628094425> |
| Result | `failure` |

## Verified Jobs

| Job | Result | Evidence |
| --- | --- | --- |
| `Build and smoke wheel` | `success` | Built the wheel, installed it into a fresh virtual environment, ran `tests/python_smoke/smoke.py`, ran `tests/python_smoke/evidence_workflow_guide.py`, and uploaded the wheel artifact. |
| `Publish to TestPyPI` | `failure` | `pypa/gh-action-pypi-publish@v1.14.0` failed during Trusted Publishing token exchange with `invalid-publisher`. |
| `Install from TestPyPI and smoke` | `skipped` | Expected after the publish job failed; no install-back proof was completed. |

Observed job IDs:

```text
Build and smoke wheel: 75226681602
Publish to TestPyPI: 75226796522
Install from TestPyPI and smoke: 75226807596
```

## Trusted Publisher Claims

The publish action reported these claims for the failed exchange:

| Claim | Value |
| --- | --- |
| `sub` | `repo:EffortlessMetrics/hl7v2-rs:environment:testpypi` |
| `repository` | `EffortlessMetrics/hl7v2-rs` |
| `repository_owner` | `EffortlessMetrics` |
| `workflow_ref` | `EffortlessMetrics/hl7v2-rs/.github/workflows/python-testpypi.yml@refs/heads/main` |
| `ref` | `refs/heads/main` |
| `environment` | `testpypi` |

The workflow and repository claims match the intended setup documented in
[`docs/guides/python-testpypi-release-proof.md`](../guides/python-testpypi-release-proof.md).
The remaining action is to configure the corresponding TestPyPI pending
publisher for `hl7v2-python`.

## Follow-up Preflight Run

After `ci: summarize TestPyPI publisher setup (#561)` merged, the guarded
workflow was rerun from current `main` to prove the new preflight step and the
external boundary.

| Field | Value |
| --- | --- |
| Commit | `14f2c767654935e594eb02ea47bddc037e07ab03` |
| Run ID | `25628479479` |
| Run URL | <https://github.com/EffortlessMetrics/hl7v2-rs/actions/runs/25628479479> |
| Result | `failure` |

| Job | Result | Evidence |
| --- | --- | --- |
| `Build and smoke wheel` | `success` | Built the wheel, installed it into a fresh virtual environment, and uploaded the wheel artifact. |
| `Publish to TestPyPI` | `failure` | The new `Record trusted publisher setup` step completed, then `pypa/gh-action-pypi-publish@v1.14.0` failed during Trusted Publishing token exchange with `invalid-publisher`. |
| `Install from TestPyPI and smoke` | `skipped` | Expected after the publish job failed; no install-back proof was completed. |

Observed job IDs:

```text
Build and smoke wheel: 75227714888
Publish to TestPyPI: 75227768867
Install from TestPyPI and smoke: 75227784755
```

The publish action reported the same trusted-publisher subject:

```text
repo:EffortlessMetrics/hl7v2-rs:environment:testpypi
```

This confirms the repository-side workflow now records the setup fields before
attempting upload. The remaining blocker is still external TestPyPI Trusted
Publisher configuration for project `hl7v2-python`.

## Boundaries

- `hl7v2-python` remains `publish = false` for crates.io.
- No TestPyPI package was published by this run.
- No install-back from TestPyPI was completed by this run.
- No production PyPI publishing was attempted.
- Do not switch to token-based publishing or `skip-existing` without a separate
  security/release decision.

## Next Step

Configure the TestPyPI Trusted Publisher for project `hl7v2-python` with the
fields below. The external setup work is tracked in
[#563](https://github.com/EffortlessMetrics/hl7v2-rs/issues/563).

| TestPyPI field | Value |
| --- | --- |
| Owner | `EffortlessMetrics` |
| Repository name | `hl7v2-rs` |
| Workflow filename | `python-testpypi.yml` |
| Environment name | `testpypi` |

After that setup is in place, rerun the `Python TestPyPI Proof` workflow from
`main` with `publish_to_testpypi=true` and require both publish and install-back
jobs to pass before claiming TestPyPI proof complete.
