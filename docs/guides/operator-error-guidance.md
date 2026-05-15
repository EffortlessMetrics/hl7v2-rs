# Operator Error Guidance

This guide explains how to read HL7v2 failures without exposing raw messages,
profile contents, redaction policies, configured filesystem roots, or secrets.
Use it when a parse, validation, redaction, bundle, replay, server, or Python
workflow fails and you need the next safe action.

For artifact interpretation, use
[Evidence Artifacts For Operators](evidence-artifacts-for-operators.md). For
the first command to run on each surface, use
[First Use By Surface](first-use-by-surface.md).

## Safe Failure Shape

Every operator-facing failure should answer:

| Question | Safe answer |
| --- | --- |
| What failed? | A stable code or issue code, such as `PARSE_ERROR` or `missing_required_field`. |
| Where did it fail? | A request field, artifact role, endpoint, or HL7 path when known. |
| Is PHI exposed? | Raw message/profile/policy content should not be echoed by default. |
| Can I continue? | Validation failures may still produce reports; parse, redaction, unsafe-path, and missing-root failures usually fail closed. |
| What should I try next? | One concrete action, such as profile lint, redaction policy review, bundle root configuration, or replay. |

The REST server exposes this shape directly in JSON error responses:

```json
{
  "code": "PROFILE_LOAD_ERROR",
  "message": "profile could not be loaded; run profile lint for details",
  "safe_detail": "The supplied inline profile could not be loaded. Raw profile content is not echoed.",
  "location": "profile",
  "suggested_next_action": "Run profile lint on the profile, then retry validation with the corrected profile.",
  "details": null
}
```

CLI and Python workflows should be interpreted with the same questions even
when the immediate failure is an exit code or exception. Prefer machine-readable
reports, redacted bundles, and replay receipts over copied terminal output.

## Surface Notes

| Surface | Where to look first | Safe next action |
| --- | --- | --- |
| Rust | `Result` error plus any generated report or receipt | Convert failures into local operator messages without printing raw input. |
| CLI | Exit code, stderr, `--report json`, `--format json`, bundle/replay output | Rerun with JSON output where supported; create a redacted bundle for shareable evidence. |
| REST | JSON `ErrorResponse` fields | Use `code`, `safe_detail`, `location`, and `suggested_next_action`. |
| gRPC | RPC status plus typed response/error fields where implemented | Preserve the same safe-detail posture as REST; do not log request payloads. |
| Python | Exception plus helper return dicts/artifacts | Catch exceptions, avoid printing raw messages, and use validation/redaction/bundle helper outputs for evidence. |

## Common Failures

| Failure | Usually means | Safe next action |
| --- | --- | --- |
| Bad `MSH` or malformed message | The parser could not identify a valid HL7 message shape. | Check the `MSH` segment, segment terminators, field separator, encoding characters, and `mllp_framed` setting. |
| Invalid timestamp | The message parsed, but a profile/datatype rule rejected the value. | Inspect the validation issue path and profile datatype rule; do not paste the original field value into shared tickets. |
| Unexpected or missing segment | The message parsed, but the profile expected a different structure. | Run profile lint/explain, then validate against the intended profile version. |
| Unsupported schema version | A request asked for an artifact schema version the endpoint does not emit. | Use schema version `1` or `2` for current evidence artifacts unless a later receipt says otherwise. |
| Profile load failure | Inline profile YAML is malformed or structurally incomplete. | Run `hl7v2-cli profile lint` or the Python/server profile lint helper, then retry with the corrected profile. |
| Redaction policy error | The safe-analysis policy is malformed, lacks reasons, misses present sensitive fields, or tries an unsafe retain. | Check rule paths, actions, reasons, optional flags, and built-in sensitive fields before retrying. |
| Unsafe bundle id | The bundle id looks like a path or contains traversal characters. | Use one ASCII label segment with letters, numbers, `.`, `_`, or `-`; do not send filesystem paths. |
| Bundle root not configured | The server cannot write or replay bundles because no root is configured. | Configure `HL7V2_BUNDLE_OUTPUT_ROOT`, restart if needed, and check `/ready`. |
| Quarantine root not configured | Quarantine output is enabled but has no server-controlled output root. | Configure quarantine output or disable quarantine before retrying. |
| Replay mismatch | The bundle was changed, is incomplete, or cannot regenerate the stored validation result. | Treat the bundle as not reproduced; inspect replay checks before sharing or relying on it. |

## What To Share

Prefer sharing:

- validation report JSON;
- profile lint or explain report;
- redaction receipt;
- field-path trace;
- corpus summary, fingerprint, or diff;
- evidence bundle after safe-analysis redaction;
- replay report.

Do not share by default:

- raw HL7 messages;
- raw profile YAML that embeds sensitive local context;
- raw redaction policies that include sensitive operational context;
- configured filesystem roots;
- API keys or bearer tokens;
- raw bundle or quarantine root paths;
- terminal logs that may include pasted input payloads.

## Escalation Pattern

1. Reproduce locally with the smallest message/profile/policy that still fails.
2. Prefer JSON output over screenshots or copied logs.
3. Redact before sharing.
4. Bundle and replay when another person needs to reproduce the evidence.
5. Include the exact package version and surface: Rust, CLI, REST, gRPC, or Python.
6. State what the receipt proves and what it does not prove.

Replay proves the stored bundle artifacts still reproduce their checks. It does
not prove the original raw message is safe, that every possible PHI value was
removed, or that a Python/TestPyPI/PyPI release exists.
