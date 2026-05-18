# Python Registry Proof Parity Boundary

Date: 2026-05-18

## Scope

This receipt records the evidence-parity policy update that makes the public
Python registry install-back proof boundary machine-readable while the actual
TestPyPI/PyPI upload remains blocked.

## Change

`policy/evidence-parity.toml` now records the blocked public-registry proof
commands for the Python `hl7v2` surface:

```bash
cargo run -p xtask -- python-public-registry-proof --index testpypi --version <version>
cargo run -p xtask -- python-public-registry-proof --index pypi --version <version>
```

`cargo run -p xtask -- check-evidence-parity` rejects the manifest if either
blocked proof command is removed. This keeps the local/TestPyPI/PyPI proof
boundary visible to agents without promoting local-wheel proof into a public
package claim.

## Boundary

This change does not run public registry install-back. The commands are
recorded as blocked proof until the `hl7v2` package is visible on TestPyPI or
PyPI and upload/install-back receipts exist.

## Non-Claims

- No TestPyPI upload occurred.
- No TestPyPI install-back success is claimed.
- No PyPI upload occurred.
- No PyPI install-back success is claimed.
- No token fallback was added.
- No `skip-existing` behavior was added.
- No npm package was created.
- No new crates.io release, tag, or GitHub release occurred.
