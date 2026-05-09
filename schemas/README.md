# HL7v2-rs JSON Schemas

This directory contains JSON Schemas for all data structures in the hl7v2-rs project.

## Schema-Driven Design

All YAML configuration files and JSON outputs are validated against these schemas to ensure:
- **Consistency** - Same structure across all instances
- **Validation** - Catch errors early before runtime
- **Documentation** - Self-documenting via schema descriptions
- **Tooling** - IDE autocomplete and validation

## Schemas

### Profile (`profile/profile-v1.schema.json`)
Defines the structure of HL7v2 validation profiles including:
- Message structure and version
- Constraints (required, length, pattern, table, data_type)
- Cross-field validation rules
- Parent profile inheritance

### Message (`message/message-v1.schema.json`)
JSON representation of a parsed HL7v2 message:
- Delimiters configuration
- Segments with fields/components/subcomponents
- Presence semantics (missing/empty/null/value)

### Error (`error/error-v1.schema.json`)
Standardized error response format:
- Machine-readable error codes (P_*, V_*, S_*)
- Human-readable messages with advice
- Location information (segment/field/component)
- Trace IDs for correlation

### Manifest (`manifest/corpus-manifest-v1.schema.json`)
Corpus generation reproducibility tracking:
- Tool version and random seed
- Template/profile SHA-256 hashes
- Generated message inventory
- Train/validation/test splits

### Config (`config/hl7v2-config-v1.schema.json`)
CLI and server configuration (hl7v2.toml):
- Server host, port, and optional API key
- CLI defaults
- Logging defaults

### Evidence (`evidence/*-v*.schema.json`)
Machine-readable evidence artifacts emitted by the CLI, library, server, and
Python lanes:
- Doctor reports, validation reports, and profile lint/test/explain reports
- Corpus summary, fingerprint, and diff reports
- Redaction receipts, field-path traces, bundle environment metadata, and
  evidence bundle/replay summaries

`doctor-report-v1.schema.json` validates the current `hl7v2 doctor --format
json` output. The doctor report has no embedded `schema_version`; v1 is the
current compatible shape.

The first target v2 evidence schemas are `validation-report-v2.schema.json`,
`profile-lint-report-v2.schema.json`,
`profile-test-report-v2.schema.json`,
`profile-explain-report-v2.schema.json`, `corpus-summary-v2.schema.json`,
`corpus-fingerprint-v2.schema.json`, `corpus-diff-v2.schema.json`, and
`redaction-receipt-v2.schema.json`,
`safe-analysis-redaction-output-v2.schema.json`,
`quarantine-output-v2.schema.json`, `evidence-bundle-v2.schema.json`,
`evidence-bundle-manifest-v2.schema.json`,
`evidence-bundle-environment-v2.schema.json`, `field-path-trace-v2.schema.json`,
and `evidence-replay-v2.schema.json`. They add embedded `schema_version`,
`tool_name`, and `tool_version` fields where the artifact did not already carry
tool provenance, while keeping their v1 counterparts valid until
implementation PRs explicitly move producer output shapes.
Validation reports are the first artifact with an opt-in v2 producer path:
`hl7v2 val --report json --schema-version 2` emits the v2 shape, while the
default output remains v1. Python validation reports expose the same opt-in
shape through `report.to_dict(2)` and `report.to_json(2)`. Server validation
endpoints keep their existing v1-compatible response fields by default and add
`validation_report_v2` when requests include `"report_schema_version": 2`.
Profile lint reports can opt into their target v2 shape with
`hl7v2 profile lint --report json --schema-version 2`; defaults remain v1.
Profile explain reports can opt into their target v2 shape with
`hl7v2 profile explain --format json --schema-version 2`; defaults remain v1.
Profile test reports can opt into their target v2 shape with
`hl7v2 profile test --report json --schema-version 2`; defaults remain v1 and
the nested validation reports preserve their current serialized shape.
Corpus summary, fingerprint, and diff reports can opt into their target v2
shapes with `hl7v2 corpus summarize --format json --schema-version 2`,
`hl7v2 corpus fingerprint --format json --schema-version 2`, and `hl7v2 corpus
diff --format json --schema-version 2`. Python exposes the same opt-in shapes
with `corpus_summary(..., schema_version=2)`,
`corpus_fingerprint(..., schema_version=2)`, and
`corpus_diff(..., schema_version=2)`; defaults remain v1.
Redaction receipts can opt into their target v2 shape with
`hl7v2 redact --format json --schema-version 2`, Python
`redact(..., schema_version=2)`, or server `/hl7/validate-redacted` requests
that set `"redaction_receipt_schema_version": 2`; defaults remain v1.
Safe-analysis redaction output has a v1 schema that validates the default CLI
and Python output form with a nested receipt v1, plus the transitional
v1-compatible outer form with a nested receipt v2. CLI and Python can opt into
the target `safe-analysis-redaction-output-v2.schema.json` shape with
`hl7v2 redact --format json --schema-version 2` and
`redact(..., schema_version=2)`.
Server quarantine output summaries can opt into their target v2 shape with
`/hl7/validate-redacted` requests that set `"quarantine_schema_version": 2`;
defaults remain v1.
Evidence bundle summaries can opt into their target v2 shape with
`hl7v2 bundle ... --schema-version 2`, and Python exposes the same shape with
`bundle(..., schema_version=2)`. Server `/hl7/bundle` responses remain v1 by
default.
Evidence replay reports can opt into their target v2 shape with
`hl7v2 replay ... --format json --schema-version 2`, and Python exposes the
same shape with `replay(..., schema_version=2)`.
Bundle-internal `manifest.json`, `environment.json`, and `field-paths.json`
have target v2 schemas and fixtures. Live bundle writers still emit the v1
artifact shapes until a compatibility PR explicitly migrates bundle artifact
producers and replay verification together.

#### Evidence Null And Empty Semantics

Evidence schemas use a small set of conventions so CI jobs and data pipelines
can distinguish "not applicable" from "not evaluated":

- Required fields are always present, even when their value is empty.
- Empty arrays mean the category was evaluated and no entries were found. For
  example, `issues: []`, `parse_errors: []`, `new_segments: []`, and
  `validation_issue_code_counts: []` are successful empty results, not missing
  scans.
- Explicit `null` means the field is known but not applicable or not available
  for this artifact. For example, corpus `profile: null` means no profile was
  supplied, replay `message_type: null` means replay could not recover a message
  type, and replay `validation_valid: null` means validation was not regenerated.
- Optional fields that are absent are additive context fields, not failed
  checks. For example, `validation_report` is absent from replay reports when
  replay fails before a report can be regenerated.
- Numeric counters use `0` when the category was evaluated and empty. Consumers
  should prefer the explicit status fields, such as `valid`, `reproduced`, and
  replay check statuses, when deciding whether work succeeded.
- Profile explain fields such as `message_type`, `parent`, component bounds,
  datatype bounds, patterns, and expression guardrail limits use `null` for
  unspecified profile configuration.

These conventions are part of the evidence contract for the `*-v1` schemas.
Changing a field from absent to required, from empty array to omitted, or from
`null` to a sentinel string is a contract change.

#### Evidence Profile Labels

`ValidationReport.profile` is a display label for the profile used by that
surface, not a canonical cross-surface profile identity:

- CLI validation reports use the profile path supplied by the user.
- Server validation reports use the loaded profile `message_structure`.
- Python validation reports use the loaded profile `message_structure`.

Consumers that need reproducible profile identity should use artifacts that
carry `profile_sha256` or profile metadata, such as profile explain reports,
corpus fingerprints/diffs with a profile, or evidence bundle environment and
manifest files. Do not compare `ValidationReport.profile` across CLI, server,
and Python as if it were a stable hash or normalized profile id.

## Usage

### Validate YAML Against Schema

```bash
# Install ajv-cli and ajv-formats for validation
npm install -g ajv-cli ajv-formats

# Validate a profile
ajv validate -c ajv-formats -s schemas/profile/profile-v1.schema.json -d profiles/adt_a01.yaml --spec=draft7

# Validate all profiles
ajv validate -c ajv-formats -s schemas/profile/profile-v1.schema.json -d 'profiles/*.yaml' --spec=draft7

# Validate the checked-in TOML config fixture
python3 - <<'PY'
import json
import tomllib
from pathlib import Path

data = tomllib.loads(Path("config.example.toml").read_text(encoding="utf-8"))
Path("target/contracts/config").mkdir(parents=True, exist_ok=True)
Path("target/contracts/config/config.example.json").write_text(json.dumps(data), encoding="utf-8")
PY
ajv validate -c ajv-formats -s schemas/config/hl7v2-config-v1.schema.json -d target/contracts/config/config.example.json --spec=draft7

# Validate a representative evidence artifact fixture
ajv validate -c ajv-formats \
  -s schemas/evidence/validation-report-v1.schema.json \
  -d fixtures/evidence/validation-report.json \
  --spec=draft7
```

### In Rust Code

```rust
use schemars::{JsonSchema, schema_for};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct Profile {
    pub message_structure: String,
    pub version: String,
    // ...
}

// Generate schema at build time
fn main() {
    let schema = schema_for!(Profile);
    std::fs::write(
        "schemas/profile/profile-v1.schema.json",
        serde_json::to_string_pretty(&schema).unwrap()
    ).unwrap();
}
```

### CI Integration

The API Contracts workflow validates profiles, converted config fixtures, and
evidence fixtures, then compiles every schema:

```yaml
# .github/workflows/contracts.yml
- name: Validate Schemas
  run: |
    npm install -g ajv-cli ajv-formats
    ajv validate -c ajv-formats -s schemas/profile/profile-v1.schema.json -d 'profiles/*.yaml' --spec=draft7
    python3 - <<'PY'
    import json
    import tomllib
    from pathlib import Path

    data = tomllib.loads(Path("config.example.toml").read_text(encoding="utf-8"))
    Path("target/contracts/config").mkdir(parents=True, exist_ok=True)
    Path("target/contracts/config/config.example.json").write_text(json.dumps(data), encoding="utf-8")
    PY
    ajv validate -c ajv-formats -s schemas/config/hl7v2-config-v1.schema.json -d 'target/contracts/config/*.json' --spec=draft7
    for schema in schemas/evidence/*-v*.schema.json; do
      name="$(basename "$schema")"
      name="${name%.schema.json}"
      data="fixtures/evidence/${name}.json"
      if [ ! -f "$data" ]; then
        legacy_name="${name%-v1}"
        data="fixtures/evidence/${legacy_name}.json"
      fi
      ajv validate -c ajv-formats -s "$schema" -d "$data" --spec=draft7
    done
```

## Schema Versioning

Schemas are versioned with `-v1`, `-v2` suffixes. Breaking changes require:
1. Create new schema version (e.g., `profile-v2.schema.json`)
2. Update `$id` field
3. Maintain backward compatibility for 2 versions
4. Document migration path

Evidence artifacts also have a dedicated provenance/versioning plan in
[`docs/architecture/evidence-provenance-versioning.md`](../docs/architecture/evidence-provenance-versioning.md).
For evidence contracts, adding embedded `schema_version`, `tool_name`, or
`tool_version` fields to an already published `*-v1` JSON shape is a contract
change because evidence schemas use `additionalProperties: false`. Add those
fields through explicit v2 schemas and golden fixtures unless a PR documents a
different compatibility path.

## References

- [JSON Schema Specification](https://json-schema.org/)
- [Understanding JSON Schema](https://json-schema.org/understanding-json-schema/)
- [schemars Rust crate](https://docs.rs/schemars/)
