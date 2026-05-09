"""Import and parse smoke test for the hl7v2 Python binding."""

from __future__ import annotations

import json
import sys
import tempfile
from pathlib import Path

import hl7v2


def main() -> int:
    raw = (
        "MSH|^~\\&|SENDAPP|SENDFAC|RECVAPP|RECVFAC|202605080101||ADT^A01|CTRL123|P|2.5\r"
        "PID|1||123456^^^HOSP^MR||Doe^John||19700101|M"
    )

    version = getattr(hl7v2, "__version__", "")
    if not isinstance(version, str) or not version:
        print("hl7v2.__version__ is missing", file=sys.stderr)
        return 1

    message = hl7v2.PyMessage.parse(raw)
    segment_count = message.segment_count()
    if segment_count != 2:
        print(f"expected 2 segments, got {segment_count}", file=sys.stderr)
        return 1

    payload = json.loads(message.to_json())
    if not isinstance(payload, dict):
        print("message JSON did not decode to an object", file=sys.stderr)
        return 1

    top_level_message = hl7v2.parse(raw)
    if top_level_message.segment_count() != 2:
        print("top-level parse did not return a two-segment message", file=sys.stderr)
        return 1

    top_level_payload = json.loads(hl7v2.to_json(raw))
    if not isinstance(top_level_payload, dict):
        print("top-level to_json did not decode to an object", file=sys.stderr)
        return 1

    normalized = hl7v2.normalize(raw)
    if "MSH|^~\\&" not in normalized or "PID|" not in normalized:
        print("normalize did not return expected HL7 content", file=sys.stderr)
        return 1

    profile_yaml = """
message_structure: ADT_A01
version: "2.5.1"
segments:
  - id: MSH
  - id: PID
constraints:
  - path: MSH.9
    required: true
  - path: PID.3
    required: true
"""
    report = hl7v2.validate(raw, profile_yaml)
    if not report.valid:
        print("expected validation report to be valid", file=sys.stderr)
        return 1
    if report.message_type != "ADT^A01":
        print(f"unexpected message type: {report.message_type}", file=sys.stderr)
        return 1
    report_dict = report.to_dict()
    if report_dict["valid"] is not True or report_dict["issue_count"] != 0:
        print("validation report dict did not match expected shape", file=sys.stderr)
        return 1
    if report.profile != "ADT_A01" or report_dict["profile"] != "ADT_A01":
        print("validation report did not preserve profile identity", file=sys.stderr)
        return 1
    report_json = json.loads(report.to_json())
    if report_json["message_type"] != "ADT^A01":
        print("validation report JSON did not preserve message_type", file=sys.stderr)
        return 1

    failing_profile_yaml = """
message_structure: ADT_A01
version: "2.5.1"
segments:
  - id: MSH
  - id: PID
constraints:
  - path: PID.13
    required: true
"""
    failing_report = hl7v2.validate(raw, failing_profile_yaml)
    if failing_report.valid:
        print("expected missing PID.13 validation to fail", file=sys.stderr)
        return 1
    failing_dict = failing_report.to_dict()
    issue = failing_dict["issues"][0]
    if issue["code"] != "missing_required_field" or issue["path"] != "PID.13":
        print(f"unexpected validation issue: {issue}", file=sys.stderr)
        return 1

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        before = root / "before"
        after = root / "after"
        before.mkdir()
        after.mkdir()

        before_message = raw
        after_adt = raw.replace("CTRL123", "CTRL124")
        after_oru = (
            "MSH|^~\\&|LAB|FAC|EHR|FAC|202605080102||ORU^R01|CTRL125|P|2.5\r"
            "PID|1||987654^^^HOSP^MR||Roe^Jane||19800101|F\r"
            "OBR|1||ORDER1|718-7^Hemoglobin^LN\r"
            "OBX|1|NM|718-7^Hemoglobin^LN||13.2|g/dL"
        )
        (before / "before.hl7").write_text(before_message, encoding="utf-8")
        (after / "after-adt.hl7").write_text(after_adt, encoding="utf-8")
        (after / "after-oru.hl7").write_text(after_oru, encoding="utf-8")

        summary = hl7v2.corpus_summary(str(before))
        if summary["message_count"] != 1 or summary["parse_error_count"] != 0:
            print(f"unexpected corpus summary: {summary}", file=sys.stderr)
            return 1
        if summary["message_types"][0] != {"value": "ADT^A01", "count": 1}:
            print(f"unexpected corpus message type counts: {summary}", file=sys.stderr)
            return 1

        fingerprint = hl7v2.corpus_fingerprint(
            str(before),
            profile_yaml=failing_profile_yaml,
        )
        if fingerprint["fingerprint_version"] != "1":
            print(f"unexpected fingerprint version: {fingerprint}", file=sys.stderr)
            return 1
        if fingerprint["profile"]["path"] != "<inline-profile>":
            print(f"unexpected fingerprint profile path: {fingerprint}", file=sys.stderr)
            return 1
        if len(fingerprint["profile"]["sha256"]) != 64:
            print(f"unexpected profile hash: {fingerprint}", file=sys.stderr)
            return 1
        issue_counts = {
            item["value"]: item["count"]
            for item in fingerprint["validation_issue_code_counts"]
        }
        if issue_counts.get("missing_required_field") != 1:
            print(f"unexpected fingerprint issue counts: {fingerprint}", file=sys.stderr)
            return 1

        diff = hl7v2.corpus_diff(
            str(before),
            str(after),
            profile_yaml=failing_profile_yaml,
        )
        if diff["diff_version"] != "1" or diff["message_count"]["delta"] != 1:
            print(f"unexpected corpus diff totals: {diff}", file=sys.stderr)
            return 1
        if "ORU^R01" not in diff["new_message_types"]:
            print(f"diff did not report new ORU message type: {diff}", file=sys.stderr)
            return 1
        diff_issue_counts = {
            item["value"]: item
            for item in diff["validation_issue_code_counts"]
        }
        missing_required = diff_issue_counts.get("missing_required_field")
        if (
            missing_required is None
            or missing_required["before"] != 1
            or missing_required["after"] != 2
            or missing_required["delta"] != 1
        ):
            print(f"unexpected diff issue counts: {diff}", file=sys.stderr)
            return 1

    try:
        hl7v2.parse("not an hl7 message")
    except ValueError as exc:
        if "Parse error" not in str(exc):
            print(f"parse error did not include stable context: {exc}", file=sys.stderr)
            return 1
    else:
        print("expected invalid HL7 parse to raise ValueError", file=sys.stderr)
        return 1

    print(f"hl7v2-python smoke ok version={version} segments={segment_count}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
