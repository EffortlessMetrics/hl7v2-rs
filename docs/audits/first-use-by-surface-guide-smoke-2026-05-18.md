# First Use By Surface Guide Smoke - 2026-05-18

## Scope

This receipt records an executable source-checkout smoke for
[`docs/guides/first-use-by-surface.md`](../guides/first-use-by-surface.md).

The smoke proves the current local Rust, CLI, and server first-use routes from
the guide without changing release state. Python remains delegated to the local
wheel proof and the external TestPyPI/PyPI blocker.

## Command

```powershell
cargo +1.95.0 run -p xtask -- check-first-use-by-surface-guide
```

## Proof

The smoke verifies:

- Rust user journey test:
  `journey_rust_validate_redact_bundle_replay_produces_shareable_receipts`.
- CLI `doctor --format json` returns version and checks.
- CLI profile lint for `profiles/generic.yaml` is valid with zero errors.
- CLI validation for `test_data/valid_message.hl7` returns a valid report with
  zero issues.
- CLI corpus summary for `test_data` reports two files and two messages.
- Server `--print-config` returns the documented default bind address and
  explicit false values for API key, bundle root, and quarantine configuration.
- Generated reports do not contain the guide PHI sentinel strings.

## Delegated Boundary

Python local-wheel proof remains owned by:

```powershell
cargo +1.95.0 run -p xtask -- python-local-wheel-proof
```

Public Python registry proof remains blocked by issue #563 until TestPyPI
Trusted Publisher is configured for project `hl7v2` and upload/install-back
passes.

## Non-Claims

This is not a TestPyPI, PyPI, npm, crates.io, tag, or GitHub release receipt.
It does not prove public Python registry install-back.

## Validation

- `cargo +1.95.0 run -p xtask -- check-first-use-by-surface-guide`
- `cargo +1.95.0 test -p xtask check_first_use_by_surface_guide --locked`
- `cargo +1.95.0 clippy -p xtask --all-targets --locked -- -D warnings`
- `cargo +1.95.0 fmt --all -- --check`
- `cargo +1.95.0 run -p xtask -- check-doc-links`
- `cargo +1.95.0 run -p xtask -- check-file-policy`
- `cargo +1.95.0 run -p xtask -- badges --check`
- `cargo +1.95.0 run -p xtask -- impacted-evidence`
- `cargo +1.95.0 run -p xtask -- impacted-evidence --check`
- `git diff --check`
