# Python Evidence Workflow

This guide shows the Python binding as an analyst and QA workflow over the same
evidence contracts used by the Rust crate, CLI, and server. It keeps the Python
lane focused on deterministic artifacts: generated fixtures, ACKs, profile
reports, validation reports, corpus summaries, corpus diffs, redaction
receipts, bundles, and replay reports.

The examples use synthetic messages. They are safe to run from a source checkout
after installing a locally built `hl7v2` wheel.

## Install From A Local Wheel

From the repository root:

```powershell
cargo +1.95.0 run -p xtask -- python-local-wheel-proof
```

The proof command builds a local wheel, installs it into a scratch virtual
environment, imports `hl7v2`, runs `tests/python_smoke/smoke.py`, and runs this
guide's evidence workflow script. It does not claim TestPyPI or PyPI
availability.

The Python distribution is `hl7v2`; the import module is `hl7v2`. If you want to
inspect the module interactively after the proof, activate the scratch virtual
environment under `target/hl7v2-python-local-wheel-proof/venv` or pass
`--root <scratch-dir>` to the proof command.

```python
import hl7v2

print(hl7v2.__version__)
```

## End-To-End Script

Create `target/hl7v2-python-evidence/workflow.py`:

```python
from __future__ import annotations

import json
import shutil
from pathlib import Path

import hl7v2


ROOT = Path("target/hl7v2-python-evidence")
shutil.rmtree(ROOT, ignore_errors=True)
(ROOT / "before").mkdir(parents=True)
(ROOT / "after").mkdir(parents=True)
(ROOT / "reports").mkdir()
(ROOT / "profile-fixtures" / "valid").mkdir(parents=True)
(ROOT / "profile-fixtures" / "invalid").mkdir(parents=True)

profile_yaml = """
message_structure: "GENERIC"
version: "2.5.1"
segments:
  - id: "MSH"
  - id: "PID"
constraints:
  - path: "PID.3"
    required: true
  - path: "PID.5"
    components:
      min: 2
valuesets:
  - path: "PID.8"
    name: "HL70001"
    codes:
      - "F"
      - "M"
      - "O"
      - "U"
      - "A"
      - "N"
"""

redaction_policy = """
[[rules]]
path = "PID.3"
action = "hash"
reason = "patient identifier"

[[rules]]
path = "PID.5"
action = "drop"
reason = "patient name"

[[rules]]
path = "PID.7"
action = "drop"
reason = "date of birth"

[[rules]]
path = "MSH.9"
action = "retain"
reason = "message type"

[[rules]]
path = "MSH.10"
action = "retain"
reason = "message control id"
"""

template_yaml = """
name: "ADT_A01_Template"
delims: "^~\\\\&"
segments:
  - "MSH|^~\\\\&|TestSystem|TestFacility|ReceivingSystem|ReceivingFacility|20250101000000||ADT^A01^ADT_A01|MSG00001|P|2.5.1"
  - "PID|1||123456^^^HOSP^MR||Doe^Generated^A||19800101|M"
values: {}
"""

before_message = (
    "MSH|^~\\&|SEND|FAC|RECV|FAC|202605090101||ADT^A01|CTRL100|P|2.5\r"
    "PID|1||MRN-100^^^HOSP^MR||Example^Valid||19700101|M"
)

after_message = (
    "MSH|^~\\&|SEND|FAC|RECV|FAC|202605090201||ADT^A01|CTRL200|P|2.5\r"
    "PID|1||MRN-200^^^HOSP^MR||Example^Invalid||19700101|X"
)

generated_messages = hl7v2.generate(template_yaml, seed=1337, count=2)
ack_message = hl7v2.ack(after_message, code="AE")

(ROOT / "before" / "site-a-001.hl7").write_text(before_message, encoding="utf-8")
(ROOT / "after" / "site-a-001.hl7").write_text(after_message, encoding="utf-8")
(ROOT / "profile-fixtures" / "valid" / "before.hl7").write_text(
    before_message,
    encoding="utf-8",
)
(ROOT / "profile-fixtures" / "invalid" / "after.hl7").write_text(
    after_message,
    encoding="utf-8",
)
(ROOT / "reports" / "generated-message-001.hl7").write_text(
    generated_messages[0],
    encoding="utf-8",
)
(ROOT / "reports" / "ack.hl7").write_text(ack_message, encoding="utf-8")

profile_lint_v2 = hl7v2.profile_lint(profile_yaml, schema_version=2)
profile_explain_v2 = hl7v2.profile_explain(
    profile_yaml,
    profile_name="profiles/generic.yaml",
    schema_version=2,
)
profile_test_v2 = hl7v2.profile_test(
    profile_yaml,
    str(ROOT / "profile-fixtures"),
    profile_name="profiles/generic.yaml",
    schema_version=2,
)

report = hl7v2.validate(after_message, profile_yaml)
report_v2 = report.to_dict(2)

summary_v2 = hl7v2.corpus_summary(str(ROOT / "after"), schema_version=2)
fingerprint_v2 = hl7v2.corpus_fingerprint(
    str(ROOT / "after"),
    profile_yaml=profile_yaml,
    schema_version=2,
)
diff_v2 = hl7v2.corpus_diff(
    str(ROOT / "before"),
    str(ROOT / "after"),
    profile_yaml=profile_yaml,
    schema_version=2,
)

redaction_v2 = hl7v2.redact(after_message, redaction_policy, schema_version=2)
bundle_v2 = hl7v2.bundle(
    after_message,
    profile_yaml,
    redaction_policy,
    str(ROOT / "issue-bundle"),
    schema_version=2,
)
replay_v2 = hl7v2.replay(str(ROOT / "issue-bundle"), schema_version=2)

artifacts = {
    "profile-lint-v2.json": profile_lint_v2,
    "profile-explain-v2.json": profile_explain_v2,
    "profile-test-v2.json": profile_test_v2,
    "validation-report-v2.json": report_v2,
    "corpus-summary-v2.json": summary_v2,
    "corpus-fingerprint-v2.json": fingerprint_v2,
    "corpus-diff-v2.json": diff_v2,
    "redaction-output-v2.json": redaction_v2,
    "bundle-summary-v2.json": bundle_v2,
    "replay-report-v2.json": replay_v2,
}

for name, artifact in artifacts.items():
    (ROOT / "reports" / name).write_text(
        json.dumps(artifact, indent=2, sort_keys=True),
        encoding="utf-8",
    )

print(
    json.dumps(
        {
            "version": hl7v2.__version__,
            "validation_valid": report_v2["valid"],
            "validation_issue_codes": [
                issue["code"] for issue in report_v2["issues"]
            ],
            "profile_lint_valid": profile_lint_v2["valid"],
            "profile_test_valid": profile_test_v2["valid"],
            "profile_explain_segments": profile_explain_v2["summary"]["segment_count"],
            "ack_msa": "MSA|AE|CTRL200" in ack_message,
            "after_message_count": summary_v2["message_count"],
            "diff_field_presence_deltas": len(diff_v2["field_presence"]),
            "generated_message_count": len(generated_messages),
            "redaction_phi_removed": redaction_v2["receipt"]["phi_removed"],
            "bundle_artifacts": len(bundle_v2["artifacts"]),
            "replay_reproduced": replay_v2["reproduced"],
        },
        indent=2,
        sort_keys=True,
    )
)
```

Run it:

```powershell
python target\hl7v2-python-evidence\workflow.py
```

The checked-in Python wheel workflows also execute this guide block directly:

```powershell
python tests\python_smoke\evidence_workflow_guide.py
```

Expected output has the same evidence semantics as the CLI and server:

```json
{
  "ack_msa": true,
  "after_message_count": 1,
  "bundle_artifacts": 10,
  "diff_field_presence_deltas": 0,
  "generated_message_count": 2,
  "profile_explain_segments": 2,
  "profile_lint_valid": true,
  "profile_test_valid": true,
  "redaction_phi_removed": true,
  "replay_reproduced": true,
  "validation_issue_codes": [
    "value_not_in_set"
  ],
  "validation_valid": false,
  "version": "1.5.0"
}
```

The exact `version` follows the installed wheel.

## Outputs

The script writes machine-readable artifacts under:

```text
target/hl7v2-python-evidence/reports/
```

The evidence bundle is written under:

```text
target/hl7v2-python-evidence/issue-bundle/
```

The bundle contains the redacted HL7 message, validation report, field-path
trace, redaction receipt, environment metadata, manifest, README, and replay
scripts. Replay verifies the manifest hashes before trusting bundle artifacts.

## Safety Notes

- Redaction receipts prove configured policy actions, not universal PHI absence.
- User-authored profile YAML is included in bundles as supplied.
- Bundle paths and replay reports should not expose local filesystem roots.
- Use `schema_version=2` when you need provenance fields such as
  `schema_version`, `tool_name`, and `tool_version`.
