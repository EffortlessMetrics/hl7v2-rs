# Package Registry State Audit

Date: 2026-05-15
Branch: `docs/package-registry-state-2026-05-15`
Scope: non-publishing registry-state check after the v1.5.0 readiness refresh
and release graph decision.

This audit records live package index state only. It is not a crates.io,
TestPyPI, PyPI, npm, tag, or GitHub release receipt.

## Result

The public registries still match the expected pre-release boundary:

| Surface | Registry state |
| --- | --- |
| crates.io `hl7v2` | Present; latest visible version `1.4.0`. |
| crates.io `hl7v2-server` | Present; latest visible version `1.4.0`. |
| crates.io `hl7v2-cli` | Present; latest visible version `1.4.0`. |
| crates.io `hl7v2-python` | Not present. |
| PyPI `hl7v2` | Not present; JSON API returned `404`. |
| TestPyPI `hl7v2` | Not present; JSON API returned `404`. |

The selected v1.5.0 crates.io graph remains:

1. `hl7v2`
2. `hl7v2-python`
3. `hl7v2-server`
4. `hl7v2-cli`

`hl7v2-python` is selected only as binding backend infrastructure. It is not
the recommended Rust API, and its crates.io publication would not prove PyPI
availability for the public Python package `hl7v2`.

## Commands

Recorded:

```powershell
Invoke-WebRequest https://crates.io/api/v1/crates/hl7v2
Invoke-WebRequest https://crates.io/api/v1/crates/hl7v2-server
Invoke-WebRequest https://crates.io/api/v1/crates/hl7v2-cli
Invoke-WebRequest https://crates.io/api/v1/crates/hl7v2-python
Invoke-WebRequest https://pypi.org/pypi/hl7v2/json
Invoke-WebRequest https://test.pypi.org/pypi/hl7v2/json
cargo search hl7v2 --limit 10
gh run list --branch main --limit 8
git status --short --branch
```

Observed results:

```text
hl7v2 status=200 max_version=1.4.0
hl7v2-server status=200 max_version=1.4.0
hl7v2-cli status=200 max_version=1.4.0
hl7v2-python status=404
https://pypi.org/pypi/hl7v2/json status=404
https://test.pypi.org/pypi/hl7v2/json status=404
```

`cargo search hl7v2 --limit 10` also reported the published primary Rust
product crates at `1.4.0`.

Hosted `main` checks after the latest readiness refresh were green for CI,
Security, and CI Policy. Pending Droid Manual runs are unrelated to the
release registry state.

## Non-Claims

- No crates.io upload was run.
- No TestPyPI upload was run.
- No PyPI upload was run.
- No npm package exists or was published.
- No `v1.5.0` tag was created.
- No GitHub release was created.
- No install-back proof exists for TestPyPI or production PyPI.

## Next Required Receipts

The next registry-mutating step requires explicit release approval. If
approved, publish the selected v1.5.0 crates.io graph in dependency order and
record registry resolution for each crate.

The Python distribution remains blocked on external TestPyPI Trusted Publisher
setup for project `hl7v2`. After that setup, the dedicated Python TestPyPI
proof workflow must upload, install back, import `hl7v2`, run `smoke.py`, and
run `evidence_workflow_guide.py` before any TestPyPI success claim.
