# HL7V2-SPEC-0002: Python Distribution Proof

Status: Accepted
Date: 2026-05-12
Proposal: [HL7V2-PROP-0001](../proposals/HL7V2-PROP-0001-source-of-truth-and-release-governance.md)
Source-of-truth stack: [HL7V2-SPEC-0001](HL7V2-SPEC-0001-source-of-truth-stack.md)

## Contract

`hl7v2-python` is a Python/maturin distribution lane. It is not a Rust
crates.io product crate and must not be treated as part of the Rust publish
graph.

Python distribution proof requires:

```text
local wheel build
local wheel install
smoke.py
evidence_workflow_guide.py
TestPyPI publish from main
TestPyPI install-back
receipt PR
optional production PyPI only after same-commit TestPyPI proof
```

No document or PR may claim TestPyPI or production PyPI success until upload and
install-back both pass for the same intended artifact stream.

## Current Sources

- TestPyPI proof guide:
  [docs/guides/python-testpypi-release-proof.md](../guides/python-testpypi-release-proof.md)
- Production PyPI proof guide:
  [docs/guides/python-pypi-release.md](../guides/python-pypi-release.md)
- Current blocked publish receipt:
  [docs/audits/python-testpypi-publish-attempt-2026-05-10.md](../audits/python-testpypi-publish-attempt-2026-05-10.md)
- External blocker:
  [issue #563](https://github.com/EffortlessMetrics/hl7v2-rs/issues/563)
- Rust publish process: [RELEASE_PROCESS.md](../../RELEASE_PROCESS.md)

## Required Trusted Publisher Boundary

TestPyPI publishing for `hl7v2-python` requires external Trusted Publisher
configuration:

| Field | Value |
| --- | --- |
| Owner | `EffortlessMetrics` |
| Repository name | `hl7v2-rs` |
| Workflow filename | `python-testpypi.yml` |
| Environment name | `testpypi` |

Expected subject:

```text
repo:EffortlessMetrics/hl7v2-rs:environment:testpypi
```

Until that external configuration exists and a guarded workflow run passes,
TestPyPI remains blocked. This is not a repo-side code failure.

## Proof Requirements

### Local Wheel Proof

Local proof must show:

- maturin or workflow wheel build succeeds;
- wheel install succeeds in a clean environment;
- `tests/python_smoke/smoke.py` passes;
- `tests/python_smoke/evidence_workflow_guide.py` passes.

### TestPyPI Proof

TestPyPI proof must show:

- workflow runs from `main`;
- `publish_to_testpypi=true`;
- wheel build and smoke pass before upload;
- Trusted Publishing upload succeeds;
- install-back from TestPyPI succeeds;
- smoke output is recorded;
- production PyPI upload is not attempted;
- no token fallback is used;
- no skip-existing workaround is used.

### Production PyPI Proof

Production PyPI proof is optional and requires an explicit release decision
after TestPyPI proof.

Production proof must show:

- same-commit TestPyPI proof URL is supplied and verified;
- `publish_to_pypi=true`;
- production PyPI upload succeeds;
- install-back from `https://pypi.org/simple/` succeeds;
- smoke output is recorded;
- receipt PR lands.

## Hard Rules

- Do not publish `hl7v2-python` to crates.io.
- Do not use token fallback.
- Do not use skip-existing.
- Do not claim TestPyPI success until upload and install-back pass.
- Do not claim production PyPI success until upload and install-back pass from
  `pypi.org`.
- Do not conflate Rust crates.io receipts with Python TestPyPI or PyPI receipts.

## Required Receipts

A successful TestPyPI receipt PR records:

- workflow run URL;
- commit SHA;
- package version;
- TestPyPI URL;
- publish job result;
- install-back job result;
- smoke output;
- confirmation that production PyPI was not attempted.

A production PyPI receipt PR records the same facts for production PyPI and
links the same-commit TestPyPI proof.

## Acceptance Examples

### Blocked TestPyPI Run

A run that builds and smokes the wheel but fails upload with `invalid-publisher`
is a blocked proof. It may update blocker receipts and issue #563, but it must
not be called a successful TestPyPI publish.

### TestPyPI-Proven But Not Production-Released

After TestPyPI upload and install-back pass, the repo may claim
TestPyPI-proven. It must still claim production PyPI as unreleased unless the
separate production proof passes.

### Rust Release Candidate

A Rust release candidate may use the Rust publish graph:

```text
hl7v2
hl7v2-server
hl7v2-cli
```

It must not include `hl7v2-python` as a crates.io publish target.

## Non-Goals

- No workflow behavior changes in this spec.
- No Python publishing by adding this spec.
- No crates.io publishing.
- No evidence schema changes.
- No weakening of the current guarded workflow boundaries.
