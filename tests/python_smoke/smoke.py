"""Import and parse smoke test for the hl7v2 Python binding."""

from __future__ import annotations

import json
import sys
import tempfile
from pathlib import Path

import hl7v2


SECURITY_FIXTURE = json.loads(
    (
        Path(__file__).resolve().parents[2]
        / "test_data"
        / "security"
        / "safe-error-phi-parity.json"
    ).read_text(encoding="utf-8")
)
SCHEMA_VERSION_FIXTURE = json.loads(
    (
        Path(__file__).resolve().parents[2]
        / "test_data"
        / "evidence"
        / "schema-version-parity.json"
    ).read_text(encoding="utf-8")
)

PHI_LEAK_SENTINEL_MESSAGE = SECURITY_FIXTURE["phi"]["message"]
PHI_LEAK_SENTINEL_POLICY = SECURITY_FIXTURE["phi"]["policy"]
RAW_INPUT_FILE_SENTINEL = SECURITY_FIXTURE["phi"]["raw_input_file"]
RAW_POLICY_FILE_SENTINEL = SECURITY_FIXTURE["phi"]["raw_policy_file"]
PHI_LEAK_SENTINELS = tuple(
    (f"manifest forbidden value {index}", value)
    for index, value in enumerate(SECURITY_FIXTURE["phi"]["forbidden"], start=1)
)
V2_REPORT_SCHEMA_VERSION = SCHEMA_VERSION_FIXTURE["v2_report_schema_version"]
EXPECTED_V2_SCHEMA_VERSION = SCHEMA_VERSION_FIXTURE["expected_v2_schema_version"]
UNSUPPORTED_REPORT_SCHEMA_VERSION = SCHEMA_VERSION_FIXTURE[
    "unsupported_report_schema_version"
]
UNSUPPORTED_SCHEMA_VERSION_FRAGMENT = SCHEMA_VERSION_FIXTURE[
    "unsupported_error_contains"
]
PYTHON_TOOL_NAME = SCHEMA_VERSION_FIXTURE["tool_names"]["python"]
VALIDATION_SCHEMA_FIXTURE = SCHEMA_VERSION_FIXTURE["validation"]


def assert_no_phi_leak_sentinels(context: str, content: str) -> None:
    for label, value in PHI_LEAK_SENTINELS:
        if value in content:
            raise AssertionError(f"{context} leaked {label}: {value}")


def assert_no_phi_leak_sentinels_or_paths(
    context: str,
    content: str,
    *paths: Path,
) -> None:
    assert_no_phi_leak_sentinels(context, content)
    for path in paths:
        path_text = str(path)
        if path_text in content:
            raise AssertionError(f"{context} leaked local path: {path_text}")
    for file_name in (RAW_INPUT_FILE_SENTINEL, RAW_POLICY_FILE_SENTINEL):
        if file_name in content:
            raise AssertionError(f"{context} leaked raw fixture file name: {file_name}")


def normalize_fixture_segments(contents: bytes) -> bytes:
    return (
        contents.decode("utf-8", errors="replace")
        .replace("\r\n", "\n")
        .replace("\n", "\r")
        .encode("utf-8")
    )


def materialize_dirty_corpus_dir(source: Path, target: Path) -> None:
    target.mkdir(parents=True, exist_ok=True)
    for path in sorted(source.iterdir()):
        if path.is_file():
            (target / path.name).write_bytes(normalize_fixture_segments(path.read_bytes()))


def add_generated_mllp_fixtures(source: Path, target: Path) -> None:
    normalized = normalize_fixture_segments(source.read_bytes())
    framed = b"\x0b" + normalized + b"\x1c\r"
    (target / "mllp-framed.hl7").write_bytes(framed)
    (target / "mllp-truncated.hl7").write_bytes(framed[:-1])


def has_count(entries: list[dict[str, object]], value: str, count: int) -> bool:
    return any(entry.get("value") == value and entry.get("count") == count for entry in entries)


def main() -> int:
    repo_root = Path(__file__).resolve().parents[2]

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

    custom_delimiter_raw = (repo_root / "test_data" / "custom_delimiters.hl7").read_text(
        encoding="utf-8"
    ).rstrip("\r\n")
    canonical_custom = hl7v2.normalize(custom_delimiter_raw, canonical_delims=True)
    if not canonical_custom.startswith("MSH|^~\\&|"):
        print(
            f"canonical normalization did not use standard delimiters: {canonical_custom}",
            file=sys.stderr,
        )
        return 1
    if "ADT^A01" not in canonical_custom or "ADT%A01" in canonical_custom:
        print(
            f"canonical normalization did not rewrite component delimiters: {canonical_custom}",
            file=sys.stderr,
        )
        return 1
    if "MSH*%$!?*" in canonical_custom:
        print(
            f"canonical normalization retained custom field delimiters: {canonical_custom}",
            file=sys.stderr,
        )
        return 1
    if hl7v2.normalize(canonical_custom, canonical_delims=True) != canonical_custom:
        print("canonical normalization was not idempotent", file=sys.stderr)
        return 1

    ack_message = hl7v2.ack(raw)
    if "MSA|AA|CTRL123" not in ack_message or "ACK" not in ack_message:
        print(
            f"default ACK did not preserve expected status/control id: {ack_message}",
            file=sys.stderr,
        )
        return 1
    for requested_code, expected_code in [
        ("ae", "AE"),
        ("AR", "AR"),
        ("CA", "CA"),
        ("CE", "CE"),
        ("CR", "CR"),
    ]:
        explicit_ack = hl7v2.ack(raw, code=requested_code)
        if f"MSA|{expected_code}|CTRL123" not in explicit_ack:
            print(
                f"explicit ACK code {requested_code} did not round-trip: {explicit_ack}",
                file=sys.stderr,
            )
            return 1
    try:
        hl7v2.ack(raw, code="ZZ")
    except ValueError as exc:
        if "ack code must be one of AA, AE, AR, CA, CE, CR" not in str(exc):
            print(f"unexpected ACK code failure: {exc}", file=sys.stderr)
            return 1
    else:
        print("expected unsupported ACK code to fail", file=sys.stderr)
        return 1

    template_yaml = """
name: "ADT_A01_Template"
delims: "^~\\\\&"
segments:
  - "MSH|^~\\\\&|TestSystem|TestFacility|ReceivingSystem|ReceivingFacility|20250101000000||ADT^A01^ADT_A01|MSG00001|P|2.5.1"
  - "PID|1||123456^^^HOSP^MR||Doe^John^A||19800101|M"
values: {}
"""
    generated = hl7v2.generate(template_yaml, seed=1337, count=2)
    if len(generated) != 2:
        print(f"expected two generated messages: {generated}", file=sys.stderr)
        return 1
    if not all("MSH|^~\\&" in message and "PID|" in message for message in generated):
        print(f"generated messages did not look like HL7: {generated}", file=sys.stderr)
        return 1
    try:
        hl7v2.generate("not: [valid", count=1)
    except ValueError as exc:
        if "Template parse error" not in str(exc):
            print(f"unexpected template parse failure: {exc}", file=sys.stderr)
            return 1
    else:
        print("expected invalid template YAML to fail", file=sys.stderr)
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
    profile_lint = hl7v2.profile_lint(profile_yaml)
    if profile_lint["valid"] is not True or profile_lint["issue_count"] != 0:
        print(f"unexpected profile lint report: {profile_lint}", file=sys.stderr)
        return 1
    invalid_profile_yaml = SECURITY_FIXTURE["invalid_profile"]["yaml"]
    invalid_profile_lint = hl7v2.profile_lint(invalid_profile_yaml)
    if invalid_profile_lint["valid"] is not False:
        print(
            f"expected invalid profile lint report to fail: {invalid_profile_lint}",
            file=sys.stderr,
        )
        return 1
    invalid_profile_text = json.dumps(invalid_profile_lint, sort_keys=True)
    for forbidden in SECURITY_FIXTURE["invalid_profile"]["forbidden"]:
        if forbidden in invalid_profile_text:
            print(
                f"Python profile lint safe error leaked manifest value: {forbidden}",
                file=sys.stderr,
            )
            return 1
    profile_lint_v2 = hl7v2.profile_lint(
        profile_yaml, schema_version=V2_REPORT_SCHEMA_VERSION
    )
    if (
        profile_lint_v2["schema_version"] != EXPECTED_V2_SCHEMA_VERSION
        or profile_lint_v2["tool_name"] != PYTHON_TOOL_NAME
        or profile_lint_v2["valid"] is not True
    ):
        print(f"unexpected profile lint v2 report: {profile_lint_v2}", file=sys.stderr)
        return 1
    try:
        hl7v2.profile_lint(
            profile_yaml, schema_version=UNSUPPORTED_REPORT_SCHEMA_VERSION
        )
    except ValueError as exc:
        if UNSUPPORTED_SCHEMA_VERSION_FRAGMENT not in str(exc):
            print(f"unexpected profile lint schema failure: {exc}", file=sys.stderr)
            return 1
    else:
        print("expected unsupported profile lint schema version to fail", file=sys.stderr)
        return 1

    profile_explain = hl7v2.profile_explain(
        profile_yaml,
        profile_name="profiles/adt_a01.yaml",
    )
    if (
        profile_explain["profile"] != "profiles/adt_a01.yaml"
        or profile_explain["message_structure"] != "ADT_A01"
        or profile_explain["summary"]["segment_count"] != 2
        or len(profile_explain["profile_sha256"]) != 64
    ):
        print(f"unexpected profile explain report: {profile_explain}", file=sys.stderr)
        return 1
    profile_explain_v2 = hl7v2.profile_explain(
        profile_yaml,
        profile_name="profiles/adt_a01.yaml",
        schema_version=V2_REPORT_SCHEMA_VERSION,
    )
    if (
        profile_explain_v2["schema_version"] != EXPECTED_V2_SCHEMA_VERSION
        or profile_explain_v2["tool_name"] != PYTHON_TOOL_NAME
        or profile_explain_v2["profile"] != "profiles/adt_a01.yaml"
    ):
        print(f"unexpected profile explain v2 report: {profile_explain_v2}", file=sys.stderr)
        return 1
    try:
        hl7v2.profile_explain(
            profile_yaml, schema_version=UNSUPPORTED_REPORT_SCHEMA_VERSION
        )
    except ValueError as exc:
        if UNSUPPORTED_SCHEMA_VERSION_FRAGMENT not in str(exc):
            print(f"unexpected profile explain schema failure: {exc}", file=sys.stderr)
            return 1
    else:
        print("expected unsupported profile explain schema version to fail", file=sys.stderr)
        return 1

    with tempfile.TemporaryDirectory() as tmp:
        fixture_root = Path(tmp) / "profile-fixtures"
        (fixture_root / "valid").mkdir(parents=True)
        (fixture_root / "invalid").mkdir(parents=True)
        (fixture_root / "expected").mkdir(parents=True)
        (fixture_root / "valid" / "adt.hl7").write_text(raw, encoding="utf-8")
        (fixture_root / "invalid" / "missing_pid3.hl7").write_text(
            (
                "MSH|^~\\&|SENDAPP|SENDFAC|RECVAPP|RECVFAC|202605080101||ADT^A01|CTRL999|P|2.5\r"
                "PID|1||||Doe^John||19700101|M"
            ),
            encoding="utf-8",
        )
        (fixture_root / "expected" / "missing_pid3.report.json").write_text(
            json.dumps(
                {
                    "valid": False,
                    "issues": [
                        {
                            "code": "missing_required_field",
                            "severity": "error",
                            "path": "PID.3",
                        }
                    ],
                }
            ),
            encoding="utf-8",
        )

        profile_test = hl7v2.profile_test(
            profile_yaml,
            str(fixture_root),
            profile_name="profiles/adt_a01.yaml",
        )
        if (
            profile_test["valid"] is not True
            or profile_test["case_count"] != 2
            or profile_test["passed_count"] != 2
        ):
            print(f"unexpected profile test report: {profile_test}", file=sys.stderr)
            return 1
        invalid_cases = [
            case
            for case in profile_test["cases"]
            if case["expectation"] == "invalid"
        ]
        if not invalid_cases or not invalid_cases[0]["expected_report"]["matched"]:
            print(f"profile test did not match expected report: {profile_test}", file=sys.stderr)
            return 1
        profile_test_json = json.dumps(profile_test, sort_keys=True)
        fixture_root_text = str(fixture_root)
        if fixture_root_text in profile_test_json:
            print("profile test report leaked local fixture root", file=sys.stderr)
            return 1
        if "profiles/adt_a01.yaml" in profile_test_json:
            print("profile test report leaked caller profile path", file=sys.stderr)
            return 1
        if "expected/missing_pid3.report.json" not in profile_test_json:
            print(
                f"profile test report did not keep relative expected report path: {profile_test}",
                file=sys.stderr,
            )
            return 1

        profile_test_v2 = hl7v2.profile_test(
            profile_yaml,
            str(fixture_root),
            profile_name="profiles/adt_a01.yaml",
            schema_version=V2_REPORT_SCHEMA_VERSION,
        )
        if (
            profile_test_v2["schema_version"] != EXPECTED_V2_SCHEMA_VERSION
            or profile_test_v2["tool_name"] != PYTHON_TOOL_NAME
            or profile_test_v2["valid"] is not True
        ):
            print(f"unexpected profile test v2 report: {profile_test_v2}", file=sys.stderr)
            return 1
        profile_test_v2_json = json.dumps(profile_test_v2, sort_keys=True)
        if fixture_root_text in profile_test_v2_json:
            print("profile test v2 report leaked local fixture root", file=sys.stderr)
            return 1
        if "profiles/adt_a01.yaml" in profile_test_v2_json:
            print("profile test v2 report leaked caller profile path", file=sys.stderr)
            return 1
        try:
            hl7v2.profile_test(
                profile_yaml,
                str(fixture_root),
                schema_version=UNSUPPORTED_REPORT_SCHEMA_VERSION,
            )
        except ValueError as exc:
            if UNSUPPORTED_SCHEMA_VERSION_FRAGMENT not in str(exc):
                print(f"unexpected profile test schema failure: {exc}", file=sys.stderr)
                return 1
        else:
            print("expected unsupported profile test schema version to fail", file=sys.stderr)
            return 1

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
    report_v2 = report.to_dict(V2_REPORT_SCHEMA_VERSION)
    if (
        report_v2["schema_version"] != EXPECTED_V2_SCHEMA_VERSION
        or report_v2["tool_name"] != PYTHON_TOOL_NAME
        or report_v2["profile_identity"]["label"] != "<inline-profile>"
        or report_v2["profile_identity"]["message_structure"]
        != VALIDATION_SCHEMA_FIXTURE["profile_label"]
        or len(report_v2["profile_identity"]["sha256"]) != 64
    ):
        print(f"validation report v2 did not preserve provenance: {report_v2}", file=sys.stderr)
        return 1
    report_v2_json = json.loads(report.to_json(V2_REPORT_SCHEMA_VERSION))
    if report_v2_json["schema_version"] != EXPECTED_V2_SCHEMA_VERSION:
        print("validation report v2 JSON did not preserve schema_version", file=sys.stderr)
        return 1
    try:
        report.to_dict(UNSUPPORTED_REPORT_SCHEMA_VERSION)
    except ValueError as exc:
        if UNSUPPORTED_SCHEMA_VERSION_FRAGMENT not in str(exc):
            print(f"unexpected schema version failure: {exc}", file=sys.stderr)
            return 1
    else:
        print("expected unsupported report schema version to fail", file=sys.stderr)
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
        summary_v2 = hl7v2.corpus_summary(
            str(before), schema_version=V2_REPORT_SCHEMA_VERSION
        )
        if (
            summary_v2["schema_version"] != EXPECTED_V2_SCHEMA_VERSION
            or summary_v2["tool_name"] != PYTHON_TOOL_NAME
            or summary_v2["tool_version"] != hl7v2.__version__
            or summary_v2["message_count"] != 1
        ):
            print(f"unexpected summary v2 provenance: {summary_v2}", file=sys.stderr)
            return 1
        try:
            hl7v2.corpus_summary(
                str(before), schema_version=UNSUPPORTED_REPORT_SCHEMA_VERSION
            )
        except ValueError as exc:
            if UNSUPPORTED_SCHEMA_VERSION_FRAGMENT not in str(exc):
                print(f"unexpected summary schema version failure: {exc}", file=sys.stderr)
                return 1
        else:
            print("expected unsupported summary schema version to fail", file=sys.stderr)
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
        fingerprint_v2 = hl7v2.corpus_fingerprint(
            str(before),
            profile_yaml=failing_profile_yaml,
            schema_version=V2_REPORT_SCHEMA_VERSION,
        )
        if (
            fingerprint_v2["schema_version"] != EXPECTED_V2_SCHEMA_VERSION
            or fingerprint_v2["tool_name"] != PYTHON_TOOL_NAME
            or fingerprint_v2["fingerprint_version"] != "1"
            or fingerprint_v2["profile"]["path"] != "<inline-profile>"
        ):
            print(f"unexpected fingerprint v2 provenance: {fingerprint_v2}", file=sys.stderr)
            return 1
        try:
            hl7v2.corpus_fingerprint(
                str(before), schema_version=UNSUPPORTED_REPORT_SCHEMA_VERSION
            )
        except ValueError as exc:
            if UNSUPPORTED_SCHEMA_VERSION_FRAGMENT not in str(exc):
                print(f"unexpected fingerprint schema version failure: {exc}", file=sys.stderr)
                return 1
        else:
            print("expected unsupported fingerprint schema version to fail", file=sys.stderr)
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
        diff_v2 = hl7v2.corpus_diff(
            str(before),
            str(after),
            profile_yaml=failing_profile_yaml,
            schema_version=V2_REPORT_SCHEMA_VERSION,
        )
        if (
            diff_v2["schema_version"] != EXPECTED_V2_SCHEMA_VERSION
            or diff_v2["tool_name"] != PYTHON_TOOL_NAME
            or diff_v2["diff_version"] != "1"
            or diff_v2["message_count"]["delta"] != 1
        ):
            print(f"unexpected diff v2 provenance: {diff_v2}", file=sys.stderr)
            return 1
        try:
            hl7v2.corpus_diff(
                str(before), str(after), schema_version=UNSUPPORTED_REPORT_SCHEMA_VERSION
            )
        except ValueError as exc:
            if UNSUPPORTED_SCHEMA_VERSION_FRAGMENT not in str(exc):
                print(f"unexpected diff schema version failure: {exc}", file=sys.stderr)
                return 1
        else:
            print("expected unsupported diff schema version to fail", file=sys.stderr)
            return 1

    dirty_fixture_root = repo_root / "test_data" / "dirty-real-world"
    with tempfile.TemporaryDirectory() as tmp:
        dirty_root = Path(tmp)
        dirty_before = dirty_root / "before"
        dirty_after = dirty_root / "after"
        materialize_dirty_corpus_dir(dirty_fixture_root / "before", dirty_before)
        materialize_dirty_corpus_dir(dirty_fixture_root / "after", dirty_after)
        add_generated_mllp_fixtures(
            dirty_fixture_root / "sources" / "mllp-source.hl7",
            dirty_after,
        )

        dirty_summary = hl7v2.corpus_summary(
            str(dirty_after), schema_version=V2_REPORT_SCHEMA_VERSION
        )
        if (
            dirty_summary["schema_version"] != EXPECTED_V2_SCHEMA_VERSION
            or dirty_summary["tool_name"] != PYTHON_TOOL_NAME
            or dirty_summary["message_count"] != 8
            or dirty_summary["file_count"] != 11
            or dirty_summary["parse_error_count"] != 3
        ):
            print(
                "dirty corpus summary did not preserve expected aggregate counts",
                file=sys.stderr,
            )
            return 1
        if not (
            has_count(dirty_summary["message_types"], "ADT^A01", 2)
            and has_count(dirty_summary["message_types"], "ADT^A08", 1)
            and has_count(dirty_summary["message_types"], "ADT^A04", 1)
            and has_count(dirty_summary["message_types"], "ADT^A03", 1)
            and has_count(dirty_summary["message_types"], "ADT^A31", 1)
            and has_count(dirty_summary["message_types"], "ORU^R01", 2)
            and has_count(dirty_summary["segments"], "ZPV", 1)
            and has_count(dirty_summary["segments"], "ZSB", 1)
            and has_count(dirty_summary["segments"], "OBX", 22)
            and has_count(dirty_summary["segments"], "NTE", 1)
        ):
            print("dirty corpus summary did not preserve expected shape counts", file=sys.stderr)
            return 1
        parse_error_paths = {item["path"] for item in dirty_summary["parse_errors"]}
        if parse_error_paths != {
            "malformed-delimiters.hl7",
            "mllp-truncated.hl7",
            "partial-batch.hl7",
        }:
            print(
                f"unexpected dirty corpus parse-error paths: {parse_error_paths}",
                file=sys.stderr,
            )
            return 1

        dirty_fingerprint = hl7v2.corpus_fingerprint(
            str(dirty_after), schema_version=V2_REPORT_SCHEMA_VERSION
        )
        if (
            dirty_fingerprint["schema_version"] != EXPECTED_V2_SCHEMA_VERSION
            or dirty_fingerprint["tool_name"] != PYTHON_TOOL_NAME
            or dirty_fingerprint["fingerprint_version"] != "1"
            or dirty_fingerprint["message_count"] != 8
            or dirty_fingerprint["file_count"] != 11
            or dirty_fingerprint["parse_error_count"] != 3
        ):
            print(
                "dirty corpus fingerprint did not preserve expected aggregate counts",
                file=sys.stderr,
            )
            return 1
        if not any(
            field["path"] == "OBX.5"
            and field["max_per_message"] == 20
            and field["total_occurrences"] == 22
            for field in dirty_fingerprint["field_cardinality"]
        ):
            print(
                "dirty corpus fingerprint did not preserve OBX.5 cardinality",
                file=sys.stderr,
            )
            return 1
        if not any(
            field["path"] == "ZPV.1" and field["total_occurrences"] == 1
            for field in dirty_fingerprint["field_cardinality"]
        ):
            print(
                "dirty corpus fingerprint did not preserve ZPV.1 cardinality",
                file=sys.stderr,
            )
            return 1
        if not any(
            field["path"] == "MSH.3" and field["total_occurrences"] == 8
            for field in dirty_fingerprint["field_cardinality"]
        ):
            print(
                "dirty corpus fingerprint did not preserve MSH.3 cardinality",
                file=sys.stderr,
            )
            return 1
        if not any(
            field["path"] == "ZSB.1" and field["total_occurrences"] == 1
            for field in dirty_fingerprint["field_cardinality"]
        ):
            print(
                "dirty corpus fingerprint did not preserve ZSB.1 cardinality",
                file=sys.stderr,
            )
            return 1
        if not any(
            shape["path"] == "OBX.5"
            and shape["null_count"] == 1
            and shape["text_count"] >= 1
            for shape in dirty_fingerprint["value_shape_stats"]
        ):
            print(
                "dirty corpus fingerprint did not preserve OBX.5 null/text shapes",
                file=sys.stderr,
            )
            return 1
        if not any(
            shape["path"] == "PID.7" and shape["numeric_count"] >= 1
            for shape in dirty_fingerprint["value_shape_stats"]
        ):
            print(
                "dirty corpus fingerprint did not preserve legacy PID.7 timestamp shape",
                file=sys.stderr,
            )
            return 1
        if not any(
            shape["path"] == "PID.3"
            and shape["text_count"] >= 1
            for shape in dirty_fingerprint["value_shape_stats"]
        ):
            print(
                "dirty corpus fingerprint did not preserve redacted PID.3 hash shape",
                file=sys.stderr,
            )
            return 1

        dirty_diff = hl7v2.corpus_diff(
            str(dirty_before),
            str(dirty_after),
            schema_version=V2_REPORT_SCHEMA_VERSION,
        )
        if (
            dirty_diff["schema_version"] != EXPECTED_V2_SCHEMA_VERSION
            or dirty_diff["tool_name"] != PYTHON_TOOL_NAME
            or dirty_diff["diff_version"] != "1"
            or dirty_diff["file_count"]["before"] != 2
            or dirty_diff["file_count"]["after"] != 11
            or dirty_diff["file_count"]["delta"] != 9
            or dirty_diff["message_count"]["delta"] != 6
            or dirty_diff["parse_error_count"]["delta"] != 3
        ):
            print(
                "dirty corpus diff did not preserve expected aggregate deltas",
                file=sys.stderr,
            )
            return 1
        if not any(
            field["path"] == "OBX.5"
            and field["max_per_message_delta"] == 15
            and field["total_occurrences_delta"] == 17
            for field in dirty_diff["field_cardinality"]
        ):
            print(
                "dirty corpus diff did not preserve OBX.5 cardinality delta",
                file=sys.stderr,
            )
            return 1
        if not any(
            shape["path"] == "OBX.5"
            and shape["null_count"]["delta"] == 1
            and shape["text_count"]["delta"] >= 1
            for shape in dirty_diff["value_shape_stats"]
        ):
            print(
                "dirty corpus diff did not preserve OBX.5 null/text shape deltas",
                file=sys.stderr,
            )
            return 1

        dirty_evidence_text = "\n".join(
            json.dumps(report, sort_keys=True)
            for report in [dirty_summary, dirty_fingerprint, dirty_diff]
        )
        if "MRN-DIRTY" in dirty_evidence_text:
            print("dirty corpus evidence leaked the synthetic MRN marker", file=sys.stderr)
            return 1

    phi_raw = PHI_LEAK_SENTINEL_MESSAGE
    redaction_policy = PHI_LEAK_SENTINEL_POLICY
    redaction = hl7v2.redact(phi_raw, redaction_policy)
    if redaction["message_type"] != "ADT^A01":
        print(f"unexpected redaction message type: {redaction}", file=sys.stderr)
        return 1
    if len(redaction["input_sha256"]) != 64 or len(redaction["policy_sha256"]) != 64:
        print(f"redaction hashes are not SHA-256 digests: {redaction}", file=sys.stderr)
        return 1
    redacted_hl7 = redaction["redacted_hl7"]
    receipt = redaction["receipt"]
    if receipt["hash_algorithm"] != "sha256" or receipt["phi_removed"] is not True:
        print(f"unexpected redaction receipt: {receipt}", file=sys.stderr)
        return 1
    if "hash:sha256:" not in redacted_hl7:
        print(f"redacted HL7 did not include hash marker: {redacted_hl7}", file=sys.stderr)
        return 1
    try:
        assert_no_phi_leak_sentinels(
            "Python redacted HL7",
            redacted_hl7,
        )
        assert_no_phi_leak_sentinels(
            "Python redaction receipt",
            json.dumps(receipt, sort_keys=True),
        )
    except AssertionError as exc:
        print(str(exc), file=sys.stderr)
        return 1
    actions = {item["path"]: item for item in receipt["actions"]}
    if (
        actions["PID.3"]["action"] != "hash"
        or actions["PID.3"]["status"] != "applied"
        or actions["PID.5"]["action"] != "drop"
        or actions["PID.7"]["action"] != "drop"
        or actions["PID.11"]["action"] != "drop"
        or actions["PID.13"]["action"] != "drop"
        or actions["NK1.2"]["action"] != "drop"
        or actions["NK1.4"]["action"] != "drop"
        or actions["NK1.5"]["action"] != "drop"
    ):
        print(f"unexpected redaction actions: {receipt}", file=sys.stderr)
        return 1

    segment_repetition_raw = (
        "MSH|^~\\&|LAB|L|EHR|E|202605030101||ADT^A01|CTRL123|P|2.5\r"
        "NK1|1\r"
        "NK1|2|Kin^Jane\r"
    )
    segment_repetition_policy = """
[[rules]]
path = "NK1[2]-2"
action = "drop"
reason = "Remove populated next-of-kin name"
"""
    segment_repetition_redaction = hl7v2.redact(
        segment_repetition_raw,
        segment_repetition_policy,
    )
    segment_repetition_hl7 = segment_repetition_redaction["redacted_hl7"]
    if "Kin^Jane" in segment_repetition_hl7 or "NK1|1" not in segment_repetition_hl7:
        print(
            "segment repetition redaction touched the wrong content: "
            f"{segment_repetition_redaction}",
            file=sys.stderr,
        )
        return 1
    segment_repetition_actions = {
        item["path"]: item
        for item in segment_repetition_redaction["receipt"]["actions"]
    }
    segment_repetition_action = segment_repetition_actions.get("NK1[2].2")
    if (
        segment_repetition_action is None
        or segment_repetition_action["action"] != "drop"
        or segment_repetition_action["matched_count"] != 1
        or segment_repetition_action["status"] != "applied"
    ):
        print(
            "unexpected segment repetition redaction receipt: "
            f"{segment_repetition_redaction}",
            file=sys.stderr,
        )
        return 1

    unsafe_segment_repetition_policy = """
[[rules]]
path = "NK1[2]-2"
action = "retain"
reason = "Unsafe retain"
"""
    try:
        hl7v2.redact(segment_repetition_raw, unsafe_segment_repetition_policy)
    except ValueError as exc:
        error_text = str(exc)
        if (
            "redaction rule NK1[2].2 cannot retain a built-in sensitive field"
            not in error_text
        ):
            print(
                f"unexpected segment repetition retain failure: {exc}",
                file=sys.stderr,
            )
            return 1
        if "Kin^Jane" in error_text or "Kin" in error_text or "Jane" in error_text:
            print(
                f"segment repetition retain failure leaked PHI: {exc}",
                file=sys.stderr,
            )
            return 1
    else:
        print(
            "expected segment repetition retain policy to fail closed",
            file=sys.stderr,
        )
        return 1

    redaction_v2 = hl7v2.redact(
        phi_raw, redaction_policy, schema_version=V2_REPORT_SCHEMA_VERSION
    )
    receipt_v2 = redaction_v2["receipt"]
    if (
        redaction_v2["schema_version"] != EXPECTED_V2_SCHEMA_VERSION
        or redaction_v2["tool_name"] != PYTHON_TOOL_NAME
        or redaction_v2["tool_version"] != hl7v2.__version__
        or receipt_v2["schema_version"] != EXPECTED_V2_SCHEMA_VERSION
        or receipt_v2["tool_name"] != PYTHON_TOOL_NAME
        or receipt_v2["tool_version"] != hl7v2.__version__
        or receipt_v2["phi_removed"] is not True
        or receipt_v2["hash_algorithm"] != "sha256"
    ):
        print(f"unexpected redaction output v2 provenance: {redaction_v2}", file=sys.stderr)
        return 1
    try:
        assert_no_phi_leak_sentinels(
            "Python redacted HL7 v2",
            redaction_v2["redacted_hl7"],
        )
        assert_no_phi_leak_sentinels(
            "Python redaction receipt v2",
            json.dumps(receipt_v2, sort_keys=True),
        )
    except AssertionError as exc:
        print(str(exc), file=sys.stderr)
        return 1
    try:
        hl7v2.redact(
            phi_raw,
            redaction_policy,
            schema_version=UNSUPPORTED_REPORT_SCHEMA_VERSION,
        )
    except ValueError as exc:
        if UNSUPPORTED_SCHEMA_VERSION_FRAGMENT not in str(exc):
            print(f"unexpected redaction schema version failure: {exc}", file=sys.stderr)
            return 1
    else:
        print("expected unsupported redaction schema version to fail", file=sys.stderr)
        return 1

    unsafe_policy = """
[[rules]]
path = "PID.3"
action = "hash"
reason = "Patient identifier"
"""
    try:
        hl7v2.redact(phi_raw, unsafe_policy)
    except ValueError as exc:
        if "redaction policy does not protect present sensitive field" not in str(exc):
            print(f"unexpected redaction failure: {exc}", file=sys.stderr)
            return 1
    else:
        print("expected incomplete redaction policy to fail closed", file=sys.stderr)
        return 1

    with tempfile.TemporaryDirectory() as tmp:
        bundle_dir = Path(tmp) / "issue-bundle"
        bundle = hl7v2.bundle(phi_raw, profile_yaml, redaction_policy, str(bundle_dir))
        if bundle["bundle_version"] != "1" or bundle["output_dir"] != ".":
            print(f"unexpected bundle summary: {bundle}", file=sys.stderr)
            return 1
        if (
            bundle["message_type"] != "ADT^A01"
            or bundle["validation_valid"] is not True
            or bundle["redaction_phi_removed"] is not True
        ):
            print(f"unexpected bundle evidence status: {bundle}", file=sys.stderr)
            return 1
        for artifact in [
            "message.redacted.hl7",
            "validation-report.json",
            "field-paths.json",
            "profile.yaml",
            "redaction-receipt.json",
            "environment.json",
            "replay.sh",
            "replay.ps1",
            "README.md",
            "SAFE-SHARING.md",
            "manifest.json",
        ]:
            if artifact not in bundle["artifacts"] or not (bundle_dir / artifact).is_file():
                print(f"bundle missing artifact {artifact}: {bundle}", file=sys.stderr)
                return 1

        bundle_v2_dir = Path(tmp) / "issue-bundle-v2"
        bundle_v2 = hl7v2.bundle(
            phi_raw,
            profile_yaml,
            redaction_policy,
            str(bundle_v2_dir),
            schema_version=V2_REPORT_SCHEMA_VERSION,
        )
        if (
            bundle_v2["schema_version"] != EXPECTED_V2_SCHEMA_VERSION
            or bundle_v2["tool_name"] != PYTHON_TOOL_NAME
            or "tool_version" not in bundle_v2
            or bundle_v2["bundle_version"] != "1"
        ):
            print(f"unexpected bundle v2 summary: {bundle_v2}", file=sys.stderr)
            return 1
        for artifact in [
            "manifest.json",
            "field-paths.json",
            "redaction-receipt.json",
            "environment.json",
        ]:
            artifact_json = json.loads(
                (bundle_v2_dir / artifact).read_text(encoding="utf-8")
            )
            if (
                artifact_json["schema_version"] != EXPECTED_V2_SCHEMA_VERSION
                or artifact_json["tool_name"] != PYTHON_TOOL_NAME
            ):
                print(
                    f"unexpected bundle v2 artifact {artifact}: {artifact_json}",
                    file=sys.stderr,
                )
                return 1

        try:
            hl7v2.bundle(
                phi_raw,
                profile_yaml,
                redaction_policy,
                str(Path(tmp) / "bad-schema-bundle"),
                schema_version=UNSUPPORTED_REPORT_SCHEMA_VERSION,
            )
        except ValueError as exc:
            if UNSUPPORTED_SCHEMA_VERSION_FRAGMENT not in str(exc):
                print(f"unexpected bundle schema failure: {exc}", file=sys.stderr)
                return 1
        else:
            print("expected unsupported bundle schema_version to fail", file=sys.stderr)
            return 1

        manifest = json.loads((bundle_dir / "manifest.json").read_text(encoding="utf-8"))
        if manifest["tool_name"] != PYTHON_TOOL_NAME:
            print(f"unexpected bundle manifest tool: {manifest}", file=sys.stderr)
            return 1
        environment = json.loads((bundle_dir / "environment.json").read_text(encoding="utf-8"))
        if environment["tool_name"] != PYTHON_TOOL_NAME:
            print(f"unexpected bundle environment tool: {environment}", file=sys.stderr)
            return 1

        replay = hl7v2.replay(str(bundle_dir))
        if replay["reproduced"] is not True or replay["tool_name"] != PYTHON_TOOL_NAME:
            print(f"unexpected replay report: {replay}", file=sys.stderr)
            return 1
        replay_checks = {item["name"]: item["status"] for item in replay["checks"]}
        if replay_checks.get("manifest-hashes") != "pass":
            print(f"replay did not verify manifest hashes: {replay}", file=sys.stderr)
            return 1

        replay_v2 = hl7v2.replay(
            str(bundle_dir), schema_version=V2_REPORT_SCHEMA_VERSION
        )
        if (
            replay_v2["schema_version"] != EXPECTED_V2_SCHEMA_VERSION
            or replay_v2["tool_name"] != PYTHON_TOOL_NAME
            or replay_v2["replay_version"] != "1"
            or replay_v2["reproduced"] is not True
        ):
            print(f"unexpected replay v2 report: {replay_v2}", file=sys.stderr)
            return 1
        try:
            hl7v2.replay(
                str(bundle_dir), schema_version=UNSUPPORTED_REPORT_SCHEMA_VERSION
            )
        except ValueError as exc:
            if UNSUPPORTED_SCHEMA_VERSION_FRAGMENT not in str(exc):
                print(f"unexpected replay schema failure: {exc}", file=sys.stderr)
                return 1
        else:
            print("expected unsupported replay schema_version to fail", file=sys.stderr)
            return 1

        evidence_text = "\n".join(
            (bundle_dir / artifact).read_text(encoding="utf-8")
            for artifact in [
                "message.redacted.hl7",
                "validation-report.json",
                "field-paths.json",
                "profile.yaml",
                "redaction-receipt.json",
                "environment.json",
                "manifest.json",
                "replay.sh",
                "replay.ps1",
                "README.md",
                "SAFE-SHARING.md",
            ]
        )
        evidence_text += json.dumps(replay)
        evidence_text += json.dumps(replay_v2)
        evidence_text += json.dumps(bundle_v2)
        evidence_text += "\n".join(
            (bundle_v2_dir / artifact).read_text(encoding="utf-8")
            for artifact in [
                "field-paths.json",
                "redaction-receipt.json",
                "environment.json",
                "manifest.json",
            ]
        )
        try:
            assert_no_phi_leak_sentinels_or_paths(
                "Python bundle/replay evidence",
                evidence_text,
                bundle_dir,
                Path(tmp),
            )
        except AssertionError as exc:
            print(str(exc), file=sys.stderr)
            return 1

        (bundle_dir / "message.redacted.hl7").write_text(
            "MSH|^~\\&|SEND|FAC|RECV|FAC|202605080101||ADT^A01|TAMPER|P|2.5",
            encoding="utf-8",
        )
        tampered_replay = hl7v2.replay(str(bundle_dir))
        if tampered_replay["reproduced"] is not False:
            print(f"expected tampered bundle replay to fail: {tampered_replay}", file=sys.stderr)
            return 1
        tampered_checks = {item["name"]: item["status"] for item in tampered_replay["checks"]}
        if tampered_checks.get("manifest-hashes") != "fail":
            print(f"tampered replay did not report hash failure: {tampered_replay}", file=sys.stderr)
            return 1

        existing_dir = Path(tmp) / "existing-bundle"
        existing_dir.mkdir()
        try:
            hl7v2.bundle(phi_raw, profile_yaml, redaction_policy, str(existing_dir))
        except ValueError as exc:
            if "bundle output directory already exists" not in str(exc):
                print(f"unexpected existing bundle failure: {exc}", file=sys.stderr)
                return 1
        else:
            print("expected existing bundle directory to fail closed", file=sys.stderr)
            return 1

    malformed = SECURITY_FIXTURE["malformed_message"]["message"]
    try:
        hl7v2.parse(malformed)
    except ValueError as exc:
        error_text = str(exc)
        if "Parse error" not in error_text:
            print(f"parse error did not include stable context: {exc}", file=sys.stderr)
            return 1
        for expected in SECURITY_FIXTURE["malformed_message"]["expected_error_substrings"]:
            if expected not in error_text:
                print(
                    f"parse error omitted expected safe context {expected!r}: {exc}",
                    file=sys.stderr,
                )
                return 1
        try:
            assert_no_phi_leak_sentinels("Python parse safe error", error_text)
        except AssertionError as safe_error:
            print(str(safe_error), file=sys.stderr)
            return 1
    else:
        print("expected invalid HL7 parse to raise ValueError", file=sys.stderr)
        return 1

    print(f"hl7v2 smoke ok version={version} segments={segment_count}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
