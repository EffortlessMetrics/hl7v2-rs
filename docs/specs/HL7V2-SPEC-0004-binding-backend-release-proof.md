# HL7V2-SPEC-0004: Binding Backend Release Proof

Status: Accepted
Date: 2026-05-14
Proposal: [HL7V2-PROP-0001](../proposals/HL7V2-PROP-0001-source-of-truth-and-release-governance.md)
Source-of-truth stack: [HL7V2-SPEC-0001](HL7V2-SPEC-0001-source-of-truth-stack.md)
Related ADR: [HL7V2-ADR-0003](../adr/HL7V2-ADR-0003-publishable-binding-backend-crates.md)
Related Python proof spec: [HL7V2-SPEC-0002](HL7V2-SPEC-0002-python-distribution-proof.md)

## Contract

Binding backend crates are publishable implementation surfaces for
foreign-language APIs. They are honest APIs at the language boundary, but they
are not the recommended Rust API.

The primary Rust product graph remains:

```text
hl7v2
hl7v2-server
hl7v2-cli
```

The binding backend graph may include:

```text
hl7v2-python
future hl7v2-wasm
future hl7v2-node
```

A binding backend crate can be published only when it is:

- thin over `hl7v2`;
- version-locked to the workspace release;
- clearly documented as a backend for a language package;
- not promoted as the Rust API;
- present in `cargo run -p xtask -- publish-plan --surface bindings`;
- reviewed with `cargo package --list`;
- proven with `cargo publish --dry-run`;
- covered by that language package's install/import smoke proof;
- recorded in a release receipt.

Current `hl7v2-python` metadata may make the crate publishable as binding
infrastructure. It remains unpublished until a dedicated release PR records the
required proof and a crates.io upload plus registry resolution succeeds.

## Package Classes

| Class | Examples | Audience | Registry |
| --- | --- | --- | --- |
| Primary Rust product | `hl7v2`, `hl7v2-server`, `hl7v2-cli` | Rust users and operators | crates.io |
| Language package | PyPI `hl7v2`, future npm `@effortlessmetrics/hl7v2` | Python and TypeScript users | PyPI, npm |
| Binding backend crate | `hl7v2-python`, future `hl7v2-wasm`, future `hl7v2-node` | Packagers and binding maintainers | crates.io, if governed |
| Internal/dev crate | e2e tests, test utilities, examples, `xtask` | Repo maintainers | unpublished |

Do not use binding backend publication to recreate retired implementation
microcrates such as `hl7v2-parser`, `hl7v2-model`, `hl7v2-redact`, or
`hl7v2-mllp`.

## Required Proof

### Surface Classification

Before a binding backend crate can publish, release tooling must show the crate
in the binding backend graph:

```powershell
cargo +1.95.0 run -p xtask -- publish-plan --surface bindings
cargo +1.95.0 run -p xtask -- publish-dry-run --surface bindings
```

Before the primary `hl7v2` version has been published to crates.io, local
candidate proof may use workspace patches:

```powershell
cargo +1.95.0 run -p xtask -- publish-dry-run --surface bindings --workspace-patches --allow-dirty
```

The default publish plan must continue to show only the primary Rust product
graph unless an explicit release decision changes the command or receipt being
used. If a backend crate has `publish = false`, the binding dry-run must list
its package files and stop with a policy error that explains the crate is not
publishable yet.

### Package Review

The release PR must record the package file list:

```powershell
cargo +1.95.0 package -p hl7v2-python --list
```

The reviewer checks that the package contains only expected binding backend
sources, metadata, and license/readme files. It must not grow unrelated parser,
model, redaction, MLLP, batch, or stream implementation crates.

### crates.io Dry-Run

The release PR must record a dry-run:

```powershell
cargo +1.95.0 publish -p hl7v2-python --dry-run
```

If the backend release is being prepared before the primary `hl7v2` version is
available from crates.io, the candidate PR records the workspace-patched dry-run
instead. The final backend release receipt must explain which dependency source
was used and must not claim upload success until crates.io registry resolution
passes.

Dry-run success proves crates.io package shape. It does not prove a PyPI, npm,
or language-package release.

### Language Install/Import Smoke

The backend release proof must include the relevant language package smoke path.
For the Python backend, that means a local or hosted wheel proof that installs
the public Python package artifact and imports `hl7v2`:

```powershell
python tests/python_smoke/smoke.py
python tests/python_smoke/evidence_workflow_guide.py
```

When a future npm package exists, the proof must install the public npm package
and import `@effortlessmetrics/hl7v2`. It must not use `hl7v2-rs` as the
user-facing npm package name.

### Release Receipt

A binding backend release receipt records:

- release decision and version;
- binding backend graph from `xtask publish-plan --surface bindings`;
- `cargo package --list` review result;
- `cargo publish --dry-run` result;
- upload result if publication is approved;
- crates.io registry resolution if publication happened;
- language install/import smoke result;
- confirmation that the crate is not the recommended Rust API;
- confirmation that PyPI/npm success is not claimed unless the language
  registry upload and install-back passed.

## Boundary Rules

- A crates.io backend publish does not prove PyPI or npm release success.
- A PyPI or npm release does not require users to depend on the backend crate
  directly.
- `hl7v2-python` may publish as a binding backend crate only after metadata,
  dry-run tooling, and release receipts are ready.
- `hl7v2-python` must not be added to the primary Rust product graph.
- The public Python package and import remain `hl7v2`.
- The future public TypeScript package should be `@effortlessmetrics/hl7v2`.

## Acceptance Examples

### Primary-Only Rust Release

A Rust release that publishes only:

```text
hl7v2
hl7v2-server
hl7v2-cli
```

is valid when the receipt says it is primary-only and does not claim binding
backend publication.

### Binding Backend Ready But Not Published

A PR that adds metadata, `cargo package --list`, and `cargo publish --dry-run`
proof for `hl7v2-python` may claim the backend is crates.io-ready. It must not
claim crates.io publication until upload and registry resolution are proven.

### Python Published But Backend Not Published

A successful PyPI `hl7v2` release may claim the public Python package is
published. It must not imply Rust users should depend on `hl7v2-python`, and it
must not claim the backend crate was published to crates.io unless that separate
receipt exists.

## Non-Goals

- No Cargo metadata changes in this spec.
- No crates.io publish.
- No TestPyPI, PyPI, or npm publish.
- No workflow behavior changes.
- No JS/TS implementation.
- No new public implementation microcrates.
