# hl7v2-python

Python bindings for the Rust `hl7v2` toolkit.

This package is intentionally outside the crates.io Rust publish graph. Build
and validate it through the Python/maturin lane before any PyPI or TestPyPI
release.

## Build

```bash
python -m pip install "maturin==1.13.1"
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 maturin build --release --out dist
python -m pip install dist/*.whl
python tests/python_smoke/smoke.py
```

On PowerShell:

```powershell
python -m pip install "maturin==1.13.1"
$env:PYO3_USE_ABI3_FORWARD_COMPATIBILITY = "1"
maturin build --release --out dist
python -m pip install (Get-ChildItem dist\*.whl | Select-Object -First 1).FullName
python tests\python_smoke\smoke.py
```

## Current API

```python
import hl7v2

message = hl7v2.PyMessage.parse(
    "MSH|^~\\&|SEND|FAC|RECV|FAC|202605080101||ADT^A01|CTRL1|P|2.5\r"
    "PID|1||123456^^^HOSP^MR||Doe^John||19700101|M"
)

print(hl7v2.__version__)
print(message.segment_count())
print(message.to_json())
```

The first stable binding proof is build/install/import plus parse and JSON
smoke coverage. Broader Python APIs should be added in focused follow-up PRs.
