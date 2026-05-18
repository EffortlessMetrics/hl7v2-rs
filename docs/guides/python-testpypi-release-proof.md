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
| crates.io publish policy | `hl7v2-python` is publishable only as a governed binding backend; backend publication requires a separate release receipt and does not make it the recommended Rust API |
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
cargo +1.95.0 run -p xtask -- python-local-wheel-proof
```

Expected result:

```text
hl7v2 smoke ok version=<version> segments=2
```

The command creates a scratch virtual environment, builds the `hl7v2` wheel with
maturin, installs that wheel, imports `hl7v2`, and runs
`tests/python_smoke/smoke.py`, `tests/python_smoke/evidence_workflow_guide.py`,
and `tests/python_smoke/dirty_evidence_workflow.py`.
It is a local wheel proof only; TestPyPI success still requires upload and
install-back from TestPyPI.

After TestPyPI upload succeeds, the install-back smoke can also be reproduced
from a source checkout with:

```powershell
cargo +1.95.0 run -p xtask -- python-public-registry-proof --index testpypi --version <workspace version>
```

That command creates a scratch virtual environment, installs
`hl7v2==<workspace version>` from `https://test.pypi.org/simple/` with
`--no-deps --force-reinstall`, imports `hl7v2`, and runs the same three Python
smoke/evidence scripts. It is a TestPyPI install-back proof only after the
package is actually visible on TestPyPI; it is not a production PyPI claim.

## Manual TestPyPI Proof

Run the **Python TestPyPI Proof** workflow manually.

First run with:

```text
publish_to_testpypi = false
```

This builds the wheel, installs it into a fresh virtual environment, runs the
Python smoke test, evidence workflow guide, and dirty evidence workflow smoke,
and uploads the wheel as a short-retention artifact. It does not publish.

The current hosted non-publishing proof passed on `main` after the v1.5.0
release and public `hl7v2` package retarget; see
[`docs/audits/python-testpypi-nonpublish-proof-2026-05-15.md`](../audits/python-testpypi-nonpublish-proof-2026-05-15.md).

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
   virtual environment and reruns `tests/python_smoke/smoke.py`,
   `tests/python_smoke/evidence_workflow_guide.py`, and
   `tests/python_smoke/dirty_evidence_workflow.py`.

The hosted install-back job runs the same proof boundary as the local
reproduction command:

```bash
cargo run -p xtask -- python-public-registry-proof --index testpypi --version "${PACKAGE_VERSION}"
```

This keeps the workflow proof and local reproduction path aligned. A passing
install-back job proves only the TestPyPI package for the selected version; it
does not claim production PyPI availability.

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
  `tests/python_smoke/smoke.py`,
  `tests/python_smoke/evidence_workflow_guide.py`, and
  `tests/python_smoke/dirty_evidence_workflow.py` successfully.
- Optional local reproduction with
  `cargo +1.95.0 run -p xtask -- python-public-registry-proof --index testpypi --version <workspace version>`
  also installs from TestPyPI and runs the same smoke scripts.

This is still not a production PyPI release. Treat it as packaging evidence for
the separate Python lane. A crates.io binding-backend publish, if later
approved, does not replace TestPyPI upload and install-back proof.

After the upload/install-back proof passes, use
[Python PyPI Release](python-pypi-release.md) for the guarded production PyPI
release path.

Current status: the non-publishing proof is complete on current `main` for
public package `hl7v2`. A 2026-05-10 publishing-mode run from `main` built and
smoke-tested the wheel, then failed during Trusted Publishing token exchange
with `invalid-publisher`; see
[docs/audits/python-testpypi-publish-attempt-2026-05-10.md](../audits/python-testpypi-publish-attempt-2026-05-10.md).
A 2026-05-17 publishing-mode run from current `main` commit
`764647e79ab61cd9814d07a777cbf1eed27a5ee8` again built and smoke-tested the
wheel successfully, then failed at the same Trusted Publishing exchange boundary
with `repo:EffortlessMetrics/hl7v2-rs:environment:testpypi`; see
[docs/audits/python-testpypi-publish-attempt-2026-05-17.md](../audits/python-testpypi-publish-attempt-2026-05-17.md).
The TestPyPI upload/install-back proof remains incomplete until the TestPyPI
Trusted Publisher is configured for project `hl7v2` and a rerun passes. Track
the external setup blocker in
[#563](https://github.com/EffortlessMetrics/hl7v2-rs/issues/563).
