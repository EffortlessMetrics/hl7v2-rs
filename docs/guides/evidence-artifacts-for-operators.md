# Evidence Artifacts For Operators

HL7v2 evidence artifacts are receipts. They record what the tool saw, which
checks ran, what changed, what was redacted, and whether another person can
replay the packet. This guide explains how to read those receipts without
needing the repository layout.

For the normative schema and producer map, use the
[Evidence Contract Index](../contracts/evidence-contract-index.md). For the
schema files themselves, use [schemas/README.md](../../schemas/README.md). For
stable/advisory field rules and semver expectations, use the
[Evidence Artifact Compatibility Policy](../contracts/evidence-artifact-compatibility-policy.md).

Executable source-checkout smoke:

```text
cargo +1.95.0 run -p xtask -- check-evidence-artifacts-guide
```

That command generates representative doctor, profile, validation, corpus,
redaction, bundle, manifest, environment, field-path, and replay artifacts
under `target/hl7v2-evidence-artifacts/` and checks the reader fields this
guide tells operators to inspect.

## Quick Routing

| Artifact | Use when | Safe to share by default? | First check |
| --- | --- | --- | --- |
| Doctor report | You need local tool and environment proof. | Usually, after checking local paths. | Required checks are `ok` or explainable warnings. |
| Profile lint report | You need to know whether a profile file is usable. | Usually, after reviewing profile-derived messages. | `valid` is true and `error_count` is zero. |
| Profile explain report | You need to show what a profile enforces. | Review first. It may reveal profile rules or value-set names. | Required fields, value sets, and profile hash match the intended profile. |
| Profile test report | You need proof that fixtures pass or fail as expected. | Usually, after reviewing fixture labels. | Expected-valid and expected-invalid cases behave as declared. |
| Validation report | You need proof that one message passed or failed validation. | Usually, but confirm no raw payload was attached elsewhere. | `valid`, `issue_count`, issue `code`, issue `path`, and `severity`. |
| Corpus summary | You need counts and parse-health for a folder or batch. | Usually. It should not include raw messages. | `message_count`, `parse_error_count`, and message-type counts. |
| Corpus fingerprint | You need a deterministic feed signature. | Usually. It should not include raw field values. | Fingerprint version, counts, value-shape stats, and profile hash if used. |
| Corpus diff | You need before/after drift evidence. | Usually. It reports deltas, not raw messages. | Parse-error deltas, field-presence deltas, and validation issue deltas. |
| Safe-analysis redaction output | You need to preview redaction before bundling. | Review carefully. It includes redacted HL7 output. | `receipt.phi_removed` and each retained field's reason. |
| Redaction receipt | You need proof of configured redaction actions. | Usually, after confirming no raw dropped values appear. | Actions, reasons, match counts, and `phi_removed`. |
| Field path trace | You need path-level evidence for bundle contents. | Usually, but review value shapes. | Paths and redaction actions are present without raw PHI values. |
| Evidence bundle summary | You need a top-level bundle receipt. | Usually. It should not expose local roots. | Validation status, redaction status, output ID, and artifact list. |
| Bundle manifest | You need integrity proof for bundle files. | Yes if bundle-relative paths are acceptable. | Every artifact has a role, relative path, and SHA-256 hash. |
| Bundle environment | You need tool/version/hash provenance. | Review first. It may reveal operational metadata. | Tool version, input/profile/policy hashes, and replay command. |
| Replay report | You need proof that a bundle still reproduces. | Usually. It should not dump artifact contents. | `reproduced` and checks such as manifest hashes and report match. |
| Quarantine output summary | You need server-side failed-validation evidence. | Usually, after confirming root-relative IDs only. | Reason, issue count, artifact names, and no configured root path. |

## How To Read Common Artifacts

### Doctor Report

What it proves: the installed tool can run local diagnostic checks such as
sample parsing and MLLP round-trip checks.

What it does not prove: it does not prove your feed, profile, server
configuration, Python package, or registry install path works.

PHI and sharing: it should be environment proof only. Do not add hostnames,
API keys, raw paths, or raw server response bodies to shared doctor output.

Version behavior: JSON and YAML output default to the current v1 shape; opt-in
v2 output adds schema and tool provenance where supported.

Next action when red: fix the local install or environment before trusting
message-level evidence.

### Profile Lint Report

What it proves: the profile YAML can be parsed and its profile rules pass the
lint checks.

What it does not prove: lint success does not prove the profile accepts or
rejects real messages correctly. Use profile tests and validation reports for
that.

PHI and sharing: profile-derived paths, rule IDs, and messages may be present.
Do not attach profiles that contain site-specific comments or operational
identifiers until they have been reviewed.

Version behavior: v1 is the default. v2 adds explicit schema and tool
provenance when requested.

Next action when red: fix lint errors before running profile tests, corpus
fingerprints, or bundle creation with that profile.

### Profile Explain Report

What it proves: which required fields, value sets, and constraints the loaded
profile exposes to users.

What it does not prove: it is not a validation run and does not prove a
message satisfies the profile.

PHI and sharing: this report should not echo the whole profile file, but it can
expose profile intent, rule names, value-set names, and profile hashes.

Version behavior: v1 is the default. v2 adds schema and tool provenance when
requested.

Next action when red: if explain output does not match what the profile owner
expects, stop and fix the profile before investigating feed behavior.

### Profile Test Report

What it proves: known valid and invalid fixtures behave as the profile owner
declared.

What it does not prove: it does not prove every real-world message or future
feed variant is covered.

PHI and sharing: fixture labels and embedded validation reports may be present.
The report should not contain raw HL7 fixture bodies.

Version behavior: v1 is the default. v2 adds schema and tool provenance when
requested.

Next action when red: treat this as a profile or fixture expectation problem.
Fix it before relying on the profile in CI or support bundles.

### Validation Report

What it proves: one message was checked against the configured parser and, if
supplied, profile rules. It records whether validation passed and which issue
codes, paths, severities, and messages were emitted.

What it does not prove: it is not a PHI-safety receipt and does not prove the
message is safe to share. Pair it with redaction and bundle evidence for
support workflows.

PHI and sharing: validation reports are designed not to contain raw HL7
payloads. Still review issue messages and surrounding ticket material before
sharing.

Version behavior: v1 is the default across Rust, CLI, server, and Python. v2
can be requested on surfaces that expose schema-version controls.

Next action when red: use issue `path`, `code`, and `severity` to decide
whether this is a parser, profile, feed, or transformation problem.

### Corpus Summary

What it proves: a batch or folder has a deterministic count of files, messages,
message types, and parse errors.

What it does not prove: it does not prove field-level stability or validation
compatibility. Use fingerprints and diffs for deeper evidence.

PHI and sharing: it should not contain raw message bodies.

Version behavior: v1 is the default. v2 adds schema and tool provenance when
requested.

Next action when red: investigate parse errors first. HL7 segment separators,
encoding, and partial files are common causes.

### Corpus Fingerprint

What it proves: the corpus has a deterministic signature of message counts,
field presence, value shapes, validation issue counts, and optional profile
identity.

What it does not prove: it is not a cryptographic proof that a partner sent
the same original files unless you also retain source-controlled inputs or
bundle hashes.

PHI and sharing: it should report counts and shapes, not raw field values.

Version behavior: v1 is the default. v2 adds schema and tool provenance when
requested.

Next action when red: compare the fingerprint with the last known-good receipt
and inspect deltas before blaming the parser.

### Corpus Diff

What it proves: before and after corpora differ in parse health, message
composition, field presence, value shapes, and validation issue counts.

What it does not prove: it does not explain the business reason for the drift
or prove that the after state is acceptable.

PHI and sharing: it should report deltas, not raw messages.

Version behavior: v1 is the default. v2 adds schema and tool provenance when
requested.

Next action when red: separate parse-error drift from validation drift. Parse
drift usually needs feed or encoding triage before profile triage.

### Safe-Analysis Redaction Output

What it proves: a configured safe-analysis policy ran against a message and
produced redacted output plus a receipt.

What it does not prove: it is not a universal PHI absence certificate. It
proves the configured policy actions and their results.

PHI and sharing: review this artifact carefully because it can include
redacted HL7 output. Retained fields are intentionally still visible and must
have a support reason.

Version behavior: v1 is the default. v2 adds schema and tool provenance when
requested.

Next action when red: fix policy errors before bundling. A fail-closed policy
failure is preferable to creating an unsafe support packet.

### Redaction Receipt

What it proves: configured redaction actions ran, which paths matched, and
whether the tool reports that PHI was removed.

What it does not prove: it does not guarantee every free-text value or local
identifier is safe. It only proves the configured policy.

PHI and sharing: receipts should record actions, reasons, counts, and hashes,
not raw dropped values.

Version behavior: v1 is the default. v2 adds schema and tool provenance when
requested.

Next action when red: review missing, retained, or unexpected matches before
sharing the message, bundle, or support ticket.

### Field Path Trace

What it proves: the bundle captured field paths, value shapes, and redaction
actions needed to explain the redacted message.

What it does not prove: it does not replace the redaction receipt or replay
report.

PHI and sharing: it should not contain raw PHI values. Review value-shape
metadata before sharing if your organization treats some shapes as sensitive.

Version behavior: bundle artifacts default to v1 unless bundle schema version
2 is requested.

Next action when red: if paths or actions are missing, rebuild the bundle from
reviewed inputs and policy.

### Evidence Bundle Summary

What it proves: a bundle was created with a known validation status, redaction
status, message type, output identifier, and artifact list.

What it does not prove: it does not by itself prove the bundle can be replayed.
Use the replay report for that.

PHI and sharing: the summary should not expose configured filesystem roots or
raw server bundle IDs.

Version behavior: v1 is the default for summaries. Bundle-internal artifacts
can opt into v2 where supported.

Next action when red: if validation failed, decide whether that failure is the
intended support evidence. If redaction failed, do not share the bundle.

### Bundle Manifest

What it proves: the bundle has a recorded integrity catalog of artifact roles,
bundle-relative paths, and SHA-256 hashes.

What it does not prove: it does not prove the artifacts are semantically
correct. It proves they have not changed since bundling.

PHI and sharing: paths should be bundle-relative. The manifest should not
contain raw HL7, configured roots, or raw bundle IDs.

Version behavior: bundle artifacts default to v1 unless bundle schema version
2 is requested.

Next action when red: any manifest hash mismatch invalidates the packet as
evidence until the bundle is regenerated or the mismatch is explained.

### Bundle Environment

What it proves: tool version, input/profile/policy hashes, validation summary,
and replay command were recorded with the bundle.

What it does not prove: it does not prove the recipient has the same local
environment or that the original raw message is safe to share.

PHI and sharing: review this artifact before sharing. It should avoid
hostnames, absolute local paths, and raw policy paths, but operational metadata
can still matter.

Version behavior: bundle artifacts default to v1 unless bundle schema version
2 is requested.

Next action when red: if tool or hash provenance is missing, regenerate the
bundle with the current release before using it as a support receipt.

### Replay Report

What it proves: replay verified the bundle manifest and regenerated evidence
well enough to reproduce the stored result.

What it does not prove: replay does not prove the original unredacted message
was safe, correct, or unchanged outside the bundle. It proves the redacted
packet reproduces.

PHI and sharing: replay reports should not dump raw artifact contents.

Version behavior: v1 is the default. v2 adds schema and tool provenance when
requested.

Next action when red: do not share the bundle as proof. Use the failing check
name to identify missing artifacts, hash mismatches, parse/profile errors, or
report mismatches.

### Quarantine Output Summary

What it proves: the server wrote configured quarantine evidence for a failed
redacted-validation request.

What it does not prove: it does not prove the original request was safe to
share or that production retention policy permits disclosure.

PHI and sharing: responses should expose root-relative output IDs only. They
must not expose configured roots or raw HL7.

Version behavior: v1 is the default. Server requests can ask for v2 quarantine
shape where supported.

Next action when red: confirm quarantine is configured, inspect the redacted
bundle or summary, and avoid sharing any server-local root path.

## Sharing Rules

Use these rules before attaching evidence to a ticket, vendor case, CI
artifact, or support bundle:

- Share replayable redacted bundles instead of raw HL7 whenever possible.
- Treat redaction receipts as policy receipts, not universal PHI clearance.
- Review retained fields and profile files before disclosure.
- Do not share raw input messages, raw configured filesystem roots, API keys,
  tokens, server response bodies, or unreviewed redaction policies.
- Prefer bundle-relative paths, hashes, schema versions, and replay reports as
  the durable proof.
- If replay does not reproduce, the bundle is not a proof packet yet.

## What Not To Infer

Evidence artifacts are intentionally narrow. Do not infer more than they
claim:

- A validation pass does not mean a message is safe to share.
- A redaction receipt does not prove every possible PHI value was removed.
- A corpus fingerprint does not explain why a partner feed changed.
- A bundle summary does not prove replay success.
- A crates.io `hl7v2-python` backend publish does not prove public PyPI
  `hl7v2` upload or install-back.
- A local Python wheel smoke does not prove TestPyPI or production PyPI.
- Server quarantine evidence does not authorize disclosure of server-local
  paths or raw request payloads.

## First Next Action

When you receive an artifact and are unsure what to do, use this order:

1. Identify the artifact type and schema version.
2. Check whether it contains raw HL7, profile content, local paths, or retained
   values.
3. If it is a bundle, run replay and inspect the replay report first.
4. If it is a validation or profile artifact, start with issue code, path,
   severity, and profile hash.
5. If it is corpus evidence, separate parse drift from validation drift.
6. If it is redaction evidence, confirm the retained fields are justified.
7. Link the artifact to a support, CI, release, or audit receipt instead of
   pasting raw message content into prose.

## Normative References

- [Evidence Contract Index](../contracts/evidence-contract-index.md)
- [Evidence Artifact Compatibility Policy](../contracts/evidence-artifact-compatibility-policy.md)
- [Evidence artifact architecture](../architecture/evidence-artifacts.md)
- [Evidence provenance and versioning](../architecture/evidence-provenance-versioning.md)
- [Schema README](../../schemas/README.md)
- [Safe Support Bundle](safe-support-bundle.md)
- [First 10 Minutes](first-10-minutes.md)
