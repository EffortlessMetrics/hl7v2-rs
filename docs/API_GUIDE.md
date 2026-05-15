# HL7v2-rs API Guide

This guide provides examples and best practices for interacting with the `hl7v2-server` REST API.

---

## Base URL

By default, the server runs on `http://localhost:8080`. All API paths are relative to this base.

---

## Authentication

If `HL7V2_API_KEY` is configured on the server, all requests to `/hl7/*` routes must include the `X-API-Key` header.

```bash
-H "X-API-Key: your-secret-api-key"
```

---

## Endpoints

### 1. Parse HL7 Message
**POST** `/hl7/parse`

Converts a raw HL7 v2 message into a structured JSON representation.

**Request Body:**
```json
{
  "message": "MSH|^~\\&|SENDER|FACILITY|RECEIVER|FACILITY|20230101120000||ADT^A01|MSG123|P|2.5\rPID|1||123456^^^MRN||Doe^John||19800101|M",
  "mllp_framed": false
}
```

**cURL Example:**
```bash
curl -X POST http://localhost:8080/hl7/parse \
  -H "Content-Type: application/json" \
  -d '{
    "message": "MSH|^~\\&|SENDER|FAC|REC|FAC|20240101||ADT^A01|123|P|2.5\rPID|1||MRN123||DOE^JOHN\r"
  }'
```

---

### 2. Validate HL7 Message
**POST** `/hl7/validate`

Validates an HL7 v2 message against a provided conformance profile.

**Request Body:**
```json
{
  "message": "MSH|^~\\&|...",
  "profile": "...",
  "mllp_framed": false
}
```

**Response Body:**
```json
{
  "valid": false,
  "message_type": "ADT^A01",
  "profile": "ADT_A01",
  "segment_count": 2,
  "issue_count": 1,
  "issues": [
    {
      "code": "missing_required_field",
      "severity": "error",
      "path": "PID.3",
      "rule_id": "missing_required_field",
      "message": "Required field PID.3 is missing",
      "segment_index": 1,
      "field_index": 3
    }
  ],
  "metadata": {
    "message_type": "ADT^A01",
    "version": "2.5",
    "sending_application": "SENDER",
    "sending_facility": "FAC",
    "message_control_id": "123",
    "segment_count": 2,
    "charsets": []
  }
}
```

**cURL Example:**
```bash
# Using a local profile file
PROFILE_CONTENT=$(cat profiles/examples/adt/ADT_A01.yaml)
MESSAGE="MSH|^~\\&|..."

curl -X POST http://localhost:8080/hl7/validate \
  -H "Content-Type: application/json" \
  --data-binary @- <<EOF
{
  "message": "$MESSAGE",
  "profile": "$(echo "$PROFILE_CONTENT" | sed 's/"/\\"/g' | awk '{printf "%s\\n", $0}' ORS='')"
}
EOF
```

---

### 3. Validate Redacted HL7 Message
**POST** `/hl7/validate-redacted`

Applies a safe-analysis redaction policy first, then validates the redacted
message against the supplied profile. The response includes a
`validation_report`, a `redaction_receipt`, and an optional `redacted_hl7`
field when `include_redacted_hl7` is true.

Set `redaction_receipt_schema_version` to `2` to include the additive
`redaction_receipt_v2` field with embedded `schema_version`, `tool_name`, and
`tool_version` provenance. Omitting the field, or setting it to `1`, preserves
the existing response shape.

When `[quarantine]` is enabled in `HL7V2_CONFIG`, failed validation writes
configured quarantine output under the server-controlled quarantine root and
adds a `quarantine` summary. The summary reports only a root-relative output id
and artifact names; it does not expose the configured filesystem path.
Set `quarantine_schema_version` to `2` to also include `quarantine_v2` with
embedded evidence provenance when quarantine output is written.

```toml
[quarantine]
enabled = true
path = "quarantine"
write_redacted = true
write_report = true
write_bundle = true
```

**Request Body:**
```json
{
  "message": "MSH|^~\\&|...",
  "profile": "message_structure: ADT_A01\nversion: \"2.5\"\nsegments:\n  - id: MSH\n",
  "redaction_policy": "[[rules]]\npath = \"PID.3\"\naction = \"hash\"\nreason = \"hash patient identifier\"\n",
  "redaction_receipt_schema_version": 2,
  "quarantine_schema_version": 2,
  "include_redacted_hl7": true
}
```

---

### 4. Create Redacted Evidence Bundle
**POST** `/hl7/bundle`

Applies a safe-analysis redaction policy, validates the redacted message,
and writes a replayable evidence bundle under the configured
`HL7V2_BUNDLE_OUTPUT_ROOT`. The request supplies a `bundle_id`, not a
filesystem path. The bundle id must be one safe path segment using ASCII
letters, numbers, `.`, `_`, or `-`; `.` and `..` are rejected.

The endpoint fails closed with `503 BUNDLE_OUTPUT_NOT_CONFIGURED` when the
server has no configured bundle output root.

Set `bundle_artifact_schema_version` to `2` to write v2 bundle-internal
artifacts for `manifest.json`, `environment.json`, `field-paths.json`, and
`redaction-receipt.json`. The response body remains the v1-compatible bundle
summary.

**Request Body:**
```json
{
  "bundle_id": "case-001",
  "message": "MSH|^~\\&|...",
  "profile": "message_structure: ADT_A01\nversion: \"2.5\"\nsegments:\n  - id: MSH\n",
  "redaction_policy": "[[rules]]\npath = \"PID.3\"\naction = \"hash\"\nreason = \"hash patient identifier\"\n",
  "mllp_framed": false,
  "bundle_artifact_schema_version": 2
}
```

**Response Body:**
```json
{
  "bundle_version": "1",
  "output_dir": "case-001",
  "message_type": "ADT^A01",
  "validation_valid": true,
  "validation_issue_count": 0,
  "redaction_phi_removed": true,
  "artifacts": [
    "message.redacted.hl7",
    "validation-report.json",
    "field-paths.json",
    "profile.yaml",
    "redaction-receipt.json",
    "environment.json",
    "replay.sh",
    "replay.ps1",
    "README.md",
    "manifest.json"
  ]
}
```

**cURL Example:**
```bash
curl -X POST http://localhost:8080/hl7/bundle \
  -H "X-API-Key: your-secret-api-key" \
  -H "Content-Type: application/json" \
  -d '{
    "bundle_id": "case-001",
    "message": "MSH|^~\\&|...",
    "profile": "...",
    "redaction_policy": "[[rules]]\npath = \"PID.3\"\naction = \"hash\"\nreason = \"hash patient identifier\"\n",
    "bundle_artifact_schema_version": 2
  }'
```

---

### 5. Replay Evidence Bundle
**POST** `/hl7/replay`

Replays and verifies a previously written evidence bundle under the configured
`HL7V2_BUNDLE_OUTPUT_ROOT`. The request supplies a `bundle_id`, not a
filesystem path. Replay returns the same evidence replay report shape used by
the CLI and Python surfaces.

The endpoint fails closed with `503 BUNDLE_OUTPUT_NOT_CONFIGURED` when the
server has no configured bundle output root. Unknown bundle ids return
`404 BUNDLE_NOT_FOUND`.

Set `replay_report_schema_version` to `2` to return the v2 replay report with
embedded `schema_version` provenance.

**Request Body:**
```json
{
  "bundle_id": "case-001",
  "replay_report_schema_version": 2
}
```

**Response Body:**
```json
{
  "schema_version": "2",
  "replay_version": "1",
  "bundle_version": "1",
  "tool_name": "hl7v2-server",
  "tool_version": "1.3.0",
  "message_type": "ADT^A01",
  "reproduced": true,
  "validation_valid": true,
  "validation_issue_count": 0,
  "checks": [
    {
      "name": "manifest-hashes",
      "status": "pass",
      "message": "manifest artifact hashes match bundle contents"
    }
  ]
}
```

**cURL Example:**
```bash
curl -X POST http://localhost:8080/hl7/replay \
  -H "X-API-Key: your-secret-api-key" \
  -H "Content-Type: application/json" \
  -d '{
    "bundle_id": "case-001",
    "replay_report_schema_version": 2
  }'
```

---

### 6. Inline Corpus Evidence

The server can summarize, fingerprint, and diff caller-supplied message sets
without reading filesystem paths from the request. Each message may include an
optional safe `id` label; the label is used only for parse-error attribution
and must be a single ASCII label, not a path.

These endpoints do not echo raw message payloads in success or validation-error
responses. Parse failures report the safe message id and parser error.

**POST** `/hl7/corpus/summarize`

```json
{
  "messages": [
    {
      "id": "before-adt-1",
      "message": "MSH|^~\\&|..."
    }
  ],
  "summary_schema_version": 2
}
```

**POST** `/hl7/corpus/fingerprint`

```json
{
  "messages": [
    {
      "id": "site-a-1",
      "message": "MSH|^~\\&|..."
    }
  ],
  "profile": "message_structure: ADT_A01\nversion: \"2.5\"\nsegments:\n  - id: MSH\n",
  "fingerprint_schema_version": 2
}
```

**POST** `/hl7/corpus/diff`

```json
{
  "before": [
    {
      "id": "before-1",
      "message": "MSH|^~\\&|..."
    }
  ],
  "after": [
    {
      "id": "after-1",
      "message": "MSH|^~\\&|..."
    }
  ],
  "profile": "message_structure: ADT_A01\nversion: \"2.5\"\nsegments:\n  - id: MSH\n",
  "diff_schema_version": 2
}
```

The response shapes match the CLI corpus artifacts. V2 responses add
`schema_version` and `tool_name` provenance while preserving the v1 fields:

```json
{
  "schema_version": "2",
  "tool_name": "hl7v2-server",
  "diff_version": "1",
  "before_root": "<inline-before>",
  "after_root": "<inline-after>",
  "message_count": {
    "before": 1,
    "after": 2,
    "delta": 1
  },
  "new_message_types": ["ORU^R01"],
  "validation_issue_code_counts": []
}
```

---

### 7. Generate ACK
**POST** `/hl7/ack`

Generates an HL7 ACK response from an inbound message with an explicit
caller-supplied ACK code.

**Request Body:**
```json
{
  "message": "MSH|^~\\&|...",
  "code": "AA",
  "mllp_framed": false,
  "mllp_frame": false
}
```

---

### 8. Generate Policy-Driven ACK
**POST** `/hl7/ack-policy`

Parses and validates an inbound message, then chooses an ACK or NAK code from
the server ACK policy. Configure the policy in `HL7V2_CONFIG`:

```toml
[ack]
mode = "original" # original|enhanced
accept_on = "valid"
reject_on = ["parse_error", "validation_error"]
include_error_text = true
```

Default behavior preserves original-mode application ACK codes: `AA` for valid
messages and `AR` for parse or validation failures. Enhanced mode uses `CA` and
`CR`. Error text is intentionally generic and does not include raw field values.

**Request Body:**
```json
{
  "message": "MSH|^~\\&|...",
  "profile": "message_structure: ADT_A01\nversion: \"2.5\"\nsegments:\n  - id: MSH\n",
  "mllp_framed": false,
  "mllp_frame": false
}
```

**Response Body:**
```json
{
  "ack_message": "MSH|^~\\&|...",
  "ack_code": "AA",
  "decision": {
    "mode": "original",
    "outcome": "accepted",
    "reason": "valid",
    "ack_code": "AA",
    "include_error_text": false
  },
  "validation_report": {
    "valid": true,
    "message_type": "ADT^A01",
    "profile": "ADT_A01",
    "segment_count": 2,
    "issue_count": 0,
    "issues": []
  },
  "metadata": {
    "message_type": "ACK^ADT",
    "version": "2.5",
    "sending_application": "RECEIVER",
    "sending_facility": "FACILITY",
    "message_control_id": "MSG123",
    "segment_count": 2,
    "charsets": []
  }
}
```

---

### 9. Normalize HL7 Message
**POST** `/hl7/normalize`

Rewrites an HL7 message with stable delimiters and optional MLLP output framing.

**Request Body:**
```json
{
  "message": "MSH|^~\\&|...",
  "mllp_framed": false,
  "options": {
    "canonical_delimiters": true,
    "mllp_frame": false
  }
}
```

---

### 10. Health & Metrics

**Health Check:**
```bash
curl http://localhost:8080/health
# Returns: {"status":"healthy","uptime_seconds":3600}
```

**Readiness Check:**
```bash
curl http://localhost:8080/ready
# Returns startup checks such as config, configured_profiles, and validation_report
```

**Prometheus Metrics:**
```bash
curl http://localhost:8080/metrics
# Returns: hl7v2_requests_total{endpoint="/hl7/parse",status="200"} 42 ...
```

---

## Error Responses

The API uses standard HTTP status codes and returns a JSON error body:

```json
{
  "code": "PROFILE_LOAD_ERROR",
  "message": "profile could not be loaded; run profile lint for details",
  "safe_detail": "The supplied inline profile could not be loaded. Raw profile content is not echoed.",
  "location": "profile",
  "suggested_next_action": "Run profile lint on the profile, then retry validation with the corrected profile.",
  "details": null
}
```

`code` and `message` are the compatibility fields. Newer clients should prefer
`code`, `safe_detail`, `location`, and `suggested_next_action` for operator
workflows. Error responses do not echo raw HL7 payloads, raw profile YAML,
redaction policies, configured filesystem roots, API keys, or raw bundle IDs by
default.

Common operator actions:

| Code | First action |
| --- | --- |
| `PARSE_ERROR` | Check the `MSH` segment, segment terminators, encoding, and `mllp_framed` setting. |
| `PROFILE_LOAD_ERROR` | Run profile lint and retry with the corrected profile. |
| `VALIDATION_ERROR` | Check request parameters, schema-version fields, and validation issue paths where available. |
| `REDACTION_ERROR` | Check safe-analysis policy paths, actions, reasons, and required-field matches. |
| `BUNDLE_OUTPUT_NOT_CONFIGURED` | Configure the server bundle output root and verify readiness. |
| `BUNDLE_ERROR` | Use a simple bundle id without path traversal and retry after validating inputs. |
| `QUARANTINE_OUTPUT_NOT_CONFIGURED` | Configure the quarantine output path or disable quarantine output before retrying. |

### Common Status Codes:
- `200 OK`: Success.
- `400 Bad Request`: Invalid JSON, malformed HL7, or profile load error.
- `401 Unauthorized`: Missing or invalid `X-API-Key`.
- `409 Conflict`: Requested evidence bundle id already exists.
- `429 Too Many Requests`: Rate limit exceeded.
- `503 Service Unavailable`: Server-side evidence bundle or quarantine output is not configured or not ready.
- `500 Internal Server Error`: Server configuration error.

---

## Best Practices

1. **Use MLLP Framing**: If you are sending messages from a system that already supports MLLP, set `"mllp_framed": true` to have the server handle the `\x0b` and `\x1c\x0d` bytes automatically.
2. **Batching**: For high-volume processing, consider using a persistent connection or sending multiple messages in a single batch if supported by your workflow.
3. **API Key Rotation**: Periodically rotate your `HL7V2_API_KEY` environment variable.
4. **Client-side Validation**: Use `api/openapi/hl7v2-api-v1.yaml` or the server's documentation endpoint to generate type-safe clients in your preferred language.
