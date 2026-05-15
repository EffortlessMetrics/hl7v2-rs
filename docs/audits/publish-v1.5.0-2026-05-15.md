# v1.5.0 crates.io Publish Receipt

Date: 2026-05-15

This receipt records the crates.io upload, registry-resolution, tag, GitHub
release, and public install-back proof for v1.5.0.

## Release Identity

| Field | Value |
| --- | --- |
| Release | `v1.5.0` |
| Release theme | Rust 1.95 Quality Ratchet |
| Source commit | `04760587b83e2b4aaf410814b46ad1818c881371` |
| Rust toolchain | Rust `1.95.0` |
| Git tag | `v1.5.0` |
| GitHub release | <https://github.com/EffortlessMetrics/hl7v2-rs/releases/tag/v1.5.0> |
| GitHub release published at | `2026-05-15T19:11:54Z` |

## Published crates.io Graph

The selected v1.5.0 crates.io graph was published in dependency order:

1. `hl7v2`
2. `hl7v2-python`
3. `hl7v2-server`
4. `hl7v2-cli`

`hl7v2-python` is included only as binding backend infrastructure for the
public Python `hl7v2` package. It is not the recommended Rust API, and this
crates.io upload is not a TestPyPI or PyPI release.

## Publish Commands

```powershell
cargo +1.95.0 publish -p hl7v2
$env:PYO3_USE_ABI3_FORWARD_COMPATIBILITY = "1"
cargo +1.95.0 publish -p hl7v2-python
cargo +1.95.0 publish -p hl7v2-server
cargo +1.95.0 publish -p hl7v2-cli
```

Each publish command completed with crates.io reporting the corresponding
`v1.5.0` crate as published.

## Registry Resolution

Registry resolution was checked with:

```powershell
cargo +1.95.0 info hl7v2@1.5.0
cargo +1.95.0 info hl7v2-python@1.5.0
cargo +1.95.0 info hl7v2-server@1.5.0
cargo +1.95.0 info hl7v2-cli@1.5.0
```

Observed results:

| Crate | Registry result |
| --- | --- |
| `hl7v2@1.5.0` | Resolved from crates.io with `rust-version: 1.95`. |
| `hl7v2-python@1.5.0` | Resolved from crates.io as the PyO3 backend crate for the Python `hl7v2` package. |
| `hl7v2-server@1.5.0` | Resolved from crates.io. |
| `hl7v2-cli@1.5.0` | Resolved from crates.io. |

## Tag And GitHub Release

The release tag was created and pushed:

```powershell
git tag -a v1.5.0 -m "v1.5.0"
git push origin v1.5.0
git ls-remote --tags origin v1.5.0
```

The remote tag lookup returned:

```text
b0b8ace2be687ad326a68590be9b61d74470c063 refs/tags/v1.5.0
```

The GitHub release was created as:

```text
v1.5.0 - Rust 1.95 Quality Ratchet
```

and verified as not draft and not prerelease.

## Public Install-Back Smoke

The install-back smoke used a disposable directory under
`F:\cargo-target\hl7v2-rs-install-smoke`.

### Rust Library

```powershell
cargo +1.95.0 new rust-smoke --bin
cargo +1.95.0 add hl7v2@1.5.0
cargo +1.95.0 check
```

Result: a clean scratch project resolved `hl7v2@1.5.0` from crates.io and
compiled successfully.

### CLI

```powershell
cargo +1.95.0 install hl7v2-cli --version 1.5.0 --root <install-root>
hl7v2-cli doctor --format json
```

Result: `hl7v2-cli.exe` installed successfully. `doctor --format json`
reported CLI version `1.5.0`, sample parse success, and MLLP round-trip
success.

The local machine also had an ambient Python `hl7v2` module reporting `1.4.0`
when `doctor` checked optional Python presence. That is a local environment
observation only. It is not a TestPyPI, PyPI, or Python release claim.

### Server

```powershell
cargo +1.95.0 install hl7v2-server --version 1.5.0 --root <install-root>
hl7v2-server --print-config
$env:BIND_ADDRESS = "127.0.0.1:18080"
hl7v2-server
Invoke-WebRequest http://127.0.0.1:18080/ready
```

Result: `hl7v2-server.exe` installed successfully. `--print-config` succeeded.
The server started with `BIND_ADDRESS=127.0.0.1:18080`, and `/ready` returned
HTTP `200` with version `1.5.0` and passing checks for config, bind address,
configured profiles, bundle output root, quarantine output, and validation
report self-check.

## Validation Context

The final non-publishing pre-publish proof passed before upload at commit
`9fc95604d8950b565b6b6b7941ad275fd5624178`; see
[`publish-dry-run-v1.5.0-2026-05-15-final-prepublish.md`](publish-dry-run-v1.5.0-2026-05-15-final-prepublish.md).

After the pre-publish proof PR merged, the post-merge main checks observed
required CI success, Security success, and CI Policy success. The GitHub checks
watcher hit API rate limiting after reporting CI success; this receipt records
only the checks directly observed.

## Non-Claims

- This receipt does not claim a TestPyPI upload.
- This receipt does not claim a production PyPI upload.
- This receipt does not claim public Python `hl7v2` install-back from TestPyPI
  or PyPI.
- This receipt does not claim an npm package.
- This receipt does not make `hl7v2-python` the recommended Rust API.
- This receipt does not publish old implementation microcrate names.

## Rollback And Forward-Fix Posture

Do not yank by default. Prefer a forward-fix release unless a security, legal,
or severe packaging integrity issue requires yanking.
