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

Use `xtask` to derive the publish order from the workspace dependency graph so the sequence stays correct as crates are added or dependencies change.

```bash
# Preview the publish order
cargo run -p xtask -- publish-plan

# Publish the full sequence
cargo run -p xtask -- publish --yes

# Resume from a specific crate if crates.io index propagation interrupted the run
cargo run -p xtask -- publish --yes --from hl7v2-template-values
```

The publish sequence excludes non-published workspace members such as `hl7v2-python`, `hl7v2-bench`, `hl7v2-test-utils`, `hl7v2-e2e-tests`, `xtask`, and the root `hl7v2-examples` package.

For GitHub Actions based releases, use the manual `Publish to crates.io` workflow. It prints the derived order first and only publishes when `execute=true` is selected and the `CARGO_REGISTRY_TOKEN` secret is configured.

### 6. Create GitHub Release

Create a release on GitHub based on the tag, copying the relevant entries from `CHANGELOG.md`. Attach the CLI binaries for major platforms.

## Post-Release

- [ ] Update the `ROADMAP.md` if necessary.
- [ ] Announce the release in relevant channels.
- [ ] Start the next development cycle by adding a new `[Unreleased]` section to `CHANGELOG.md`.
