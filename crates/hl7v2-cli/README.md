# hl7v2-cli

Command-line interface for HL7 v2 message manipulation and validation.

## Usage

Run first-use diagnostics:

```bash
hl7v2 doctor
hl7v2 doctor --sample message.hl7 --profile profiles/adt_a01.yaml
hl7v2 doctor --server-url http://127.0.0.1:8080/health --format json
```

Lint a profile before using it as an interface contract:

```bash
hl7v2 profile lint profiles/adt_a01.yaml
hl7v2 profile lint profiles/adt_a01.yaml --report json
hl7v2 profile explain profiles/adt_a01.yaml --format json
hl7v2 profile test profiles/adt_a01.yaml fixtures/adt_a01/ --report json
```

Summarize a directory or file corpus:

```bash
hl7v2 corpus summarize corpus/
hl7v2 corpus summarize corpus/ --format json
hl7v2 corpus fingerprint corpus/ --format json
hl7v2 corpus fingerprint corpus/ --profile profiles/adt_a01.yaml --format json
hl7v2 corpus diff feeds/before feeds/after --profile profiles/adt_a01.yaml --format json
```

Redact a message for safe analysis:

```bash
hl7v2 redact message.hl7 --policy safe-analysis.toml --format json
hl7v2 redact message.hl7 --policy safe-analysis.toml --format hl7 > message.redacted.hl7
hl7v2 bundle message.hl7 --profile profiles/adt_a01.yaml --redact-policy safe-analysis.toml --out issue-bundle/
```

Policy files use explicit rules with required reasons:

```toml
[[rules]]
path = "PID.3"
action = "hash"
reason = "patient identifier"

[[rules]]
path = "PID.5"
action = "drop"
reason = "patient name"

[[rules]]
path = "OBX.3"
action = "retain"
reason = "observation identifier is needed for analysis"
```

Rules default to fail-closed: `hash` and `drop` rules must match at least one
field unless `optional = true` is set. Built-in sensitive fields such as
`PID.3`, `PID.5`, `PID.7`, `PID.11`, `PID.13`, `PID.19`, and next-of-kin
fields must be protected when present and cannot be retained.

`hl7v2 bundle` writes a redacted evidence packet containing:

- `message.redacted.hl7`
- `validation-report.json`
- `field-paths.json`
- `profile.yaml`
- `redaction-receipt.json`
- `environment.json`
- `replay.sh` and `replay.ps1`

For usage examples, see the [examples/](https://github.com/EffortlessMetrics/hl7v2-rs/tree/main/examples) directory in the root of the repository.
