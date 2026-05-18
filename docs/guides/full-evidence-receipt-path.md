# Full Evidence Receipt Path

This guide gives one job-first path from an HL7 message to evidence a user can
inspect, share, and replay. It is not a feature tour and it does not require
learning the workspace layout first.

The shape is the same across the current surfaces:

```text
message.hl7
  -> validate
  -> redact
  -> bundle
  -> replay
  -> evidence summary
```

Use this guide when the question is:

```text
I have an HL7 message. Is it valid, can I safely share a diagnostic packet,
and can someone else replay the result?
```

From a source checkout, the executable guide smoke is:

```powershell
cargo +1.95.0 run -p xtask -- check-first-use-guides
```

That command runs the CLI receipt recipe below into
`target/hl7v2-receipt`, checks the Rust and CLI user-journey acceptance
tests, and keeps Python registry and npm proof as explicit non-claims. Use
`--include-python` only after installing a local `hl7v2` wheel, and use
`--include-public-crates` only when refreshing crates.io install-back proof.

## Current Release Boundary

Rust, CLI, and server users can use the published v1.5.0 crates:

```bash
cargo add hl7v2
cargo install hl7v2-cli --version 1.5.0
cargo install hl7v2-server --version 1.5.0
```

Python is different. The public Python package is `hl7v2`, but TestPyPI/PyPI
upload and install-back proof is still blocked by Trusted Publisher setup.
Until that proof lands, use the local wheel proof:

```powershell
cargo +1.95.0 run -p xtask -- python-local-wheel-proof
```

That command proves local wheel build, install, `import hl7v2`, smoke tests, and
the Python evidence scripts. It is not a TestPyPI or PyPI claim.

## Shared Inputs

The repository includes sample inputs that are safe for a first run:

| Path | Purpose |
| --- | --- |
| `test_data/invalid_message.hl7` | Message that parses but fails validation. |
| `profiles/generic.yaml` | Small validation profile. |
| `target/hl7v2-receipt/safe-analysis.toml` | Redaction policy you create for the run. |

Create a work area:

```powershell
New-Item -ItemType Directory -Force target/hl7v2-receipt/reports | Out-Null
```

Create `target/hl7v2-receipt/safe-analysis.toml`:

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
path = "PID.7"
action = "drop"
reason = "date of birth"

[[rules]]
path = "PID.8"
action = "retain"
reason = "administrative sex is required to reproduce validation"
```

## CLI Receipt

The CLI is the shortest operator path. It proves local tool health, validates
the message, previews redaction, creates a support bundle, and replays it.
The sample message is intentionally invalid, so the validation command may
return a non-zero exit code while still writing `validation-report.json`.

```powershell
hl7v2-cli doctor --format json

hl7v2-cli val test_data/invalid_message.hl7 `
  --profile profiles/generic.yaml `
  --report json `
  --output target/hl7v2-receipt/reports/validation-report.json

hl7v2-cli redact test_data/invalid_message.hl7 `
  --policy target/hl7v2-receipt/safe-analysis.toml `
  --format json `
  --output target/hl7v2-receipt/reports/redaction-preview.json

hl7v2-cli support-bundle test_data/invalid_message.hl7 `
  --profile profiles/generic.yaml `
  --redact-policy target/hl7v2-receipt/safe-analysis.toml `
  --out target/hl7v2-receipt/issue-bundle `
  --output target/hl7v2-receipt/reports/bundle-summary.json

hl7v2-cli replay target/hl7v2-receipt/issue-bundle `
  --format json `
  --output target/hl7v2-receipt/reports/replay-report.json
```

What this proves:

- the CLI can run and diagnose itself;
- the message fails validation with a stable report;
- the configured redaction policy ran before sharing;
- the support bundle contains hashed artifacts and replay instructions;
- replay can reproduce the stored validation result.

What it does not prove:

- it does not prove every possible PHI value was removed;
- it does not prove the profile file is safe to disclose;
- it does not prove Python public registry availability.

Use [Safe Support Bundle](safe-support-bundle.md) for the detailed artifact
list and ticket-sharing guidance.

## Server Receipt

Use the server path when the product is running as a sidecar or local service.
The flow is:

```text
--print-config
  -> GET /ready
  -> POST /hl7/validate-redacted
  -> POST /hl7/bundle
  -> POST /hl7/replay
```

Start with sanitized configuration:

```powershell
$env:HL7V2_CONFIG = "target/hl7v2-sidecar/server.toml"
$env:HL7V2_API_KEY = "dev-secret"
$env:HL7V2_PROFILE_PATHS = "profiles/generic.yaml"

hl7v2-server --print-config
hl7v2-server
```

The guide smoke does not start a server by default. Server first-use proof is
owned by the sidecar smoke path: start the sidecar from
[Deploy Validation Sidecar](deploy-validation-sidecar.md), then run:

```powershell
$env:HL7V2_SERVER_URL = "http://127.0.0.1:18080"
python tests/server_smoke/smoke.py
```

Then, in another shell:

```powershell
Invoke-RestMethod http://127.0.0.1:18080/ready
```

The detailed request bodies for `validate-redacted`, `bundle`, and `replay`
live in [Deploy Validation Sidecar](deploy-validation-sidecar.md). Use those
commands with the same sample message, profile, and redaction policy above.

What this proves:

- effective configuration is printable without exposing the API key;
- readiness checks pass before traffic;
- redacted validation can produce a safe report;
- bundle and replay use configured server roots instead of request-supplied
  filesystem paths.

What it does not prove:

- it does not prove the sidecar is internet-ready;
- it does not authorize logging raw request bodies;
- it does not imply every REST endpoint has a matching gRPC claim.

## Python Receipt

Use Python when the job belongs in a notebook, QA script, or data-repair
automation. Until public registry proof lands, start with the local wheel:

```powershell
cargo +1.95.0 run -p xtask -- python-local-wheel-proof
```

That command runs:

```text
wheel build
  -> scratch venv install
  -> import hl7v2
  -> tests/python_smoke/smoke.py
  -> tests/python_smoke/evidence_workflow_guide.py
  -> tests/python_smoke/dirty_evidence_workflow.py
```

Then use [Python Evidence Workflow](python-evidence-workflow.md) for the full
scripted receipt path: profile reports, validation, ACK, corpus summary,
fingerprint, diff, redaction, bundle, and replay.

What this proves:

- the local Python wheel can be built and imported;
- Python helpers produce the same local evidence semantics as the Rust core;
- the Python evidence workflow can create and replay a bundle.

What it does not prove:

- it does not prove `pip install hl7v2` from TestPyPI or PyPI;
- it does not prove production PyPI release success;
- it does not make `hl7v2-python` the Rust user API.

## Rust Receipt

Use Rust when embedding the evidence workflow in an application. The public
dependency is `hl7v2`:

```bash
cargo add hl7v2
```

The first-use Rust example in
[First Use By Surface](first-use-by-surface.md#rust-library) shows parse,
validation, normalization, and ACK output. For the complete bundle/replay
operator loop, use the CLI receipt above as the reference workflow and call the
same shared evidence APIs from your application.

What this proves:

- Rust code uses the canonical product crate;
- application code can produce the same validation and evidence artifacts;
- runtime wrappers are not required when embedding the library directly.

What it does not prove:

- it does not prove CLI, server, or Python packaging;
- it does not change the binding-backend boundary for `hl7v2-python`.

## Interpreting The Receipt

After any route, inspect the artifacts in this order:

1. `validation-report.json`: check `valid`, `issue_count`, issue `code`,
   issue `path`, and `severity`.
2. `redaction-preview.json` or `redaction-receipt.json`: confirm
   `phi_removed` and retained-field reasons.
3. `bundle-summary.json`: confirm artifact list, validation status, and
   redaction status.
4. `manifest.json`: confirm bundle-relative paths and hashes.
5. `SAFE-SHARING.md`: review replay, retained-field, profile, raw-input, and
   whole-bundle sharing checks.
6. `replay-report.json`: require `reproduced = true` before treating the
   bundle as shareable proof.

Use [Evidence Artifacts For Operators](evidence-artifacts-for-operators.md) to
interpret each artifact's proof boundary, PHI posture, schema-version behavior,
and safe-sharing limits.

## Stop Conditions

Stop and fix the earlier step when:

- `doctor` fails;
- the profile does not lint;
- redaction policy validation fails;
- a retained sensitive field lacks a support reason;
- bundle creation reports missing artifacts;
- replay does not reproduce;
- a report includes raw HL7, raw local filesystem roots, API keys, tokens, or
  unexpected patient identifiers.

Do not attach a bundle to a vendor ticket until replay passes and the retained
fields have been reviewed.

## Current Non-Claims

- A crates.io `hl7v2-python` backend publish is not a PyPI `hl7v2` proof.
- A local Python wheel proof is not a TestPyPI or PyPI proof.
- A server receipt is not a deployment security review.
- A redaction receipt proves configured policy actions, not universal PHI
  absence.
- Future TypeScript users should install `@effortlessmetrics/hl7v2`; no npm
  package is claimed here.
