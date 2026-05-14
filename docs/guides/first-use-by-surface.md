# First Use By Surface

This guide routes a new user to the right install surface without requiring
them to understand the workspace topology. It gives one first useful receipt
for each current runtime:

- Rust users depend on `hl7v2`.
- Operators use `hl7v2-cli` and `hl7v2-server`.
- Python users import `hl7v2`.

The repository may contain binding backend crates such as `hl7v2-python`, but
those are packaging and provenance surfaces. They are not the recommended Rust
API and they are not the Python package name.

## Current Release Boundary

`v1.5.0` is prepared and dry-run proven, but it is not a published release until
the crates.io upload, tag, GitHub release, and registry-resolution receipts
exist. For current source-checkout proof, use the `cargo run` and local wheel
commands below. After the release is published, the registry install commands
become the normal user path.

## Rust Library

Released user path after publication:

```bash
cargo add hl7v2
cargo add serde_json
```

`hl7v2` is the product dependency. `serde_json` is used below only to print the
first validation receipt.

Source-checkout proof from this repository:

```bash
cargo test -p hl7v2 --all-features
```

Minimal Rust shape:

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let profile_yaml = r#"
message_structure: "GENERIC"
version: "2.5"
segments:
  - id: "MSH"
  - id: "PID"
constraints:
  - path: "PID.3"
    required: true
"#;

    let hl7 = b"MSH|^~\\&|SEND|FAC|RECV|FAC|202605140101||ADT^A01|CTRL1|P|2.5\rPID|1||MRN-1^^^HOSP^MR||Example^Patient";
    let message = hl7v2::parse(hl7)?;
    let profile = hl7v2::load_profile_checked(profile_yaml)?;
    let issues = hl7v2::validate(&message, &profile);
    let report =
        hl7v2::ValidationReport::from_issues(&message, Some("inline".to_string()), issues);
    let normalized = hl7v2::normalize(hl7, true)?;
    let ack = hl7v2::ack(&message, hl7v2::AckCode::AA)?;

    println!("{}", serde_json::to_string_pretty(&report)?);
    println!("normalized_bytes={}", normalized.len());
    println!("{}", String::from_utf8_lossy(&hl7v2::write(&ack)));
    Ok(())
}
```

First useful receipt:

```text
validation report JSON + normalized byte count + ACK output
```

## CLI

Released user path after publication:

```bash
cargo install hl7v2-cli
```

Source-checkout command shape:

```bash
cargo run -q -p hl7v2-cli -- doctor --format json
```

Then run the full CLI evidence loop:

```bash
cargo run -q -p hl7v2-cli -- profile lint profiles/generic.yaml --report json
cargo run -q -p hl7v2-cli -- val test_data/valid_message.hl7 --profile profiles/generic.yaml --report json
cargo run -q -p hl7v2-cli -- corpus summarize test_data --format json
```

For the copy/paste ten-minute flow that adds corpus fingerprint, corpus diff,
redacted bundle, and replay proof, use
[First 10 Minutes](first-10-minutes.md).

First useful receipt:

```text
doctor JSON + profile lint JSON + validation report JSON + corpus summary JSON
```

## Server

Released user path after publication:

```bash
cargo install hl7v2-server
```

Source-checkout command shape:

```bash
cargo run -q -p hl7v2-server -- --print-config
cargo run -q -p hl7v2-server --
```

In another shell, check readiness:

```bash
curl http://127.0.0.1:8080/ready
```

Create a validation receipt:

```bash
curl -X POST http://127.0.0.1:8080/hl7/validate \
  -H "Content-Type: application/json" \
  -d '{
    "message": "MSH|^~\\&|SEND|FAC|RECV|FAC|202605140101||ADT^A01|CTRL1|P|2.5\rPID|1||MRN-1^^^HOSP^MR||Example^Patient",
    "profile": "message_structure: \"GENERIC\"\nversion: \"2.5\"\nsegments:\n  - id: \"MSH\"\n  - id: \"PID\"\nconstraints:\n  - path: \"PID.3\"\n    required: true\n"
  }'
```

For deployment, authentication, redacted validation, bundles, replay, metrics,
and quarantine hooks, use
[Deploy Validation Sidecar](deploy-validation-sidecar.md).

First useful receipt:

```text
sanitized config + /ready response + validation report JSON
```

## Python

Future registry user path after TestPyPI/PyPI proof:

```bash
python -m pip install hl7v2
python -c "import hl7v2; print(hl7v2.__version__)"
```

Current source-checkout proof uses a local wheel:

```powershell
python -m pip install --upgrade pip "maturin==1.13.1"
$env:PYO3_USE_ABI3_FORWARD_COMPATIBILITY = "1"
maturin build --release --out dist
python -m pip install --force-reinstall (Get-ChildItem dist\*.whl | Select-Object -First 1).FullName
python -c "import hl7v2; print(hl7v2.__version__)"
```

Minimal Python shape:

```python
import json
import hl7v2

profile_yaml = """
message_structure: "GENERIC"
version: "2.5"
segments:
  - id: "MSH"
  - id: "PID"
constraints:
  - path: "PID.3"
    required: true
"""

message = (
    "MSH|^~\\&|SEND|FAC|RECV|FAC|202605140101||ADT^A01|CTRL1|P|2.5\r"
    "PID|1||MRN-1^^^HOSP^MR||Example^Patient"
)

report = hl7v2.validate(message, profile_yaml).to_dict(2)
summary = hl7v2.corpus_summary("test_data", schema_version=2)

print(json.dumps(
    {
        "version": hl7v2.__version__,
        "valid": report["valid"],
        "message_type": report["message_type"],
        "corpus_messages": summary["message_count"],
    },
    indent=2,
    sort_keys=True,
))
```

For the full Python evidence workflow with profile reports, redaction, bundle,
and replay, use [Python Evidence Workflow](python-evidence-workflow.md).

First useful receipt:

```text
import smoke + validation report dict + corpus summary dict
```

## What Not To Infer

- A crates.io `hl7v2-python` backend proof is not a PyPI `hl7v2` proof.
- A local Python wheel proof is not a TestPyPI or PyPI upload proof.
- A server validation response does not imply every REST endpoint has gRPC
  parity.
- A future TypeScript package must use `@effortlessmetrics/hl7v2`, not
  `hl7v2-rs`.

Use [HL7V2-SPEC-0006](../specs/HL7V2-SPEC-0006-cross-surface-evidence-parity.md)
for the cross-surface parity contract and
[STATUS.md](../STATUS.md) for the current release and support state.
