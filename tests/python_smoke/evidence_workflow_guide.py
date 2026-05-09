"""Execute the Python evidence workflow guide's checked example.

This keeps docs/guides/python-evidence-workflow.md tied to the installed
hl7v2-python wheel instead of letting the long guide script drift from the
binding API.
"""

from __future__ import annotations

import contextlib
import io
import json
import re
import sys
from pathlib import Path


GUIDE_PATH = Path("docs/guides/python-evidence-workflow.md")
SCRIPT_MARKER = 'ROOT = Path("target/hl7v2-python-evidence")'


def extract_workflow_script() -> str:
    """Extract the guide's end-to-end Python workflow block."""
    guide = GUIDE_PATH.read_text(encoding="utf-8")
    for block in re.findall(r"```python\n(.*?)\n```", guide, flags=re.DOTALL):
        if SCRIPT_MARKER in block:
            return block
    raise RuntimeError(f"could not find Python workflow block in {GUIDE_PATH}")


def main() -> int:
    script = extract_workflow_script()
    stdout = io.StringIO()

    namespace = {
        "__builtins__": __builtins__,
        "__file__": str(GUIDE_PATH),
        "__name__": "__main__",
    }
    with contextlib.redirect_stdout(stdout):
        exec(compile(script, str(GUIDE_PATH), "exec"), namespace)

    raw_output = stdout.getvalue().strip()
    try:
        summary = json.loads(raw_output)
    except json.JSONDecodeError as error:
        print(f"guide workflow did not emit JSON: {raw_output}", file=sys.stderr)
        raise SystemExit(1) from error

    expected = {
        "after_message_count": 1,
        "bundle_artifacts": 10,
        "diff_field_presence_deltas": 0,
        "redaction_phi_removed": True,
        "replay_reproduced": True,
        "validation_issue_codes": ["value_not_in_set"],
        "validation_valid": False,
    }

    mismatches = {
        key: {"expected": value, "actual": summary.get(key)}
        for key, value in expected.items()
        if summary.get(key) != value
    }
    if mismatches:
        print(
            "guide workflow summary mismatches: "
            + json.dumps(mismatches, sort_keys=True),
            file=sys.stderr,
        )
        return 1

    version = summary.get("version")
    if not isinstance(version, str) or not version:
        print(f"guide workflow did not report an installed version: {summary}", file=sys.stderr)
        return 1

    reports_dir = Path("target/hl7v2-python-evidence/reports")
    for artifact in [
        "validation-report-v2.json",
        "corpus-summary-v2.json",
        "corpus-fingerprint-v2.json",
        "corpus-diff-v2.json",
        "redaction-output-v2.json",
        "bundle-summary-v2.json",
        "replay-report-v2.json",
    ]:
        if not (reports_dir / artifact).is_file():
            print(f"guide workflow did not write {artifact}", file=sys.stderr)
            return 1

    print(f"python evidence workflow guide ok version={version}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
