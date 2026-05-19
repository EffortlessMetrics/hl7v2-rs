# Python TestPyPI OIDC Diagnostic - 2026-05-19

## Purpose

Record the first hosted no-upload TestPyPI Trusted Publisher diagnostic run
after [#825](https://github.com/EffortlessMetrics/hl7v2-rs/pull/825) added
`diagnose_trusted_publisher` to the Python TestPyPI proof workflow.

This is not a TestPyPI release receipt. It records only the hosted OIDC
diagnostic boundary and the continued absence of public Python registry
artifacts.

## Workflow Run

| Field | Value |
| --- | --- |
| Workflow | `Python TestPyPI Proof` |
| Run | <https://github.com/EffortlessMetrics/hl7v2-rs/actions/runs/26103750173> |
| Event | `workflow_dispatch` |
| Commit | `bc78d7a004925cdd1b3e3510d764440b4af40884` |
| Branch | `main` |
| Created | `2026-05-19T14:27:39Z` |
| Conclusion | `success` |

Dispatch inputs:

```text
publish_to_testpypi = false
diagnose_trusted_publisher = true
```

## Job Results

| Job | Result | Evidence |
| --- | --- | --- |
| `Build and smoke wheel` | success | Built `hl7v2` wheel, installed it into a fresh venv, ran `smoke.py`, `evidence_workflow_guide.py`, and `dirty_evidence_workflow.py`, then uploaded the short-retention wheel artifact. |
| `Publish to TestPyPI` | success | Entered the `testpypi` GitHub environment, downloaded the wheel artifact, recorded trusted-publisher setup, and ran `Record actual OIDC publisher claims`. |
| `Publish package distributions to TestPyPI` step | skipped | Expected because `publish_to_testpypi=false`. |
| `Install from TestPyPI and smoke` | skipped | Expected because no upload was attempted. |

Wheel artifact:

| Field | Value |
| --- | --- |
| Name | `python-testpypi-wheel` |
| Size | `2112802` bytes |
| Expired | `false` at receipt time |

The publish job evaluated the TestPyPI environment URL as:

```text
https://test.pypi.org/project/hl7v2/1.5.0/
```

## OIDC Diagnostic Result

The `Record actual OIDC publisher claims` step passed inside the `testpypi`
environment with these expected values configured:

| Claim | Expected value |
| --- | --- |
| `sub` | `repo:EffortlessMetrics/hl7v2-rs:environment:testpypi` |
| `repository` | `EffortlessMetrics/hl7v2-rs` |
| `ref` | `refs/heads/main` |

The workflow step exits non-zero if any of those three claims differ. The
successful step therefore proves that GitHub supplied a matching OIDC identity
for the TestPyPI Trusted Publisher subject.

The full decoded claim table is written to the GitHub Actions job summary. This
receipt records only the machine-verifiable pass/fail boundary and the expected
identity fields because GitHub job logs do not expose the rendered step summary.

## Registry Visibility

Commands:

```powershell
rtk curl -I https://test.pypi.org/pypi/hl7v2/json
rtk curl -I https://pypi.org/pypi/hl7v2/json
```

Results:

- TestPyPI `hl7v2` returned `404 Not Found`.
- PyPI `hl7v2` returned `404 Not Found`.

## Remaining Blocker

The hosted GitHub OIDC identity now matches the expected TestPyPI Trusted
Publisher subject. The remaining external work is to configure or accept the
TestPyPI Trusted Publisher for:

| Field | Value |
| --- | --- |
| Project name | `hl7v2` |
| Owner | `EffortlessMetrics` |
| Repository name | `hl7v2-rs` |
| Workflow filename | `python-testpypi.yml` |
| Environment name | `testpypi` |
| Subject | `repo:EffortlessMetrics/hl7v2-rs:environment:testpypi` |

After that external setup, rerun **Python TestPyPI Proof** from `main` with
`publish_to_testpypi=true` and `diagnose_trusted_publisher=false`.

## Non-Claims

- No TestPyPI upload was attempted.
- No TestPyPI install-back proof exists in this receipt.
- No production PyPI upload was attempted.
- No production PyPI install-back proof exists in this receipt.
- No token fallback was added or used.
- No `skip-existing` path was added or used.
- No npm package was published.
- No new crates.io release, tag, or GitHub release was created.

