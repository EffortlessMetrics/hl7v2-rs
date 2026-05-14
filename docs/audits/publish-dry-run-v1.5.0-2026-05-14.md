# v1.5.0 Current-Main Publish Dry-Run Refresh

This receipt refreshes the non-publishing v1.5.0 release-readiness proof after
the profile evidence, Python profile helper, and RIPR evidence-surface updates
landed on `main`.

It is not a crates.io publish receipt. It is not a TestPyPI, PyPI, npm, tag, or
GitHub release receipt.

## Candidate

| Field | Value |
| --- | --- |
| Version | `1.5.0` |
| Commit SHA | `b4b7962e6f3f9d7ae5d91adf603e6328e3d13297` |
| Branch | `release/refresh-v1.5-readiness-after-evidence` |
| Date | 2026-05-14 |
| Result | Local current-main readiness refresh passed |

## Refresh Scope

This refresh covers the current release candidate after:

- #616 added the profile evidence facade;
- #617 exposed Python profile helpers and evidence smoke coverage;
- #618 added the first RIPR evidence surface, including badge endpoints,
  PR evidence commands, review guidance, annotations, impacted-evidence
  receipts, and advisory workflow artifacts.

## Publish Surfaces

The release tooling now reports separate publish surfaces:

```text
primary:
  hl7v2
  hl7v2-server
  hl7v2-cli

bindings:
  hl7v2-python

all-publishable:
  hl7v2
  hl7v2-python
  hl7v2-server
  hl7v2-cli
```

This receipt does not choose the v1.5.0 publish graph. The next release
decision must explicitly choose either primary-only or primary plus binding
backend.

## Local Proof

| Command | Result |
| --- | --- |
| `cargo +1.95.0 run -p xtask -- publish-plan --surface primary` | pass; primary graph reports `hl7v2`, `hl7v2-server`, `hl7v2-cli` |
| `cargo +1.95.0 run -p xtask -- publish-plan --surface bindings` | pass; binding graph reports `hl7v2-python` |
| `cargo +1.95.0 run -p xtask -- publish-plan --surface all-publishable` | pass; all-publishable graph reports `hl7v2`, `hl7v2-python`, `hl7v2-server`, `hl7v2-cli` |
| `cargo +1.95.0 run -p xtask -- publish-dry-run --surface primary --workspace-patches --allow-dirty` | pass; primary crates packaged and verified, upload aborted because this was a dry-run |
| `PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo +1.95.0 run -p xtask -- publish-dry-run --surface bindings --workspace-patches --allow-dirty` | pass; `hl7v2-python` packaged and verified as the binding backend, upload aborted because this was a dry-run |
| `cargo +1.95.0 run -p xtask -- check-python-publish-policy` | pass; public Python distribution remains `hl7v2`, with `hl7v2-python` as separately receipted binding backend |
| `cargo +1.95.0 run -p xtask -- badges --check` | pass; RIPR badge endpoints are current |
| `cargo +1.95.0 run -p xtask -- impacted-evidence --check` | pass; impacted evidence receipts are current |
| `cargo +1.95.0 run -p xtask -- check-doc-links` | pass; local Markdown links resolve |
| `git diff --check` | pass |

## Package Dry-Run Details

The primary surface dry-run packaged and verified:

- `hl7v2 v1.5.0`;
- `hl7v2-server v1.5.0`;
- `hl7v2-cli v1.5.0`.

The binding surface dry-run packaged and verified:

- `hl7v2-python v1.5.0`.

`hl7v2-python` is a binding backend crate for the public Python `hl7v2`
package. Publishing it to crates.io would not prove TestPyPI or PyPI release
success, and it would not make it the recommended Rust API.

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

Before publishing v1.5.0, choose one release graph:

- primary-only: `hl7v2`, `hl7v2-server`, `hl7v2-cli`;
- primary plus binding backend: `hl7v2`, `hl7v2-python`, `hl7v2-server`,
  `hl7v2-cli`.

The public Python `hl7v2` release remains blocked until TestPyPI Trusted
Publisher is configured for the `hl7v2` project and upload plus install-back
proof passes without token fallback or `skip-existing`.
