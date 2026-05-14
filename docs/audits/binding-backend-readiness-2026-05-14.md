# Binding Backend Readiness Audit

Date: 2026-05-14

This audit records the binding-backend package-boundary work that landed after
the initial #604-#608 closeout. It is a readiness receipt only. It is not a
crates.io publish receipt, a TestPyPI receipt, a PyPI receipt, an npm receipt,
a tag receipt, or a GitHub release receipt.

## Scope

| Field | Value |
| --- | --- |
| Repository commit | `26964ccbc2b01301f590765b6e92c06e24f08333` |
| Release line | `1.5.0` candidate |
| Primary Rust product graph | `hl7v2`, `hl7v2-server`, `hl7v2-cli` |
| Binding backend graph | `hl7v2-python` |
| Public Python distribution | `hl7v2` |
| Future public npm package | `@effortlessmetrics/hl7v2` |

## Recorded Work

| PR | Result |
| --- | --- |
| #610 | Added the binding-backend release-proof spec, defining package review, dry-run, language smoke, and receipt requirements before backend publish claims. |
| #611 | Added the binding backend dry-run surface so release tooling can prove binding backend packaging separately from the primary Rust graph. |
| #612 | Prepared `hl7v2-python` as a publishable PyO3 backend crate while keeping it outside the recommended Rust API. |
| #613 | Defined the npm/WASM package model: public TypeScript users target `@effortlessmetrics/hl7v2`; Rust backend crates stay binding infrastructure. |
| #614 | Added a publish-surface classification guard so publishable workspace packages must be classified instead of silently joining the wrong graph. |

## Current Release Meaning

The repo can now express three Rust publish surfaces:

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

`hl7v2-python` is publishable as binding infrastructure, not as the recommended
Rust API. Rust users should depend on `hl7v2`. Python users should install the
public `hl7v2` distribution from PyPI after the Python release lane is proven.

## Non-Claims

This audit does not claim:

- `hl7v2-python` was uploaded to crates.io;
- the primary Rust v1.5.0 graph was uploaded to crates.io;
- a `v1.5.0` tag or GitHub release was created;
- the public Python `hl7v2` package was uploaded to TestPyPI or PyPI;
- an npm package exists or was published.

## Release Decision Boundary

Before any v1.5.x crates.io release, refresh the release-readiness proof against
the current package surfaces and make the release graph explicit:

- primary-only: `hl7v2`, `hl7v2-server`, `hl7v2-cli`;
- primary plus binding backend: `hl7v2`, `hl7v2-python`, `hl7v2-server`,
  `hl7v2-cli`.

A crates.io backend publish does not prove PyPI release success. A PyPI release
does not require Python users to depend on `hl7v2-python` directly.

## Python Boundary

The public Python lane remains separate:

- TestPyPI project: `hl7v2`;
- workflow: `python-testpypi.yml`;
- environment: `testpypi`;
- external blocker: issue #563 until Trusted Publisher is configured.

No token fallback, `skip-existing`, or production PyPI claim is acceptable
without upload and install-back receipts.

## Validation

This audit PR verified the current docs and publish-surface model with:

| Command | Result |
| --- | --- |
| `cargo +1.95.0 run -p xtask -- check-doc-links` | pass |
| `cargo +1.95.0 run -p xtask -- publish-plan --surface primary` | pass; primary graph remains `hl7v2`, `hl7v2-server`, `hl7v2-cli` |
| `cargo +1.95.0 run -p xtask -- publish-plan --surface bindings` | pass; binding graph reports `hl7v2-python` |
| `cargo +1.95.0 run -p xtask -- publish-plan --surface all-publishable` | pass; all-publishable graph reports `hl7v2`, `hl7v2-python`, `hl7v2-server`, `hl7v2-cli` |
| `cargo +1.95.0 run -p xtask -- check-python-publish-policy` | pass; Python distribution remains `hl7v2`, backend crate remains separately receipted |
| `git diff --check` | pass |

The later release-readiness refresh should also run the relevant primary and
binding backend dry-runs before deciding which graph to publish.
