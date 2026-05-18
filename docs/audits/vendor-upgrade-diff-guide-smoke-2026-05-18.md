# Vendor Upgrade Diff Guide Smoke - 2026-05-18

## Scope

This receipt records an executable source-checkout smoke for
[`docs/guides/vendor-upgrade-diff.md`](../guides/vendor-upgrade-diff.md).

The smoke prepares a synthetic before/after vendor-upgrade corpus, runs the
profile lint, corpus summary, corpus fingerprint, and corpus diff commands from
the guide, and verifies the operator-facing fields the guide tells users to
inspect.

## Command

```powershell
cargo +1.95.0 run -p xtask -- check-vendor-upgrade-diff-guide
```

## Proof

The smoke verifies:

- `profiles/generic.yaml` lints successfully with zero errors or warnings.
- The before and after summaries both contain one parseable
  `ADT^A01^ADT_A01` message and zero parse errors.
- The before fingerprint has no validation issue counts.
- The after fingerprint records `value_not_in_set` once.
- The before and after fingerprints use the same profile hash.
- The corpus diff reports no parse-error delta.
- The corpus diff reports `PID.5` disappeared with
  `message_count_delta = -1`.
- The corpus diff reports `value_not_in_set` increased with `delta = 1`.
- The generated corpus diff does not contain the configured PHI sentinel
  strings.

## Non-Claims

This is not a TestPyPI, PyPI, npm, crates.io, tag, or GitHub release receipt.
It does not prove public Python registry install-back. It only proves that the
source-checkout vendor-upgrade diff guide recipe remains executable and that
the documented artifact fields retain their expected meaning.

## Validation

- `cargo +1.95.0 run -p xtask -- check-vendor-upgrade-diff-guide`
- `cargo +1.95.0 test -p xtask check_vendor_upgrade_diff_guide --locked`
- `cargo +1.95.0 clippy -p xtask --all-targets --locked -- -D warnings`
- `cargo +1.95.0 fmt --all -- --check`
- `cargo +1.95.0 run -p xtask -- check-doc-links`
- `cargo +1.95.0 run -p xtask -- check-file-policy`
- `cargo +1.95.0 run -p xtask -- badges --check`
- `cargo +1.95.0 run -p xtask -- impacted-evidence`
- `cargo +1.95.0 run -p xtask -- impacted-evidence --check`
- `git diff --check`
