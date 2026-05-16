# Python Local Wheel Proof

Date: 2026-05-15
Branch: `docs/python-local-wheel-proof-1.5.0`
Scope: local wheel build, fresh virtualenv install, import smoke, and evidence
workflow proof for the public Python package identity `hl7v2`.

This receipt proves the current source tree can build and locally install the
Python `hl7v2` wheel for CPython 3.14 on Windows. It does not prove TestPyPI or
production PyPI publication.

## Package Identity

| Field | Value |
| --- | --- |
| Python distribution | `hl7v2` |
| Python import module | `hl7v2` |
| Rust backend crate | `hl7v2-python` |
| Workspace/package version | `1.5.0` |
| Built wheel | `hl7v2-1.5.0-cp314-cp314-win_amd64.whl` |
| Disposable build root | `F:\cargo-target\hl7v2-rs-python-wheel-proof` |

## Environment

| Tool | Observed value |
| --- | --- |
| Python | `Python 3.14.3` |
| pip | `pip 26.1.1` |
| Rust | `rustc 1.95.0 (59807616e 2026-04-14)` |
| Cargo | `cargo 1.95.0 (f2d3ce0bd 2026-03-21)` |
| maturin | `1.13.1` |

`PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1` was set because the local Python is
newer than the PyO3 stable CPython target table.

## Commands

```powershell
$root = "F:\cargo-target\hl7v2-rs-python-wheel-proof"
$dist = Join-Path $root "dist"
$venv = Join-Path $root "venv"
$env:RUSTUP_TOOLCHAIN = "1.95.0"
$env:PYO3_USE_ABI3_FORWARD_COMPATIBILITY = "1"
$env:CARGO_TARGET_DIR = $root
$env:CARGO_INCREMENTAL = "0"
python -m pip install --upgrade pip "maturin==1.13.1"
python -m maturin build --release --out $dist --target-dir $root
python -m venv $venv
F:\cargo-target\hl7v2-rs-python-wheel-proof\venv\Scripts\python.exe -m pip install --upgrade pip
F:\cargo-target\hl7v2-rs-python-wheel-proof\venv\Scripts\python.exe -m pip install --force-reinstall F:\cargo-target\hl7v2-rs-python-wheel-proof\dist\hl7v2-1.5.0-cp314-cp314-win_amd64.whl
F:\cargo-target\hl7v2-rs-python-wheel-proof\venv\Scripts\python.exe -c "import hl7v2; print(hl7v2.__version__)"
F:\cargo-target\hl7v2-rs-python-wheel-proof\venv\Scripts\python.exe tests\python_smoke\smoke.py
F:\cargo-target\hl7v2-rs-python-wheel-proof\venv\Scripts\python.exe tests\python_smoke\evidence_workflow_guide.py
```

## Observed Results

```text
Built wheel for CPython 3.14 to F:\cargo-target\hl7v2-rs-python-wheel-proof\dist\hl7v2-1.5.0-cp314-cp314-win_amd64.whl
Successfully installed hl7v2-1.5.0
installed 1.5.0
hl7v2 smoke ok version=1.5.0 segments=2
python evidence workflow guide ok version=1.5.0
```

A public registry spot-check in this PR still returned `404` for both:

```text
https://test.pypi.org/pypi/hl7v2/json
https://pypi.org/pypi/hl7v2/json
```

Repository rails also passed:

```text
cargo +1.95.0 run -p xtask -- check-doc-links
cargo +1.95.0 run -p xtask -- check-file-policy
cargo +1.95.0 run -p xtask -- check-python-publish-policy
cargo +1.95.0 run -p xtask -- badges --check
cargo +1.95.0 run -p xtask -- impacted-evidence --check
git diff --check
```

## Non-Claims

- This receipt does not upload to TestPyPI.
- This receipt does not upload to production PyPI.
- This receipt does not install back from TestPyPI or production PyPI.
- This receipt does not use token fallback.
- This receipt does not use `skip-existing`.
- This receipt does not create or publish an npm package.
- This receipt does not make `hl7v2-python` the recommended Rust API.

## Remaining Gap

Issue #563 remains the external blocker for TestPyPI Trusted Publisher setup for
project `hl7v2`. The Python lane can claim TestPyPI success only after the
manual workflow uploads `hl7v2==1.5.0` to TestPyPI, installs it back from
`https://test.pypi.org/simple/`, imports `hl7v2`, and runs both Python smoke
scripts successfully.
