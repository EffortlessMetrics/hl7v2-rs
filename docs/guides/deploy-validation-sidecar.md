# Deploy Validation Sidecar

This guide shows how to run `hl7v2-server` as a small validation sidecar. The
server is not an interface engine. It is an edge guard that can parse, validate,
redact, decide ACK/NAK policy, write quarantine artifacts, and expose readiness
and metrics around those decisions.

The examples use the product binary name `hl7v2-server`. From a source checkout,
use `cargo run -q -p hl7v2-server --` instead:

```bash
cargo run -q -p hl7v2-server -- --print-config
```

## What You Will Prove

| Surface | Purpose |
| --- | --- |
| `--print-config` | Print sanitized effective configuration without leaking secrets. |
| `GET /ready` | Prove config, profile loading, output roots, and validation-report self-checks are ready. |
| `POST /hl7/validate-redacted` | Validate after safe-analysis redaction and optionally quarantine failures. |
| `POST /hl7/corpus/*` | Summarize, fingerprint, and diff inline message sets without request path reads. |
| `POST /hl7/bundle` | Create server-side redacted evidence bundles under a configured root. |
| `POST /hl7/replay` | Verify server-created bundles by id and return a replay report. |
| `POST /hl7/ack-policy` | Generate ACK/NAK responses from parse and validation results. |
| `GET /metrics` | Expose Prometheus metrics for sidecar observation. |

## Inputs

From the repository root, this guide uses:

| Path | Role |
| --- | --- |
| `test_data/invalid_message.hl7` | Message that parses but fails `PID.8` value-set validation. |
| `profiles/generic.yaml` | Profile contract loaded by readiness and request bodies. |
| `target/hl7v2-sidecar/safe-analysis.toml` | Safe-analysis policy created below. |
| `target/hl7v2-sidecar/server.toml` | Sidecar config created below. |

Create local output roots:

```bash
mkdir -p target/hl7v2-sidecar/bundles
mkdir -p target/hl7v2-sidecar/quarantine
mkdir -p target/hl7v2-sidecar/reports
```

For PowerShell:

```powershell
New-Item -ItemType Directory -Force target/hl7v2-sidecar/bundles | Out-Null
New-Item -ItemType Directory -Force target/hl7v2-sidecar/quarantine | Out-Null
New-Item -ItemType Directory -Force target/hl7v2-sidecar/reports | Out-Null
```

## 1. Configure the Sidecar

Create a local server config:

```toml
# target/hl7v2-sidecar/server.toml
[server]
host = "127.0.0.1"
port = 18080
bundle_output_root = "target/hl7v2-sidecar/bundles"

[ack]
mode = "original"
accept_on = "valid"
reject_on = ["parse_error", "validation_error"]
include_error_text = true

[quarantine]
enabled = true
path = "target/hl7v2-sidecar/quarantine"
write_redacted = true
write_report = true
write_bundle = true
```

Keep the API key out of the file for local proof:

```bash
export HL7V2_CONFIG=target/hl7v2-sidecar/server.toml
export HL7V2_API_KEY=dev-secret
export HL7V2_PROFILE_PATHS=profiles/generic.yaml
```

For PowerShell:

```powershell
$env:HL7V2_CONFIG = "target/hl7v2-sidecar/server.toml"
$env:HL7V2_API_KEY = "dev-secret"
$env:HL7V2_PROFILE_PATHS = "profiles/generic.yaml"
```

Create a safe-analysis policy:

```toml
# target/hl7v2-sidecar/safe-analysis.toml
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
path = "PID.8"
action = "retain"
reason = "administrative sex is required to reproduce validation"
```

## 2. Print Sanitized Configuration

Before starting the server, inspect the effective config:

```bash
hl7v2-server --print-config
```

Expected fields:

```json
{
  "bind_address": "127.0.0.1:18080",
  "max_body_size": 10485760,
  "api_key_configured": true,
  "profile_paths": [
    "profiles/generic.yaml"
  ],
  "config_source": "target/hl7v2-sidecar/server.toml",
  "bundle_output_root_configured": true,
  "ack_policy": {
    "mode": "original",
    "accept_on": "valid",
    "reject_on": [
      "parse_error",
      "validation_error"
    ],
    "include_error_text": true
  },
  "quarantine": {
    "enabled": true,
    "path_configured": true,
    "write_redacted": true,
    "write_report": true,
    "write_bundle": true
  }
}
```

The API key value should not appear. If `api_key_configured` is `false`, `/hl7/*`
routes are public; do not deploy that way unless the network boundary is doing
the authentication.

## 3. Start the Server

Run the sidecar:

```bash
hl7v2-server
```

From a source checkout:

```bash
cargo run -q -p hl7v2-server
```

The server binds `127.0.0.1:18080` from the config above.

For a container smoke check from the repository root, start Docker first and use
the checked-in Compose stack:

```bash
docker compose -f infrastructure/docker/docker-compose.yml up --build -d
python tests/server_smoke/smoke.py
docker compose -f infrastructure/docker/docker-compose.yml down -v
```

The smoke script exercises health, readiness, redacted validation, bundle,
replay, and corpus diff against the running sidecar.

## 4. Check Readiness

Readiness is the deployment gate:

```bash
curl http://127.0.0.1:18080/ready
```

Expected checks include:

```json
{
  "ready": true,
  "status": "ready",
  "checks": [
    {
      "name": "config",
      "status": "pass"
    },
    {
      "name": "configured_profiles",
      "status": "pass"
    },
    {
      "name": "bundle_output_root",
      "status": "pass"
    },
    {
      "name": "quarantine_output",
      "status": "pass"
    },
    {
      "name": "validation_report",
      "status": "pass"
    }
  ]
}
```

If `/ready` returns `503`, do not send traffic. Fix the failed check first:
profile path, bundle root, quarantine root, bind address, or validation-report
self-check. The public readiness response identifies failed configured profile
checks by position instead of echoing local profile paths; use `--print-config`
locally when you need to inspect the configured path list.

## 5. Validate After Redaction

Use the same request shape in scripts, CI, and sidecar smoke checks. This
PowerShell example builds the JSON body from local files:

```powershell
$message = Get-Content test_data/invalid_message.hl7 -Raw
$profile = Get-Content profiles/generic.yaml -Raw
$policy = Get-Content target/hl7v2-sidecar/safe-analysis.toml -Raw

$body = @{
    message = $message
    profile = $profile
    redaction_policy = $policy
    include_redacted_hl7 = $false
} | ConvertTo-Json -Depth 8

Invoke-RestMethod `
    -Method Post `
    -Uri http://127.0.0.1:18080/hl7/validate-redacted `
    -Headers @{ "X-API-Key" = "dev-secret" } `
    -ContentType "application/json" `
    -Body $body |
    ConvertTo-Json -Depth 20 |
    Set-Content target/hl7v2-sidecar/reports/validate-redacted.json
```

Expected fields:

```json
{
  "validation_report": {
    "valid": false,
    "message_type": "ADT^A01",
    "issue_count": 1,
    "issues": [
      {
        "code": "value_not_in_set",
        "path": "PID.8",
        "severity": "error"
      }
    ]
  },
  "redaction_receipt": {
    "phi_removed": true
  },
  "quarantine": {
    "quarantine_version": "1",
    "reason": "validation_error",
    "validation_issue_count": 1
  }
}
```

Because quarantine is enabled, failed validation writes redacted artifacts under
the configured quarantine root. The response reports a root-relative output id,
not the server filesystem path.

## 6. Inspect Inline Corpus Drift

The corpus endpoints accept inline message bodies. They do not accept server
filesystem paths, so callers can use them from CI or a migration script without
granting read access to arbitrary server locations.

```powershell
$before = Get-Content test_data/invalid_message.hl7 -Raw
$after = Get-Content test_data/valid_message.hl7 -Raw

$body = @{
    before = @(
        @{ id = "before-1"; message = $before }
    )
    after = @(
        @{ id = "after-1"; message = $after }
    )
    profile = $profile
    diff_schema_version = 2
} | ConvertTo-Json -Depth 8

Invoke-RestMethod `
    -Method Post `
    -Uri http://127.0.0.1:18080/hl7/corpus/diff `
    -Headers @{ "X-API-Key" = "dev-secret" } `
    -ContentType "application/json" `
    -Body $body |
    ConvertTo-Json -Depth 20 |
    Set-Content target/hl7v2-sidecar/reports/corpus-diff.json
```

Expected fields:

```json
{
  "schema_version": "2",
  "tool_name": "hl7v2-server",
  "before_root": "<inline-before>",
  "after_root": "<inline-after>",
  "message_count": {
    "before": 1,
    "after": 1,
    "delta": 0
  },
  "field_presence": [
    {
      "path": "PID.5",
      "message_count_delta": 1
    }
  ],
  "validation_issue_code_counts": [
    {
      "value": "value_not_in_set",
      "before": 1,
      "after": 0,
      "delta": -1
    }
  ]
}
```

Use `/hl7/corpus/summarize` for aggregate counts and
`/hl7/corpus/fingerprint` for a deterministic feed signature. In this fixture,
the after message is cleaner than the before message: `PID.5` appears and the
`PID.8` value-set issue disappears.

## 7. Create a Server-Side Evidence Bundle

The bundle endpoint writes under `bundle_output_root`. The request supplies a
safe `bundle_id`, not an arbitrary filesystem path:

```powershell
$body = @{
    bundle_id = "case-001"
    message = $message
    profile = $profile
    redaction_policy = $policy
} | ConvertTo-Json -Depth 8

Invoke-RestMethod `
    -Method Post `
    -Uri http://127.0.0.1:18080/hl7/bundle `
    -Headers @{ "X-API-Key" = "dev-secret" } `
    -ContentType "application/json" `
    -Body $body |
    ConvertTo-Json -Depth 20 |
    Set-Content target/hl7v2-sidecar/reports/bundle-summary.json
```

Expected fields:

```json
{
  "bundle_version": "1",
  "output_dir": "case-001",
  "message_type": "ADT^A01",
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

If the endpoint returns `503 BUNDLE_OUTPUT_NOT_CONFIGURED`, set
`[server].bundle_output_root` or `HL7V2_BUNDLE_OUTPUT_ROOT` to an existing
writable directory and restart the server.

## 8. Replay the Server Bundle

Replay verifies bundle integrity and regenerates the validation report from the
stored redacted message and profile:

```powershell
$body = @{
    bundle_id = "case-001"
    replay_report_schema_version = 2
} | ConvertTo-Json -Depth 8

Invoke-RestMethod `
    -Method Post `
    -Uri http://127.0.0.1:18080/hl7/replay `
    -Headers @{ "X-API-Key" = "dev-secret" } `
    -ContentType "application/json" `
    -Body $body |
    ConvertTo-Json -Depth 20 |
    Set-Content target/hl7v2-sidecar/reports/replay-report.json
```

Expected fields:

```json
{
  "schema_version": "2",
  "replay_version": "1",
  "message_type": "ADT^A01",
  "reproduced": true,
  "validation_valid": false,
  "validation_issue_count": 1,
  "checks": [
    {
      "name": "manifest-hashes",
      "status": "pass"
    },
    {
      "name": "report-match",
      "status": "pass"
    },
    {
      "name": "environment-match",
      "status": "pass"
    }
  ]
}
```

If replay does not reproduce, treat the packet as untrusted. The report tells
you whether the failure is a missing artifact, manifest/hash mismatch,
parse/profile problem, report drift, or environment mismatch.

## 9. Generate ACK/NAK from Policy

Use `/hl7/ack-policy` when the sidecar needs to decide an ACK from the same
validation evidence:

```powershell
$body = @{
    message = $message
    profile = $profile
    mllp_framed = $false
    mllp_frame = $false
} | ConvertTo-Json -Depth 8

Invoke-RestMethod `
    -Method Post `
    -Uri http://127.0.0.1:18080/hl7/ack-policy `
    -Headers @{ "X-API-Key" = "dev-secret" } `
    -ContentType "application/json" `
    -Body $body |
    ConvertTo-Json -Depth 20 |
    Set-Content target/hl7v2-sidecar/reports/ack-policy.json
```

For the invalid sample, expected fields include:

```json
{
  "ack_code": "AR",
  "decision": {
    "mode": "original",
    "outcome": "rejected",
    "reason": "validation_error"
  },
  "validation_report": {
    "valid": false,
    "issue_count": 1
  }
}
```

Enhanced mode uses `CA` and `CR` instead of `AA` and `AR`.

## 10. Observe the Sidecar

Check liveness and metrics:

```bash
curl http://127.0.0.1:18080/health
curl http://127.0.0.1:18080/metrics
```

Use `/health` for process liveness. Use `/ready` for deployment readiness. Use
the evidence endpoints for interface decisions.

The Prometheus metrics contract is intentionally small and low-cardinality:

```text
hl7v2_requests_total
hl7v2_request_duration_seconds
hl7v2_messages_parsed_total
hl7v2_messages_validated_total
hl7v2_message_size_bytes
hl7v2_parse_failures_total
hl7v2_validation_failures_total
hl7v2_redaction_failures_total
hl7v2_bundles_created_total
hl7v2_replays_total
hl7v2_replay_failures_total
hl7v2_corpus_diffs_total
```

Metric labels use bounded operation/status values. They do not include raw HL7
payloads, profile YAML, redaction policies, local filesystem roots, raw bundle
IDs, raw message control IDs, or patient identifiers.

The server also emits structured evidence workflow logs for parse, validate,
validate-redacted, bundle, replay, ACK, and ACK-policy requests. The useful
fields are intentionally operational rather than payload-bearing:

```text
event
message_type
message_control_id_hash
correlation_id
validation_status
issue_count
redaction_status
bundle_id_hash
quarantine_output_id
ack_code
ack_outcome
ack_reason
```

The raw HL7 message, raw `MSH.10` control ID, profile YAML, redaction policy,
bundle output root, and quarantine root are not logged by default. Use the
bundle, quarantine, and replay artifacts when you need deeper evidence.

## Deployment Notes

- Keep `HL7V2_API_KEY` configured for `/hl7/*` routes.
- Put TLS, network policy, and external authentication at the edge; the local
  server examples are not a public internet deployment.
- Mount profile, bundle, and quarantine directories explicitly.
- Treat `/ready` failure as a deployment stop.
- Do not log or forward raw request bodies unless a separate deployment policy
  explicitly allows it.
- Keep redaction policies fail-closed; a rejected policy is safer than a leaky
  bundle.
- Replay server-created bundles with `POST /hl7/replay` or
  `hl7v2 replay <bundle-dir> --format json` before attaching them to tickets.

## Workflow Summary

```text
ingress
  -> parse
  -> safe-analysis redaction
  -> validation report
  -> ACK/NAK decision
  -> quarantine or evidence bundle
  -> metrics and readiness
```

The sidecar is useful because it keeps the answer narrow: it does not route the
enterprise, transform every feed, or replace your interface engine. It creates
stable evidence at the boundary so operators can decide what should happen next.
