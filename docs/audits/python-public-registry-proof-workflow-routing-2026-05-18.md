# Python Public Registry Proof Workflow Routing

Date: 2026-05-18

## Scope

This receipt records the repo-side workflow routing change that makes future
TestPyPI and PyPI install-back jobs use the shared
`xtask python-public-registry-proof` command.

## Change

- `.github/workflows/python-testpypi.yml` still builds and smokes the local
  wheel before upload. When `publish_to_testpypi=true` and the upload succeeds,
  the install-back job now runs:

  ```bash
  cargo run -p xtask -- python-public-registry-proof --index testpypi --version "${PACKAGE_VERSION}"
  ```

- `.github/workflows/python-pypi.yml` still requires the same-commit successful
  TestPyPI proof before production upload. When `publish_to_pypi=true` and the
  upload succeeds, the install-back job now runs:

  ```bash
  cargo run -p xtask -- python-public-registry-proof --index pypi --version "${PACKAGE_VERSION}"
  ```

- `cargo run -p xtask -- check-python-publish-policy` now rejects publish
  workflows whose install-back jobs bypass the shared public-registry proof
  command or use the wrong package index.

## Boundary

This change does not publish anything. It does not configure TestPyPI or PyPI
Trusted Publishing. It does not claim TestPyPI or PyPI upload/install-back
success.

## Non-Claims

- No TestPyPI upload occurred.
- No TestPyPI install-back success is claimed.
- No PyPI upload occurred.
- No PyPI install-back success is claimed.
- No token fallback was added.
- No `skip-existing` behavior was added.
- No npm package was created.
- No new crates.io release, tag, or GitHub release occurred.
