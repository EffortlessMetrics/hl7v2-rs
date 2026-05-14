# Python TestPyPI Release Proof

Use this guide when you need to prove the `hl7v2` distribution as a Python
package before any production PyPI release. This lane is separate from the Rust
primary product graph.

## Package Identity

| Field | Value |
| --- | --- |
| Python distribution | `hl7v2` |
| Python import module | `hl7v2` |
| Rust backend crate | `hl7v2-python` PyO3 backend |
| crates.io publish policy | currently `publish = false`; binding-backend publication requires a separate release PR and does not make it the recommended Rust API |
| TestPyPI workflow | `.github/workflows/python-testpypi.yml` |
| GitHub environment | `testpypi` |

Do not publish `hl7v2-python` as part of TestPyPI proof. The primary Rust
product graph remains `hl7v2`, `hl7v2-server`, and `hl7v2-cli`. Rust users
should depend on `hl7v2`; Python users should install/import `hl7v2`.

## One-Time TestPyPI Setup

Use TestPyPI Trusted Publishing. Do not add repository tokens unless a separate
security review chooses token-based publishing.

Configure a pending publisher in TestPyPI with:

| TestPyPI field | Value |
| --- | --- |
| Project name | `hl7v2` |
| Owner | `EffortlessMetrics` |
| Repository name | `hl7v2-rs` |
| Workflow filename | `python-testpypi.yml` |
| Environment name | `testpypi` |

In GitHub, create an environment named `testpypi`. Add reviewer protection if
you want a second human confirmation before upload.

## Local Wheel Proof

Run the policy rail before the manual TestPyPI workflow:

```powershell
cargo run -p xtask -- check-python-publish-policy
```

Then run the local wheel proof:

```powershell
python -m pip install --upgrade pip "maturin==1.13.1"
$env:PYO3_USE_ABI3_FORWARD_COMPATIBILITY = "1"
maturin build --release --out dist
python -m pip install --force-reinstall (Get-ChildItem dist\*.whl | Select-Object -First 1).FullName
python tests\python_smoke\smoke.py
python tests\python_smoke\evidence_workflow_guide.py
```

Expected result:

```text
hl7v2 smoke ok version=<version> segments=2
```

## Manual TestPyPI Proof

Run the **Python TestPyPI Proof** workflow manually.

First run with:

```text
publish_to_testpypi = false
```

This builds the wheel, installs it into a fresh virtual environment, runs the
Python smoke test plus the evidence workflow guide, and uploads the wheel as a
short-retention artifact. It does not publish.

The 2026-05-09 run of this non-publishing mode passed on `main`; see
[`docs/audits/python-testpypi-nonpublish-proof-2026-05-09.md`](../audits/python-testpypi-nonpublish-proof-2026-05-09.md).

After the local wheel proof and non-publishing workflow pass, rerun with:

```text
publish_to_testpypi = true
```

Run publishing mode from `main`. The workflow fails early if
`publish_to_testpypi=true` is selected from any other ref.

Before upload, the publish job writes the expected TestPyPI Trusted Publisher
fields to the GitHub Actions job summary. If the upload fails with
`invalid-publisher`, compare the summary fields to the TestPyPI pending
publisher configuration and fix TestPyPI before rerunning. Do not switch to a
repository token or `skip-existing` as a shortcut around Trusted Publishing.

This does three things:

1. Builds and smoke-tests the wheel.
2. Publishes the wheel to TestPyPI using Trusted Publishing.
3. Installs `hl7v2==<workspace version>` back from TestPyPI in a fresh
   virtual environment and reruns `tests/python_smoke/smoke.py` plus
   `tests/python_smoke/evidence_workflow_guide.py`.

TestPyPI does not allow overwriting an existing file for the same version. If
the upload fails because the version already exists, stop and choose a new
workspace version for the next proof attempt. Do not use `skip-existing` for
release proof, because that can accidentally test an older artifact.

## Stop Conditions

A TestPyPI proof is complete only when all of these are true:

- The local wheel proof passes.
- The manual workflow with `publish_to_testpypi=false` passes.
- The manual workflow with `publish_to_testpypi=true` uploads the current
  version to TestPyPI.
- The install-back job installs from `https://test.pypi.org/simple/` and runs
  `tests/python_smoke/smoke.py` plus
  `tests/python_smoke/evidence_workflow_guide.py` successfully.

This is still not a production PyPI release. Treat it as packaging evidence for
the separate Python lane. A crates.io binding-backend publish, if later
approved, does not replace TestPyPI upload and install-back proof.

After the upload/install-back proof passes, use
[Python PyPI Release](python-pypi-release.md) for the guarded production PyPI
release path.

Current status: the non-publishing proof is complete. A 2026-05-10
publishing-mode run from `main` built and smoke-tested the wheel, then failed
during Trusted Publishing token exchange with `invalid-publisher`; see
[docs/audits/python-testpypi-publish-attempt-2026-05-10.md](../audits/python-testpypi-publish-attempt-2026-05-10.md).
The public distribution has since been retargeted from `hl7v2-python` to
`hl7v2`. The TestPyPI upload/install-back proof remains incomplete until the
TestPyPI Trusted Publisher is configured for project `hl7v2` and a rerun
passes. Track the external setup blocker in
[#563](https://github.com/EffortlessMetrics/hl7v2-rs/issues/563).
