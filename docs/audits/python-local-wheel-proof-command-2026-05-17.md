# Python Local Wheel Proof Command - 2026-05-17

Scope: record the self-contained `xtask python-local-wheel-proof` command that
builds the public Python `hl7v2` wheel from the Rust/PyO3 backend, installs it
into a scratch virtual environment, imports `hl7v2`, and runs the Python smoke
and evidence workflow guide scripts.

This is a local wheel proof only. It does not claim TestPyPI or PyPI upload,
install-back, or registry availability.

## Command

```powershell
$env:CARGO_TARGET_DIR="F:\cargo-target\hl7v2-rs-xtask-python-local-wheel-proof-cargo"
$env:CARGO_INCREMENTAL="0"
cargo +1.95.0 run -p xtask -- python-local-wheel-proof --root F:\cargo-target\hl7v2-rs-python-local-wheel-proof-2026-05-17
```

## Result

| Step | Result |
| --- | --- |
| Create scratch virtual environment | Pass |
| Install `maturin==1.13.1` | Pass |
| Build local `hl7v2` wheel | Pass |
| Install local wheel into scratch virtualenv | Pass |
| `import hl7v2` | Pass; reported version `1.5.0` |
| `tests/python_smoke/smoke.py` | Pass; `hl7v2 smoke ok version=1.5.0 segments=2` |
| `tests/python_smoke/evidence_workflow_guide.py` | Pass; `python evidence workflow guide ok version=1.5.0` |

Built wheel:

```text
hl7v2-1.5.0-cp314-cp314-win_amd64.whl
```

## Python-Included Parity Acceptance

The scratch virtual environment was then placed first on `PATH` and the aggregate
parity gate was run with Python included:

```powershell
$env:CARGO_TARGET_DIR="F:\cargo-target\hl7v2-rs-xtask-python-local-wheel-proof-cargo"
$env:CARGO_INCREMENTAL="0"
$env:PATH="F:\cargo-target\hl7v2-rs-python-local-wheel-proof-2026-05-17\venv\Scripts;$env:PATH"
cargo +1.95.0 run -p xtask -- check-evidence-parity-acceptance --include-python
```

Result: pass. The gate ran the Rust, CLI, REST, gRPC, schema-version,
dirty-corpus, bundle/replay, safe-error, PHI-sentinel, and local Python smoke
checks with the locally installed `hl7v2` wheel.

## Non-Claims

- No TestPyPI upload occurred.
- No TestPyPI install-back occurred.
- No production PyPI upload occurred.
- No production PyPI install-back occurred.
- No token fallback was used.
- No `skip-existing` workaround was used.
- This does not close issue
  [#563](https://github.com/EffortlessMetrics/hl7v2-rs/issues/563).
