# TestPyPI Closure Plan

Status: Blocked
Spec: [HL7V2-SPEC-0002](../../docs/specs/HL7V2-SPEC-0002-python-distribution-proof.md)
ADR: [HL7V2-ADR-0002](../../docs/adr/HL7V2-ADR-0002-python-is-separate-distribution-lane.md)
Blocker: [issue #563](https://github.com/EffortlessMetrics/hl7v2-rs/issues/563)

## Goal

Close the `hl7v2-python` TestPyPI proof boundary only after external Trusted
Publisher setup exists and a guarded upload plus install-back passes.

## Production Delta

None until an explicit release proof run is requested. This plan does not
publish anything.

## External Setup

Configure TestPyPI Trusted Publisher for project `hl7v2-python`:

| Field | Value |
| --- | --- |
| Owner | `EffortlessMetrics` |
| Repository name | `hl7v2-rs` |
| Workflow filename | `python-testpypi.yml` |
| Environment name | `testpypi` |

Expected subject:

```text
repo:EffortlessMetrics/hl7v2-rs:environment:testpypi
```

## Execution

After external setup:

```text
Workflow: Python TestPyPI Proof
Branch: main
publish_to_testpypi: true
```

## Acceptance

- Build and smoke wheel: success.
- Publish to TestPyPI: success.
- Install from TestPyPI and smoke: success.
- No production PyPI upload.
- No token fallback.
- No skip-existing workaround.
- Receipt PR records run URL, commit SHA, package version, TestPyPI URL,
  publish job result, install-back result, smoke output, and production-PyPI
  non-attempt.

## Rollback

If upload fails with `invalid-publisher`, leave #563 open and update the audit
receipt only if the new run adds useful evidence. Do not switch to token fallback
or skip-existing.
