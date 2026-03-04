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
  "profile_yaml": "...",
  "mllp_framed": false
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
  "profile_yaml": "$(echo "$PROFILE_CONTENT" | sed 's/"/\\"/g' | awk '{printf "%s\\n", $0}' ORS='')"
}
EOF
```

---

### 3. Health & Metrics

**Health Check:**
```bash
curl http://localhost:8080/health
# Returns: {"status":"healthy","uptime_seconds":3600}
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
  "error": "Validation failed",
  "details": [
    "PID.5.1 (Family Name) is required but missing",
    "MSH.9.2 (Trigger Event) must be 'A01' for this profile"
  ]
}
```

### Common Status Codes:
- `200 OK`: Success.
- `400 Bad Request`: Invalid JSON or missing required fields.
- `401 Unauthorized`: Missing or invalid `X-API-Key`.
- `422 Unprocessable Entity`: HL7 parsing or validation failed.
- `429 Too Many Requests`: Rate limit exceeded.
- `500 Internal Server Error`: Server configuration error.

---

## Best Practices

1. **Use MLLP Framing**: If you are sending messages from a system that already supports MLLP, set `"mllp_framed": true` to have the server handle the `\x0b` and `\x1c\x0d` bytes automatically.
2. **Batching**: For high-volume processing, consider using a persistent connection or sending multiple messages in a single batch if supported by your workflow.
3. **API Key Rotation**: Periodically rotate your `HL7V2_API_KEY` environment variable.
4. **Client-side Validation**: Use the OpenAPI spec (`/api/docs`) to generate type-safe clients in your preferred language.
