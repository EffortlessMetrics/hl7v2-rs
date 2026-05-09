# v1.3.0 Publish Dry-Run Receipt

Date: 2026-05-09

This receipt records release-head package verification for the v1.3.0 Evidence
Loop release candidate. No crates were published by these commands.

## Version Line

The workspace package line and active workspace packages were prepared as
`1.3.0`.

Verified with:

```powershell
cargo +1.93.0 metadata --format-version 1 --no-deps
```

Observed public package versions:

| Package | Version |
| ------- | ------- |
| `hl7v2` | `1.3.0` |
| `hl7v2-server` | `1.3.0` |
| `hl7v2-cli` | `1.3.0` |

`hl7v2-python` is also versioned as `1.3.0` for the Python/maturin lane, but
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

## Rust Dry-Runs

Direct `hl7v2` dry-run:

```powershell
cargo +1.93.0 publish -p hl7v2 --dry-run
```

Result: pass.

```text
Packaging hl7v2 v1.3.0
Packaged 62 files, 881.9KiB (168.8KiB compressed)
Verifying hl7v2 v1.3.0
Uploading hl7v2 v1.3.0
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
| `hl7v2` | `1.3.0` | 881.9 KiB | 168.8 KiB |
| `hl7v2-server` | `1.3.0` | 428.8 KiB | 91.1 KiB |
| `hl7v2-cli` | `1.3.0` | 511.5 KiB | 92.2 KiB |

Direct dependent dry-runs before publishing `hl7v2` are expected to stop at the
crates.io index boundary:

```powershell
cargo +1.93.0 publish -p hl7v2-server --dry-run
cargo +1.93.0 publish -p hl7v2-cli --dry-run
```

Both reported that `hl7v2 = "^1.3.0"` could not be resolved because crates.io
currently exposes `hl7v2` `1.2.1` and does not yet contain `1.3.0`.

## Gate

```powershell
$env:PYO3_USE_ABI3_FORWARD_COMPATIBILITY='1'
cargo +1.93.0 run -p xtask -- gate --check
```

Result: pass.

The gate checked lint policy, no-panic-family policy, non-Rust file policy,
formatting, dependency graph warmup, clippy, and test compilation.

## Python Lane Proof

The Python binding remains outside the Rust crates.io publish graph. The wheel
proof was run with the build backend version required by `pyproject.toml`:

```powershell
python -m maturin --version
$env:PYO3_USE_ABI3_FORWARD_COMPATIBILITY='1'
python -m maturin build --release --out dist
python -m pip install --force-reinstall dist\hl7v2_python-1.3.0-cp314-cp314-win_amd64.whl
python tests/python_smoke/smoke.py
```

Result: pass.

```text
maturin 1.13.1
Built wheel for CPython 3.14 to dist\hl7v2_python-1.3.0-cp314-cp314-win_amd64.whl
Successfully installed hl7v2-python-1.3.0
hl7v2-python smoke ok version=1.3.0 segments=2
```

## Publish Boundary

These checks do not publish anything. Actual publish order remains:

1. `cargo +1.93.0 publish -p hl7v2`
2. wait for `hl7v2` `1.3.0` to appear in the crates.io index
3. `cargo +1.93.0 publish -p hl7v2-server`
4. `cargo +1.93.0 publish -p hl7v2-cli`

`hl7v2-python` should stay on the Python packaging lane unless a separate
TestPyPI/PyPI release decision is made.
