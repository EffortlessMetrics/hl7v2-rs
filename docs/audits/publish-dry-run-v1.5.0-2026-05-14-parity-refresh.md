# v1.5.0 Parity Refresh Publish Dry-Run Receipt

Date: 2026-05-14
Branch: `release/refresh-v1.5-readiness-after-parity`
Commit SHA: `cc1e3046e2496ea0c10a25239b9d077641d01c36`
Version: `1.5.0`
Result: Passed

This receipt refreshes the non-publishing v1.5.0 release-readiness proof after
the gRPC corpus-summary parity PR and cross-surface evidence parity spec landed
on `main`.

It is not a crates.io publish receipt. It is not a TestPyPI, PyPI, npm, tag, or
GitHub release receipt.

## Refresh Scope

This refresh covers the current release candidate after:

- #621 added gRPC `CorpusSummarize` parity for inline corpus summaries and
  updated the server evidence contract docs;
- #622 added
  [HL7V2-SPEC-0006](../specs/HL7V2-SPEC-0006-cross-surface-evidence-parity.md)
  as the cross-surface evidence parity contract.

The selected v1.5.0 crates.io graph remains primary plus binding backend:

1. `hl7v2`
2. `hl7v2-python`
3. `hl7v2-server`
4. `hl7v2-cli`

`hl7v2-python` remains binding backend infrastructure for the public Python
`hl7v2` package. Publishing it to crates.io would not prove TestPyPI or PyPI
release success, and it would not make it the recommended Rust API.

## Local Proof

| Command | Result |
| --- | --- |
| `cargo +1.95.0 run -p xtask -- publish-plan --surface primary` | pass; primary graph reports `hl7v2`, `hl7v2-server`, `hl7v2-cli` |
| `cargo +1.95.0 run -p xtask -- publish-plan --surface bindings` | pass; binding graph reports `hl7v2-python` |
| `cargo +1.95.0 run -p xtask -- publish-plan --surface all-publishable` | pass; all-publishable graph reports `hl7v2`, `hl7v2-python`, `hl7v2-server`, `hl7v2-cli` |
| `cargo +1.95.0 run -p xtask -- publish-dry-run --surface primary --workspace-patches --allow-dirty` | pass; `hl7v2`, `hl7v2-server`, and `hl7v2-cli` packaged and verified; upload aborted because this was a dry-run |
| `PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo +1.95.0 run -p xtask -- publish-dry-run --surface bindings --workspace-patches --allow-dirty` | pass; `hl7v2-python` package files were listed, then the crate packaged and verified; upload aborted because this was a dry-run |
| `cargo +1.95.0 run -p xtask -- check-python-publish-policy` | pass; Python distribution remains `hl7v2`, and `hl7v2-python` remains a separately receipted binding backend crate |
| `cargo +1.95.0 run -p xtask -- badges --check` | pass; RIPR badge endpoints are current |
| `cargo +1.95.0 run -p xtask -- impacted-evidence --check` | pass; impacted evidence receipts are current |
| `cargo +1.95.0 run -p xtask -- evidence-schema-check` | pass; 33 evidence fixtures validated against schema contracts |
| `cargo +1.95.0 run -p xtask -- check-doc-links` | pass; 153 Markdown files and 335 local links checked |
| `git diff --check` | pass |

The local Python interpreter is Python 3.14, so the binding backend dry-run
requires `PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1` with PyO3 0.24.2. A first
rerun used incorrect PowerShell quoting for that environment variable and
failed at PyO3's version guard; the passing command above is the recorded proof.

## Package Dry-Run Details

The primary surface dry-run packaged and verified:

- `hl7v2 v1.5.0`;
- `hl7v2-server v1.5.0`;
- `hl7v2-cli v1.5.0`.

The binding surface dry-run packaged and verified:

- `hl7v2-python v1.5.0`.

The `hl7v2-python` package file list was reviewed by the dry-run surface:

```text
.cargo_vcs_info.json
Cargo.lock
Cargo.toml
Cargo.toml.orig
README.md
src/lib.rs
```

## Non-Claims

This refresh does not claim:

- any crates.io upload;
- any `hl7v2-python` backend upload;
- any TestPyPI or PyPI upload;
- any npm package;
- any tag;
- any GitHub release;
- any production install-back proof.

## Remaining Decisions

The release graph has already been selected as primary plus binding backend in
[the v1.5.0 release graph decision](v1.5.0-release-graph-decision-2026-05-14.md).

Before publishing, a release operator still needs explicit approval to upload
the selected crates.io graph, verify registry resolution, tag `v1.5.0`, create
the GitHub release, and record a post-publish receipt.

The public Python `hl7v2` release remains blocked until TestPyPI Trusted
Publisher is configured for the `hl7v2` project and upload plus install-back
proof passes without token fallback or `skip-existing`.
