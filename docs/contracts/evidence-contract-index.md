# Evidence Contract Index

This index maps the current machine-readable evidence artifacts to their
producer surfaces, schemas, fixtures, default shapes, opt-in v2 controls, and
PHI/logging notes. It is a routing document for maintainers and agents; the
schemas remain the normative JSON contracts.
Use the
[Evidence Artifact Compatibility Policy](evidence-artifact-compatibility-policy.md)
for stable/advisory field rules, semver expectations, and consumer guidance.

Versioning rules:

- v1 remains the default output shape unless a release explicitly changes it.
- v2 shapes are opt-in provenance-bearing artifacts with `schema_version`,
  `tool_name`, and `tool_version` where the schema defines those fields.
- Server corpus endpoints accept inline message content only. They must not read
  request-supplied filesystem paths.
- Server REST bundle and replay endpoints, and gRPC bundle creation/replay, operate
  under configured server roots. They must not allow arbitrary path writes or
  reads from unauthenticated request bodies.
- Redaction receipts prove configured policy actions. They are not a universal
  PHI absence certificate.

Schema validation is maintained by:

```bash
cargo run -p xtask -- evidence-schema-check
```

The command validates every primary evidence fixture against its matching
`schemas/evidence/*-v*.schema.json` contract and covers documented supplemental
fixtures such as the v1 redaction output containing a v2 nested receipt.

## Contract Map

| Artifact | Producer surfaces | v1 schema | v2 schema | Default shape | v2 opt-in | Golden fixtures | PHI / logging notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Doctor report | CLI `hl7v2 doctor` | `schemas/evidence/doctor-report-v1.schema.json` | `schemas/evidence/doctor-report-v2.schema.json` | v1 JSON/YAML/text; includes tool `version` and diagnostic checks | CLI `--schema-version 2` with JSON/YAML output | `fixtures/evidence/doctor-report.json`, `fixtures/evidence/doctor-report-v2.json` | Environment proof only. Do not add hostnames, raw paths, API keys, or raw server response bodies. |
| Validation report | Rust `ValidationReport`; CLI `hl7v2 val`; Python `validate().to_dict()` / `to_json()`; server REST `/hl7/validate`, `/hl7/validate-redacted`, `/hl7/ack-policy`; gRPC `Validate` and `ValidateRedacted`; bundle/replay artifacts | `schemas/evidence/validation-report-v1.schema.json` | `schemas/evidence/validation-report-v2.schema.json` | v1 report fields by default | CLI `--schema-version 2`; Python `to_dict(2)` / `to_json(2)`; REST request `report_schema_version: 2` adds nested `validation_report_v2`; gRPC `ValidateRequest.report_schema_version = 2` or `ValidateRedactedRequest.report_schema_version = 2` adds `validation_report_v2` | `fixtures/evidence/validation-report.json`, `fixtures/evidence/validation-report-v2.json` | Contains issue paths, codes, severities, rule IDs, and messages. It must not contain raw HL7 payloads. `profile` is a display label, not canonical identity. |
| Profile lint report | Rust `lint_profile_yaml`; CLI `hl7v2 profile lint`; gRPC `ProfileLint` | `schemas/evidence/profile-lint-report-v1.schema.json` | `schemas/evidence/profile-lint-report-v2.schema.json` | v1 report by default | CLI `--schema-version 2` with JSON/YAML output; gRPC `ProfileLintRequest.report_schema_version = 2` adds `profile_lint_report_v2` | `fixtures/evidence/profile-lint-report.json`, `fixtures/evidence/profile-lint-report-v2.json` | May include profile lint messages and paths from user-authored profile YAML. It should not echo the full profile document. |
| Profile explain report | CLI `hl7v2 profile explain`; gRPC `ProfileExplain` | `schemas/evidence/profile-explain-report-v1.schema.json` | `schemas/evidence/profile-explain-report-v2.schema.json` | v1 report by default | CLI `--schema-version 2` with JSON/YAML output; gRPC `ProfileExplainRequest.report_schema_version = 2` adds `profile_explain_report_v2` | `fixtures/evidence/profile-explain-report.json`, `fixtures/evidence/profile-explain-report-v2.json` | Includes profile-derived constraints, rule descriptions, value-set metadata, and profile hash. gRPC malformed-profile diagnostics must not echo raw profile YAML. |
| Profile test report | CLI `hl7v2 profile test`; gRPC `ProfileTest`; Rust `run_profile_fixture_tests`; Python `profile_test` helper | `schemas/evidence/profile-test-report-v1.schema.json` | `schemas/evidence/profile-test-report-v2.schema.json` | v1 report by default | CLI `--schema-version 2` with JSON/YAML output; gRPC `ProfileTestRequest.report_schema_version = 2` adds `profile_test_report_v2` | `fixtures/evidence/profile-test-report.json`, `fixtures/evidence/profile-test-report-v2.json` | Includes fixture labels and embedded validation reports, but not raw HL7 fixture bodies. gRPC accepts inline fixture messages, not filesystem paths. |
| Corpus summary | Rust corpus module; CLI `hl7v2 corpus summarize`; Python `corpus_summary()`; server REST `/hl7/corpus/summarize`; gRPC `CorpusSummarize` | `schemas/evidence/corpus-summary-v1.schema.json` | `schemas/evidence/corpus-summary-v2.schema.json` | v1 report by default | CLI `--schema-version 2`; Python `schema_version=2`; REST `summary_schema_version: 2`; gRPC `CorpusSummarizeRequest.summary_schema_version = 2` | `fixtures/evidence/corpus-summary.json`, `fixtures/evidence/corpus-summary-v2.json` | Reports counts and parse errors. It should not include raw message bodies. Server accepts inline messages, not filesystem paths. |
| Corpus fingerprint | Rust corpus module; CLI `hl7v2 corpus fingerprint`; Python `corpus_fingerprint()`; server REST `/hl7/corpus/fingerprint`; gRPC `CorpusFingerprint` | `schemas/evidence/corpus-fingerprint-v1.schema.json` | `schemas/evidence/corpus-fingerprint-v2.schema.json` | v1 report by default | CLI `--schema-version 2`; Python `schema_version=2`; REST `fingerprint_schema_version: 2`; gRPC `CorpusFingerprintRequest.fingerprint_schema_version = 2` | `fixtures/evidence/corpus-fingerprint.json`, `fixtures/evidence/corpus-fingerprint-v2.json` | Reports deterministic counts, presence/cardinality, value-shape stats, and optional profile hash. It should not expose raw field values. Server accepts inline messages, not caller paths. |
| Corpus diff report | Rust corpus module; CLI `hl7v2 corpus diff`; Python `corpus_diff()`; server REST `/hl7/corpus/diff`; gRPC `CorpusDiff` | `schemas/evidence/corpus-diff-v1.schema.json` | `schemas/evidence/corpus-diff-v2.schema.json` | v1 report by default | CLI `--schema-version 2`; Python `schema_version=2`; REST `diff_schema_version: 2`; gRPC `CorpusDiffRequest.diff_schema_version = 2` | `fixtures/evidence/corpus-diff.json`, `fixtures/evidence/corpus-diff-v2.json` | Reports before/after deltas, not raw messages. Server accepts inline before/after messages, not caller paths. |
| Safe-analysis redaction output | CLI `hl7v2 redact --format json`; Python `redact()` | `schemas/evidence/safe-analysis-redaction-output-v1.schema.json` | `schemas/evidence/safe-analysis-redaction-output-v2.schema.json` | v1 output by default | CLI `--schema-version 2`; Python `schema_version=2` | `fixtures/evidence/safe-analysis-redaction-output.json`, `fixtures/evidence/safe-analysis-redaction-output-receipt-v2.json`, `fixtures/evidence/safe-analysis-redaction-output-v2.json` | Includes redacted HL7 output and hashes. Retained fields may be present by policy; receipts must not include dropped raw values. |
| Redaction receipt | Rust redaction module; CLI redaction output; Python redaction output; server REST `/hl7/validate-redacted`; gRPC `ValidateRedacted`; bundle `redaction-receipt.json` | `schemas/evidence/redaction-receipt-v1.schema.json` | `schemas/evidence/redaction-receipt-v2.schema.json` | v1 receipt by default | CLI/Python redaction `schema_version=2`; REST `redaction_receipt_schema_version: 2`; gRPC `ValidateRedactedRequest.redaction_receipt_schema_version = 2`; bundle artifact schema version 2 | `fixtures/evidence/redaction-receipt.json`, `fixtures/evidence/redaction-receipt-v2.json` | Records actions, reasons, match counts, and `phi_removed`; never raw dropped values. |
| Field path trace | Bundle `field-paths.json` from CLI/Python/server bundle writers, including gRPC `CreateEvidenceBundle` | `schemas/evidence/field-path-trace-v1.schema.json` | `schemas/evidence/field-path-trace-v2.schema.json` | v1 bundle artifact by default | CLI/Python bundle `schema_version=2`; server REST `bundle_artifact_schema_version: 2`; gRPC `CreateEvidenceBundleRequest.bundle_artifact_schema_version = 2` | `fixtures/evidence/field-path-trace.json`, `fixtures/evidence/field-path-trace-v2.json` | Contains field paths, value shapes, and redaction actions. It should not contain raw PHI values. |
| Evidence bundle summary | CLI `hl7v2 bundle` stdout; Python `bundle()`; server `/hl7/bundle` response; gRPC `CreateEvidenceBundle` response | `schemas/evidence/evidence-bundle-v1.schema.json` | `schemas/evidence/evidence-bundle-v2.schema.json` | v1 summary by default | CLI/Python bundle `schema_version=2`; server REST and gRPC responses stay v1 while bundle artifacts can opt into v2 | `fixtures/evidence/evidence-bundle.json`, `fixtures/evidence/evidence-bundle-v2.json` | Server reports configured-root-relative or hashed public output IDs. Python reports `.`. Do not expose configured filesystem roots or raw bundle IDs. |
| Quarantine output summary | Server REST `/hl7/validate-redacted` and gRPC `ValidateRedacted` when quarantine is enabled and validation fails | `schemas/evidence/quarantine-output-v1.schema.json` | `schemas/evidence/quarantine-output-v2.schema.json` | v1 `quarantine` field by default | REST request `quarantine_schema_version: 2`; gRPC `ValidateRedactedRequest.quarantine_schema_version = 2` adds nested `quarantine_v2` | `fixtures/evidence/quarantine-output.json`, `fixtures/evidence/quarantine-output-v2.json` | Root-relative output IDs only; no configured root path and no raw HL7. Full-bundle mode reuses bundle artifacts. |
| Evidence bundle manifest | Bundle `manifest.json` from CLI/Python/server bundle writers, including gRPC `CreateEvidenceBundle` | `schemas/evidence/evidence-bundle-manifest-v1.schema.json` | `schemas/evidence/evidence-bundle-manifest-v2.schema.json` | v1 bundle artifact by default | CLI/Python bundle `schema_version=2`; server REST `bundle_artifact_schema_version: 2`; gRPC `CreateEvidenceBundleRequest.bundle_artifact_schema_version = 2` | `fixtures/evidence/evidence-bundle-manifest.json`, `fixtures/evidence/evidence-bundle-manifest-v2.json` | Integrity spine for bundles. Paths are bundle-relative and artifact hashes are SHA-256. Do not include raw HL7, local roots, or raw bundle IDs. |
| Evidence bundle environment | Bundle `environment.json` from CLI/Python/server bundle writers, including gRPC `CreateEvidenceBundle` | `schemas/evidence/evidence-bundle-environment-v1.schema.json` | `schemas/evidence/evidence-bundle-environment-v2.schema.json` | v1 bundle artifact by default | CLI/Python bundle `schema_version=2`; server REST `bundle_artifact_schema_version: 2`; gRPC `CreateEvidenceBundleRequest.bundle_artifact_schema_version = 2` | `fixtures/evidence/evidence-bundle-environment.json`, `fixtures/evidence/evidence-bundle-environment-v2.json` | Carries hashes, tool/version metadata, validation summary, and replay command. Avoid hostnames, local absolute paths, and raw policy paths. |
| Evidence replay report | Rust evidence module; CLI `hl7v2 replay`; Python `replay()`; server REST `/hl7/replay`; gRPC `ReplayEvidenceBundle` | `schemas/evidence/evidence-replay-v1.schema.json` | `schemas/evidence/evidence-replay-v2.schema.json` | v1 report by default | CLI `--schema-version 2`; Python `schema_version=2`; REST `replay_report_schema_version: 2`; gRPC `ReplayEvidenceBundleRequest.replay_report_schema_version = 2` | `fixtures/evidence/evidence-replay.json`, `fixtures/evidence/evidence-replay-v2.json` | Replay verifies manifest hashes before using artifacts and reports failures without dumping raw artifact contents. Server replay operates only under the configured bundle root. |

## Surface Notes

### Rust

The Rust crate owns the shared validation, profile lint, profile explain,
profile test, corpus, redaction, and evidence bundle/replay types used by the
other surfaces.

### CLI

Evidence commands write machine-readable JSON/YAML to stdout unless `--output`
is supplied. `--output` writes the same artifact to a file and keeps stdout
quiet. Diagnostics and top-level errors use stderr. `redact --format hl7`
writes the redacted HL7 body to stdout and a short receipt to stderr.

### Server

The server keeps v1-compatible response fields by default and exposes v2 shapes
through explicit request fields. REST `/hl7/validate`, REST
`/hl7/validate-redacted`, gRPC `Validate`, and gRPC `ValidateRedacted` include
validation report fields while preserving their legacy response fields where
those existed before the redacted-validation surface.
gRPC `ProfileLint` returns the shared profile lint report shape, gRPC
`ProfileExplain` returns the shared profile explain report shape, and gRPC
`ProfileTest` returns the shared profile test report shape. These RPCs must not
echo raw profile YAML in malformed-profile diagnostics.
gRPC `CreateEvidenceBundle` writes the shared bundle artifacts under the
configured server bundle root and returns the shared bundle summary shape with a
root-redacted, hashed public output ID. gRPC `ReplayEvidenceBundle` verifies
configured-root bundles and returns the shared replay report shape with opt-in
v2 replay provenance.
gRPC `ValidateRedacted` can write the shared quarantine output summary when
quarantine is configured and redacted validation fails. The response exposes only
root-relative output IDs and can include `quarantine_v2` with
`ValidateRedactedRequest.quarantine_schema_version = 2`.
It logs evidence workflow events with hashed message-control and bundle
identifiers. Logs must not include raw HL7 payloads, profile YAML, redaction
policy TOML, configured filesystem roots, raw bundle IDs, API keys, or raw
message control IDs.

### Python

The Python binding is a separate language package surface backed by the
`hl7v2-python` binding backend crate. It mirrors the same evidence shapes for
validation, corpus, redaction, bundle, and replay APIs with opt-in v2
`schema_version` arguments where the contract exists. Rust users should depend
on `hl7v2` for the primary Rust API.

## Related References

- [Evidence artifacts](../architecture/evidence-artifacts.md)
- [Evidence artifact compatibility policy](evidence-artifact-compatibility-policy.md)
- [Evidence provenance and versioning](../architecture/evidence-provenance-versioning.md)
- [Schema README](../../schemas/README.md)
- [Safe support bundle guide](../guides/safe-support-bundle.md)
- [Deploy validation sidecar guide](../guides/deploy-validation-sidecar.md)
