# Evidence Loop Current-State Audit

Date: 2026-05-08

## Purpose

This audit records the current evidence-loop state after the product-usefulness
lane added first-run diagnostics, typed validation reports, profile commands,
corpus observability, safe redaction, evidence bundle/replay, and Python API
parity.

The goal is to make the next hardening work concrete: JSON Schemas, golden
fixtures, CLI output semantics, bundle manifest verification, server parity, and
Python corpus/redaction APIs should be based on the artifacts that exist now.

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

The server exposes validation report parity through `/hl7/validate` and safe
redacted validation through `/hl7/validate-redacted`. Python exposes parse, JSON
conversion, normalization, and validation report dict/JSON parity.

## Artifact Status

| Artifact | Current state | Main gap |
| --- | --- | --- |
| `ValidationReport` | Shared Rust type used by CLI, Python, and server validation response fields, including `/hl7/validate-redacted`. Includes stable issue code, severity, path, rule ID, message, segment index, and field index. | Missing artifact `schema_version`, `tool_version`, JSON Schema, and golden fixture set. |
| `ProfileLintReport` | Shared Rust type from profile linting. CLI emits text/JSON/YAML. | Missing version fields and JSON Schema. |
| `ProfileTestReport` | CLI report with fixture cases, pass/fail status, embedded validation reports, and optional expected-report comparison. | CLI-local type; missing version fields and JSON Schema. |
| `ProfileExplainReport` | CLI report with profile SHA-256, structure, version, segments, constraints, tables, rules, and lint summary. | CLI-local type; missing artifact version and tool version. |
| `CorpusSummary` | Shared Rust corpus summary type. CLI emits text/JSON/YAML. | Missing version fields; no JSON Schema. |
| `CorpusFingerprint` | Shared Rust fingerprint type with `fingerprint_version`, `tool_version`, optional profile hash, counts, field presence/cardinality, value shapes, and validation issue-code counts. | Needs JSON Schema and golden fixtures. |
| `CorpusDiffReport` | Shared Rust diff type with `diff_version`, `tool_version`, optional profile hash, totals, new/removed message types and segments, field deltas, value-shape deltas, and validation issue-code deltas. | Needs JSON Schema and golden fixtures. |
| `RedactionReceipt` | CLI and server receipts record PHI removal status, hash algorithm, per-path action, reason, match count, optional flag, and status. | Missing version fields, JSON Schema, and dedicated leak-sentinel fixture family. |
| `EvidenceBundleSummary` | CLI stdout JSON summary includes `bundle_version`, output directory, message type, validation status, redaction status, and artifact list. | Includes `manifest.json`; no `tool_version` in the summary itself. |
| Bundle artifacts | `message.redacted.hl7`, `validation-report.json`, `field-paths.json`, `profile.yaml`, `redaction-receipt.json`, `environment.json`, `replay.sh`, `replay.ps1`, `README.md`, and `manifest.json`. | Bundle README is generated and manifest-hashed. |
| `EvidenceBundleManifest` | Bundle `manifest.json` records bundle-relative artifact paths, roles, and SHA-256 hashes. | Replay verifies manifest catalog and hashes before using artifacts. |
| `EvidenceReplayReport` | CLI report with `replay_version`, `bundle_version`, `tool_name`, `tool_version`, replay checks, reproduction status, and optional regenerated validation report. | Fails closed on malformed manifests, missing artifacts, and hash mismatches; report schema exists. |
| Python validation report | `report.valid`, `message_type`, `profile`, `segment_count`, `issue_count`, `to_dict()`, and `to_json()` mirror `ValidationReport`. | Python does not yet expose corpus, redaction, bundle, or replay artifact APIs. |

## Surface Parity

| Surface | Evidence-loop coverage |
| --- | --- |
| Rust | Shared validation, profile lint, corpus summary/fingerprint/diff, parse, normalize, write, ACK, and redaction module APIs. Several evidence packet reports remain CLI-local. |
| CLI | Complete current loop: doctor, profile lint/test/explain, validation, corpus summarize/fingerprint/diff, redact, bundle, and replay. |
| Server | Validation report parity for `/hl7/validate`; redacted validation parity for `/hl7/validate-redacted`; bundle, replay, corpus, ACK policy, and quarantine hooks remain follow-up work. |
| Python | Minimum API parity for parse, `to_json`, normalize, and validation reports. Corpus, redaction, bundle, and replay APIs remain follow-up work. |

One known validation parity detail remains: the CLI report `profile` value is
the profile path supplied by the user, while the server and Python surfaces use
the loaded profile message structure. The shape is shared; the profile identity
semantics still need to be made explicit before schemas are treated as stable.

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

The evidence loop exists, but it is not yet fully contract-grade. Remaining
hardening work:

1. Add artifact version and tool version fields consistently.
2. Document null/empty-list behavior for optional fields.
3. Add fuller external guides for sharing and replaying bundles.
4. Add synthetic PHI leak sentinels for redaction, bundle, replay, and later
   Python wrappers.
5. Promote shared report types out of CLI-local structs when server or Python
   parity needs them.

## Verification Performed For This Audit

| Check | Result | Notes |
| --- | --- | --- |
| Inspected CLI command definitions and formatters | Pass | `doctor`, `profile`, `corpus`, `redact`, `bundle`, and `replay` commands are present. |
| Inspected shared Rust validation and corpus types | Pass | `ValidationReport`, `ProfileLintReport`, `CorpusSummary`, `CorpusFingerprint`, and `CorpusDiffReport` are library types. |
| Inspected server validation handlers and response models | Pass | `/hl7/validate` builds `ValidationReport`; `/hl7/validate-redacted` returns a validation report plus redaction receipt. |
| Inspected Python binding API | Pass | Python exposes parse, JSON conversion, normalization, and validation report dict/JSON access. |
| Checked existing integration tests | Pass | CLI tests cover JSON outputs, redaction no-PHI assertions, bundle artifacts, replay success, replay drift failure, and missing-artifact failure. |

## Result

The evidence loop is real enough to harden. The next work should not add broad
new features first; it should make the existing artifacts stable contracts with
schemas, goldens, version fields, CLI output semantics, manifest verification,
and parity plans for server and Python surfaces.
