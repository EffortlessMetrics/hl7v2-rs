# Guides

Task-focused guides for using HL7v2 as an evidence layer around real HL7 v2
feeds.

| Guide | Use when |
| --- | --- |
| [First 10 Minutes](first-10-minutes.md) | You want to verify the CLI, validate a message, inspect a tiny corpus, and create a replayable evidence bundle. |
| [Vendor Upgrade Diff](vendor-upgrade-diff.md) | You need to compare before/after HL7 corpora and produce drift evidence for a migration, vendor change, or CI gate. |
| [Safe Support Bundle](safe-support-bundle.md) | You need to redact one failing message, bundle the evidence, and prove someone else can replay it safely. |
| [Deploy Validation Sidecar](deploy-validation-sidecar.md) | You need to run `hl7v2-server` as a small edge guard with readiness, redacted validation, ACK policy, quarantine, bundles, and metrics. |
| [Python Evidence Workflow](python-evidence-workflow.md) | You want to use the Python binding for validation reports, corpus diffs, redaction, bundles, and replay in notebooks or QA scripts. |
| [Python TestPyPI Release Proof](python-testpypi-release-proof.md) | You need to prove the separate `hl7v2-python` packaging lane through TestPyPI without changing the Rust crates.io graph. |
