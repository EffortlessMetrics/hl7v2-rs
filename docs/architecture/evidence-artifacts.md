# Evidence Artifacts

This document maps the current evidence-loop artifacts and their contract
status. It is a current-state reference for the schema, golden-fixture, CLI
output-contract, server parity, and Python parity follow-up work.

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
| Validation report | `hl7v2 val --report json|yaml|text`, Rust `ValidationReport`, Python `report.to_dict()` / `report.to_json()`, server `/hl7/validate` fields | `hl7v2::ValidationReport` | Shared across Rust/CLI/Python and embedded in server response; no `schema_version`; no `tool_version`; no JSON Schema. |
| Profile lint report | `hl7v2 profile lint --report json|yaml|text`, Rust `lint_profile_yaml` | `hl7v2::ProfileLintReport` | Library type; no `schema_version`; no `tool_version`; no JSON Schema. |
| Profile test report | `hl7v2 profile test --report json|yaml|text` | CLI-local `ProfileTestReport` | Includes validation reports per case; no `schema_version`; no `tool_version`; no JSON Schema. |
| Profile explain report | `hl7v2 profile explain --format json|yaml|text` | CLI-local `ProfileExplainReport` | Includes profile SHA-256 and loaded profile version; no artifact `schema_version`; no tool version; no JSON Schema. |
| Corpus summary | `hl7v2 corpus summarize --format json|yaml|text`, Rust corpus module | `hl7v2::synthetic::corpus::CorpusSummary` | Library type; no `schema_version`; no `tool_version`; no JSON Schema. |
| Corpus fingerprint | `hl7v2 corpus fingerprint --format json|yaml|text`, Rust corpus module | `hl7v2::synthetic::corpus::CorpusFingerprint` | Has `fingerprint_version`, `tool_version`, optional profile SHA-256 metadata; no JSON Schema yet. |
| Corpus diff report | `hl7v2 corpus diff --format json|yaml|text`, Rust corpus module | `hl7v2::synthetic::corpus::CorpusDiffReport` | Has `diff_version`, `tool_version`, optional profile SHA-256 metadata; no JSON Schema yet. |
| Redaction output | `hl7v2 redact --format json` | CLI-local `RedactionOutput` | Has input and policy SHA-256 values plus receipt; no `schema_version`; no `tool_version`; no JSON Schema. |
| Redaction receipt | `hl7v2 redact --format json`, bundle `redaction-receipt.json` | CLI-local `RedactionReceipt` | Captures actions and PHI removal status; no `schema_version`; no `tool_version`; no JSON Schema. |
| Field path trace | Bundle `field-paths.json` | CLI-local `FieldPathTraceReport` | Captures redacted message field paths and value shapes; no `schema_version`; no JSON Schema. |
| Evidence bundle summary | `hl7v2 bundle ...` stdout | CLI-local `EvidenceBundleSummary` | Has `bundle_version`; no `tool_version`; no manifest or artifact hashes yet. |
| Evidence bundle environment | Bundle `environment.json` | CLI-local `EvidenceBundleEnvironment` | Has `bundle_version`, `tool_name`, `tool_version`, input/profile/policy hashes, and replay command. |
| Evidence replay report | `hl7v2 replay --format json|yaml|text` | CLI-local `EvidenceReplayReport` | Has `replay_version`, `bundle_version`, `tool_name`, `tool_version`, replay checks, and optional regenerated validation report; no manifest hash verification yet. |

## Current Parity

| Surface | Current parity |
| --- | --- |
| Rust library | Owns the shared `ValidationReport`, `ProfileLintReport`, and corpus summary/fingerprint/diff types. Bundle, replay, profile test, and profile explain report types are currently CLI-local. |
| CLI | Produces the complete evidence loop today. Most commands support JSON/YAML/text. `redact` supports JSON or HL7 output. `bundle` currently emits JSON summary only. |
| Server | `/hl7/validate` exposes the shared validation issue fields and preserves legacy `errors` / `warnings`; it does not yet expose redaction, bundle, replay, or corpus artifacts. |
| Python | Exposes parse, JSON conversion, normalize, and validation report dict/JSON parity. It does not yet expose corpus, redaction, bundle, or replay artifacts. |

One current validation-report parity wrinkle is the profile label. The CLI uses
the profile file path, while the server and Python binding use the loaded
profile `message_structure`. The report fields are otherwise shared through
`hl7v2::ValidationReport`.

## Output Semantics

The CLI has useful machine-readable outputs, but the script contract is not yet
uniform.

- JSON/YAML output generally goes to stdout.
- Human text output also goes to stdout.
- Diagnostics and top-level errors are written to stderr by the global error
  path.
- `hl7v2 redact --format hl7` writes the redacted message to stdout and a short
  receipt to stderr.
- `hl7v2 val` exits `1` for invalid validation reports.
- `profile lint`, `profile test`, and `replay` return errors for failed
  evidence checks, which currently route through the global `exit(1)` handler.
- Parse/config/profile/policy/IO failures also use the same global `exit(1)`
  behavior today.

The planned output-contract work should split these into the documented
automation classes:

```text
0 = success
1 = validation/profile/evidence failed
2 = parse/config/profile/policy input error
3 = IO/runtime/environment error
```

## Contract Hardening Gaps

The next contract-hardening work should lock:

- `schema_version` or artifact-specific version naming for every machine
  artifact.
- `tool_version` where users need provenance outside an environment file.
- JSON Schemas under `schemas/evidence/`.
- Golden fixtures under `fixtures/evidence/`.
- Explicit null and empty-list behavior for optional fields.
- Stable stdout/stderr/exit-code behavior across the CLI.
- Bundle manifest and artifact SHA-256 verification during replay.
- Redaction leak sentinel tests for synthetic PHI fixtures.
- Server and Python parity for any artifact promoted beyond CLI-only use.

## Non-Goals

This document does not define new runtime behavior. It records the current
evidence artifacts and the gaps that should be addressed by follow-up PRs.
