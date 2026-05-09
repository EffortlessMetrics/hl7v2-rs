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
| Validation report | `hl7v2 val --report json|yaml|text`, Rust `ValidationReport`, Python `report.to_dict()` / `report.to_json()`, server `/hl7/validate` and `/hl7/validate-redacted` fields | `hl7v2::ValidationReport` | Shared across Rust/CLI/Python and embedded in server responses; JSON Schema exists at `schemas/evidence/validation-report-v1.schema.json`; no `schema_version`; no `tool_version`. |
| Profile lint report | `hl7v2 profile lint --report json|yaml|text`, Rust `lint_profile_yaml` | `hl7v2::ProfileLintReport` | Library type; JSON Schema exists at `schemas/evidence/profile-lint-report-v1.schema.json`; no `schema_version`; no `tool_version`. |
| Profile test report | `hl7v2 profile test --report json|yaml|text` | CLI-local `ProfileTestReport` | Includes validation reports per case; JSON Schema exists at `schemas/evidence/profile-test-report-v1.schema.json`; no `schema_version`; no `tool_version`. |
| Profile explain report | `hl7v2 profile explain --format json|yaml|text` | CLI-local `ProfileExplainReport` | Includes profile SHA-256 and loaded profile version; JSON Schema exists at `schemas/evidence/profile-explain-report-v1.schema.json`; no artifact `schema_version`; no tool version. |
| Corpus summary | `hl7v2 corpus summarize --format json|yaml|text`, Rust corpus module, Python `corpus_summary()` | `hl7v2::synthetic::corpus::CorpusSummary` | Library type; JSON Schema exists at `schemas/evidence/corpus-summary-v1.schema.json`; no `schema_version`; no `tool_version`. |
| Corpus fingerprint | `hl7v2 corpus fingerprint --format json|yaml|text`, Rust corpus module, Python `corpus_fingerprint()` | `hl7v2::synthetic::corpus::CorpusFingerprint` | Has `fingerprint_version`, `tool_version`, optional profile SHA-256 metadata, and JSON Schema at `schemas/evidence/corpus-fingerprint-v1.schema.json`. |
| Corpus diff report | `hl7v2 corpus diff --format json|yaml|text`, Rust corpus module, Python `corpus_diff()` | `hl7v2::synthetic::corpus::CorpusDiffReport` | Has `diff_version`, `tool_version`, optional profile SHA-256 metadata, and JSON Schema at `schemas/evidence/corpus-diff-v1.schema.json`. |
| Redaction output | `hl7v2 redact --format json`, Python `redact()` | `hl7v2::redact::SafeAnalysisRedactionOutput` for Python; CLI-local wrapper still matches the same fields | Has input and policy SHA-256 values plus receipt; no `schema_version`; no `tool_version`; only the nested receipt has a JSON Schema today. |
| Redaction receipt | `hl7v2 redact --format json`, Python `redact()`, bundle `redaction-receipt.json`, server `/hl7/validate-redacted` | `hl7v2::redact::RedactionReceipt` for Python; CLI/server-local shapes still match the same schema | Captures actions and PHI removal status; JSON Schema exists at `schemas/evidence/redaction-receipt-v1.schema.json`; no `schema_version`; no `tool_version`. |
| Field path trace | Bundle `field-paths.json`, Python `bundle()` artifact | `hl7v2::evidence::FieldPathTraceReport` for Python; CLI/server-local shapes still match the same fields | Captures redacted message field paths and value shapes; no `schema_version`; no JSON Schema. |
| Evidence bundle summary | `hl7v2 bundle ...` stdout; Python `bundle()`; server `/hl7/bundle` response | `hl7v2::evidence::EvidenceBundleSummary` for Python; CLI/server-local shapes still match the same schema | Has `bundle_version`; JSON Schema exists at `schemas/evidence/evidence-bundle-v1.schema.json`; no `tool_version`; includes `README.md` and `manifest.json` in the artifact list. Server reports the configured-root-relative bundle id, Python reports `.` to avoid exposing a local filesystem path. |
| Quarantine output summary | Server `/hl7/validate-redacted` response when `[quarantine]` is enabled and validation fails | Server-local `QuarantineOutputSummary` | Has `quarantine_version`, root-relative output id, reason, issue count, artifact names, and JSON Schema at `schemas/evidence/quarantine-output-v1.schema.json`. It does not expose the configured filesystem path. Full-bundle quarantine output reuses the evidence bundle artifact set. |
| Evidence bundle manifest | Bundle `manifest.json` | `hl7v2::evidence::EvidenceBundleManifest` for Python; CLI/server-local shapes still match the same schema | Records `bundle_version`, `tool_name`, `tool_version`, bundle-relative artifact paths, roles, and SHA-256 hashes; JSON Schema exists at `schemas/evidence/evidence-bundle-manifest-v1.schema.json`; replay verifies manifest catalog and hashes before using artifacts. |
| Evidence bundle environment | Bundle `environment.json` | `hl7v2::evidence::EvidenceBundleEnvironment` for Python; CLI/server-local shapes still match the same fields | Has `bundle_version`, `tool_name`, `tool_version`, input/profile/policy hashes, and replay command. |
| Evidence replay report | `hl7v2 replay --format json|yaml|text`, Python `replay()` | `hl7v2::evidence::EvidenceReplayReport` for Python; CLI-local shape still matches the same schema | Has `replay_version`, `bundle_version`, `tool_name`, `tool_version`, replay checks, optional regenerated validation report, and JSON Schema at `schemas/evidence/evidence-replay-v1.schema.json`; fails closed on malformed manifests, missing artifacts, and hash mismatches. |

## Current Parity

| Surface | Current parity |
| --- | --- |
| Rust library | Owns the shared `ValidationReport`, `ProfileLintReport`, corpus summary/fingerprint/diff types, redaction output/receipt types, and Python-facing evidence bundle/replay types. Profile test and profile explain report types are currently CLI-local. |
| CLI | Produces the complete evidence loop today. Most commands support JSON/YAML/text. `redact` supports JSON or HL7 output. `bundle` currently emits JSON summary only. |
| Server | `/hl7/validate` exposes the shared validation issue fields and preserves legacy `errors` / `warnings`; `/hl7/validate-redacted` applies safe-analysis redaction, returns a validation report plus redaction receipt, and can write configured quarantine output for failed validation; `/hl7/bundle` writes redacted evidence bundles under a configured server root; `/hl7/ack-policy` returns policy-driven ACK/NAK decisions backed by validation reports. Replay endpoints and corpus artifacts remain follow-up work. |
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
