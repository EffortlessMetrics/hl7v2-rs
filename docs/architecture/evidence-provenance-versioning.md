# Evidence Provenance And Versioning

This document defines the next contract-hardening step for evidence artifacts.
It is a plan, not a live schema change. The v1.3.0 evidence loop is already
schema-backed and golden-tested, but not every standalone artifact embeds enough
version and producer information to remain self-describing when copied out of a
bundle, sent through a pipeline, or compared across CLI, server, Rust, and
Python producers.

## Current Rule

The checked-in `*-v1.schema.json` files describe the current v1 artifact
contracts. A producer must not silently add required fields to those shapes or
rename existing fields under the same v1 contract.

This matters because the evidence schemas use `additionalProperties: false`.
Adding a new top-level field to a v1 artifact is observable to consumers that
validate output against the published v1 schema. Treat that as a contract
change, even when the field is purely provenance.

## Terms

Use these terms consistently in future schema and implementation work:

- `schema_version`: the JSON artifact contract version. This matches the schema
  family suffix, such as `1` for `validation-report-v1.schema.json`.
- Domain version fields such as `fingerprint_version`, `diff_version`,
  `bundle_version`, `quarantine_version`, and `replay_version`: algorithm or
  artifact-family versions. They do not replace `schema_version`.
- `tool_name`: the producer surface that generated the artifact. Allowed values
  should remain bounded, such as `hl7v2`, `hl7v2-cli`, `hl7v2-server`, and
  `hl7v2-python`.
- `tool_version`: the semantic version of the producing crate or binding
  package.
- Profile identity: a reproducible profile reference, preferably a SHA-256 hash
  plus message structure/version metadata. A display label such as
  `ValidationReport.profile` is not a canonical identity.

## Target V2 Provenance Fields

When an artifact moves to a v2 evidence schema, standalone machine-readable
outputs should include these top-level fields unless there is a documented
exception:

```json
{
  "schema_version": "2",
  "tool_name": "hl7v2-cli",
  "tool_version": "1.4.0"
}
```

Artifacts that already have domain version fields keep those fields. For
example, a future corpus fingerprint should carry both `schema_version` and
`fingerprint_version`; the former identifies the JSON contract, while the latter
identifies the fingerprinting algorithm.

Do not add timestamps, local absolute paths, raw input filenames, raw policy
paths, or hostnames to the common provenance block. Those values make golden
fixtures noisy and can leak environment details. Bundle-level environment
artifacts may carry explicit, sanitized runtime context when a specific
incident packet needs it.

## Profile Identity

Future v2 artifacts that depend on a profile should prefer a structured
identity block instead of overloading display labels:

```json
{
  "profile_identity": {
    "label": "profiles/oru_r01.yaml",
    "message_structure": "ORU_R01",
    "version": "2.5.1",
    "sha256": "..."
  }
}
```

Rules:

- `label` is display-only and may differ across CLI, server, Python, and Rust
  library call sites.
- `sha256` is the reproducible identity when the profile bytes are available.
- `message_structure` and `version` are descriptive metadata, not a hash.
- Do not expose local absolute paths in profile identity fields.

## Artifact Disposition

| Artifact | V1 state | V2 direction |
| --- | --- | --- |
| `DoctorReport` | Has tool `version` plus diagnostic checks; no embedded `schema_version`. | A v1 schema exists for the current CLI output. Keep v1-compatible unless a future PR adds an explicit v2 producer path. |
| `ValidationReport` | Shared Rust/CLI/server/Python type with no embedded `schema_version` or `tool_version`. | A target v2 schema exists, Rust exposes an explicit v2 conversion helper, and CLI/Python validation can emit v2 when requested. Server validation keeps its v1 response shape by default and can include nested `validation_report_v2` when requests set `report_schema_version` to `2`. |
| `ProfileLintReport` | Shared Rust type with no embedded version fields by default. | A target v2 schema exists, Rust exposes an explicit v2 conversion helper, and CLI lint output can emit v2 when requested. |
| `ProfileTestReport` | CLI-local type with embedded validation reports. | A target v2 schema exists, and CLI test output can emit v2 when requested. Promote to a shared type only if server/Python need it. |
| `ProfileExplainReport` | CLI-local report with profile hash but no artifact/tool version by default. | A target v2 schema exists, and CLI explain output can emit v2 when requested. |
| `CorpusSummary` | Shared Rust/Python/CLI type with no embedded version fields by default. | A target v2 schema exists, Rust exposes an explicit v2 conversion helper, and CLI/Python summary output can emit v2 when requested. |
| `CorpusFingerprint` | Has `fingerprint_version`, `tool_version`, and profile hash metadata. | A target v2 schema exists, Rust exposes an explicit v2 conversion helper, and CLI/Python fingerprint output can emit v2 when requested. |
| `CorpusDiffReport` | Has `diff_version`, `tool_version`, and profile hash metadata. | A target v2 schema exists, Rust exposes an explicit v2 conversion helper, and CLI/Python diff output can emit v2 when requested. |
| `SafeAnalysisRedactionOutput` | Has input/policy hashes and nested receipt; no output-level schema/tool version by default. | A v1 outer schema exists for current CLI/Python default outputs. CLI and Python redaction output can emit the target v2 schema with top-level provenance when requested. |
| `RedactionReceipt` | Shared receipt schema without embedded version fields by default. | A target v2 schema exists, Rust exposes an explicit v2 conversion helper, and CLI/Python/server redaction output can emit v2 when requested. CLI/Python/server bundle artifacts can also emit v2 receipts through explicit bundle artifact schema opt-in. |
| `FieldPathTraceReport` | Bundle artifact with a v1 JSON Schema but no embedded version fields. | A target v2 bundle-artifact schema and fixture exist. CLI/Python/server bundle writers can emit v2 field-path traces through explicit bundle artifact schema opt-in. |
| `EvidenceBundleSummary` | Has `bundle_version`; no `tool_version` in the summary by default. | A target v2 schema exists, Rust exposes an explicit v2 conversion helper, and CLI/Python bundle output can emit v2 when requested. Server `/hl7/bundle` keeps its v1 response shape by default while allowing v2 bundle-internal artifacts. |
| `QuarantineOutputSummary` | Has `quarantine_version`; server-local schema. | A target v2 schema exists, server responses can include the additive `quarantine_v2` field when requests set `quarantine_schema_version` to `2`, and root-relative output ids remain the only exposed path. |
| `EvidenceBundleManifest` | Has `bundle_version`, `tool_name`, `tool_version`, and hashed artifact catalog. | A target v2 schema and fixture exist with `schema_version`; CLI/Python/server bundle writers can emit v2 manifests through explicit bundle artifact schema opt-in. |
| `EvidenceBundleEnvironment` | Has `bundle_version`, `tool_name`, `tool_version`, input/profile/policy hashes, validation summary, replay command, and a v1 JSON Schema. | A target v2 schema and fixture exist with `schema_version`; CLI/Python/server bundle writers can emit v2 environments through explicit bundle artifact schema opt-in. |
| `EvidenceReplayReport` | Has `replay_version`, `bundle_version`, `tool_name`, and `tool_version` by default. | A target v2 schema exists, Rust exposes an explicit v2 conversion helper, and CLI/Python replay output can emit v2 when requested. |

## Migration Sequence

Do the migration in narrow PRs:

1. Add v2 schemas and v2 golden fixtures for the highest-value shared reports:
   `ValidationReport`, `ProfileLintReport`, `ProfileExplainReport`,
   `CorpusSummary`, `CorpusFingerprint`, `CorpusDiffReport`, and
   `RedactionReceipt`. These artifacts now have target v2 schemas and
   fixtures; producers still emit the current v1 shapes until migrated
   explicitly.
2. Add additive Rust fields behind explicit v2 serializers or conversion
   helpers. Do not break callers that still expect the v1 shapes.
   `ValidationReport` now has an explicit v2 conversion helper and `hl7v2 val`
   can opt into v2 JSON/YAML output with `--schema-version 2`. Python
   validation reports can opt into the same shape with `report.to_dict(2)` and
   `report.to_json(2)`. Server validation responses can include the same shape
   as nested `validation_report_v2` when requests set `report_schema_version`
   to `2`. Defaults remain v1-compatible.
   `ProfileLintReport` now has an explicit v2 conversion helper, and `hl7v2
   profile lint` can opt into v2 JSON/YAML output with `--schema-version 2`.
   Defaults remain v1-compatible.
   `ProfileExplainReport` can opt into v2 JSON/YAML output with
   `--schema-version 2`. Defaults remain v1-compatible.
   `ProfileTestReport` can opt into v2 JSON/YAML output with
   `--schema-version 2`. Defaults remain v1-compatible; nested validation
   reports preserve the current serialized shape.
   `CorpusSummary`, `CorpusFingerprint`, and `CorpusDiffReport` now have
   explicit v2 conversion helpers. `hl7v2 corpus summarize` / `hl7v2 corpus
   fingerprint` / `hl7v2 corpus diff` can opt into v2 JSON/YAML output with
   `--schema-version 2`, and Python can opt into the same shapes with
   `corpus_summary(..., schema_version=2)`,
   `corpus_fingerprint(..., schema_version=2)`, and
   `corpus_diff(..., schema_version=2)`. Defaults remain v1-compatible.
   `RedactionReceipt` now has an explicit v2 conversion helper. `hl7v2 redact
   --format json --schema-version 2`, Python `redact(..., schema_version=2)`,
   and server `/hl7/validate-redacted` requests with
   `redaction_receipt_schema_version: 2` can opt into the v2 nested receipt.
   Defaults remain v1-compatible.
   `SafeAnalysisRedactionOutput` now has a v1 schema and fixture for the
   default output, a compatibility fixture for the transitional v1 outer shape
   with a nested v2 receipt, and an opt-in v2 producer path for CLI and Python
   redaction output with top-level provenance. Defaults remain v1-compatible.
   `QuarantineOutputSummary` now has an explicit v2 conversion helper. Server
   `/hl7/validate-redacted` requests with `quarantine_schema_version: 2` can
   opt into the nested `quarantine_v2` summary when quarantine output is
   written. Defaults remain v1-compatible.
   `EvidenceBundleSummary` now has an explicit v2 conversion helper.
   `hl7v2 bundle ... --schema-version 2` and Python
   `bundle(..., schema_version=2)` can opt into the v2 summary. Defaults
   remain v1-compatible, and server `/hl7/bundle` keeps its v1 response shape
   while allowing v2 bundle-internal artifacts with
   `bundle_artifact_schema_version: 2`.
   `EvidenceReplayReport` now has an explicit v2 conversion helper.
   `hl7v2 replay ... --format json --schema-version 2` and Python
   `replay(..., schema_version=2)` can opt into the v2 replay report. Defaults
   remain v1-compatible.
   Bundle-internal `manifest.json`, `environment.json`, `field-paths.json`,
   and `redaction-receipt.json` now have opt-in v2 producer paths in CLI,
   Python, and server bundle writers. Default bundle artifacts remain
   v1-compatible.
3. Update CLI JSON/YAML output to choose the v2 shape only when the command or
   release notes explicitly say so. If a compatibility flag is needed, document
   it before exposing it.
4. Update server responses only after OpenAPI examples and integration tests
   cover the v2 shape.
5. Update Python dictionaries and `to_json()` outputs after the Rust and CLI
   contracts are stable.
6. Keep v1 schemas and fixtures for at least two minor releases after v2 output
   becomes the default.

## Compatibility Rules

- Adding optional metadata to a Rust struct is not enough. The serialized JSON
  shape and JSON Schema decide whether a producer-consumer contract changed.
- A v1 producer may continue to emit v1 artifacts without embedded provenance
  when provenance is available in a containing bundle manifest or environment
  file.
- A v2 producer must emit the common provenance fields for standalone artifacts.
- A v2 consumer should reject unknown `schema_version` values with a typed
  input/contract error, not by silently accepting an unrecognized shape.
- No provenance field may include raw HL7 payloads, redacted values before
  hashing, API keys, environment variables, local absolute paths, or raw policy
  paths.

## Non-Goals

This plan does not require immediate v2 output in the CLI, server, Rust library,
or Python binding. It also does not redefine validation issue codes, redaction
policy semantics, ACK/NAK policy, or corpus diff algorithms. Those are separate
contract changes.
