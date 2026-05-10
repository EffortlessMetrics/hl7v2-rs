# Current Source Tree Evidence Objective Gap Audit

Date: 2026-05-10

This audit records the current durable repository state and the boundary around
the broad local evidence-lane workbench that still needs to be split. It is not
a release receipt, does not replace `docs/STATUS.md`, and does not claim
TestPyPI or production PyPI publication.

## Durability Boundary

The default branch is the durable source of truth. A large local workbench
contains useful validated work across docs, xtask rails, Python publishing
policy, gRPC auth/config, sensitive-output sanitization, and Python evidence
schema smoke coverage, but it is not durable until it is split into focused PRs
and merged.

The audit distinction is:

- **Published release state**: `hl7v2`, `hl7v2-server`, and `hl7v2-cli` v1.4.0
  are published to crates.io.
- **Separate Python lane**: `hl7v2-python` remains outside the Rust crates.io
  graph.
- **Current local workbench**: broad, dirty, and intentionally not reviewable as
  one PR.
- **External Python package state**: no visible TestPyPI or production PyPI
  `hl7v2-python` package was found during this audit.

## Package-State Receipt

| Surface | Result |
| --- | --- |
| `cargo +1.93.0 info hl7v2@1.4.0 --registry crates-io` | Reported `hl7v2` version `1.4.0` with crates.io URL. |
| `cargo +1.93.0 info hl7v2-server@1.4.0 --registry crates-io` | Reported `hl7v2-server` version `1.4.0` with crates.io URL. |
| `cargo +1.93.0 info hl7v2-cli@1.4.0 --registry crates-io` | Reported `hl7v2-cli` version `1.4.0` with crates.io URL. |
| `https://pypi.org/pypi/hl7v2-python/json` | Returned `404`. |
| `https://test.pypi.org/pypi/hl7v2-python/json` | Returned `404`. |

## Split Plan

The local workbench should be retired through narrow PRs, not merged as a
single patch:

| Order | Branch | Scope |
| ---: | --- | --- |
| 1 | `docs/source-tree-truth-audit` | Source-tree audit, status wording, docs navigation, and external package-state receipts. |
| 2 | `xtask/file-doc-policy-rails` | Untracked file policy, doc-link changed-gate behavior, and generated/vendor skip coverage. |
| 3 | `xtask/python-publish-policy` | PyPI/TestPyPI policy hardening, TOML-backed `pyproject.toml`, and Trusted Publishing guards. |
| 4 | `server/grpc-auth-config` | gRPC `x-api-key`, CLI `serve --mode grpc` shared config path, and transport auth tests. |
| 5 | `server/sensitive-error-sanitization` | Profile-load/lint sanitization plus bundle/replay ID no-echo behavior. |
| 6 | `python/evidence-schema-smoke` | Python guide validates generated v2 artifacts and bundle internals against checked-in schemas. |
| 7 | `docs/final-source-tree-gap-audit` | Final audit refresh after PRs 1-6 land. |

## Stop Conditions

- Do not publish `hl7v2-python` to crates.io.
- Do not run production PyPI without explicit release approval.
- Do not claim TestPyPI upload/install-back proof until the publishing mode
  uploads the current package and installs it back from TestPyPI.
- Keep the Rust publish graph as `hl7v2`, `hl7v2-server`, and `hl7v2-cli`.
- Keep CI-economics work separate from evidence-product behavior.

## Verification Performed For This Audit

| Check | Result |
| --- | --- |
| `gh pr list --state open --limit 20 --json number,title,headRefName,isDraft,mergeStateStatus,statusCheckRollup` | Returned `[]` before this PR was opened. |
| `cargo +1.93.0 info hl7v2@1.4.0 --registry crates-io` | Passed. |
| `cargo +1.93.0 info hl7v2-server@1.4.0 --registry crates-io` | Passed. |
| `cargo +1.93.0 info hl7v2-cli@1.4.0 --registry crates-io` | Passed. |
| `Invoke-WebRequest https://pypi.org/pypi/hl7v2-python/json` | Returned `404`. |
| `Invoke-WebRequest https://test.pypi.org/pypi/hl7v2-python/json` | Returned `404`. |

## Conclusion

The repo is in a good post-release state, but the next durable work is
disposition, not more feature construction. The broad local workbench should be
split and reviewed in the PR sequence above before any TestPyPI publishing
proof is attempted from `main`.
