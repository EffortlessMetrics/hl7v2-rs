# v1.5.0 gRPC Corpus Evidence Readiness Refresh - 2026-05-14

This receipt refreshes the v1.5.0 release-readiness proof after the gRPC inline
corpus fingerprint and diff parity work landed.

It is not a crates.io publish, tag, GitHub release, TestPyPI, PyPI, npm, or
production install-back receipt.

## Scope

Refreshed commit:
`acfeacec0eda6d632d52e61440f2c85fda93d95f`.

Changes included since the prior gRPC status refresh:

- #629 added gRPC inline corpus fingerprint parity.
- #630 added gRPC inline corpus diff parity.

Selected v1.5.0 crates.io graph remains:

1. `hl7v2`
2. `hl7v2-python`
3. `hl7v2-server`
4. `hl7v2-cli`

`hl7v2-python` remains binding backend infrastructure, not the recommended Rust
API. A crates.io backend publish still does not prove TestPyPI or PyPI success
for the public Python `hl7v2` package.

## Local Environment

- Rust toolchain: `+1.95.0`
- Target directory: `F:\cargo-target\hl7v2-rs-readiness-grpc-corpus-refresh`
- Cargo incremental: `0`
- Binding dry-run environment: `PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1`

## Proof Commands

```powershell
cargo +1.95.0 run -p xtask -- publish-plan --surface primary
cargo +1.95.0 run -p xtask -- publish-plan --surface bindings
cargo +1.95.0 run -p xtask -- publish-plan --surface all-publishable
cargo +1.95.0 run -p xtask -- publish-dry-run --surface primary --workspace-patches --allow-dirty
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo +1.95.0 run -p xtask -- publish-dry-run --surface bindings --workspace-patches --allow-dirty
cargo +1.95.0 run -p xtask -- check-python-publish-policy
cargo +1.95.0 run -p xtask -- badges --check
cargo +1.95.0 run -p xtask -- impacted-evidence --check
cargo +1.95.0 run -p xtask -- evidence-schema-check
cargo +1.95.0 run -p xtask -- check-doc-links
cargo +1.95.0 run -p xtask -- check-file-policy
git diff --check
```

## Results

| Check | Result |
| --- | --- |
| `publish-plan --surface primary` | Passed. Primary graph: `hl7v2`, `hl7v2-server`, `hl7v2-cli`. |
| `publish-plan --surface bindings` | Passed. Binding graph: `hl7v2-python`. |
| `publish-plan --surface all-publishable` | Passed. Order: `hl7v2`, `hl7v2-python`, `hl7v2-server`, `hl7v2-cli`. |
| `publish-dry-run --surface primary --workspace-patches --allow-dirty` | Passed. `hl7v2`, `hl7v2-server`, and `hl7v2-cli` packaged and verified; upload steps aborted because this was a dry run. |
| `publish-dry-run --surface bindings --workspace-patches --allow-dirty` | Passed. `hl7v2-python` package list and dry-run verification passed; upload step aborted because this was a dry run. |
| `check-python-publish-policy` | Passed. Python distribution is `hl7v2`; `hl7v2-python` is a publishable binding backend with separate release receipts required. |
| `badges --check` | Passed. Badge endpoints are current. |
| `impacted-evidence --check` | Passed. Impacted evidence is current. |
| `evidence-schema-check` | Passed. 33 evidence fixtures validated. |
| `check-doc-links` | Passed. 158 Markdown files and 362 local links checked. |
| `check-file-policy` | Passed. 517 non-ignored files checked with 39 allowlist entries and 30 companion ledger entries. |
| `git diff --check` | Passed. |

## Non-Claims

- No crates.io upload was run.
- No crates.io registry resolution proof exists for v1.5.0 from this receipt.
- No TestPyPI upload was run.
- No PyPI upload was run.
- No npm package was created or published.
- No tag was created.
- No GitHub release was created.
- No production install-back proof exists from this receipt.

## Known Boundaries

- TestPyPI remains externally blocked until Trusted Publisher setup for the
  public `hl7v2` project is complete.
- Production PyPI remains a separate explicit release decision after same-commit
  TestPyPI proof.
- `ripr` remains advisory static mutation-exposure proof.
- Runtime mutation remains targeted by risk, label, manual dispatch, nightly, or
  release-readiness needs; it is not an ordinary PR tax.
