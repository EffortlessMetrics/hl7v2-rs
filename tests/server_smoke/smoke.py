"""Smoke test for a running hl7v2-server sidecar.

The script intentionally uses only Python's standard library so it can run
against a locally built Docker Compose sidecar without installing test
dependencies.
"""

from __future__ import annotations

import json
import os
import sys
import time
import uuid
from typing import Any
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen


BASE_URL = os.environ.get("HL7V2_SERVER_URL", "http://127.0.0.1:8080").rstrip("/")
API_KEY = os.environ.get("HL7V2_API_KEY", "dev-secret")
TIMEOUT_SECONDS = float(os.environ.get("HL7V2_SERVER_SMOKE_TIMEOUT", "45"))

PHI_MESSAGE = (
    "MSH|^~\\&|LAB|L|EHR|E|202605030101||ADT^A01|CTRL123|P|2.5\r"
    "PID|1||123456^^^HOSP^MR||Doe^John||19700101|M|||123 Main St||5558675309\r"
    "NK1|1|Watcher^Nora||900 Support Way|5550001234\r"
    "OBX|1|NM|718-7^Hemoglobin^LN||13.2|g/dL\r"
)

ORU_MESSAGE = (
    "MSH|^~\\&|LAB|L|EHR|E|202605030101||ORU^R01|CTRL124|P|2.5\r"
    "PID|1||123456^^^HOSP^MR||Doe^John||19700101|M\r"
    "OBR|1|ORD123|FIL456|718-7^Hemoglobin^LN\r"
    "OBX|1|NM|718-7^Hemoglobin^LN||13.2|g/dL\r"
)

PROFILE = """
message_structure: "ADT_A01"
version: "2.5"
segments:
  - id: "MSH"
    required: true
    max_uses: 1
  - id: "PID"
    required: true
    max_uses: 1
constraints:
  - path: "PID.3"
    required: true
"""

REDACTION_POLICY = """
[[rules]]
path = "PID.3"
action = "hash"
reason = "hash patient identifier for support analysis"

[[rules]]
path = "PID.5"
action = "drop"
reason = "drop patient name"

[[rules]]
path = "PID.7"
action = "drop"
reason = "drop date of birth"

[[rules]]
path = "PID.11"
action = "drop"
reason = "drop patient address"

[[rules]]
path = "PID.13"
action = "drop"
reason = "drop patient phone"

[[rules]]
path = "NK1.2"
action = "drop"
reason = "drop next-of-kin name"

[[rules]]
path = "NK1.4"
action = "drop"
reason = "drop next-of-kin address"

[[rules]]
path = "NK1.5"
action = "drop"
reason = "drop next-of-kin phone"
"""

PHI_SENTINELS = [
    "Doe^John",
    "123 Main St",
    "5558675309",
    "Watcher^Nora",
    "900 Support Way",
    "5550001234",
]


class SmokeFailure(RuntimeError):
    """Raised when the sidecar smoke proof fails."""


def request_json(method: str, path: str, body: dict[str, Any] | None = None) -> dict[str, Any]:
    data = None if body is None else json.dumps(body).encode("utf-8")
    headers = {"Accept": "application/json"}
    if body is not None:
        headers["Content-Type"] = "application/json"
    if API_KEY:
        headers["X-API-Key"] = API_KEY
    request = Request(f"{BASE_URL}{path}", data=data, headers=headers, method=method)

    try:
        with urlopen(request, timeout=5) as response:
            payload = response.read().decode("utf-8")
    except HTTPError as error:
        payload = error.read().decode("utf-8", errors="replace")
        raise SmokeFailure(f"{method} {path} returned {error.code}: {payload}") from error
    except URLError as error:
        raise SmokeFailure(f"{method} {path} failed: {error}") from error

    try:
        return json.loads(payload)
    except json.JSONDecodeError as error:
        raise SmokeFailure(f"{method} {path} returned non-JSON: {payload}") from error


def assert_no_phi(label: str, value: Any) -> None:
    text = json.dumps(value, sort_keys=True)
    leaked = [sentinel for sentinel in PHI_SENTINELS if sentinel in text]
    if leaked:
        raise SmokeFailure(f"{label} leaked PHI sentinels: {', '.join(leaked)}")


def wait_for_ready() -> dict[str, Any]:
    deadline = time.monotonic() + TIMEOUT_SECONDS
    last_error: Exception | None = None

    while time.monotonic() < deadline:
        try:
            ready = request_json("GET", "/ready")
            if ready.get("ready") is True:
                return ready
            last_error = SmokeFailure(f"/ready returned not-ready: {ready}")
        except Exception as error:  # noqa: BLE001 - surface last startup failure in smoke output
            last_error = error
        time.sleep(1)

    raise SmokeFailure(f"server did not become ready: {last_error}")


def wait_for_health() -> dict[str, Any]:
    deadline = time.monotonic() + TIMEOUT_SECONDS
    last_error: Exception | None = None

    while time.monotonic() < deadline:
        try:
            health = request_json("GET", "/health")
            if health.get("status") == "healthy":
                return health
            last_error = SmokeFailure(f"/health returned non-healthy status: {health}")
        except Exception as error:  # noqa: BLE001 - surface last startup failure in smoke output
            last_error = error
        time.sleep(1)

    raise SmokeFailure(f"server did not become healthy: {last_error}")


def main() -> int:
    wait_for_health()
    ready = wait_for_ready()
    if not any(check.get("name") == "validation_report" for check in ready.get("checks", [])):
        raise SmokeFailure("/ready did not include validation_report self-check")

    redacted = request_json(
        "POST",
        "/hl7/validate-redacted",
        {
            "message": PHI_MESSAGE,
            "profile": PROFILE,
            "redaction_policy": REDACTION_POLICY,
            "include_redacted_hl7": True,
            "report_schema_version": 2,
            "redaction_receipt_schema_version": 2,
        },
    )
    if redacted.get("validation_report", {}).get("valid") is not True:
        raise SmokeFailure(f"redacted validation was not valid: {redacted}")
    if redacted.get("redaction_receipt", {}).get("phi_removed") is not True:
        raise SmokeFailure(f"redaction receipt did not remove PHI: {redacted}")
    assert_no_phi("validate-redacted response", redacted)

    bundle_id = f"smoke-{uuid.uuid4().hex}"
    bundle = request_json(
        "POST",
        "/hl7/bundle",
        {
            "message": PHI_MESSAGE,
            "profile": PROFILE,
            "redaction_policy": REDACTION_POLICY,
            "bundle_id": bundle_id,
            "bundle_artifact_schema_version": 2,
        },
    )
    if bundle.get("output_dir") != bundle_id:
        raise SmokeFailure(f"bundle output id mismatch: {bundle}")
    if bundle.get("validation_valid") is not True:
        raise SmokeFailure(f"bundle validation was not valid: {bundle}")
    assert_no_phi("bundle response", bundle)

    replay = request_json(
        "POST",
        "/hl7/replay",
        {"bundle_id": bundle_id, "replay_report_schema_version": 2},
    )
    if replay.get("schema_version") != "2" or replay.get("reproduced") is not True:
        raise SmokeFailure(f"replay did not reproduce bundle: {replay}")
    assert_no_phi("replay response", replay)

    diff = request_json(
        "POST",
        "/hl7/corpus/diff",
        {
            "before": [{"id": "before-adt", "message": PHI_MESSAGE}],
            "after": [
                {"id": "after-adt", "message": PHI_MESSAGE},
                {"id": "after-oru", "message": ORU_MESSAGE},
            ],
            "diff_schema_version": 2,
        },
    )
    if diff.get("schema_version") != "2":
        raise SmokeFailure(f"corpus diff did not return v2 schema: {diff}")
    if "ORU^R01" not in diff.get("new_message_types", []):
        raise SmokeFailure(f"corpus diff did not report new ORU^R01 type: {diff}")
    assert_no_phi("corpus diff response", diff)

    print(
        "hl7v2-server smoke ok "
        f"url={BASE_URL} bundle_id={bundle_id} checks={len(ready.get('checks', []))}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except SmokeFailure as error:
        print(f"hl7v2-server smoke failed: {error}", file=sys.stderr)
        raise SystemExit(1) from error
