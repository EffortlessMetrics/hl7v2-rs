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
| `ValidationReport` | Shared Rust/CLI/server/Python type with no embedded `schema_version` or `tool_version`. | Add `schema_version`, `tool_name`, `tool_version`, and optional `profile_identity` in a v2 schema and type migration. |
| `ProfileLintReport` | Shared Rust type with no embedded version fields. | Add common provenance fields in v2. |
| `ProfileTestReport` | CLI-local type with embedded validation reports. | Promote to shared type only if server/Python need it; otherwise add v2 provenance in the CLI schema first. |
| `ProfileExplainReport` | CLI-local report with profile hash but no artifact/tool version. | Add common provenance fields; preserve profile hash metadata. |
| `CorpusSummary` | Shared Rust/Python/CLI type with no embedded version fields. | Add common provenance fields. |
| `CorpusFingerprint` | Has `fingerprint_version`, `tool_version`, and profile hash metadata. | Add `schema_version`; keep `fingerprint_version` as algorithm version. |
| `CorpusDiffReport` | Has `diff_version`, `tool_version`, and profile hash metadata. | Add `schema_version`; keep `diff_version` as algorithm version. |
| `SafeAnalysisRedactionOutput` | Has input/policy hashes and nested receipt; no output-level schema/tool version. | Add common provenance fields to the outer output. |
| `RedactionReceipt` | Shared receipt schema without embedded version fields. | Add common provenance fields if receipts remain useful outside a bundle or redaction output. |
| `FieldPathTraceReport` | Bundle artifact with no JSON Schema. | Add a v1 schema first or fold into a v2 bundle-artifact schema set with common provenance. |
| `EvidenceBundleSummary` | Has `bundle_version`; no `tool_version` in the summary. | Add `schema_version`, `tool_name`, and `tool_version`; keep `bundle_version`. |
| `QuarantineOutputSummary` | Has `quarantine_version`; server-local schema. | Add `schema_version`, `tool_name`, and `tool_version`; keep root-relative output ids only. |
| `EvidenceBundleManifest` | Has `bundle_version`, `tool_name`, `tool_version`, and hashed artifact catalog. | Add `schema_version`; keep manifest hash rules unchanged. |
| `EvidenceBundleEnvironment` | Has `bundle_version`, `tool_name`, `tool_version`, and input/profile/policy hashes. | Add a JSON Schema and `schema_version` before treating it as a stable standalone artifact. |
| `EvidenceReplayReport` | Has `replay_version`, `bundle_version`, `tool_name`, and `tool_version`. | Add `schema_version`; keep replay/domain version fields. |

## Migration Sequence

Do the migration in narrow PRs:

1. Add v2 schemas and v2 golden fixtures for the highest-value shared reports:
   `ValidationReport`, `CorpusFingerprint`, `CorpusDiffReport`, and
   `RedactionReceipt`.
2. Add additive Rust fields behind explicit v2 serializers or conversion
   helpers. Do not break callers that still expect the v1 shapes.
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
