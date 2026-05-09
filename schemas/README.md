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
- Validation reports and profile lint/test/explain reports
- Corpus summary, fingerprint, and diff reports
- Redaction receipts and evidence bundle/replay summaries

`validation-report-v2.schema.json` is the first target evidence schema with
embedded `schema_version`, `tool_name`, and `tool_version` fields. It is a
contract artifact for the planned v2 migration; current v1 producers remain
valid until implementation PRs explicitly move their output shape.

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
