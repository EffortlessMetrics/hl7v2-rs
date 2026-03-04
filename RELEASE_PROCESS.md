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
- [ ] `IMPLEMENTATION_STATUS.md` is updated.
- [ ] `README.md` and crate-specific READMEs are up to date.
- [ ] MSRV is verified.

## Release Steps

### 1. Update Version

Update the `version` field in the root `Cargo.toml`. Since we use workspace inheritance, this will update all crates.

```toml
[workspace.package]
version = "1.2.0"
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
git tag -a v1.2.0 -m "Release v1.2.0"
git push origin v1.2.0
```

### 5. Publish to Crates.io

We publish crates in order of their dependencies. Use the following order:

1.  **Microcrates**: `hl7v2-model`, `hl7v2-escape`, `hl7v2-mllp`, `hl7v2-parser`, `hl7v2-writer`, `hl7v2-json`, `hl7v2-normalize`, `hl7v2-datetime`, `hl7v2-datatype`, `hl7v2-path`, `hl7v2-query`, `hl7v2-batch`.
2.  **Feature Crates**: `hl7v2-network`, `hl7v2-stream`, `hl7v2-validation`, `hl7v2-prof`, `hl7v2-ack`, `hl7v2-faker`, `hl7v2-template`, `hl7v2-template-values`, `hl7v2-corpus`.
3.  **Facade**: `hl7v2-core`.
4.  **Applications**: `hl7v2-cli`, `hl7v2-server`.

Note: `hl7v2-bench`, `hl7v2-test-utils`, and `hl7v2-e2e-tests` are set to `publish = false`.

```bash
# Example for publishing a crate
cd crates/hl7v2-model
cargo publish
```

### 6. Create GitHub Release

Create a release on GitHub based on the tag, copying the relevant entries from `CHANGELOG.md`. Attach the CLI binaries for major platforms.

## Post-Release

- [ ] Update the `ROADMAP.md` if necessary.
- [ ] Announce the release in relevant channels.
- [ ] Start the next development cycle by adding a new `[Unreleased]` section to `CHANGELOG.md`.
