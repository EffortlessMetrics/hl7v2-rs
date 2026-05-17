"""Dirty real-world evidence workflow smoke for the local hl7v2 wheel.

This verifies that the Python binding can validate, redact, bundle, and replay
the shared dirty Z-segment fixture without exposing synthetic PHI markers.
"""

from __future__ import annotations

import json
import sys
import tempfile
from pathlib import Path

import hl7v2


DIRTY_ADT_PROFILE = """
message_structure: ADT_A01
version: "2.5"
segments:
  - id: MSH
  - id: PID
  - id: ZPV
constraints:
  - path: MSH.9
    required: true
  - path: PID.3
    required: true
"""

DIRTY_SAFE_ANALYSIS_POLICY = """
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
reason = "message type is needed for analysis"

[[rules]]
path = "MSH.10"
action = "retain"
reason = "control id is needed for replay correlation"

[[rules]]
path = "ZPV.1"
action = "retain"
reason = "synthetic room marker is useful for dirty-corpus analysis"

[[rules]]
path = "ZPV.2"
action = "retain"
reason = "synthetic dirty-corpus note is useful for support triage"
"""

DIRTY_PHI_SENTINELS = ("MRN-Z", "Example^Zed", "19700101")
EXPECTED_SCHEMA_VERSION = "2"
V2_REPORT_SCHEMA_VERSION = 2
PYTHON_TOOL_NAME = "hl7v2-python"


def normalized_fixture_text(path: Path) -> str:
    """Read a dirty fixture with canonical HL7 segment separators."""
    return (
        path.read_text(encoding="utf-8")
        .replace("\r\n", "\n")
        .replace("\n", "\r")
        .rstrip("\r")
    )


def assert_no_dirty_phi(context: str, content: str, *paths: Path) -> None:
    """Fail if a report exposes synthetic dirty-fixture PHI or local paths."""
    for sentinel in DIRTY_PHI_SENTINELS:
        if sentinel in content:
            raise AssertionError(f"{context} leaked dirty fixture sentinel {sentinel!r}")
    for path in paths:
        path_text = str(path)
        if path_text in content:
            raise AssertionError(f"{context} leaked local path {path_text!r}")


def main() -> int:
    repo_root = Path(__file__).resolve().parents[2]
    fixture = repo_root / "test_data" / "dirty-real-world" / "after" / "z-segment.hl7"
    raw = normalized_fixture_text(fixture)

    version = getattr(hl7v2, "__version__", "")
    if not isinstance(version, str) or not version:
        print("hl7v2.__version__ is missing", file=sys.stderr)
        return 1

    validation = hl7v2.validate(raw, DIRTY_ADT_PROFILE)
    validation_dict = validation.to_dict(V2_REPORT_SCHEMA_VERSION)
    if (
        validation.valid is not True
        or validation.message_type != "ADT^A01"
        or validation_dict["schema_version"] != EXPECTED_SCHEMA_VERSION
        or validation_dict["tool_name"] != PYTHON_TOOL_NAME
        or validation_dict["issue_count"] != 0
    ):
        print(f"unexpected dirty validation report: {validation_dict}", file=sys.stderr)
        return 1
    try:
        assert_no_dirty_phi("dirty Python validation report", json.dumps(validation_dict))
    except AssertionError as exc:
        print(str(exc), file=sys.stderr)
        return 1

    redaction = hl7v2.redact(
        raw,
        DIRTY_SAFE_ANALYSIS_POLICY,
        schema_version=V2_REPORT_SCHEMA_VERSION,
    )
    redacted_hl7 = redaction["redacted_hl7"]
    if (
        redaction["schema_version"] != EXPECTED_SCHEMA_VERSION
        or redaction["tool_name"] != PYTHON_TOOL_NAME
        or redaction["message_type"] != "ADT^A01"
        or redaction["receipt"]["phi_removed"] is not True
        or "hash:sha256:" not in redacted_hl7
        or "ZPV|legacy-room|dirty interface note" not in redacted_hl7
    ):
        print(f"unexpected dirty redaction output: {redaction}", file=sys.stderr)
        return 1
    try:
        assert_no_dirty_phi("dirty Python redaction output", json.dumps(redaction))
    except AssertionError as exc:
        print(str(exc), file=sys.stderr)
        return 1

    with tempfile.TemporaryDirectory() as tmp:
        tmp_root = Path(tmp)
        bundle_dir = tmp_root / "dirty-python-support-bundle"
        bundle = hl7v2.bundle(
            raw,
            DIRTY_ADT_PROFILE,
            DIRTY_SAFE_ANALYSIS_POLICY,
            str(bundle_dir),
            schema_version=V2_REPORT_SCHEMA_VERSION,
        )
        if (
            bundle["schema_version"] != EXPECTED_SCHEMA_VERSION
            or bundle["tool_name"] != PYTHON_TOOL_NAME
            or bundle["message_type"] != "ADT^A01"
            or bundle["validation_valid"] is not True
            or bundle["redaction_phi_removed"] is not True
        ):
            print(f"unexpected dirty bundle summary: {bundle}", file=sys.stderr)
            return 1

        bundle_artifacts = [
            "message.redacted.hl7",
            "validation-report.json",
            "redaction-receipt.json",
            "field-paths.json",
            "manifest.json",
            "environment.json",
            "README.md",
            "replay.sh",
            "replay.ps1",
        ]
        for artifact in bundle_artifacts:
            if artifact not in bundle["artifacts"] or not (bundle_dir / artifact).is_file():
                print(f"dirty bundle missing artifact {artifact}: {bundle}", file=sys.stderr)
                return 1

        bundled_redacted = (bundle_dir / "message.redacted.hl7").read_text(encoding="utf-8")
        if "hash:sha256:" not in bundled_redacted:
            print("dirty bundle redacted HL7 did not include hash marker", file=sys.stderr)
            return 1
        if "ZPV|legacy-room|dirty interface note" not in bundled_redacted:
            print("dirty bundle redacted HL7 lost retained ZPV evidence", file=sys.stderr)
            return 1

        replay = hl7v2.replay(str(bundle_dir), schema_version=V2_REPORT_SCHEMA_VERSION)
        if (
            replay["schema_version"] != EXPECTED_SCHEMA_VERSION
            or replay["tool_name"] != PYTHON_TOOL_NAME
            or replay["message_type"] != "ADT^A01"
            or replay["validation_valid"] is not True
            or replay["reproduced"] is not True
        ):
            print(f"unexpected dirty replay report: {replay}", file=sys.stderr)
            return 1
        replay_checks = {check["name"]: check["status"] for check in replay["checks"]}
        if replay_checks.get("manifest-hashes") != "pass":
            print(f"dirty replay did not verify manifest hashes: {replay}", file=sys.stderr)
            return 1

        evidence_text = "\n".join(
            [
                json.dumps(bundle, sort_keys=True),
                bundled_redacted,
                json.dumps(replay, sort_keys=True),
                *(
                    (bundle_dir / artifact).read_text(encoding="utf-8")
                    for artifact in bundle_artifacts
                    if artifact != "message.redacted.hl7"
                ),
            ]
        )
        try:
            assert_no_dirty_phi("dirty Python bundle/replay evidence", evidence_text, tmp_root)
        except AssertionError as exc:
            print(str(exc), file=sys.stderr)
            return 1

    print(f"python dirty evidence workflow ok version={version}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
