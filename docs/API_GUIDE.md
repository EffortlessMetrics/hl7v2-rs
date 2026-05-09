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

**Request Body:**
```json
{
  "message": "MSH|^~\\&|...",
  "profile": "message_structure: ADT_A01\nversion: \"2.5\"\nsegments:\n  - id: MSH\n",
  "redaction_policy": "[[rules]]\npath = \"PID.3\"\naction = \"hash\"\nreason = \"hash patient identifier\"\n",
  "include_redacted_hl7": true
}
```

---

### 4. Generate ACK
**POST** `/hl7/ack`

Generates an HL7 ACK response from an inbound message.

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

### 5. Normalize HL7 Message
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

### 6. Health & Metrics

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
# Returns: hl7v2_requests_total{method="POST",path="/hl7/parse",status="200"} 42 ...
```

---

## Error Responses

The API uses standard HTTP status codes and returns a JSON error body:

```json
{
  "code": "PROFILE_LOAD_ERROR",
  "message": "Failed to load profile: missing required field `version`",
  "details": null
}
```

### Common Status Codes:
- `200 OK`: Success.
- `400 Bad Request`: Invalid JSON, malformed HL7, or profile load error.
- `401 Unauthorized`: Missing or invalid `X-API-Key`.
- `429 Too Many Requests`: Rate limit exceeded.
- `500 Internal Server Error`: Server configuration error.

---

## Best Practices

1. **Use MLLP Framing**: If you are sending messages from a system that already supports MLLP, set `"mllp_framed": true` to have the server handle the `\x0b` and `\x1c\x0d` bytes automatically.
2. **Batching**: For high-volume processing, consider using a persistent connection or sending multiple messages in a single batch if supported by your workflow.
3. **API Key Rotation**: Periodically rotate your `HL7V2_API_KEY` environment variable.
4. **Client-side Validation**: Use `api/openapi/hl7v2-api-v1.yaml` or the server's documentation endpoint to generate type-safe clients in your preferred language.
