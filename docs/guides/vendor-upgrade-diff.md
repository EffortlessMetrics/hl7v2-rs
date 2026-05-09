# Vendor Upgrade Diff

This guide shows the before/after workflow for a vendor upgrade, interface
migration, acquisition cleanup, or feed regression. The goal is not to inspect
one message by hand. The goal is to produce deterministic evidence of what
changed across two corpora.

The examples use the product command name `hl7v2`. From a source checkout, use
`cargo run -q -p hl7v2-cli --` before each command instead:

```bash
cargo run -q -p hl7v2-cli -- corpus diff before/ after/ --format json
```

## What You Will Produce

| Artifact | Purpose |
| --- | --- |
| `before-summary.json` | Basic counts and parse errors for the pre-change feed. |
| `after-summary.json` | Basic counts and parse errors for the post-change feed. |
| `before-fingerprint.json` | Deterministic pre-change feed signature. |
| `after-fingerprint.json` | Deterministic post-change feed signature. |
| `corpus-diff.json` | Before/after drift report with message, segment, field, value-shape, parse, and validation issue deltas. |

These artifacts are designed for CI logs, vendor tickets, upgrade review, and
release evidence. They contain counts, paths, issue codes, and hashes, not raw
message bodies.

## Inputs

From the repository root, this guide uses:

| Path | Role |
| --- | --- |
| `test_data/valid_message.hl7` | Pre-change ADT-style message. |
| `test_data/invalid_message.hl7` | Post-change message with an invalid `PID.8` value and missing `PID.5`. |
| `profiles/generic.yaml` | Small profile that requires `PID.3` and checks `PID.8`. |

Create a working before/after layout:

```bash
mkdir -p target/hl7v2-vendor-upgrade-diff/before
mkdir -p target/hl7v2-vendor-upgrade-diff/after
mkdir -p target/hl7v2-vendor-upgrade-diff/reports
cp test_data/valid_message.hl7 target/hl7v2-vendor-upgrade-diff/before/site-a-001.hl7
cp test_data/invalid_message.hl7 target/hl7v2-vendor-upgrade-diff/after/site-a-001.hl7
```

For PowerShell:

```powershell
New-Item -ItemType Directory -Force target/hl7v2-vendor-upgrade-diff/before | Out-Null
New-Item -ItemType Directory -Force target/hl7v2-vendor-upgrade-diff/after | Out-Null
New-Item -ItemType Directory -Force target/hl7v2-vendor-upgrade-diff/reports | Out-Null
Copy-Item test_data/valid_message.hl7 target/hl7v2-vendor-upgrade-diff/before/site-a-001.hl7
Copy-Item test_data/invalid_message.hl7 target/hl7v2-vendor-upgrade-diff/after/site-a-001.hl7
```

## 1. Prove the Profile Is Loadable

Lint the profile before treating it as the comparison contract:

```bash
hl7v2 profile lint profiles/generic.yaml \
  --report json \
  --output target/hl7v2-vendor-upgrade-diff/reports/profile-lint.json
```

Expected fields:

```json
{
  "valid": true,
  "error_count": 0,
  "warning_count": 0,
  "issues": []
}
```

If profile lint reports errors, stop. A corpus diff is only meaningful when both
sides are compared against a contract that actually loads.

## 2. Summarize Both Sides

Run summaries first. They tell you whether the corpus can be parsed at all.

```bash
hl7v2 corpus summarize target/hl7v2-vendor-upgrade-diff/before \
  --format json \
  --output target/hl7v2-vendor-upgrade-diff/reports/before-summary.json

hl7v2 corpus summarize target/hl7v2-vendor-upgrade-diff/after \
  --format json \
  --output target/hl7v2-vendor-upgrade-diff/reports/after-summary.json
```

Expected fields for this fixture:

```json
{
  "file_count": 1,
  "message_count": 1,
  "parse_error_count": 0,
  "message_types": [
    {
      "value": "ADT^A01^ADT_A01",
      "count": 1
    }
  ]
}
```

If `parse_error_count` increases after the upgrade, fix framing, line endings,
encoding, or message syntax before trusting field-level drift. HL7 segment
separators are carriage returns (`\r`); LF-only samples can be rejected.

## 3. Fingerprint Before and After

Fingerprints are deterministic feed signatures. They include message type
counts, segment counts, field presence, field cardinality, value-shape stats,
parse error count, profile metadata, and validation issue-code counts.

```bash
hl7v2 corpus fingerprint target/hl7v2-vendor-upgrade-diff/before \
  --profile profiles/generic.yaml \
  --format json \
  --output target/hl7v2-vendor-upgrade-diff/reports/before-fingerprint.json

hl7v2 corpus fingerprint target/hl7v2-vendor-upgrade-diff/after \
  --profile profiles/generic.yaml \
  --format json \
  --output target/hl7v2-vendor-upgrade-diff/reports/after-fingerprint.json
```

Expected before fingerprint fields:

```json
{
  "fingerprint_version": "1",
  "message_count": 1,
  "parse_error_count": 0,
  "validation_issue_code_counts": []
}
```

Expected after fingerprint fields:

```json
{
  "fingerprint_version": "1",
  "message_count": 1,
  "parse_error_count": 0,
  "validation_issue_code_counts": [
    {
      "value": "value_not_in_set",
      "count": 1
    }
  ]
}
```

The `profile.sha256` field in each fingerprint proves both sides used the same
profile text. If the profile hashes differ, do not compare the outputs as one
upgrade event.

## 4. Produce the Drift Report

Run the actual before/after comparison:

```bash
hl7v2 corpus diff \
  target/hl7v2-vendor-upgrade-diff/before \
  target/hl7v2-vendor-upgrade-diff/after \
  --profile profiles/generic.yaml \
  --format json \
  --output target/hl7v2-vendor-upgrade-diff/reports/corpus-diff.json
```

Expected fields:

```json
{
  "diff_version": "1",
  "parse_error_count": {
    "before": 0,
    "after": 0,
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
      "before": 0,
      "after": 1,
      "delta": 1
    }
  ]
}
```

This example says:

- the parser still handled both corpora;
- the message count did not change;
- `PID.5` disappeared in the after corpus;
- one new `value_not_in_set` validation issue appeared after the change.

That is enough evidence to ask a vendor or interface owner a precise question:
"Why did `PID.5` stop appearing, and why did `PID.8` start emitting a value
outside the agreed value set?"

## 5. Decide What Fails CI

`hl7v2 corpus diff` reports drift; it does not decide your policy for you. A
useful CI gate usually fails on any of these conditions:

| Field | Typical gate |
| --- | --- |
| `parse_error_count.delta > 0` | New syntax/framing failures. |
| `validation_issue_code_counts[].delta > 0` | New contract failures. |
| `removed_message_types` is not empty | Feed stopped sending a message type. |
| `new_message_types` is not empty | Feed started sending an unreviewed message type. |
| Critical `field_presence[].message_count_delta < 0` | Required operational field disappeared. |

A PowerShell gate can inspect the JSON directly:

```powershell
$diff = Get-Content target/hl7v2-vendor-upgrade-diff/reports/corpus-diff.json -Raw | ConvertFrom-Json

if ($diff.parse_error_count.delta -gt 0) {
    throw "New parse errors appeared after the upgrade"
}

$newValidationIssues = @($diff.validation_issue_code_counts | Where-Object { $_.delta -gt 0 })
if ($newValidationIssues.Count -gt 0) {
    throw "New validation issue codes appeared after the upgrade"
}
```

Use the same idea in GitHub Actions, Jenkins, Azure Pipelines, or your migration
runbook: generate the diff, keep the JSON artifact, and fail only on the deltas
your team has agreed are release-blocking.

## 6. Attach Evidence, Not Raw Messages

For vendor or support escalation, attach:

- `profile-lint.json`
- `before-summary.json`
- `after-summary.json`
- `before-fingerprint.json`
- `after-fingerprint.json`
- `corpus-diff.json`
- the exact profile file, if it is safe to share

Do not attach raw HL7 messages unless your organization has approved that path.
If a specific failing message needs to travel with the ticket, create a redacted
bundle:

```bash
hl7v2 bundle failing.hl7 \
  --profile profiles/generic.yaml \
  --redact-policy safe-analysis.toml \
  --out issue-bundle/
```

Then verify it:

```bash
hl7v2 replay issue-bundle/ --format json
```

Only share the bundle when replay reports `"reproduced": true` and your
redaction policy is appropriate for the data.

## What You Proved

The workflow gives you a concrete evidence chain:

```text
profile lint
  -> before/after corpus summaries
  -> before/after profile-backed fingerprints
  -> corpus diff
  -> CI gate decision
  -> optional redacted bundle for one failing message
```

The important distinction is that a drift report is not a vague "the feed looks
different" claim. It records specific paths, counts, issue codes, profile hash,
tool version, and parse/validation deltas that can be reviewed, stored, and
replayed in later investigations.
