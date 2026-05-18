# Python PyPI Non-Publishing Proof - 2026-05-16

## Summary

The production **Python PyPI Release Proof** workflow was run manually in
non-publishing mode. It built the public Python `hl7v2` wheel from the
`hl7v2-python` PyO3 backend, installed that wheel into a fresh virtual
environment, and ran the Python smoke and evidence workflow guide checks.

This receipt proves the production PyPI workflow's build-and-smoke path. It
does not prove a production PyPI upload or production install-back.

## Run

| Field | Value |
| --- | --- |
| Workflow | `Python PyPI Release Proof` |
| Workflow file | `.github/workflows/python-pypi.yml` |
| Run URL | <https://github.com/EffortlessMetrics/hl7v2-rs/actions/runs/25950854618> |
| Event | `workflow_dispatch` |
| Branch | `main` |
| Commit | `ae2ab80ecde203ae8399fafeb5d50cbba73889fa` |
| Package version | `1.5.0` |
| Inputs | `publish_to_pypi=false`, `testpypi_proof_url=""` |
| Conclusion | `success` |

Command used:

```powershell
rtk gh workflow run python-pypi.yml --ref main -f publish_to_pypi=false -f testpypi_proof_url=
```

## Job Results

| Job | Result | Notes |
| --- | --- | --- |
| `Build and smoke wheel` | `success` | Built the wheel, installed it into a fresh venv, ran `tests/python_smoke/smoke.py`, and ran `tests/python_smoke/evidence_workflow_guide.py`. |
| `Publish to PyPI` | `skipped` | Skipped because `publish_to_pypi=false`. |
| `Install from PyPI and smoke` | `skipped` | Skipped because no production PyPI upload was requested. |

## Artifact

| Field | Value |
| --- | --- |
| Artifact name | `python-pypi-wheel` |
| Artifact ID | `7029874204` |
| Size | `2105162` bytes |
| Digest | `sha256:e4b7ff41b9c185bfb3f21c0618d20fcebdda82dc3022e0c262c6e2ab980afebb` |
| Created | `2026-05-16T02:51:21Z` |
| Expired at receipt time | `false` |

## Non-Claims

This receipt does not claim:

- production PyPI upload;
- production PyPI install-back;
- TestPyPI upload;
- TestPyPI install-back;
- PyPI Trusted Publisher configuration;
- TestPyPI Trusted Publisher configuration;
- token fallback;
- `skip-existing`;
- public Python `hl7v2` availability on PyPI or TestPyPI.

## Next Required Proofs

The public Python package remains unreleased until the separate TestPyPI and
production PyPI proof path completes:

1. Configure TestPyPI Trusted Publisher for project `hl7v2`, workflow
   `python-testpypi.yml`, environment `testpypi`.
2. Run **Python TestPyPI Proof** with `publish_to_testpypi=true`.
3. Record upload, install-back, import smoke, `smoke.py`, and
   `evidence_workflow_guide.py` success.
4. Decide production PyPI separately.
5. If approved, run **Python PyPI Release Proof** with `publish_to_pypi=true`
   and a same-commit successful TestPyPI proof URL.
