"""Guide-specific smoke for the deploy-validation-sidecar walkthrough."""

from __future__ import annotations

import json
import os
import sys
import time
from pathlib import Path
from typing import Any
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen


BASE_URL = os.environ.get("HL7V2_SERVER_URL", "http://127.0.0.1:18080").rstrip("/")
API_KEY = os.environ.get("HL7V2_API_KEY", "dev-secret")
ROOT = Path(os.environ.get("HL7V2_SIDECAR_GUIDE_ROOT", "target/hl7v2-sidecar"))
TIMEOUT_SECONDS = float(os.environ.get("HL7V2_SERVER_SMOKE_TIMEOUT", "45"))
PHI_SENTINELS = ("123456", "19800101", "Doe^John")


class SmokeFailure(RuntimeError):
    """Raised when the guide smoke observes an unexpected response."""


def request_json(method: str, path: str, body: dict[str, Any] | None = None) -> dict[str, Any]:
    data = None if body is None else json.dumps(body).encode("utf-8")
    headers = {"Accept": "application/json"}
    if body is not None:
        headers["Content-Type"] = "application/json"
    if path.startswith("/hl7/"):
        headers["X-API-Key"] = API_KEY
    request = Request(f"{BASE_URL}{path}", data=data, headers=headers, method=method)
    try:
        with urlopen(request, timeout=TIMEOUT_SECONDS) as response:
            return json.loads(response.read().decode("utf-8"))
    except HTTPError as error:
        detail = error.read().decode("utf-8", errors="replace")
        raise SmokeFailure(f"{method} {path} failed with {error.code}: {detail}") from error
    except URLError as error:
        raise SmokeFailure(f"{method} {path} failed: {error}") from error


def request_text(method: str, path: str) -> str:
    request = Request(f"{BASE_URL}{path}", headers={"Accept": "text/plain"}, method=method)
    try:
        with urlopen(request, timeout=TIMEOUT_SECONDS) as response:
            return response.read().decode("utf-8", errors="replace")
    except HTTPError as error:
        detail = error.read().decode("utf-8", errors="replace")
        raise SmokeFailure(f"{method} {path} failed with {error.code}: {detail}") from error
    except URLError as error:
        raise SmokeFailure(f"{method} {path} failed: {error}") from error


def wait_for_ready() -> None:
    deadline = time.monotonic() + TIMEOUT_SECONDS
    last_error: Exception | None = None
    while time.monotonic() < deadline:
        try:
            ready = request_json("GET", "/ready")
            if ready.get("ready") is True:
                return
            last_error = SmokeFailure(f"/ready returned not-ready: {ready}")
        except Exception as error:  # noqa: BLE001 - report the final startup failure.
            last_error = error
        time.sleep(1)
    raise SmokeFailure(f"server did not become ready: {last_error}")


def assert_no_phi(label: str, value: Any) -> None:
    content = json.dumps(value, sort_keys=True) if not isinstance(value, str) else value
    for sentinel in PHI_SENTINELS:
        if sentinel in content:
            raise SmokeFailure(f"{label} leaked PHI sentinel {sentinel!r}")


def write_report(name: str, value: Any) -> None:
    reports = ROOT / "reports"
    reports.mkdir(parents=True, exist_ok=True)
    text = value if isinstance(value, str) else json.dumps(value, indent=2, sort_keys=True)
    (reports / name).write_text(text, encoding="utf-8")


def read_fixture(path: str) -> str:
    return Path(path).read_bytes().decode("utf-8")


def main() -> int:
    wait_for_ready()

    message = read_fixture("test_data/invalid_message.hl7")
    valid_message = read_fixture("test_data/valid_message.hl7")
    profile = Path("profiles/generic.yaml").read_text(encoding="utf-8")
    policy = (ROOT / "safe-analysis.toml").read_text(encoding="utf-8")

    validate_redacted = request_json(
        "POST",
        "/hl7/validate-redacted",
        {
            "message": message,
            "profile": profile,
            "redaction_policy": policy,
            "include_redacted_hl7": False,
            "report_schema_version": 2,
            "redaction_receipt_schema_version": 2,
            "quarantine_schema_version": 2,
        },
    )
    write_report("validate-redacted-guide-smoke.json", validate_redacted)
    if validate_redacted.get("validation_report", {}).get("valid") is not False:
        raise SmokeFailure(f"guide validation was not invalid: {validate_redacted}")
    if validate_redacted.get("redaction_receipt", {}).get("phi_removed") is not True:
        raise SmokeFailure(f"guide redaction receipt did not remove PHI: {validate_redacted}")
    quarantine = validate_redacted.get("quarantine")
    if not isinstance(quarantine, dict) or quarantine.get("reason") != "validation_error":
        raise SmokeFailure(f"guide validation did not write quarantine output: {validate_redacted}")
    if quarantine.get("validation_issue_count") != 1:
        raise SmokeFailure(f"guide quarantine issue count drifted: {validate_redacted}")
    quarantine_v2 = validate_redacted.get("quarantine_v2")
    if not isinstance(quarantine_v2, dict) or quarantine_v2.get("schema_version") != "2":
        raise SmokeFailure(f"guide quarantine v2 summary missing: {validate_redacted}")
    assert_no_phi("validate-redacted guide response", validate_redacted)

    output_dir = quarantine.get("output_dir")
    if not isinstance(output_dir, str) or not output_dir.startswith("quarantine-"):
        raise SmokeFailure(f"guide quarantine output id was not root-relative: {validate_redacted}")
    quarantine_root = ROOT / "quarantine" / output_dir
    for artifact in ("manifest.json", "validation-report.json", "redaction-receipt.json"):
        if not (quarantine_root / artifact).exists():
            raise SmokeFailure(f"missing guide quarantine artifact {artifact}")

    ack_policy = request_json(
        "POST",
        "/hl7/ack-policy",
        {
            "message": message,
            "profile": profile,
            "mllp_framed": False,
            "mllp_frame": False,
        },
    )
    write_report("ack-policy-guide-smoke.json", ack_policy)
    if ack_policy.get("ack_code") != "AR":
        raise SmokeFailure(f"guide ACK policy did not reject invalid input: {ack_policy}")
    if ack_policy.get("decision", {}).get("reason") != "validation_error":
        raise SmokeFailure(f"guide ACK policy reason drifted: {ack_policy}")
    assert_no_phi("ack-policy guide response", ack_policy)

    corpus_diff = request_json(
        "POST",
        "/hl7/corpus/diff",
        {
            "before": [{"id": "before-1", "message": message}],
            "after": [{"id": "after-1", "message": valid_message}],
            "profile": profile,
            "diff_schema_version": 2,
        },
    )
    write_report("corpus-diff-guide-smoke.json", corpus_diff)
    if corpus_diff.get("schema_version") != "2":
        raise SmokeFailure(f"guide corpus diff did not return v2 schema: {corpus_diff}")
    assert_no_phi("corpus diff guide response", corpus_diff)

    metrics = request_text("GET", "/metrics")
    write_report("metrics-guide-smoke.txt", metrics)
    if "hl7v2_requests_total" not in metrics:
        raise SmokeFailure("guide metrics output did not include request metrics")
    assert_no_phi("metrics guide response", metrics)

    print(
        "hl7v2-server guide smoke ok "
        f"url={BASE_URL} quarantine_output={output_dir} ack={ack_policy.get('ack_code')}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except SmokeFailure as error:
        print(f"guide smoke failed: {error}", file=sys.stderr)
        raise SystemExit(1) from error
