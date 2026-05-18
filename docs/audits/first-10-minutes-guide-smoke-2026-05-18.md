# First 10 Minutes Guide Smoke

Date: 2026-05-18
Branch: `test/first-10-minutes-guide-smoke`
Scope: executable source-checkout proof for the job-first CLI onboarding guide.

## Purpose

This receipt records a dedicated guide smoke for
[`docs/guides/first-10-minutes.md`](../guides/first-10-minutes.md):

```text
cargo +1.95.0 run -p xtask -- check-first-10-minutes-guide
```

The command executes the guide's source-checkout workflow into
`target/hl7v2-first-10-minutes` and verifies the expected JSON shape for:

- `doctor`;
- sample generation and `validate-sample`;
- profile lint and explain;
- valid and invalid message validation, including the expected exit-code `1`
  invalid case;
- profile fixture testing;
- corpus summary, fingerprint, and diff;
- support-bundle creation;
- replay verification.

The aggregate first-use command also runs this guide smoke:

```text
cargo +1.95.0 run -p xtask -- check-first-use-guides
```

## Non-Claims

- No TestPyPI upload.
- No TestPyPI install-back.
- No PyPI upload.
- No PyPI install-back.
- No npm package.
- No crates.io upload.
- No tag or GitHub release.
- No public Python registry proof.

## Validation

```text
cargo +1.95.0 run -p xtask -- check-first-10-minutes-guide
cargo +1.95.0 run -p xtask -- check-first-use-guides
cargo +1.95.0 test -p xtask check_first_10_minutes_guide --locked
cargo +1.95.0 test -p xtask --locked
cargo +1.95.0 clippy -p xtask --all-targets --locked -- -D warnings
cargo +1.95.0 fmt --all -- --check
cargo +1.95.0 run -p xtask -- check-doc-links
cargo +1.95.0 run -p xtask -- check-file-policy
cargo +1.95.0 run -p xtask -- badges --check
cargo +1.95.0 run -p xtask -- impacted-evidence
cargo +1.95.0 run -p xtask -- impacted-evidence --check
git diff --check
```
