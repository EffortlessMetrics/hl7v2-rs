# v1.4.0 Publish Dry-Run Receipt

Date: 2026-05-09

This receipt records release-candidate package verification for the v1.4.0
Evidence Contracts and Server Sidecar line. No crates were published by these
commands.

## Version Line

The workspace package line and active workspace packages were prepared as
`1.4.0`.

Verified with:

```powershell
cargo +1.93.0 metadata --format-version 1 --no-deps
```

Observed public package versions:

| Package | Version |
| ------- | ------- |
| `hl7v2` | `1.4.0` |
| `hl7v2-server` | `1.4.0` |
| `hl7v2-cli` | `1.4.0` |

`hl7v2-python` is also versioned as `1.4.0` for the Python/maturin lane, but
it remains `publish = false` for crates.io.

## Publish Plan

```powershell
cargo +1.93.0 run -p xtask -- publish-plan
```

Result:

```text
crates.io publish order
 1. hl7v2
 2. hl7v2-server
 3. hl7v2-cli
```

## Contract And Gate Checks

```powershell
cargo +1.93.0 run -p xtask -- evidence-schema-check
npx @stoplight/spectral-cli lint api/openapi/hl7v2-api-v1.yaml --ruleset .spectral.yml
npx -y @bufbuild/buf lint api/proto
cargo +1.93.0 test -p hl7v2-server --test proto_packaging_test
$env:PYO3_USE_ABI3_FORWARD_COMPATIBILITY='1'
cargo +1.93.0 run -p xtask -- gate --check
```

Result: pass.

The evidence schema gate validated 33 checked-in evidence fixtures. The API
contract checks validated OpenAPI lint, proto lint, and packaged proto/OpenAPI
asset drift. The full gate checked lint policy, no-panic-family policy,
non-Rust file policy, formatting, dependency graph warmup, clippy, and test
compilation.

## Rust Dry-Runs

Direct `hl7v2` dry-run:

```powershell
cargo +1.93.0 publish -p hl7v2 --dry-run --locked
```

Result: pass.

```text
Packaging hl7v2 v1.4.0
Packaged 62 files, 912.7KiB (173.2KiB compressed)
Verifying hl7v2 v1.4.0
Uploading hl7v2 v1.4.0
warning: aborting upload due to dry run
```

Workspace-patched graph dry-run:

```powershell
$env:PYO3_USE_ABI3_FORWARD_COMPATIBILITY='1'
cargo +1.93.0 run -p xtask -- publish-dry-run --from hl7v2 --workspace-patches
```

Result: pass.

```text
Dry-running hl7v2...
Dry-running hl7v2-server...
Dry-running hl7v2-cli...
Publish dry-run checks passed!
```

Packaged artifacts:

| Package | Version | Package size | Compressed |
| ------- | ------- | ------------ | ---------- |
| `hl7v2` | `1.4.0` | 912.7 KiB | 173.2 KiB |
| `hl7v2-server` | `1.4.0` | 539.4 KiB | 105.9 KiB |
| `hl7v2-cli` | `1.4.0` | 574.2 KiB | 96.5 KiB |

Direct dependent dry-runs before publishing `hl7v2` are expected to stop at the
crates.io index boundary:

```powershell
cargo +1.93.0 publish -p hl7v2-server --dry-run --locked
cargo +1.93.0 publish -p hl7v2-cli --dry-run --locked
```

Both reported that `hl7v2 = "^1.4.0"` could not be resolved because crates.io
currently exposes `hl7v2` `1.3.0` and `1.2.1`, not `1.4.0`.

## Python Lane Proof

The Python binding remains outside the Rust crates.io publish graph. The wheel
proof was run with the build backend version required by `pyproject.toml`:

```powershell
python -m maturin --version
$env:PYO3_USE_ABI3_FORWARD_COMPATIBILITY='1'
python -m maturin build --release --out dist
python -m pip install --force-reinstall dist\hl7v2_python-1.4.0-cp314-cp314-win_amd64.whl
python tests/python_smoke/smoke.py
```

Result: pass.

```text
maturin 1.13.1
Built wheel for CPython 3.14 to dist\hl7v2_python-1.4.0-cp314-cp314-win_amd64.whl
Successfully installed hl7v2-python-1.4.0
hl7v2-python smoke ok version=1.4.0 segments=2
```

## Publish Boundary

These checks do not publish anything. Actual publish order remains:

1. `cargo +1.93.0 publish -p hl7v2`
2. wait for `hl7v2` `1.4.0` to appear in the crates.io index
3. `cargo +1.93.0 publish -p hl7v2-server`
4. `cargo +1.93.0 publish -p hl7v2-cli`

`hl7v2-python` should stay on the Python packaging lane unless a separate
TestPyPI/PyPI release decision is made.
