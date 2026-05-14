# gRPC Server Implementation

This crate implements the `HL7Service` gRPC API for HL7 v2 parsing,
validation, normalization, ACK generation, streaming parse, and redacted
validation and corpus evidence workflows.

## Contract Source

The canonical protobuf contract is:

```text
api/proto/hl7v2/v1/hl7v2.proto
```

The packaged crate copy is:

```text
crates/hl7v2-server/proto/hl7v2/v1/hl7v2.proto
```

`build.rs` uses the workspace contract when it is available and falls back to
the packaged copy for crate packaging. The packaging drift test verifies both
copies stay synchronized.

## Implemented RPCs

| RPC | Status | Notes |
| --- | --- | --- |
| `Parse` | Implemented | Parses raw or MLLP-framed HL7 bytes into protobuf message fields and metadata. |
| `ParseStream` | Implemented | Parses one request message into one response message; per-message parse or MLLP failures return error payloads without failing the whole stream. Malformed gRPC frames still return a tonic `Status`. |
| `Validate` | Implemented | Validates raw or MLLP-framed HL7 against an inline profile. Preserves legacy `valid`, `errors`, `warnings`, and `summary` fields while also returning `validation_report`; `report_schema_version = 2` adds `validation_report_v2`. |
| `ValidateRedacted` | Implemented | Applies an inline safe-analysis redaction policy before validation. Always returns v1 `validation_report` and `redaction_receipt`, can include v2 validation and redaction receipt artifacts, and includes `redacted_hl7` only when requested. |
| `CorpusSummarize` | Implemented | Summarizes caller-supplied inline messages only. The RPC does not read filesystem paths from requests; `summary_schema_version = 2` adds the v2 corpus summary provenance shape. |
| `GenerateAck` | Implemented | Generates ACK messages using the canonical `hl7v2` ACK facade. |
| `Normalize` | Implemented | Normalizes delimiter output and can optionally MLLP-frame the response. |
| `HealthCheck` | Implemented | Reports serving status and crate version. |

## Evidence Semantics

The gRPC service keeps v1-compatible response fields by default. Provenance
fields are additive and opt in:

```text
ValidateRequest.report_schema_version = 2
ValidateRedactedRequest.report_schema_version = 2
ValidateRedactedRequest.redaction_receipt_schema_version = 2
CorpusSummarizeRequest.summary_schema_version = 2
```

For `Validate`, `ValidateRedacted`, and `CorpusSummarize`, schema versions `0`
and `1` use the default v1 shape, `2` returns the requested v2 artifact, and
other values return
`InvalidArgument`.

`ValidateRedacted` fails closed when the redaction policy is invalid or misses a
present built-in sensitive path. The redacted HL7 body is omitted unless
`include_redacted_hl7` is set. The RPC does not write quarantine output; REST
`/hl7/validate-redacted` owns configured quarantine behavior.

## Validation

Useful checks for gRPC contract changes:

```powershell
npx -y @bufbuild/buf lint api/proto
npx -y @bufbuild/buf breaking api/proto --against "https://github.com/EffortlessMetrics/hl7v2-rs.git#branch=main,subdir=api/proto"
cargo +1.95.0 test -p hl7v2-server --test proto_packaging_test
cargo +1.95.0 test -p hl7v2-server --test grpc_contract_tests
cargo +1.95.0 check -p hl7v2-server --all-features --all-targets
cargo +1.95.0 clippy -p hl7v2-server --all-targets -- -D warnings
```
