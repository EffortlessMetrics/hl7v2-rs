# Python TestPyPI Release Proof

Use this guide when you need to prove the `hl7v2-python` binding as a Python
package before any production PyPI release. This lane is separate from the Rust
crates.io release graph.

## Package Identity

| Field | Value |
| --- | --- |
| Python distribution | `hl7v2-python` |
| Python import module | `hl7v2` |
| Rust package | `hl7v2-python` |
| crates.io publish policy | `publish = false` |
| TestPyPI workflow | `.github/workflows/python-testpypi.yml` |
| GitHub environment | `testpypi` |

Do not publish `hl7v2-python` to crates.io. The Rust release graph remains
`hl7v2`, `hl7v2-server`, and `hl7v2-cli`.

## One-Time TestPyPI Setup

Use TestPyPI Trusted Publishing. Do not add repository tokens unless a separate
security review chooses token-based publishing.

Configure a pending publisher in TestPyPI with:

| TestPyPI field | Value |
| --- | --- |
| Project name | `hl7v2-python` |
| Owner | `EffortlessMetrics` |
| Repository name | `hl7v2-rs` |
| Workflow filename | `python-testpypi.yml` |
| Environment name | `testpypi` |

In GitHub, create an environment named `testpypi`. Add reviewer protection if
you want a second human confirmation before upload.

## Local Wheel Proof

Run this before the manual TestPyPI workflow:

```powershell
python -m pip install --upgrade pip "maturin==1.13.1"
$env:PYO3_USE_ABI3_FORWARD_COMPATIBILITY = "1"
maturin build --release --out dist
python -m pip install --force-reinstall (Get-ChildItem dist\*.whl | Select-Object -First 1).FullName
python tests\python_smoke\smoke.py
```

Expected result:

```text
hl7v2-python smoke ok version=<version> segments=2
```

## Manual TestPyPI Proof

Run the **Python TestPyPI Proof** workflow manually.

First run with:

```text
publish_to_testpypi = false
```

This builds the wheel, installs it into a fresh virtual environment, runs the
Python smoke test, and uploads the wheel as a short-retention artifact. It does
not publish.

The 2026-05-09 run of this non-publishing mode passed on `main`; see
[`docs/audits/python-testpypi-nonpublish-proof-2026-05-09.md`](../audits/python-testpypi-nonpublish-proof-2026-05-09.md).

After the local wheel proof and non-publishing workflow pass, rerun with:

```text
publish_to_testpypi = true
```

This does three things:

1. Builds and smoke-tests the wheel.
2. Publishes the wheel to TestPyPI using Trusted Publishing.
3. Installs `hl7v2-python==<workspace version>` back from TestPyPI in a fresh
   virtual environment and reruns `tests/python_smoke/smoke.py`.

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
  `tests/python_smoke/smoke.py` successfully.

This is still not a production PyPI release. Treat it as packaging evidence for
the separate Python lane.

Current status: the non-publishing proof is complete for current `main`; the
TestPyPI upload/install-back mode remains intentionally unrun until a separate
distribution decision confirms the trusted-publisher setup and version.
