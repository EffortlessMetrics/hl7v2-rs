# v1.5.0 Final Pre-Publish Dry-Run Proof

Date: 2026-05-15
Commit: `9fc95604d8950b565b6b6b7941ad275fd5624178`
Branch: `release/v1.5.0-final-prepublish-proof`
Scope: final non-publishing proof before the selected v1.5.0 crates.io graph
is eligible for upload.

This audit records package verification only. It is not a crates.io publish,
tag, GitHub release, TestPyPI, PyPI, npm, or install-back receipt.

## Result

Passed. The selected v1.5.0 crates.io graph still resolves as:

1. `hl7v2`
2. `hl7v2-python`
3. `hl7v2-server`
4. `hl7v2-cli`

`hl7v2-python` remains a binding backend crate for the public Python `hl7v2`
package. It is not the recommended Rust API.

## Environment

```powershell
$env:CARGO_TARGET_DIR = "F:\cargo-target\hl7v2-rs-final-prepublish"
$env:CARGO_INCREMENTAL = "0"
$env:PYO3_USE_ABI3_FORWARD_COMPATIBILITY = "1"
```

The isolated Cargo target was used so the pre-publish proof does not leave
large build artifacts under the repo checkout.

## Commands

Passed:

```powershell
cargo +1.95.0 run -p xtask -- publish-plan --surface all-publishable
cargo +1.95.0 run -p xtask -- publish-dry-run --surface all-publishable --workspace-patches --allow-dirty
cargo +1.95.0 run -p xtask -- check-python-publish-policy
cargo +1.95.0 run -p xtask -- check-doc-links
cargo +1.95.0 run -p xtask -- evidence-schema-check
```

## Proof Notes

- `publish-plan --surface all-publishable` reported the dependency order
  `hl7v2`, `hl7v2-python`, `hl7v2-server`, and `hl7v2-cli`.
- `publish-dry-run --surface all-publishable --workspace-patches --allow-dirty`
  packaged and verified all four selected crates. Cargo reached the dry-run
  upload step for each crate and aborted upload as expected.
- `check-python-publish-policy` confirmed that the public Python distribution
  is `hl7v2`, while `hl7v2-python` is a separate publishable binding backend
  crate with separate release receipts required.
- `check-doc-links` checked 170 Markdown files and 411 local links.
- `evidence-schema-check` validated 33 evidence fixtures against the schema
  contracts.

## Non-Claims

- No crates.io upload was run.
- No crates.io registry resolution for v1.5.0 exists from this proof.
- No `v1.5.0` tag was created.
- No GitHub release was created.
- No TestPyPI or PyPI upload was run.
- No npm package exists or was published.
- No production install-back proof exists.

## Next Step

If release is still approved after this proof lands on `main`, publish the
selected crates.io graph in the recorded dependency order and then record
registry resolution, tag, GitHub release, and install-back receipts.
