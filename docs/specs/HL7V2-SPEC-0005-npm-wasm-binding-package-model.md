# HL7V2-SPEC-0005: npm and WASM Binding Package Model

Status: Accepted
Date: 2026-05-14
Proposal: [HL7V2-PROP-0001](../proposals/HL7V2-PROP-0001-source-of-truth-and-release-governance.md)
Source-of-truth stack: [HL7V2-SPEC-0001](HL7V2-SPEC-0001-source-of-truth-stack.md)
Related ADR: [HL7V2-ADR-0003](../adr/HL7V2-ADR-0003-publishable-binding-backend-crates.md)
Related backend proof spec: [HL7V2-SPEC-0004](HL7V2-SPEC-0004-binding-backend-release-proof.md)

## Contract

The future TypeScript user package is:

```text
npm package: @effortlessmetrics/hl7v2
import:      @effortlessmetrics/hl7v2
```

Do not use `hl7v2-rs` as the public npm package name. The `-rs` suffix is
appropriate for the repository identity and Rust implementation context, but it
is the wrong primary identity for JavaScript and TypeScript users.

Future Rust backend crates may exist only as binding infrastructure:

```text
hl7v2-wasm
hl7v2-node
```

Those crates are language-boundary APIs for packaging, ABI, lifecycle,
runtime-specific conversion, and release provenance. They are not the
recommended Rust API, and they must not become new homes for parser, model,
redaction, MLLP, batch, stream, or evidence implementation.

## Package Classes

| Class | Examples | Audience | Registry |
| --- | --- | --- | --- |
| Primary Rust product | `hl7v2`, `hl7v2-server`, `hl7v2-cli` | Rust users and operators | crates.io |
| Language package | PyPI `hl7v2`, npm `@effortlessmetrics/hl7v2` | Python and TypeScript users | PyPI, npm |
| Binding backend crate | `hl7v2-python`, future `hl7v2-wasm`, future `hl7v2-node` | Packagers and binding maintainers | crates.io, if governed |
| Internal/dev crate | e2e tests, test utilities, examples, `xtask` | Repo maintainers | unpublished |

The TypeScript package is the user-facing API. The Rust backend crate is an
implementation package boundary.

## Expected TypeScript Shape

The eventual public API should optimize for the language package identity:

```ts
import { parse, validate, normalize, redact } from "@effortlessmetrics/hl7v2";
```

The public API may internally select a WASM backend, a Node-native backend, or a
pure TypeScript fallback if one is later accepted. That selection is an
implementation concern and must not require users to import `hl7v2-wasm`,
`hl7v2-node`, or `hl7v2-rs` for normal usage.

## Backend Crate Rules

Future backend crates must:

- stay thin over the canonical `hl7v2` Rust crate;
- remain version-locked to the workspace release;
- appear in `cargo run -p xtask -- publish-plan --surface bindings` when they
  become publishable;
- follow [HL7V2-SPEC-0004](HL7V2-SPEC-0004-binding-backend-release-proof.md)
  for package list, dry-run, upload, and receipt proof;
- route Rust users to `hl7v2`;
- route TypeScript users to `@effortlessmetrics/hl7v2`;
- avoid owning parser, model, redaction, MLLP, batch, stream, or evidence
  implementation logic.

## npm Proof Requirements

A future npm release proof must record:

- package name `@effortlessmetrics/hl7v2`;
- package version;
- commit SHA;
- `npm pack --dry-run` output or equivalent package file review;
- install proof from the intended registry or tarball;
- import smoke that imports `@effortlessmetrics/hl7v2`;
- parse, validate, normalize, and redaction smoke coverage when those APIs are
  claimed;
- confirmation that no package named `hl7v2-rs` is claimed as the public SDK;
- confirmation that any Rust backend crate proof is separate from npm registry
  proof.

A crates.io backend publish does not prove npm release success. An npm release
does not prove crates.io backend publication.

## Acceptance Examples

### Correct Public npm Identity

```bash
npm install @effortlessmetrics/hl7v2
```

```ts
import { parse } from "@effortlessmetrics/hl7v2";
```

### Correct Backend Identity

```text
crates.io backend: hl7v2-wasm
npm package:       @effortlessmetrics/hl7v2
```

The backend crate can be published for provenance without becoming the public
TypeScript package name.

### Rejected Public npm Identity

```bash
npm install hl7v2-rs
```

This may be valid only for an explicitly low-level implementation package that
is not the public SDK. It must not be the default TypeScript user path.

## Non-Goals

- No JavaScript or TypeScript implementation.
- No WASM or Node backend crate.
- No npm package metadata.
- No npm publish.
- No crates.io publish.
- No workflow behavior changes.
- No new public Rust implementation microcrates.

