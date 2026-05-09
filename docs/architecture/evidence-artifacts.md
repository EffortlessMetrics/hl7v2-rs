# Evidence Artifacts

This document maps the current evidence-loop artifacts and their contract
status. It is a current-state reference for the schema, golden-fixture, CLI
output-contract, server parity, and Python parity surfaces in the v1.3.0
evidence loop.

## Product Loop

The evidence loop is:

```text
raw feed / message
  -> profile lint
  -> profile test / explain
  -> validation report
  -> corpus summarize
  -> corpus fingerprint
  -> corpus diff
  -> safe-analysis redact
  -> redacted evidence bundle
  -> replay verification
```

The target is deterministic, machine-readable proof of what arrived, what
failed, what changed, what was redacted, and how to replay the result.

## Artifact Inventory

| Artifact | Producer | Primary type | Contract status |
| --- | --- | --- | --- |
| Doctor report | `hl7v2 doctor --format json|yaml|text` | CLI-local `DoctorReport` | Has tool `version`; no `schema_version`; no JSON Schema. |
| Validation report | `hl7v2 val --report json|yaml|text`, Rust `ValidationReport`, Python `report.to_dict()` / `report.to_json()`, server `/hl7/validate` and `/hl7/validate-redacted` fields | `hl7v2::ValidationReport` plus opt-in `ValidationReportV2` conversion | Shared v1 report across Rust/CLI/Python and embedded in server responses; JSON Schema exists at `schemas/evidence/validation-report-v1.schema.json`. CLI validation can emit the target v2 schema with `hl7v2 val --report json --schema-version 2`, Python can emit it with `report.to_dict(2)` / `report.to_json(2)`, and server validation can include it as `validation_report_v2` when requests set `report_schema_version` to `2`. Defaults remain v1-compatible. |
| Profile lint report | `hl7v2 profile lint --report json|yaml|text`, Rust `lint_profile_yaml` | `hl7v2::ProfileLintReport` plus opt-in `ProfileLintReportV2` conversion | Library type; JSON Schema exists at `schemas/evidence/profile-lint-report-v1.schema.json`. CLI lint output can emit the target v2 schema with `hl7v2 profile lint --report json --schema-version 2`. Defaults remain v1-compatible. |
| Profile test report | `hl7v2 profile test --report json|yaml|text` | CLI-local `ProfileTestReport` plus opt-in `ProfileTestReportV2` wrapper | Includes validation reports per case; JSON Schema exists at `schemas/evidence/profile-test-report-v1.schema.json`. CLI test output can emit the target v2 schema with `hl7v2 profile test --report json --schema-version 2`. Defaults remain v1-compatible. |
| Profile explain report | `hl7v2 profile explain --format json|yaml|text` | CLI-local `ProfileExplainReport` plus opt-in `ProfileExplainReportV2` wrapper | Includes profile SHA-256 and loaded profile version; JSON Schema exists at `schemas/evidence/profile-explain-report-v1.schema.json`. CLI explain output can emit the target v2 schema with `hl7v2 profile explain --format json --schema-version 2`. Defaults remain v1-compatible. |
| Corpus summary | `hl7v2 corpus summarize --format json|yaml|text`, Rust corpus module, Python `corpus_summary()` | `hl7v2::synthetic::corpus::CorpusSummary` plus opt-in `CorpusSummaryV2` conversion | Library type; JSON Schema exists at `schemas/evidence/corpus-summary-v1.schema.json`. CLI summary output can emit the target v2 schema with `hl7v2 corpus summarize --format json --schema-version 2`, and Python can emit it with `corpus_summary(..., schema_version=2)`. Defaults remain v1-compatible. |
| Corpus fingerprint | `hl7v2 corpus fingerprint --format json|yaml|text`, Rust corpus module, Python `corpus_fingerprint()` | `hl7v2::synthetic::corpus::CorpusFingerprint` plus opt-in `CorpusFingerprintV2` conversion | Has `fingerprint_version`, `tool_version`, optional profile SHA-256 metadata, and JSON Schema at `schemas/evidence/corpus-fingerprint-v1.schema.json`. CLI fingerprint output can emit the target v2 schema with `hl7v2 corpus fingerprint --format json --schema-version 2`, and Python can emit it with `corpus_fingerprint(..., schema_version=2)`. Defaults remain v1-compatible. |
| Corpus diff report | `hl7v2 corpus diff --format json|yaml|text`, Rust corpus module, Python `corpus_diff()` | `hl7v2::synthetic::corpus::CorpusDiffReport` plus opt-in `CorpusDiffReportV2` conversion | Has `diff_version`, `tool_version`, optional profile SHA-256 metadata, and JSON Schema at `schemas/evidence/corpus-diff-v1.schema.json`. CLI diff output can emit the target v2 schema with `hl7v2 corpus diff --format json --schema-version 2`, and Python can emit it with `corpus_diff(..., schema_version=2)`. Defaults remain v1-compatible. |
| Redaction output | `hl7v2 redact --format json`, Python `redact()` | `hl7v2::redact::SafeAnalysisRedactionOutput` for Python; CLI-local wrapper still matches the same fields | Has input and policy SHA-256 values plus a nested receipt; the outer output has no standalone schema. Defaults remain v1-compatible. |
| Redaction receipt | `hl7v2 redact --format json --schema-version 2`, Python `redact(..., schema_version=2)`, server `/hl7/validate-redacted` with `redaction_receipt_schema_version: 2`; bundle `redaction-receipt.json` remains v1 | `hl7v2::redact::RedactionReceipt` plus opt-in `RedactionReceiptV2` conversion; CLI/server-local shapes still match the same schema | Captures actions and PHI removal status; JSON Schemas exist at `schemas/evidence/redaction-receipt-v1.schema.json` and `schemas/evidence/redaction-receipt-v2.schema.json`. v2 adds `schema_version`, `tool_name`, and `tool_version`; defaults remain v1-compatible. |
| Field path trace | Bundle `field-paths.json`, Python `bundle()` artifact | `hl7v2::evidence::FieldPathTraceReport` for Python; CLI/server-local shapes still match the same fields | Captures redacted message field paths, value shapes, and configured redaction actions; v1 and target v2 JSON Schemas exist. Live bundle writers still emit the v1 artifact shape until bundle artifact producers migrate explicitly. |
| Evidence bundle summary | `hl7v2 bundle ...` stdout; Python `bundle()`; server `/hl7/bundle` response | `hl7v2::evidence::EvidenceBundleSummary` plus opt-in `EvidenceBundleSummaryV2` conversion for Python; CLI/server-local v1 shapes still match the same schema | Has `bundle_version`; JSON Schemas exist at `schemas/evidence/evidence-bundle-v1.schema.json` and `schemas/evidence/evidence-bundle-v2.schema.json`. CLI bundle output can emit the target v2 schema with `hl7v2 bundle ... --schema-version 2`, and Python can emit it with `bundle(..., schema_version=2)`. Defaults remain v1-compatible; server reports the configured-root-relative bundle id, Python reports `.` to avoid exposing a local filesystem path. |
| Quarantine output summary | Server `/hl7/validate-redacted` response when `[quarantine]` is enabled and validation fails | Server-local `QuarantineOutputSummary` plus opt-in `QuarantineOutputSummaryV2` conversion | Has `quarantine_version`, root-relative output id, reason, issue count, artifact names, and JSON Schemas at `schemas/evidence/quarantine-output-v1.schema.json` and `schemas/evidence/quarantine-output-v2.schema.json`. Server responses can include the target v2 shape as `quarantine_v2` when requests set `quarantine_schema_version` to `2`. It does not expose the configured filesystem path. Full-bundle quarantine output reuses the evidence bundle artifact set. |
| Evidence bundle manifest | Bundle `manifest.json` | `hl7v2::evidence::EvidenceBundleManifest` for Python; CLI/server-local shapes still match the same schema | Records `bundle_version`, `tool_name`, `tool_version`, bundle-relative artifact paths, roles, and SHA-256 hashes; v1 and target v2 JSON Schemas exist. Replay verifies the live v1 manifest catalog and hashes before using artifacts. |
| Evidence bundle environment | Bundle `environment.json` | `hl7v2::evidence::EvidenceBundleEnvironment` for Python; CLI/server-local shapes still match the same fields | Has `bundle_version`, `tool_name`, `tool_version`, input/profile/policy hashes, validation summary, and replay command; v1 and target v2 JSON Schemas exist. Live bundle writers still emit the v1 artifact shape. |
| Evidence replay report | `hl7v2 replay --format json|yaml|text`, Python `replay()` | `hl7v2::evidence::EvidenceReplayReport` plus opt-in `EvidenceReplayReportV2` conversion for Python; CLI-local v1 shape still matches the same schema | Has `replay_version`, `bundle_version`, `tool_name`, `tool_version`, replay checks, optional regenerated validation report, and JSON Schemas at `schemas/evidence/evidence-replay-v1.schema.json` and `schemas/evidence/evidence-replay-v2.schema.json`. CLI replay output can emit the target v2 schema with `hl7v2 replay ... --format json --schema-version 2`, and Python can emit it with `replay(..., schema_version=2)`. Defaults remain v1-compatible; replay fails closed on malformed manifests, missing artifacts, and hash mismatches. |

## Current Parity

| Surface | Current parity |
| --- | --- |
| Rust library | Owns the shared `ValidationReport`, `ProfileLintReport`, corpus summary/fingerprint/diff types, redaction output/receipt types, and Python-facing evidence bundle/replay types. Profile test and profile explain report types are currently CLI-local. |
| CLI | Produces the complete evidence loop today. Most commands support JSON/YAML/text. `redact` supports JSON or HL7 output. `bundle` currently emits JSON summary only. |
| Server | `/hl7/validate` exposes the shared validation issue fields, preserves legacy `errors` / `warnings`, and can include `validation_report_v2` when `report_schema_version` is `2`; `/hl7/validate-redacted` applies safe-analysis redaction, returns a validation report plus redaction receipt, can include the same v2 validation artifact, and can write configured quarantine output for failed validation; `/hl7/bundle` writes redacted evidence bundles under a configured server root; `/hl7/ack-policy` returns policy-driven ACK/NAK decisions backed by validation reports. Replay endpoints and corpus artifacts remain follow-up work. |
| Python | Exposes parse, JSON conversion, normalize, validation report dict/JSON parity, corpus summary/fingerprint/diff dict parity, safe-analysis redaction dict output, evidence bundle creation, and replay verification. |

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
The next provenance/versioning contract is planned in
[`evidence-provenance-versioning.md`](evidence-provenance-versioning.md).

## Remaining Contract Hardening Gaps

The v1.3.0 evidence loop has schema-backed JSON artifacts, golden fixtures,
CLI output semantics, bundle manifest verification, server edge-guard routes,
Python parity, and workflow guides. Remaining hardening work should narrow the
few places where provenance or identity is still implicit:

- `schema_version` or artifact-specific version naming for every machine
  artifact. The compatibility plan is documented in
  [`evidence-provenance-versioning.md`](evidence-provenance-versioning.md);
  implementation still requires explicit v2 schema/type work.
- `tool_version` where users need provenance outside an environment file.
- Shared library types for any CLI-local report promoted beyond CLI-only use.

## Non-Goals

This document does not define new runtime behavior. It records the current
evidence artifacts and the gaps that should be addressed by follow-up PRs.
