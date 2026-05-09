# Evidence Loop Current-State Audit

Date: 2026-05-08

Updated: 2026-05-09 after the v1.3.0 Evidence Loop release hardening pass.

## Purpose

This audit records the evidence-loop state after the product-usefulness lane
added first-run diagnostics, typed validation reports, profile commands, corpus
observability, safe redaction, evidence bundle/replay, Python API parity, and
the v1.3.0 contract-hardening pass.

The goal is to keep the next hardening work concrete. The loop now has JSON
Schemas, golden fixtures, CLI output semantics, bundle manifest verification,
server parity, and Python bundle/replay APIs; the remaining gaps are narrower
provenance and identity details.

## Current Product Loop

The current CLI can run the full evidence path:

```text
hl7v2 doctor
hl7v2 profile lint <profile.yaml> --report json
hl7v2 profile explain <profile.yaml> --format json
hl7v2 profile test <profile.yaml> <fixtures/> --report json
hl7v2 val <message.hl7> --profile <profile.yaml> --report json
hl7v2 corpus summarize <feeds/> --format json
hl7v2 corpus fingerprint <feeds/> --profile <profile.yaml> --format json
hl7v2 corpus diff <before/> <after/> --profile <profile.yaml> --format json
hl7v2 redact <message.hl7> --policy <safe-analysis.toml> --format json
hl7v2 bundle <message.hl7> --profile <profile.yaml> --redact-policy <safe-analysis.toml> --out <bundle/>
hl7v2 replay <bundle/> --format json
```

The server exposes validation report parity through `/hl7/validate`, safe
redacted validation through `/hl7/validate-redacted`, server-side evidence
bundle creation through `/hl7/bundle` when a bundle output root is configured,
policy-driven ACK/NAK decisions through `/hl7/ack-policy`, and configured
quarantine output for failed redacted validation. Python exposes parse, JSON
conversion, normalization, validation report dict/JSON parity, corpus
summary/fingerprint/diff dict outputs, and safe-analysis redaction output.

## Artifact Status

| Artifact | Current state | Main gap |
| --- | --- | --- |
| `ValidationReport` | Shared Rust type used by CLI, Python, and server validation response fields, including `/hl7/validate-redacted`. Includes stable issue code, severity, path, rule ID, message, segment index, and field index. | JSON Schema and golden fixture exist; no embedded artifact `schema_version` or `tool_version`. |
| `ProfileLintReport` | Shared Rust type from profile linting. CLI emits text/JSON/YAML by default, with opt-in v2 output for embedded provenance. | JSON Schemas and golden fixtures exist for v1 and v2; defaults remain v1-compatible. |
| `ProfileTestReport` | CLI report with fixture cases, pass/fail status, embedded validation reports, and optional expected-report comparison; opt-in v2 output adds embedded provenance. | JSON Schemas and golden fixtures exist for v1 and v2; defaults remain v1-compatible. |
| `ProfileExplainReport` | CLI report with profile SHA-256, structure, version, segments, constraints, tables, rules, and lint summary; opt-in v2 output adds embedded provenance. | JSON Schemas and golden fixtures exist for v1 and v2; defaults remain v1-compatible. |
| `CorpusSummary` | Shared Rust corpus summary type. CLI emits text/JSON/YAML and Python returns the same dict shape by default, with opt-in v2 output for embedded provenance. | JSON Schemas and golden fixtures exist for v1 and v2; defaults remain v1-compatible. |
| `CorpusFingerprint` | Shared Rust fingerprint type with `fingerprint_version`, `tool_version`, optional profile hash, counts, field presence/cardinality, value shapes, and validation issue-code counts. CLI emits text/JSON/YAML and Python returns the same dict shape. | JSON Schema and golden fixture exist; `profile: null` and empty-array semantics are documented in the schema README. |
| `CorpusDiffReport` | Shared Rust diff type with `diff_version`, `tool_version`, optional profile hash, totals, new/removed message types and segments, field deltas, value-shape deltas, and validation issue-code deltas. CLI emits text/JSON/YAML and Python returns the same dict shape. | JSON Schema and golden fixture exist; `profile: null` and empty-array semantics are documented in the schema README. |
| `SafeAnalysisRedactionOutput` | CLI `hl7v2 redact --format json` and Python `redact()` return input and policy SHA-256 values, message type, redacted HL7 text, and a nested redaction receipt. | JSON Schema and fixtures exist for the current v1-compatible outer shape, including the opt-in nested v2 receipt form. A target v2 schema exists for future top-level `schema_version`, `tool_name`, and `tool_version`; live producers do not emit that outer v2 shape yet. |
| `RedactionReceipt` | CLI, server, and Python receipts record PHI removal status, hash algorithm, per-path action, reason, match count, optional flag, and status. | JSON Schemas, golden fixtures, and synthetic PHI leak-sentinel tests exist for v1 and opt-in v2. Defaults remain v1-compatible; v2 adds embedded `schema_version`, `tool_name`, and `tool_version`. |
| `EvidenceBundleSummary` | CLI stdout JSON summary, Python `bundle()` output, and server `/hl7/bundle` JSON response include `bundle_version`, output directory/id, message type, validation status, redaction status, and artifact list. | v1 and v2 JSON Schemas plus golden fixtures exist. CLI and Python can opt into v2 provenance with `schema_version = 2`; server stays v1 by default. |
| Bundle artifacts | CLI, Python, and server bundles write `message.redacted.hl7`, `validation-report.json`, `field-paths.json`, `profile.yaml`, `redaction-receipt.json`, `environment.json`, `replay.sh`, `replay.ps1`, `README.md`, and `manifest.json`. | Bundle README is generated and manifest-hashed; `field-paths.json`, `environment.json`, and `manifest.json` have v1 schemas plus target v2 schemas and fixtures. Live bundle writers still emit v1 artifact shapes; profile text is user-authored and included as supplied. |
| `QuarantineOutputSummary` | Server `/hl7/validate-redacted` can return a root-relative quarantine output id, reason, validation issue count, and artifact list when `[quarantine]` is enabled and validation fails. | Server-local response type; v1 and opt-in v2 JSON Schemas exist. Requests with `quarantine_schema_version: 2` include `quarantine_v2` with provenance while preserving the v1 `quarantine` field. Full-bundle mode reuses bundle artifacts. |
| `EvidenceBundleManifest` | Bundle `manifest.json` records bundle-relative artifact paths, roles, SHA-256 hashes, and the generating tool name (`hl7v2-cli`, `hl7v2-server`, or `hl7v2-python`). | v1 and target v2 JSON Schemas and golden fixtures exist; replay verifies the live v1 manifest catalog and hashes before using artifacts. |
| `EvidenceReplayReport` | CLI and Python reports include `replay_version`, `bundle_version`, `tool_name`, `tool_version`, replay checks, reproduction status, and optional regenerated validation report. | v1 and v2 JSON Schemas plus golden fixtures exist. CLI and Python can opt into v2 with `schema_version = 2`; replay fails closed on malformed manifests, missing artifacts, and hash mismatches. |
| Python validation report | `report.valid`, `message_type`, `profile`, `segment_count`, `issue_count`, `to_dict()`, and `to_json()` mirror `ValidationReport`. | Python exposes validation reports, corpus summary/fingerprint/diff dict APIs, safe-analysis redaction output, bundle creation, and replay verification. |

## Surface Parity

| Surface | Evidence-loop coverage |
| --- | --- |
| Rust | Shared validation, profile lint, corpus summary/fingerprint/diff, parse, normalize, write, ACK, redaction module APIs, and Python-facing bundle/replay evidence APIs. Profile test and profile explain reports remain CLI-local. |
| CLI | Complete current loop: doctor, profile lint/test/explain, validation, corpus summarize/fingerprint/diff, redact, bundle, and replay. |
| Server | Validation report parity for `/hl7/validate`; redacted validation parity and quarantine hooks for `/hl7/validate-redacted`; configured-root bundle creation for `/hl7/bundle`; policy-driven ACK/NAK decisions for `/hl7/ack-policy`; replay endpoint and corpus artifacts remain follow-up work. |
| Python | Minimum API parity for parse, `to_json`, normalize, validation reports, corpus summary/fingerprint/diff dict outputs, safe-analysis redaction output, bundle creation, and replay verification. |

One validation parity detail is intentionally documented as a label convention:
the CLI report `profile` value is the profile path supplied by the user, while
the server and Python surfaces use the loaded profile message structure. The
shape is schema-backed and shared, but `ValidationReport.profile` is not a
canonical profile identity. Consumers that need reproducible profile identity
should use artifacts with profile SHA-256 metadata.

## CLI Automation Semantics

Current behavior is script-grade:

- Machine-readable JSON/YAML goes to stdout as the primary artifact.
- Human text output also goes to stdout.
- Evidence report commands support `--output <path>` for file capture with
  stdout quiet, plus `--quiet` and `--no-color` for automation contexts.
- Top-level diagnostics use stderr through the global error handler.
- `redact --format hl7` sends the redacted HL7 body to stdout and a short
  receipt to stderr.

| Exit code | Intended meaning |
| --- | --- |
| `0` | Success. |
| `1` | Validation/profile/evidence check failed. |
| `2` | Parse/profile/config/policy input error. |
| `3` | IO/runtime/environment error. |

## Known Contract Gaps

The evidence loop is contract-grade enough for the v1.3.0 release: schemas,
goldens, CLI output semantics, manifest verification, server edge-guard routes,
Python parity, user guides, and documented evidence null/empty semantics are in
place. Remaining hardening work:

1. Add artifact version and tool version fields consistently.
   The compatibility plan is documented in
   [`../architecture/evidence-provenance-versioning.md`](../architecture/evidence-provenance-versioning.md);
   implementation should happen through explicit v2 schema/type PRs, not
   silent v1 shape changes.
2. Expand PHI leak sentinels beyond the current synthetic patient/contact
   fixture family as new evidence surfaces or policy modes are added.
3. Promote shared report types out of CLI-local structs when server or Python
   parity needs them.

## Verification Performed For This Audit

| Check | Result | Notes |
| --- | --- | --- |
| Inspected CLI command definitions and formatters | Pass | `doctor`, `profile`, `corpus`, `redact`, `bundle`, and `replay` commands are present. |
| Inspected shared Rust validation and corpus types | Pass | `ValidationReport`, `ProfileLintReport`, `CorpusSummary`, `CorpusFingerprint`, and `CorpusDiffReport` are library types. |
| Inspected server validation handlers and response models | Pass | `/hl7/validate` builds `ValidationReport`; `/hl7/validate-redacted` returns a validation report plus redaction receipt. |
| Inspected Python binding API | Pass | Python exposes parse, JSON conversion, normalization, validation report dict/JSON access, corpus summary/fingerprint/diff dict outputs, safe-analysis redaction output, bundle creation, and replay verification. |
| Checked existing integration tests | Pass | CLI tests cover JSON outputs, redaction no-PHI assertions, bundle artifacts, replay success, replay drift failure, and missing-artifact failure. |

## Result

The evidence loop is real enough to harden. The next work should not add broad
new features first; it should make the existing artifacts stable contracts with
schemas, goldens, version fields, CLI output semantics, manifest verification,
and parity plans for server and Python surfaces.
