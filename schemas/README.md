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
schema syntax:

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
```

## Schema Versioning

Schemas are versioned with `-v1`, `-v2` suffixes. Breaking changes require:
1. Create new schema version (e.g., `profile-v2.schema.json`)
2. Update `$id` field
3. Maintain backward compatibility for 2 versions
4. Document migration path

## References

- [JSON Schema Specification](https://json-schema.org/)
- [Understanding JSON Schema](https://json-schema.org/understanding-json-schema/)
- [schemars Rust crate](https://docs.rs/schemars/)
