# hl7v2-python

PyO3 extension crate backing the public Python `hl7v2` package.

This crate backs the Python `hl7v2` package. Rust users should depend on
`hl7v2`; Python users should install `hl7v2` from PyPI after the Python release
lane is proven.

This is language-binding infrastructure, not the recommended Rust API. The
crate metadata is publishable so release tooling can prove backend packaging
provenance. `hl7v2-python` v1.5.0 is published to crates.io as binding backend
infrastructure; that crates.io receipt is not a TestPyPI or PyPI release for
the public Python package. Build and validate it through the Python/maturin lane
before any PyPI or TestPyPI release.

The Python distribution name is `hl7v2`; the import module is also `hl7v2`.
The Rust/PyO3 backend crate remains `hl7v2-python`.

## Build

```powershell
cargo +1.95.0 run -p xtask -- python-local-wheel-proof
```

The proof command creates a scratch virtual environment, installs
`maturin==1.13.1`, builds the `hl7v2` wheel, installs it, imports `hl7v2`, and
runs the Python smoke and evidence workflow scripts. It does not claim TestPyPI
or PyPI availability.

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
print(hl7v2.ack(raw))
print(hl7v2.ack(raw, code="AE"))

template_yaml = """
name: ADT_A01_Template
delims: "^~\\\\&"
segments:
  - "MSH|^~\\\\&|TestSystem|TestFacility|ReceivingSystem|ReceivingFacility|20250101000000||ADT^A01^ADT_A01|MSG00001|P|2.5.1"
  - "PID|1||123456^^^HOSP^MR||Doe^John^A||19800101|M"
values: {}
"""
print(hl7v2.generate(template_yaml, seed=1337, count=2))

lint = hl7v2.profile_lint(profile_yaml)
lint_v2 = hl7v2.profile_lint(profile_yaml, schema_version=2)
explain = hl7v2.profile_explain(
    profile_yaml,
    profile_name="profiles/adt_a01.yaml",
)
explain_v2 = hl7v2.profile_explain(
    profile_yaml,
    profile_name="profiles/adt_a01.yaml",
    schema_version=2,
)
profile_test = hl7v2.profile_test(
    profile_yaml,
    "fixtures/adt_a01",
    profile_name="profiles/adt_a01.yaml",
)
profile_test_v2 = hl7v2.profile_test(
    profile_yaml,
    "fixtures/adt_a01",
    profile_name="profiles/adt_a01.yaml",
    schema_version=2,
)
print(lint["valid"])
print(lint_v2["schema_version"])
print(explain["summary"]["segment_count"])
print(explain_v2["schema_version"])
print(profile_test["valid"])
print(profile_test_v2["schema_version"])

report = hl7v2.validate(raw, profile_yaml)
print(report.valid)
print(report.message_type)
print(report.to_dict())
print(report.to_dict(2))  # opt-in validation report v2 with provenance
print(report.to_json(2))

summary = hl7v2.corpus_summary("feeds/site-a")
summary_v2 = hl7v2.corpus_summary("feeds/site-a", schema_version=2)
fingerprint = hl7v2.corpus_fingerprint("feeds/site-a", profile_yaml=profile_yaml)
fingerprint_v2 = hl7v2.corpus_fingerprint(
    "feeds/site-a",
    profile_yaml=profile_yaml,
    schema_version=2,
)
diff = hl7v2.corpus_diff(
    "feeds/before",
    "feeds/after",
    profile_yaml=profile_yaml,
)
diff_v2 = hl7v2.corpus_diff(
    "feeds/before",
    "feeds/after",
    profile_yaml=profile_yaml,
    schema_version=2,
)
print(summary["message_count"])
print(summary_v2["schema_version"])
print(fingerprint["fingerprint_version"])
print(fingerprint_v2["schema_version"])
print(diff["diff_version"])
print(diff_v2["schema_version"])

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
redaction_v2 = hl7v2.redact(raw, policy_toml, schema_version=2)
print(redaction["redacted_hl7"])
print(redaction["receipt"]["phi_removed"])
print(redaction_v2["schema_version"])
print(redaction_v2["receipt"]["schema_version"])

bundle = hl7v2.bundle(raw, profile_yaml, policy_toml, "issue-bundle")
bundle_v2 = hl7v2.bundle(raw, profile_yaml, policy_toml, "issue-bundle-v2", schema_version=2)
print(bundle["artifacts"])
print(bundle_v2["schema_version"])
# issue-bundle-v2/manifest.json, field-paths.json, redaction-receipt.json,
# and environment.json also carry schema_version = "2".

replay = hl7v2.replay("issue-bundle")
replay_v2 = hl7v2.replay("issue-bundle", schema_version=2)
print(replay["reproduced"])
print(replay_v2["schema_version"])
```

The current Python surface intentionally starts with the minimum evidence loop:
parse, JSON export, normalize, ACK generation, synthetic template generation,
profile lint/test/explain reports, validate, corpus summary/fingerprint/diff,
safe-analysis redaction, evidence bundle creation, and replay verification.

## TestPyPI Proof

Use the manual **Python TestPyPI Proof** GitHub Actions workflow when proving an
external Python package upload. It defaults to a non-publishing wheel build and
smoke tests. With `publish_to_testpypi=true`, it publishes to TestPyPI through
Trusted Publishing and installs the same version back from TestPyPI before
running `tests/python_smoke/smoke.py` and
`tests/python_smoke/evidence_workflow_guide.py`.

Setup and stop conditions are documented in
[`docs/guides/python-testpypi-release-proof.md`](../../docs/guides/python-testpypi-release-proof.md).

## Production PyPI Release

Use the manual **Python PyPI Release Proof** workflow only after the full
TestPyPI upload/install-back proof has passed for the current workspace version.
It defaults to a non-publishing production rehearsal. With
`publish_to_pypi=true`, it publishes to production PyPI through Trusted
Publishing and installs the same version back from PyPI before rerunning the
Python smoke and evidence workflow guide.

Setup and stop conditions are documented in
[`docs/guides/python-pypi-release.md`](../../docs/guides/python-pypi-release.md).
