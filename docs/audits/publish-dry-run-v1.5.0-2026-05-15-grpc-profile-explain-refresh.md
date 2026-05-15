# v1.5.0 Publish Dry-Run Refresh After gRPC Profile Explain

Date: 2026-05-15
Commit: `483fb572a42006c07bd2e857fb7638ec91615b7d`
Scope: non-publishing v1.5.0 readiness refresh after gRPC `ProfileExplain`
parity landed on `main`.

## Result

Passed. The v1.5.0 candidate still resolves the selected crates.io graph as:

1. `hl7v2`
2. `hl7v2-python`
3. `hl7v2-server`
4. `hl7v2-cli`

`hl7v2-python` remains a binding backend crate, not the recommended Rust API.

## Commands

Run with:

```powershell
$env:CARGO_TARGET_DIR = "F:\cargo-target\hl7v2-rs-readiness-grpc-profile-explain"
$env:CARGO_INCREMENTAL = "0"
```

Passed:

```powershell
cargo +1.95.0 run -p xtask -- publish-plan --surface primary
cargo +1.95.0 run -p xtask -- publish-plan --surface bindings
cargo +1.95.0 run -p xtask -- publish-plan --surface all-publishable
cargo +1.95.0 run -p xtask -- publish-dry-run --surface primary --workspace-patches --allow-dirty
$env:PYO3_USE_ABI3_FORWARD_COMPATIBILITY = "1"; cargo +1.95.0 run -p xtask -- publish-dry-run --surface bindings --workspace-patches --allow-dirty
cargo +1.95.0 run -p xtask -- check-python-publish-policy
cargo +1.95.0 run -p xtask -- badges --check
cargo +1.95.0 run -p xtask -- impacted-evidence --check
cargo +1.95.0 run -p xtask -- evidence-schema-check
cargo +1.95.0 run -p xtask -- check-doc-links
cargo +1.95.0 run -p xtask -- check-file-policy
git diff --check
```

## Proof Notes

- `publish-plan --surface primary` reported `hl7v2`, `hl7v2-server`, and
  `hl7v2-cli`.
- `publish-plan --surface bindings` reported `hl7v2-python`.
- `publish-plan --surface all-publishable` reported `hl7v2`,
  `hl7v2-python`, `hl7v2-server`, and `hl7v2-cli`.
- Primary publish dry-run packaged and verified `hl7v2`, `hl7v2-server`, and
  `hl7v2-cli`; dry-run uploads aborted as expected.
- Binding publish dry-run listed the `hl7v2-python` package files, packaged and
  verified `hl7v2-python`, and aborted upload as expected.
- Python publish policy still reports public Python distribution `hl7v2` and
  `hl7v2-python` as a separate publishable binding backend crate requiring
  separate release receipts.
- Evidence schema check validated 33 fixtures.
- Advisory RIPR badge and impacted-evidence receipts were current.

## Non-Claims

- No crates.io upload.
- No TestPyPI or PyPI upload.
- No npm package.
- No tag or GitHub release.
- No production install-back proof.
- No gRPC profile test, bundle, replay, or quarantine parity claim.
