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

raw = (
    "MSH|^~\\&|SEND|FAC|RECV|FAC|202605080101||ADT^A01|CTRL1|P|2.5\r"
    "PID|1||123456^^^HOSP^MR||Doe^John||19700101|M"
)
profile_yaml = """
message_structure: ADT_A01
version: "2.5.1"
segments:
  - id: MSH
  - id: PID
constraints:
  - path: PID.3
    required: true
"""

print(hl7v2.__version__)

message = hl7v2.parse(raw)
print(message.segment_count())
print(message.to_json())

print(hl7v2.to_json(raw))
print(hl7v2.normalize(raw))

report = hl7v2.validate(raw, profile_yaml)
print(report.valid)
print(report.message_type)
print(report.to_dict())

summary = hl7v2.corpus_summary("feeds/site-a")
fingerprint = hl7v2.corpus_fingerprint("feeds/site-a", profile_yaml=profile_yaml)
diff = hl7v2.corpus_diff(
    "feeds/before",
    "feeds/after",
    profile_yaml=profile_yaml,
)
print(summary["message_count"])
print(fingerprint["fingerprint_version"])
print(diff["diff_version"])

policy_toml = """
[[rules]]
path = "PID.3"
action = "hash"
reason = "Patient identifier"

[[rules]]
path = "PID.5"
action = "drop"
reason = "Patient name"

[[rules]]
path = "PID.7"
action = "drop"
reason = "Date of birth"
"""
redaction = hl7v2.redact(raw, policy_toml)
print(redaction["redacted_hl7"])
print(redaction["receipt"]["phi_removed"])
```

The current Python surface intentionally starts with the minimum evidence loop:
parse, JSON export, normalize, validate, corpus summary/fingerprint/diff, and
safe-analysis redaction. Bundle and replay APIs should be added in focused
follow-up PRs.
