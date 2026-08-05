# hl7v2-server

HTTP/gRPC runtime service for HL7 v2 validation and evidence workflows.

`hl7v2-server` is the deployable sidecar/edge-guard crate in the
[`hl7v2-rs`](https://github.com/EffortlessMetrics/hl7v2-rs) workspace. It is
separate from the `hl7v2` library crate because it carries runtime concerns:
HTTP/gRPC serving, metrics, authentication, rate limits, config loading, and
filesystem output roots.

## Sidecar workflows

Useful release-candidate workflows include:

- `hl7v2-server --print-config` for sanitized effective configuration output.
- `GET /ready` for startup readiness checks.
- `POST /hl7/validate` for typed validation reports.
- `POST /hl7/validate-redacted` for validate-and-redact workflows.
- `POST /hl7/bundle` for server-created redacted evidence bundles under a
  configured bundle output root.
- `POST /hl7/ack-policy` for policy-driven ACK/NAK decisions.
- `/metrics` for Prometheus metrics; it requires `X-API-Key` when
  `HL7V2_API_KEY` is configured, while `/health` and `/ready` remain public.

The gRPC service implements `Parse`, `ParseStream`, `Validate`, `ProfileLint`,
`ProfileExplain`, `ProfileTest`, `ValidateRedacted`, `CreateEvidenceBundle`,
`ReplayEvidenceBundle`, `CorpusSummarize`, `CorpusFingerprint`, `CorpusDiff`, `GenerateAck`,
`Normalize`, and `HealthCheck`.
`Validate` and `ValidateRedacted` return the shared validation report fields;
`ProfileLint`, `ProfileExplain`, and `ProfileTest` return the shared profile
report fields;
`ValidateRedacted` also returns a redaction receipt, keeps the redacted HL7
payload opt-in, and can write configured quarantine output when redacted
validation fails.
`CreateEvidenceBundle` writes a redacted bundle under the configured server
bundle root and returns a shared bundle summary with a hashed public output ID.
`ReplayEvidenceBundle` verifies configured-root bundles and returns the shared
replay report shape with opt-in v2 replay provenance.
`CorpusSummarize`, `CorpusFingerprint`, and `CorpusDiff` accept inline messages
only and can return v2 corpus provenance shapes with
`summary_schema_version = 2`, `fingerprint_schema_version = 2`, or
`diff_schema_version = 2`. See
[GRPC_IMPLEMENTATION.md](GRPC_IMPLEMENTATION.md) for the current protobuf and
test contract notes.

The stable Prometheus contract includes HTTP request metrics plus evidence-loop
counters for parse failures, validation failures, redaction failures, bundles
created, replay attempts/failures, and inline corpus diffs. Labels are bounded
operation/status values only; raw HL7 payloads, profile YAML, redaction policies,
local paths, raw bundle IDs, raw message control IDs, and patient identifiers
are not metric labels.

The server reuses the same evidence contracts as the CLI and `hl7v2` library
instead of defining a server-only report language.

### CORS and authentication

When `HL7V2_CORS_ALLOWED_ORIGINS` is unset, the server allows any origin,
method, and header for browser requests and emits a startup warning. Set the
variable to a comma-separated list of trusted origins to narrow that policy:

```bash
HL7V2_CORS_ALLOWED_ORIGINS="https://app.example,https://ops.example" \\
  hl7v2-server
```

CORS does not authenticate requests. Configure `HL7V2_API_KEY`, network
controls, and TLS termination independently. Invalid configured origins are
rejected as configuration errors; a manually constructed invalid state fails
closed by disabling CORS rather than panicking.

## Public Rust API boundary

The supported Rust integration surface is the root-level API: `Server`,
`ServerBuilder`, `ServerConfig`, `AppState`, `build_router`, and the documented
model types. The `handlers` module is retained as an internal compatibility
surface, and `handlers::AppError` is hidden and deprecated for older consumers;
new code must not depend on either surface.

Request failures are supported through the stable HTTP and gRPC response
contracts, including the externally observable message-size status/code
contracts. `handlers::AppError` is not a supported Rust extension point, so
future request-error cases must not be added as public enum variants. This
keeps structured request-error evolution semver-safe for the published server
crate while preserving the existing server integration surface.

Structured logs are emitted for parse, validation, redacted validation, bundle,
replay, ACK, and ACK-policy workflows. They include message type, validation
status, issue counts, redaction status, and hashed correlation identifiers. Raw
HL7 payloads, raw message control IDs, profile YAML, redaction policies, and
local filesystem roots are not logged by default.

Set `RUST_LOG_FORMAT=json` to emit these fields as JSON records. The default is
human-readable text logs.

## Usage

Inspect effective configuration without leaking secrets:

```bash
hl7v2-server --print-config
```

Run readiness after startup:

```bash
curl http://127.0.0.1:8080/ready
```

Create a redacted validation report:

```bash
curl -X POST http://127.0.0.1:8080/hl7/validate-redacted \
  -H "Content-Type: application/json" \
  -d '{
    "message": "MSH|^~\\&|App||Fac||20250128||ADT^A01|123|P|2.5.1\rPID|1||MRN123||Doe^John\r",
    "profile": "name: ADT_A01\nmessage_type: ADT^A01\nsegments: []\n",
    "redaction_policy": "[[rules]]\npath = \"PID.3\"\naction = \"hash\"\nreason = \"patient identifier\"\n[[rules]]\npath = \"PID.5\"\naction = \"drop\"\nreason = \"patient name\"\n"
  }'
```

For a full operator walkthrough, see the
[Deploy Validation Sidecar guide](https://github.com/EffortlessMetrics/hl7v2-rs/blob/main/docs/guides/deploy-validation-sidecar.md).
For source examples, see the
[examples/](https://github.com/EffortlessMetrics/hl7v2-rs/tree/main/examples)
directory in the root of the repository.
