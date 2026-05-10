# Final Source Tree Evidence Objective Gap Audit

Date: 2026-05-10

This audit records the durable source-tree state after the broad local
evidence-lane workbench was split into focused PRs and merged. It is not a
release receipt, does not replace `docs/STATUS.md`, and does not claim
TestPyPI or production PyPI publication.

## Durable State

The default branch is now the durable source of truth for the split workbench
items that were previously local-only.

| Area | Durable state |
| --- | --- |
| Source-tree truth audit | Merged in #553. |
| File and doc policy rails | Merged in #554. |
| Python publish policy rails | Merged in #555. |
| gRPC auth and CLI config path | Merged in #556. |
| Sensitive evidence error sanitization | Merged in #557. |
| Python evidence schema smoke | Merged in #558. |
| CI economics | Merged separately in #471 and remains an infrastructure lane. |

The previous broad workbench should no longer be treated as a reviewable
source of truth. Future changes should start from current `main` and open
narrow PRs.

## Package-State Receipt

| Surface | Result |
| --- | --- |
| `cargo +1.93.0 info hl7v2@1.4.0 --registry crates-io` | Reported `hl7v2` version `1.4.0` with crates.io URL. |
| `cargo +1.93.0 info hl7v2-server@1.4.0 --registry crates-io` | Reported `hl7v2-server` version `1.4.0` with crates.io URL. |
| `cargo +1.93.0 info hl7v2-cli@1.4.0 --registry crates-io` | Reported `hl7v2-cli` version `1.4.0` with crates.io URL. |
| `https://pypi.org/pypi/hl7v2-python/json` | Returned `404`. |
| `https://test.pypi.org/pypi/hl7v2-python/json` | Returned `404`. |

## Current Product Boundary

- The Rust crates.io product graph remains `hl7v2`, `hl7v2-server`, and
  `hl7v2-cli`.
- `hl7v2-python` remains a separate Python/maturin lane and is not part of the
  Rust crates.io graph.
- The repository now has policy rails for Python publishing, including manual
  TestPyPI proof controls and production PyPI guardrails.
- A 2026-05-10 TestPyPI upload/install-back attempt was run from current
  `main`; wheel build and smoke passed, but upload failed with
  `invalid-publisher` because the TestPyPI Trusted Publisher is not configured.
- Production PyPI publication has not been run and still requires explicit
  release approval.
- Old microcrate package names remain historical artifacts, not the current
  product surface.

## Verification Performed For This Audit

| Check | Result |
| --- | --- |
| `gh pr list --state open --limit 20 --json number,title,headRefName,isDraft,mergeStateStatus` | Returned `[]` before this PR was opened. |
| `cargo +1.93.0 info hl7v2@1.4.0 --registry crates-io` | Passed. |
| `cargo +1.93.0 info hl7v2-server@1.4.0 --registry crates-io` | Passed. |
| `cargo +1.93.0 info hl7v2-cli@1.4.0 --registry crates-io` | Passed. |
| `Invoke-WebRequest https://pypi.org/pypi/hl7v2-python/json` | Returned `404`. |
| `Invoke-WebRequest https://test.pypi.org/pypi/hl7v2-python/json` | Returned `404`. |
| `cargo +1.93.0 run -p xtask -- publish-plan` | Reported `hl7v2`, `hl7v2-server`, and `hl7v2-cli`. |
| `cargo +1.93.0 run -p xtask -- evidence-schema-check` | Passed; 33 evidence fixtures validated. |

## Remaining Gaps

The source-tree split is complete. Remaining gaps are release-process actions,
not hidden local code:

1. Configure the TestPyPI Trusted Publisher for `hl7v2-python`, then rerun
   the guarded TestPyPI upload/install-back proof from clean `main`.
2. Keep production PyPI blocked until an explicit release decision approves it.
3. Keep future evidence, server, Python, and policy work in separate PR lanes.

## Conclusion

The prior local workbench has been disposed into durable PRs. Current `main`
contains the source-tree, policy, server-security, and Python evidence-smoke
rails needed before the next Python packaging proof. The repo should now be
operated from `main`, not from the old broad workbench branch.
