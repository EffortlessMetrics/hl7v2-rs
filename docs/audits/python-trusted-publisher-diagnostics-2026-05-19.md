# Python Trusted Publisher Diagnostics - 2026-05-19

## Purpose

Record the current public Python registry boundary after
[#822](https://github.com/EffortlessMetrics/hl7v2-rs/pull/822) added
pre-upload GitHub OIDC claim diagnostics to the TestPyPI and PyPI release
workflows.

This is not a TestPyPI or PyPI release receipt. It records only the current
diagnostic state before the next publishing-mode proof attempt.

## Current State

- `main` commit:
  `2e7fba3861a9fafa7dfbc0c0385c42705b90193b`.
- Public Python distribution name: `hl7v2`.
- Python import module: `hl7v2`.
- Rust/PyO3 backend crate: `hl7v2-python`.
- The backend crate is binding infrastructure, not the public Python package
  proof and not the recommended Rust API.

## GitHub Environment Check

Command:

```powershell
rtk gh api repos/EffortlessMetrics/hl7v2-rs/environments/testpypi
```

Result:

- `testpypi` GitHub environment exists.
- Environment API URL:
  `https://api.github.com/repos/EffortlessMetrics/hl7v2-rs/environments/testpypi`.
- Environment was created on `2026-05-10T11:58:35Z`.

Command:

```powershell
rtk gh api repos/EffortlessMetrics/hl7v2-rs/environments/pypi
```

Result:

- `pypi` GitHub environment returned `404 Not Found`.
- This is not a production PyPI release blocker today because production PyPI
  remains a separate release decision.
- Before any production PyPI proof, create/configure the `pypi` GitHub
  environment and the production PyPI Trusted Publisher for project `hl7v2`.

## Registry Visibility Check

Command:

```powershell
rtk curl -I https://test.pypi.org/pypi/hl7v2/json
```

Result:

- `404 Not Found`.

Command:

```powershell
rtk curl -I https://pypi.org/pypi/hl7v2/json
```

Result:

- `404 Not Found`.

## Workflow Diagnostic Rail

[#822](https://github.com/EffortlessMetrics/hl7v2-rs/pull/822) added a
pre-upload step named `Record actual OIDC publisher claims` to:

- `.github/workflows/python-testpypi.yml`
- `.github/workflows/python-pypi.yml`

The step decodes the GitHub OIDC token payload and records these claims in the
job summary:

- `sub`
- `repository`
- `repository_owner`
- `ref`
- `environment`
- `workflow_ref`
- `job_workflow_ref`
- `sha`

The publish jobs fail before upload if the actual `sub`, `repository`, or
`ref` does not match the expected Trusted Publisher identity.

`cargo run -p xtask -- check-python-publish-policy` now rejects removing,
weakening, or moving that diagnostic after upload.

## Remaining Blocker

The GitHub-side `testpypi` environment exists. The public package indexes still
do not contain `hl7v2`.

The remaining external blocker is TestPyPI pending Trusted Publisher setup:

| Field | Value |
| --- | --- |
| Project name | `hl7v2` |
| Owner | `EffortlessMetrics` |
| Repository name | `hl7v2-rs` |
| Workflow filename | `python-testpypi.yml` |
| Environment name | `testpypi` |
| Expected subject | `repo:EffortlessMetrics/hl7v2-rs:environment:testpypi` |

After that external setup, rerun **Python TestPyPI Proof** from `main` with
`publish_to_testpypi=true`.

## Non-Claims

- No TestPyPI upload succeeded in this audit.
- No TestPyPI install-back proof exists in this audit.
- No production PyPI upload succeeded in this audit.
- No production PyPI install-back proof exists in this audit.
- No token fallback was added or used.
- No `skip-existing` path was added or used.
