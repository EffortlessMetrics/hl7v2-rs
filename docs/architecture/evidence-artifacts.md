# Evidence Artifacts

This document maps the current evidence-loop artifacts and their contract
status. It is a current-state reference for the schema, golden-fixture, CLI
output-contract, server parity, redacted server logging, and Python parity
surfaces in the v1.4.0 Evidence Contracts and Server Sidecar release line and
current main.
For a compact schema/fixture/producer map, see the
[Evidence Contract Index](../contracts/evidence-contract-index.md).

## Product Loop

The evidence loop is:

```text
raw feed / message
  -> doctor / environment proof
  -> profile lint
  -> profile test / explain
  -> validation report
  -> corpus summarize
  -> corpus fingerprint
  -> corpus diff
  -> safe-analysis redact
  -> redacted evidence bundle
  -> replay verification
  -> server sidecar evidence
  -> Python data workflow
```

The target is deterministic, machine-readable proof of what arrived, what
failed, what changed, what was redacted, and how to replay the result.

## Artifact Inventory

| Artifact | Producer | Primary type | Contract status |
| --- | --- | --- | --- |
| Doctor report | `hl7v2 doctor --format json|yaml|text` | CLI-local `DoctorReport` plus opt-in `DoctorReportV2` wrapper | Has tool `version`; JSON Schema exists at `schemas/evidence/doctor-report-v1.schema.json`. CLI doctor output can emit the target v2 schema with `hl7v2 doctor --format json --schema-version 2`, adding embedded `schema_version`, `tool_name`, and `tool_version`. Defaults remain v1-compatible. |
| Validation report | `hl7v2 val --report json|yaml|text`, Rust `ValidationReport`, Python `report.to_dict()` / `report.to_json()`, server REST `/hl7/validate` and `/hl7/validate-redacted` fields, gRPC `ValidateResponse` and `ValidateRedactedResponse` | `hl7v2::ValidationReport` plus opt-in `ValidationReportV2` conversion | Shared v1 report across Rust/CLI/Python and embedded in REST and gRPC validation responses; JSON Schema exists at `schemas/evidence/validation-report-v1.schema.json`. CLI validation can emit the target v2 schema with `hl7v2 val --report json --schema-version 2`, Python can emit it with `report.to_dict(2)` / `report.to_json(2)`, REST validation can include it as `validation_report_v2` when requests set `report_schema_version` to `2`, and gRPC validation can include it when `ValidateRequest.report_schema_version` or `ValidateRedactedRequest.report_schema_version` is `2`. Defaults remain v1-compatible. |
| Profile lint report | `hl7v2 profile lint --report json|yaml|text`, Rust `lint_profile_yaml` | `hl7v2::ProfileLintReport` plus opt-in `ProfileLintReportV2` conversion | Library type; JSON Schema exists at `schemas/evidence/profile-lint-report-v1.schema.json`. CLI lint output can emit the target v2 schema with `hl7v2 profile lint --report json --schema-version 2`. Defaults remain v1-compatible. |
| Profile test report | `hl7v2 profile test --report json|yaml|text` | CLI-local `ProfileTestReport` plus opt-in `ProfileTestReportV2` wrapper | Includes validation reports per case; JSON Schema exists at `schemas/evidence/profile-test-report-v1.schema.json`. CLI test output can emit the target v2 schema with `hl7v2 profile test --report json --schema-version 2`. Defaults remain v1-compatible. |
| Profile explain report | `hl7v2 profile explain --format json|yaml|text` | CLI-local `ProfileExplainReport` plus opt-in `ProfileExplainReportV2` wrapper | Includes profile SHA-256 and loaded profile version; JSON Schema exists at `schemas/evidence/profile-explain-report-v1.schema.json`. CLI explain output can emit the target v2 schema with `hl7v2 profile explain --format json --schema-version 2`. Defaults remain v1-compatible. |
| Corpus summary | `hl7v2 corpus summarize --format json|yaml|text`, Rust corpus module, Python `corpus_summary()`, server REST `/hl7/corpus/summarize`, gRPC `CorpusSummarize` | `hl7v2::synthetic::corpus::CorpusSummary` plus opt-in `CorpusSummaryV2` conversion | Library type; JSON Schema exists at `schemas/evidence/corpus-summary-v1.schema.json`. CLI summary output can emit the target v2 schema with `hl7v2 corpus summarize --format json --schema-version 2`, Python can emit it with `corpus_summary(..., schema_version=2)`, REST can emit it with `summary_schema_version: 2`, and gRPC can emit it with `CorpusSummarizeRequest.summary_schema_version = 2`. Defaults remain v1-compatible. |
| Corpus fingerprint | `hl7v2 corpus fingerprint --format json|yaml|text`, Rust corpus module, Python `corpus_fingerprint()`, server `/hl7/corpus/fingerprint` | `hl7v2::synthetic::corpus::CorpusFingerprint` plus opt-in `CorpusFingerprintV2` conversion | Has `fingerprint_version`, `tool_version`, optional profile SHA-256 metadata, and JSON Schema at `schemas/evidence/corpus-fingerprint-v1.schema.json`. CLI fingerprint output can emit the target v2 schema with `hl7v2 corpus fingerprint --format json --schema-version 2`, Python can emit it with `corpus_fingerprint(..., schema_version=2)`, and the server can emit it with `fingerprint_schema_version: 2`. Defaults remain v1-compatible. |
| Corpus diff report | `hl7v2 corpus diff --format json|yaml|text`, Rust corpus module, Python `corpus_diff()`, server `/hl7/corpus/diff` | `hl7v2::synthetic::corpus::CorpusDiffReport` plus opt-in `CorpusDiffReportV2` conversion | Has `diff_version`, `tool_version`, optional profile SHA-256 metadata, and JSON Schema at `schemas/evidence/corpus-diff-v1.schema.json`. CLI diff output can emit the target v2 schema with `hl7v2 corpus diff --format json --schema-version 2`, Python can emit it with `corpus_diff(..., schema_version=2)`, and the server can emit it with `diff_schema_version: 2`. Defaults remain v1-compatible. |
| Redaction output | `hl7v2 redact --format json`, Python `redact()` | `hl7v2::redact::SafeAnalysisRedactionOutput` plus opt-in `SafeAnalysisRedactionOutputV2` conversion for Python; CLI-local v1 and v2 wrappers match the same schema fields | Has input and policy SHA-256 values plus a nested receipt. JSON Schema exists at `schemas/evidence/safe-analysis-redaction-output-v1.schema.json`. CLI and Python redaction output can emit the target v2 schema with `hl7v2 redact --format json --schema-version 2` and `redact(..., schema_version=2)`. Defaults remain v1-compatible. |
| Redaction receipt | `hl7v2 redact --format json --schema-version 2`, Python `redact(..., schema_version=2)`, REST `/hl7/validate-redacted` with `redaction_receipt_schema_version: 2`, gRPC `ValidateRedacted` with `redaction_receipt_schema_version = 2`; bundle `redaction-receipt.json` with bundle artifact schema version 2 | `hl7v2::redact::RedactionReceipt` plus opt-in `RedactionReceiptV2` conversion; CLI/server-local shapes still match the same schema | Captures actions and PHI removal status; JSON Schemas exist at `schemas/evidence/redaction-receipt-v1.schema.json` and `schemas/evidence/redaction-receipt-v2.schema.json`. v2 adds `schema_version`, `tool_name`, and `tool_version`; defaults remain v1-compatible. |
| Field path trace | Bundle `field-paths.json`, Python `bundle()` artifact | `hl7v2::evidence::FieldPathTraceReport` plus opt-in `FieldPathTraceReportV2` conversion for Python; CLI/server-local shapes still match the same fields | Captures redacted message field paths, value shapes, and configured redaction actions; v1 and v2 JSON Schemas exist. CLI/Python bundles can opt into the v2 artifact shape with bundle `schema_version = 2`; server bundles can opt in with `bundle_artifact_schema_version = 2`. Defaults remain v1-compatible. |
| Evidence bundle summary | `hl7v2 bundle ...` stdout; Python `bundle()`; server `/hl7/bundle` response | `hl7v2::evidence::EvidenceBundleSummary` plus opt-in `EvidenceBundleSummaryV2` conversion for Python; CLI/server-local v1 shapes still match the same schema | Has `bundle_version`; JSON Schemas exist at `schemas/evidence/evidence-bundle-v1.schema.json` and `schemas/evidence/evidence-bundle-v2.schema.json`. CLI bundle output can emit the target v2 schema with `hl7v2 bundle ... --schema-version 2`, and Python can emit it with `bundle(..., schema_version=2)`. Defaults remain v1-compatible; server reports the configured-root-relative bundle id, Python reports `.` to avoid exposing a local filesystem path. |
| Quarantine output summary | Server `/hl7/validate-redacted` response when `[quarantine]` is enabled and validation fails | Server-local `QuarantineOutputSummary` plus opt-in `QuarantineOutputSummaryV2` conversion | Has `quarantine_version`, root-relative output id, reason, issue count, artifact names, and JSON Schemas at `schemas/evidence/quarantine-output-v1.schema.json` and `schemas/evidence/quarantine-output-v2.schema.json`. Server responses can include the target v2 shape as `quarantine_v2` when requests set `quarantine_schema_version` to `2`. It does not expose the configured filesystem path. Full-bundle quarantine output reuses the evidence bundle artifact set. |
| Evidence bundle manifest | Bundle `manifest.json` | `hl7v2::evidence::EvidenceBundleManifest` plus opt-in `EvidenceBundleManifestV2` conversion for Python; CLI/server-local shapes still match the same schema | Records `bundle_version`, `tool_name`, `tool_version`, bundle-relative artifact paths, roles, and SHA-256 hashes; v1 and v2 JSON Schemas exist. CLI/Python bundles can opt into the v2 artifact shape with bundle `schema_version = 2`; server bundles can opt in with `bundle_artifact_schema_version = 2`. Replay verifies both v1 and v2 manifest catalogs and hashes before using artifacts. |
| Evidence bundle environment | Bundle `environment.json` | `hl7v2::evidence::EvidenceBundleEnvironment` plus opt-in `EvidenceBundleEnvironmentV2` conversion for Python; CLI/server-local shapes still match the same fields | Has `bundle_version`, `tool_name`, `tool_version`, input/profile/policy hashes, validation summary, and replay command; v1 and v2 JSON Schemas exist. CLI/Python bundles can opt into the v2 artifact shape with bundle `schema_version = 2`; server bundles can opt in with `bundle_artifact_schema_version = 2`. Defaults remain v1-compatible. |
| Evidence replay report | `hl7v2 replay --format json|yaml|text`, Python `replay()`, server `/hl7/replay` | `hl7v2::evidence::EvidenceReplayReport` plus opt-in `EvidenceReplayReportV2` conversion for Python and server responses; CLI-local v1 shape still matches the same schema | Has `replay_version`, `bundle_version`, `tool_name`, `tool_version`, replay checks, optional regenerated validation report, and JSON Schemas at `schemas/evidence/evidence-replay-v1.schema.json` and `schemas/evidence/evidence-replay-v2.schema.json`. CLI replay output can emit the target v2 schema with `hl7v2 replay ... --format json --schema-version 2`, Python can emit it with `replay(..., schema_version=2)`, and server `/hl7/replay` can emit it when requests set `replay_report_schema_version` to `2`. Defaults remain v1-compatible; replay fails closed on malformed manifests, missing artifacts, and hash mismatches. |

## Current Parity

| Surface | Current parity |
| --- | --- |
| Rust library | Owns the shared `ValidationReport`, `ProfileLintReport`, corpus summary/fingerprint/diff types, redaction output/receipt types, and Python-facing evidence bundle/replay types. Profile test and profile explain report types are currently CLI-local. |
| CLI | Produces the complete evidence loop today. Most commands support JSON/YAML/text. `redact` supports JSON or HL7 output. `bundle` currently emits JSON summary only. |
| Server | REST `/hl7/validate` and gRPC `Validate` expose shared validation report fields while preserving legacy `errors` / `warnings`, and can include `validation_report_v2` when their report schema version field is `2`; gRPC `ParseStream` parses one request message into one response message and reports per-message parse errors without failing the whole stream; REST `/hl7/validate-redacted` and gRPC `ValidateRedacted` apply safe-analysis redaction, return a validation report plus redaction receipt, can include the same v2 validation and redaction receipt artifacts, and keep redacted HL7 payloads opt-in; REST `/hl7/validate-redacted` can also write configured quarantine output for failed validation; REST `/hl7/corpus/summarize`, `/hl7/corpus/fingerprint`, and `/hl7/corpus/diff` produce inline corpus evidence without reading request-supplied filesystem paths; gRPC `CorpusSummarize` covers the inline corpus summary slice with the same no-filesystem-path boundary and opt-in v2 provenance; `/hl7/bundle` writes redacted evidence bundles under a configured server root and can opt into v2 bundle-internal artifacts; `/hl7/replay` verifies configured-root bundles and can opt into the v2 replay report shape; `/hl7/ack-policy` returns policy-driven ACK/NAK decisions backed by validation reports; evidence workflow logs hash message-control and bundle identifiers and avoid raw HL7, profile YAML, redaction policy TOML, and configured filesystem roots by default. |
| Python | Exposes parse, JSON conversion, normalize, validation report dict/JSON parity, corpus summary/fingerprint/diff dict parity, safe-analysis redaction dict output, evidence bundle creation, and replay verification. Validation, corpus, redaction, bundle, and replay APIs support opt-in v2 shapes where those contracts exist. |

`ValidationReport.profile` is a surface-local display label, not a canonical
profile identity. The CLI uses the supplied profile path, while the server and
Python binding use the loaded profile `message_structure`. Consumers that need
reproducible profile identity should use artifacts with profile SHA-256
metadata. The report fields are otherwise shared through `hl7v2::ValidationReport`.

## Output Semantics

The CLI output contract is stable enough for CI and automation:

- JSON/YAML output goes to stdout as the primary machine-readable artifact.
- Human text output also goes to stdout.
- Evidence report commands support `--output <path>` to write the same artifact
  to a file while keeping stdout quiet.
- `--quiet` suppresses non-error diagnostics; top-level errors still use stderr.
- `--no-color` is accepted by evidence commands so automation can opt out of
  colored diagnostics as formatting evolves.
- Diagnostics and top-level errors are written to stderr by the global error
  path.
- `hl7v2 redact --format hl7` writes the redacted message to stdout and a short
  receipt to stderr.

```text
0 = success
1 = validation/profile/evidence failed
2 = parse/config/profile/policy input error
3 = IO/runtime/environment error
```

Evidence JSON null, omitted-field, and empty-array semantics are documented in
[`schemas/README.md`](../../schemas/README.md#evidence-null-and-empty-semantics).
The provenance/versioning compatibility rules are maintained in
[`evidence-provenance-versioning.md`](evidence-provenance-versioning.md).

## Remaining Contract Hardening Gaps

The v1.4.0 release line and current main have schema-backed JSON artifacts,
golden fixtures, CLI output semantics, bundle manifest verification, server
edge-guard routes, redacted structured server logs, Python parity, workflow
guides, and a maintained evidence-schema gate. Remaining hardening work should
keep provenance and identity explicit as new artifacts or producer surfaces are
added:

- `schema_version` or artifact-specific version naming for every machine
  artifact. The compatibility plan is documented in
  [`evidence-provenance-versioning.md`](evidence-provenance-versioning.md);
  remaining implementation should continue through explicit v2 schema/type work.
- `tool_version` where users need provenance outside an environment file.
- Shared library types for any CLI-local report promoted beyond CLI-only use.

## Non-Goals

This document does not define new runtime behavior. It records the current
evidence artifacts and the gaps that should be addressed by follow-up PRs.
