# Safe Support Bundle

This guide shows how to turn one failing HL7 message into a redacted,
replayable support packet. The goal is to give a vendor, support engineer,
data team, or agent enough evidence to reproduce the failure without sending
raw message PHI in reports, receipts, traces, manifests, or replay output.

The examples use the v1.5.0 installed CLI binary name `hl7v2-cli`. From a
source checkout, use `cargo run -q -p hl7v2-cli --` before each command
instead:

```bash
cargo run -q -p hl7v2-cli -- bundle failing.hl7 --profile profile.yaml --redact-policy safe-analysis.toml --out issue-bundle/
```

## What You Will Produce

| Artifact | Purpose |
| --- | --- |
| `redaction-preview.json` | Redaction receipt and redacted message preview before bundling. |
| `bundle-summary.json` | Machine-readable summary of the evidence bundle. |
| `issue-bundle/message.redacted.hl7` | Redacted HL7 message used for replay. |
| `issue-bundle/validation-report.json` | Validation report generated from the redacted message. |
| `issue-bundle/field-paths.json` | Field-path trace and redaction action metadata. |
| `issue-bundle/profile.yaml` | Profile used to reproduce validation. |
| `issue-bundle/redaction-receipt.json` | Receipt for retained, hashed, dropped, or missing fields. |
| `issue-bundle/environment.json` | Tool version, input/profile/policy hashes, and replay command. |
| `issue-bundle/manifest.json` | Bundle-relative artifact paths, roles, and SHA-256 hashes. |
| `issue-bundle/README.md` | Human-readable explanation of the bundle and its limits. |
| `replay-report.json` | Replay verification report. |

These artifacts are intended for support tickets and regression evidence. They
are not a general PHI detector. The redaction receipt proves the configured
policy ran; it does not prove every possible sensitive value in every profile or
free-text field was discovered.

## Inputs

From the repository root, this guide uses:

| Path | Role |
| --- | --- |
| `test_data/invalid_message.hl7` | Failing ADT-style message with an invalid `PID.8` value. |
| `profiles/generic.yaml` | Small profile that requires `PID.3` and checks `PID.8`. |
| `target/hl7v2-safe-support-bundle/safe-analysis.toml` | Safe-analysis redaction policy created below. |

Create a working directory:

```bash
mkdir -p target/hl7v2-safe-support-bundle/reports
```

For PowerShell:

```powershell
New-Item -ItemType Directory -Force target/hl7v2-safe-support-bundle/reports | Out-Null
```

## 1. Write a Fail-Closed Redaction Policy

Create a policy that protects patient identifier, patient name, and date of
birth, while retaining `PID.8` because the validation failure depends on it:

```toml
# target/hl7v2-safe-support-bundle/safe-analysis.toml
[[rules]]
path = "PID.3"
action = "hash"
reason = "patient identifier is needed for correlation without raw MRN"

[[rules]]
path = "PID.5"
action = "drop"
reason = "patient name is not needed for support analysis"

[[rules]]
path = "PID.7"
action = "drop"
reason = "date of birth is not needed for support analysis"

[[rules]]
path = "PID.8"
action = "retain"
reason = "administrative sex is required to reproduce the validation issue"
```

The safe-analysis policy is intentionally strict:

- a non-optional rule that matches nothing fails;
- a present built-in sensitive field without a rule fails;
- retaining a built-in sensitive field fails;
- duplicate paths fail.

That makes policy mistakes visible before an evidence packet is created.

## 2. Preview Redaction Before Bundling

Run redaction first and write the JSON preview to a file:

```bash
hl7v2-cli redact test_data/invalid_message.hl7 \
  --policy target/hl7v2-safe-support-bundle/safe-analysis.toml \
  --format json \
  --output target/hl7v2-safe-support-bundle/reports/redaction-preview.json
```

Expected fields:

```json
{
  "message_type": "ADT^A01^ADT_A01",
  "receipt": {
    "phi_removed": true,
    "actions": [
      {
        "path": "PID.3",
        "action": "hash"
      },
      {
        "path": "PID.5",
        "action": "drop"
      },
      {
        "path": "PID.7",
        "action": "drop"
      },
      {
        "path": "PID.8",
        "action": "retain"
      }
    ]
  }
}
```

Before sharing anything, inspect the preview:

- `receipt.phi_removed` should be `true`;
- no raw patient name, MRN, DOB, address, phone, or local raw file path should
  appear;
- retained fields should be justified by the support question.

For the sample fixture, `PID.8` remains visible because the support question is
why `PID.8 = X` violates the profile value set.

## 3. Build the Evidence Bundle

Create the bundle. The output directory must not already exist:

```bash
hl7v2-cli bundle test_data/invalid_message.hl7 \
  --profile profiles/generic.yaml \
  --redact-policy target/hl7v2-safe-support-bundle/safe-analysis.toml \
  --out target/hl7v2-safe-support-bundle/issue-bundle \
  --output target/hl7v2-safe-support-bundle/reports/bundle-summary.json
```

Expected summary fields:

```json
{
  "bundle_version": "1",
  "output_dir": ".",
  "message_type": "ADT^A01",
  "validation_valid": false,
  "validation_issue_count": 1,
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

The bundle is useful even though `validation_valid` is `false`; that is the
failure you are trying to reproduce. What matters for shareability is that PHI
was removed according to policy and replay can reproduce the same validation
report.

## 4. Inspect the Ticket Evidence

Open these artifacts before attaching the bundle to a ticket:

| Artifact | Check |
| --- | --- |
| `validation-report.json` | Stable issue code, path, severity, and message describe the failure. |
| `redaction-receipt.json` | Every configured rule has the expected action and reason. |
| `field-paths.json` | Paths and value shapes are present without raw sensitive values. |
| `environment.json` | Tool version, hashes, and replay command are present. |
| `manifest.json` | Every artifact has a bundle-relative path, role, and SHA-256 hash. |
| `README.md` | The packet explains how to replay it and what safety limits remain. |

For this fixture, the validation report should include:

```json
{
  "valid": false,
  "issue_count": 1,
  "issues": [
    {
      "code": "value_not_in_set",
      "path": "PID.8",
      "severity": "error"
    }
  ]
}
```

The manifest gives the recipient an integrity check. If any artifact changes
after the bundle is created, replay should fail before trusting the regenerated
validation report.

## 5. Replay the Bundle

Run replay and keep the report:

```bash
hl7v2-cli replay target/hl7v2-safe-support-bundle/issue-bundle \
  --format json \
  --output target/hl7v2-safe-support-bundle/reports/replay-report.json
```

Expected fields:

```json
{
  "replay_version": "1",
  "bundle_version": "1",
  "message_type": "ADT^A01",
  "reproduced": true,
  "validation_valid": false,
  "validation_issue_count": 1,
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

If `reproduced` is not `true`, do not send the bundle as proof. The replay
checks identify whether the packet is missing artifacts, has hash mismatches,
contains a malformed manifest, fails to parse the redacted message, cannot load
the profile, or regenerates a different validation report.

## 6. What to Share

For a normal support or vendor ticket, attach:

```text
issue-bundle/
reports/bundle-summary.json
reports/replay-report.json
```

Include the short interpretation:

```text
The redacted bundle replays successfully.
The redacted message still fails validation with value_not_in_set at PID.8.
The bundle manifest hashes pass, and replay regenerated the stored report.
```

Do not attach the raw input message. Do not attach the raw redaction policy if
it contains internal classification details you do not want to share; attach the
bundle `redaction-receipt.json` instead. The bundled `profile.yaml` is included
as supplied by the user, so review it before sharing if profiles can contain
site-specific comments, identifiers, or local operational notes.

## 7. What This Does Not Guarantee

This workflow is designed to avoid obvious raw PHI leakage in generated reports,
receipts, traces, manifests, and replay output. It does not guarantee:

- free-text fields are safe unless your policy protects them;
- the profile file is sanitized;
- every local identifier is PHI-free;
- a retained field is safe to disclose in every jurisdiction or contract;
- the recipient can ignore their own privacy review.

Use the packet as deterministic technical evidence, not as a substitute for
your organization's disclosure policy.

## Workflow Summary

```text
failing message
  -> fail-closed safe-analysis policy
  -> redaction preview
  -> redacted evidence bundle
  -> manifest/hash verification
  -> replayed validation report
  -> shareable support packet
```

The useful claim is narrow and testable: another person can replay the redacted
packet and see the same validation failure without needing the original message.
