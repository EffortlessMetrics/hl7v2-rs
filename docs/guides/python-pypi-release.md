# Python PyPI Release

Use this guide only after the separate `hl7v2` Python distribution has passed the
local wheel proof and the full TestPyPI upload/install-back proof. This is a
production package release path for Python users. It does not change the Rust
crates.io graph.

## Package Identity

| Field | Value |
| --- | --- |
| Python distribution | `hl7v2` |
| Python import module | `hl7v2` |
| Rust package | `hl7v2-python` |
| crates.io publish policy | `publish = false` |
| Production PyPI workflow | `.github/workflows/python-pypi.yml` |
| GitHub environment | `pypi` |

Do not publish `hl7v2-python` to crates.io. The Rust release graph remains
`hl7v2`, `hl7v2-server`, and `hl7v2-cli`.

## Preconditions

Before running the production publish mode, verify all of these are true:

- The Rust crates for the same release train are already published or
  intentionally not changing.
- The local wheel proof passes with `maturin build --release --out dist`,
  `tests/python_smoke/smoke.py`, and
  `tests/python_smoke/evidence_workflow_guide.py`.
- The manual **Python TestPyPI Proof** workflow has passed with
  `publish_to_testpypi=true`.
- The TestPyPI install-back job installed
  `hl7v2==<workspace version>` from `https://test.pypi.org/simple/` and
  ran `tests/python_smoke/smoke.py` plus
  `tests/python_smoke/evidence_workflow_guide.py`.
- You have the successful **Python TestPyPI Proof** workflow run URL for this
  exact version.
- The current version is not already present on production PyPI.
- Release notes identify the Python package as a separate distribution lane.

If any precondition is not true, stop. Do not use a production PyPI publish as a
substitute for TestPyPI proof.

## One-Time PyPI Setup

Use PyPI Trusted Publishing. Do not add repository PyPI tokens unless a separate
security review chooses token-based publishing.

Configure a pending publisher in PyPI with:

| PyPI field | Value |
| --- | --- |
| Project name | `hl7v2` |
| Owner | `EffortlessMetrics` |
| Repository name | `hl7v2-rs` |
| Workflow filename | `python-pypi.yml` |
| Environment name | `pypi` |

In GitHub, create an environment named `pypi`. Use required reviewers for the
environment. The workflow grants `id-token: write` only to the publish job, not
to the build/smoke job.

## Non-Publishing Production Rehearsal

Before using the production workflow, run the local policy rail:

```powershell
cargo run -p xtask -- check-python-publish-policy
```

Run the **Python PyPI Release Proof** workflow manually with:

```text
publish_to_pypi = false
```

This builds the wheel, installs it into a fresh virtual environment, runs the
Python smoke test and evidence workflow guide, and uploads the wheel as a
short-retention artifact. It does not publish to PyPI.

## Production Publish

Run the same workflow manually with:

```text
publish_to_pypi = true
testpypi_proof_url = https://github.com/EffortlessMetrics/hl7v2-rs/actions/runs/<run-id>
```

Run the publishing mode from `main`. The workflow fails early if
`publish_to_pypi=true` is selected from any other ref. It also fails before
uploading unless `testpypi_proof_url` points at a successful manual
**Python TestPyPI Proof** run from the same `main` commit, that run's
`Publish to TestPyPI` and `Install from TestPyPI and smoke` jobs both passed,
the current version is visible on TestPyPI, and the current version is absent
from production PyPI.

This does three things:

1. Builds and smoke-tests the wheel.
2. Publishes the wheel to production PyPI using Trusted Publishing.
3. Installs `hl7v2==<workspace version>` back from PyPI in a fresh
   virtual environment and reruns the Python smoke and evidence workflow guide.

PyPI does not allow overwriting an existing file for the same version. If the
upload fails because the version already exists, stop and choose a new workspace
version for the next proof attempt. Do not use `skip-existing` for release
proof, because that can accidentally test an older artifact.

## Stop Conditions

A production PyPI release is complete only when all of these are true:

- The manual workflow with `publish_to_pypi=true` uploads the current workspace
  version to PyPI.
- The install-back job installs from `https://pypi.org/simple/`.
- `tests/python_smoke/smoke.py` passes against the installed PyPI package.
- `tests/python_smoke/evidence_workflow_guide.py` passes against the installed
  PyPI package.
- A release receipt records the workflow run URL, version, package URL, and
  install-back result.

Until those are true, describe the Python lane as wheel/TestPyPI-proven, not as
production-published.
