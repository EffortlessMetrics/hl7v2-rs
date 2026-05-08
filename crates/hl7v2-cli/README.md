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
```

For usage examples, see the [examples/](https://github.com/EffortlessMetrics/hl7v2-rs/tree/main/examples) directory in the root of the repository.
