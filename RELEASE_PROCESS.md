# Release Process

This document describes the process for releasing a new version of `hl7v2-rs`.

## Versioning Strategy

We use [Semantic Versioning (SemVer)](https://semver.org/).
- **Major**: Breaking API changes.
- **Minor**: New features, backwards compatible.
- **Patch**: Bug fixes, backwards compatible.

Since this is a workspace with many crates, all crates share the same version number for simplicity and compatibility.

## Pre-Release Checklist

Before releasing, ensure:
- [ ] All tests pass (`cargo test --workspace`).
- [ ] All benchmarks pass (`cargo bench --workspace`).
- [ ] Clippy is clean (`cargo clippy --workspace --all-targets -- -D warnings`).
- [ ] Formatting is correct (`cargo fmt --all -- --check`).
- [ ] `CHANGELOG.md` is updated with the new version and changes.
- [ ] `docs/STATUS.md` is updated.
- [ ] `README.md` and crate-specific READMEs are up to date.
- [ ] MSRV is verified.

## Release Steps

### 1. Update Version

Update the `version` field in the root `Cargo.toml`. Since we use workspace inheritance, this will update all crates.

```toml
[workspace.package]
version = "<version>"
```

### 2. Update Changelog

Ensure the `[Unreleased]` section in `CHANGELOG.md` is renamed to the new version number and the date is set.

### 3. Verify Build

Run a clean build of the entire workspace.

```bash
cargo clean
cargo build --workspace --release
```

### 4. Tag the Release

Create a git tag for the new version.

```bash
git tag -a v<version> -m "Release v<version>"
git push origin v<version>
```

### 5. Publish to Crates.io

Use `xtask` to derive the primary Rust product publish order from the workspace dependency graph so the sequence stays correct as crates are added or dependencies change.

```bash
# Preview the publish order
cargo run -p xtask -- publish-plan

# Preview explicit package surfaces
cargo run -p xtask -- publish-plan --surface primary
cargo run -p xtask -- publish-plan --surface bindings
cargo run -p xtask -- publish-plan --surface all-publishable

# Publish the full sequence
cargo run -p xtask -- publish --yes

# Resume from a specific crate if crates.io index propagation interrupted the run
cargo run -p xtask -- publish --yes --from hl7v2-server
```

The default publish sequence is the primary Rust product graph: `hl7v2`, then
`hl7v2-server`, then `hl7v2-cli`. It excludes binding backend crates such as
`hl7v2-python` unless a separate binding-backend release decision deliberately
selects that surface. It also excludes internal/dev workspace members such as
`hl7v2-bench`, `hl7v2-test-utils`, `hl7v2-e2e-tests`, `xtask`, and the root
`hl7v2-examples` package. Historical old microcrate package names are not
published again unless a deliberate deprecation-only compatibility release is
approved.

Binding backend crates such as `hl7v2-python`, future `hl7v2-wasm`, and future
`hl7v2-node` are a separate package surface. They may publish only through a
binding-backend release PR with explicit metadata, dry-run proof, and language
install/import smoke receipts. They are not the recommended Rust API.
Use `cargo run -p xtask -- publish-plan --surface bindings` to inspect the
binding backend graph without changing the default primary Rust release plan.

For GitHub Actions based releases, use the manual `Publish to crates.io` workflow. It prints the derived order first and only publishes when `execute=true` is selected and the `CARGO_REGISTRY_TOKEN` secret is configured.

### Python TestPyPI Proof

`hl7v2-python` is not part of the primary Rust product graph. Prove the Python
package separately before any PyPI release:

```powershell
python -m pip install --upgrade pip "maturin==1.13.1"
$env:PYO3_USE_ABI3_FORWARD_COMPATIBILITY = "1"
maturin build --release --out dist
python -m pip install --force-reinstall (Get-ChildItem dist\*.whl | Select-Object -First 1).FullName
python tests\python_smoke\smoke.py
python tests\python_smoke\evidence_workflow_guide.py
```

Then use the manual `Python TestPyPI Proof` workflow. Run it first with
`publish_to_testpypi=false`; if that passes and the TestPyPI Trusted Publisher
is configured, rerun with `publish_to_testpypi=true` to upload and install the
same version back from TestPyPI.

The workflow uses Trusted Publishing from the `testpypi` GitHub environment and
does not require a repository PyPI token. See
[`docs/guides/python-testpypi-release-proof.md`](docs/guides/python-testpypi-release-proof.md).

After the TestPyPI upload/install-back proof passes for the same source commit,
use the manual `Python PyPI Release Proof` workflow for production PyPI. Run it
first with `publish_to_pypi=false`. Production publishing mode requires
`publish_to_pypi=true`, a successful same-commit `Python TestPyPI Proof` run
URL, the `pypi` GitHub environment, and Trusted Publishing. It does not use a
repository PyPI token and must not use `skip-existing`. See
[`docs/guides/python-pypi-release.md`](docs/guides/python-pypi-release.md).

### 6. Create GitHub Release

Create a release on GitHub based on the tag, copying the relevant entries from `CHANGELOG.md`. Attach the CLI binaries for major platforms.

## Post-Release

- [ ] Update the `ROADMAP.md` if necessary.
- [ ] Announce the release in relevant channels.
- [ ] Start the next development cycle by adding a new `[Unreleased]` section to `CHANGELOG.md`.
