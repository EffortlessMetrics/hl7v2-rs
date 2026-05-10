# First 10 Minutes

This guide proves the local CLI can turn HL7 input into evidence artifacts:
diagnostics, profile checks, validation reports, corpus summaries, fingerprints,
diffs, a redacted bundle, and replay verification.

The examples use the product command name `hl7v2`. From a source checkout, use
`cargo run -q -p hl7v2-cli --` before each command instead:

```bash
cargo run -q -p hl7v2-cli -- doctor --format json
```

## Inputs

From the repository root, this guide uses:

| Path | Role |
| --- | --- |
| `test_data/valid_message.hl7` | Valid ADT-style message for the generic profile. |
| `test_data/invalid_message.hl7` | Parseable message with an invalid `PID.8` value. |
| `profiles/generic.yaml` | Small profile that requires `PID.3` and checks `PID.8`. |

Create a working fixture layout for profile testing and corpus commands:

```bash
mkdir -p target/hl7v2-first-10-minutes/fixtures/valid
mkdir -p target/hl7v2-first-10-minutes/fixtures/invalid
cp test_data/valid_message.hl7 target/hl7v2-first-10-minutes/fixtures/valid/valid_message.hl7
cp test_data/invalid_message.hl7 target/hl7v2-first-10-minutes/fixtures/invalid/invalid_message.hl7
```

For PowerShell:

```powershell
New-Item -ItemType Directory -Force target/hl7v2-first-10-minutes/fixtures/valid | Out-Null
New-Item -ItemType Directory -Force target/hl7v2-first-10-minutes/fixtures/invalid | Out-Null
Copy-Item test_data/valid_message.hl7 target/hl7v2-first-10-minutes/fixtures/valid/valid_message.hl7
Copy-Item test_data/invalid_message.hl7 target/hl7v2-first-10-minutes/fixtures/invalid/invalid_message.hl7
```

## 1. Run Doctor

```bash
hl7v2 doctor --format json
```

Expected output includes local diagnostics:

```json
{
  "version": "1.4.0",
  "checks": [
    {
      "name": "sample-parse",
      "status": "ok"
    },
    {
      "name": "mllp-roundtrip",
      "status": "ok"
    }
  ]
}
```

If this fails, fix the local install before testing real feeds. If Python is not
installed, the Python binding check can warn without blocking the Rust CLI.

## 2. Generate and Validate a Built-In Sample

```bash
hl7v2 sample --type ADT_A01 --output target/hl7v2-first-10-minutes/sample.hl7
hl7v2 validate-sample --type ADT_A01 --profile profiles/generic.yaml --report json --schema-version 2
```

Expected validation output includes:

```json
{
  "schema_version": "2",
  "tool_name": "hl7v2-cli",
  "valid": true,
  "message_type": "ADT^A01"
}
```

If this fails, the local profile is not aligned with the built-in ADT_A01
sample. Fix that before moving to site-specific messages.

## 3. Lint and Explain the Profile

```bash
hl7v2 profile lint profiles/generic.yaml --report json
```

Expected output:

```json
{
  "valid": true,
  "error_count": 0,
  "warning_count": 0,
  "issues": []
}
```

Then inspect what the profile actually enforces:

```bash
hl7v2 profile explain profiles/generic.yaml --format json
```

Expected fields include:

```json
{
  "message_structure": "GENERIC",
  "summary": {
    "required_field_count": 1,
    "value_set_count": 1
  },
  "required_fields": [
    {
      "path": "PID.3"
    }
  ]
}
```

If lint fails, treat that as a profile input error. Fix the profile before
trusting validation, corpus fingerprints, or evidence bundles built from it.

## 4. Validate One Message

```bash
hl7v2 val test_data/valid_message.hl7 --profile profiles/generic.yaml --report json
```

Expected output:

```json
{
  "valid": true,
  "message_type": "ADT^A01",
  "issue_count": 0,
  "issues": []
}
```

Now validate the invalid fixture:

```bash
hl7v2 val test_data/invalid_message.hl7 --profile profiles/generic.yaml --report json
```

Expected behavior:

- exit code `1`
- JSON still goes to stdout
- stderr contains the top-level failure
- the report includes `code: "value_not_in_set"` and `path: "PID.8"`

This is the automation contract: pipelines can keep the evidence report while
still failing the job.

## 5. Test the Profile Fixtures

```bash
hl7v2 profile test profiles/generic.yaml target/hl7v2-first-10-minutes/fixtures --report json
```

Expected output:

```json
{
  "valid": true,
  "case_count": 2,
  "passed_count": 2,
  "failed_count": 0
}
```

The `valid/` fixture must validate. The `invalid/` fixture must fail validation.
If either expectation is wrong, the profile test command exits `1` and reports
the case-level failure.

## 6. Summarize and Fingerprint the Corpus

```bash
hl7v2 corpus summarize target/hl7v2-first-10-minutes/fixtures --format json
```

Expected fields:

```json
{
  "file_count": 2,
  "message_count": 2,
  "parse_error_count": 0,
  "message_types": [
    {
      "value": "ADT^A01^ADT_A01",
      "count": 2
    }
  ]
}
```

Create a deterministic feed signature with profile-backed validation counts:

```bash
hl7v2 corpus fingerprint target/hl7v2-first-10-minutes/fixtures \
  --profile profiles/generic.yaml \
  --format json
```

Expected fields:

```json
{
  "fingerprint_version": "1",
  "message_count": 2,
  "parse_error_count": 0,
  "validation_issue_code_counts": [
    {
      "value": "value_not_in_set",
      "count": 1
    }
  ]
}
```

If `parse_error_count` is not zero for files you expected to parse, inspect
line endings first. HL7 segment separators are carriage returns (`\r`);
LF-only samples can be rejected.

## 7. Diff Before and After

Use the valid fixture as `before` and the invalid fixture as `after`:

```bash
hl7v2 corpus diff \
  target/hl7v2-first-10-minutes/fixtures/valid \
  target/hl7v2-first-10-minutes/fixtures/invalid \
  --profile profiles/generic.yaml \
  --format json
```

Expected fields:

```json
{
  "diff_version": "1",
  "parse_error_count": {
    "delta": 0
  },
  "field_presence": [
    {
      "path": "PID.5",
      "message_count_delta": -1
    }
  ],
  "validation_issue_code_counts": [
    {
      "value": "value_not_in_set",
      "delta": 1
    }
  ]
}
```

This is the vendor-upgrade and migration workflow in miniature: same message
type, same parser status, but field presence and validation evidence changed.

## 8. Redact, Bundle, and Replay

Create a safe-analysis policy:

```toml
# target/hl7v2-first-10-minutes/safe-analysis.toml
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
reason = "administrative sex is needed for validation"
```

Build the evidence bundle:

```bash
hl7v2 bundle test_data/valid_message.hl7 \
  --profile profiles/generic.yaml \
  --redact-policy target/hl7v2-first-10-minutes/safe-analysis.toml \
  --out target/hl7v2-first-10-minutes/issue-bundle
```

Expected output:

```json
{
  "bundle_version": "1",
  "output_dir": ".",
  "message_type": "ADT^A01",
  "validation_valid": true,
  "redaction_phi_removed": true,
  "artifacts": [
    "message.redacted.hl7",
    "validation-report.json",
    "field-paths.json",
    "profile.yaml",
    "redaction-receipt.json",
    "environment.json",
    "replay.sh",
    "replay.ps1",
    "README.md",
    "manifest.json"
  ]
}
```

Replay the packet:

```bash
hl7v2 replay target/hl7v2-first-10-minutes/issue-bundle --format json
```

Expected output:

```json
{
  "replay_version": "1",
  "reproduced": true,
  "checks": [
    {
      "name": "manifest-hashes",
      "status": "pass"
    },
    {
      "name": "report-match",
      "status": "pass"
    },
    {
      "name": "environment-match",
      "status": "pass"
    }
  ]
}
```

If replay exits `1`, do not share the bundle as proof. The replay report tells
you whether the failure is a missing artifact, hash mismatch, parse/profile
problem, report mismatch, or environment mismatch.

## What You Proved

In ten minutes, you produced the core evidence loop:

```text
profile lint
  -> validation report
  -> profile fixture test
  -> corpus summary
  -> corpus fingerprint
  -> corpus diff
  -> redacted bundle
  -> replay verification
```

The important artifacts are stable JSON outputs with issue codes, HL7 paths,
profile hashes where relevant, redaction receipts, manifest hashes, and replay
checks. They are designed to be kept in CI logs, attached to support tickets, or
used as regression evidence during interface changes.
