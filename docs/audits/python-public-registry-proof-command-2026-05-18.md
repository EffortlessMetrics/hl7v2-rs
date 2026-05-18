# Python Public Registry Proof Command

Date: 2026-05-18

## Scope

This receipt records the repo-side command surface for reproducing public
Python package install-back proof after `hl7v2` is visible on TestPyPI or PyPI.

The command is:

```powershell
cargo +1.95.0 run -p xtask -- python-public-registry-proof --index testpypi --version <version>
cargo +1.95.0 run -p xtask -- python-public-registry-proof --index pypi --version <version>
```

It creates a scratch virtual environment, installs
`hl7v2==<version>` from the selected public package index with
`--no-deps --force-reinstall`, imports `hl7v2`, and runs:

```text
tests/python_smoke/smoke.py
tests/python_smoke/evidence_workflow_guide.py
tests/python_smoke/dirty_evidence_workflow.py
```

## Boundary

This command does not upload anything. It is not a replacement for the guarded
GitHub Actions publish workflows. It is a local reproduction path for the
install-back half of those workflows after a public package upload succeeds.

## Non-Claims

- No TestPyPI upload occurred.
- No TestPyPI install-back success is claimed by this receipt.
- No PyPI upload occurred.
- No PyPI install-back success is claimed by this receipt.
- No token fallback was added.
- No `skip-existing` behavior was added.
- No npm package was created.
- No new crates.io release, tag, or GitHub release occurred.

## Remaining Blocker

Issue [#563](https://github.com/EffortlessMetrics/hl7v2-rs/issues/563)
continues to track the external TestPyPI Trusted Publisher setup for public
project `hl7v2`. A real TestPyPI success receipt still requires upload,
install-back from TestPyPI, `import hl7v2`, and the smoke/evidence scripts.
