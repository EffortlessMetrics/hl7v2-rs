# Operator Error Guidance Guide Smoke - 2026-05-18

## Scope

This receipt records an executable source-checkout smoke for
[`docs/guides/operator-error-guidance.md`](../guides/operator-error-guidance.md).

The smoke ties the guide to representative REST and CLI failure paths without
changing runtime behavior. It verifies that operator-facing failures preserve
safe fields, concrete next actions, and PHI-sentinel boundaries.

## Command

```powershell
cargo +1.95.0 run -p xtask -- check-operator-error-guidance-guide
```

## Proof

The smoke verifies:

- REST parse failures expose `PARSE_ERROR`, `location`, `safe_detail`, and
  `suggested_next_action` without echoing forbidden values.
- REST validate parse failures preserve the same safe error shape.
- REST profile-load failures expose `PROFILE_LOAD_ERROR` and direct operators
  to profile lint without echoing raw profile content.
- REST unsafe bundle IDs fail closed with safe bundle-id guidance.
- REST missing bundle roots fail closed with configuration guidance.
- CLI validation failures can be read from machine-readable JSON reports with
  stable `valid`, `issue_count`, `issues.code`, `issues.path`, and
  `issues.severity` fields.
- The generated CLI validation and summary reports do not contain the guide
  PHI sentinel strings.

## Non-Claims

This is not a TestPyPI, PyPI, npm, crates.io, tag, or GitHub release receipt.
It does not prove public Python registry install-back. It only proves the
source-checkout operator error guidance remains tied to executable local REST
and CLI checks.

## Validation

- `cargo +1.95.0 run -p xtask -- check-operator-error-guidance-guide`
- `cargo +1.95.0 test -p xtask check_operator_error_guidance_guide --locked`
- `cargo +1.95.0 clippy -p xtask --all-targets --locked -- -D warnings`
- `cargo +1.95.0 fmt --all -- --check`
- `cargo +1.95.0 run -p xtask -- check-doc-links`
- `cargo +1.95.0 run -p xtask -- check-file-policy`
- `cargo +1.95.0 run -p xtask -- badges --check`
- `cargo +1.95.0 run -p xtask -- impacted-evidence`
- `cargo +1.95.0 run -p xtask -- impacted-evidence --check`
- `git diff --check`
