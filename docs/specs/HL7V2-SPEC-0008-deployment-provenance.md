# HL7V2-SPEC-0008: Deployment Provenance

Status: Proposed
Date: 2026-05-19
Source-of-truth stack: [HL7V2-SPEC-0001](HL7V2-SPEC-0001-source-of-truth-stack.md)

## Contract

Deployment examples must not use floating images.

- Version tags are acceptable for examples/dev/local deployment docs.
- Digest pinning is required for production/provenance examples once an image is published.
- Deployment success must not be claimed without deploy/smoke receipt proof.

## Planned Rails

- `policy/dependency-surface-allowlist.toml`
- `policy/process-allowlist.toml`
- `policy/network-allowlist.toml`

Current checks:

- `cargo +1.95.0 run -p xtask -- check-file-policy`
- `cargo +1.95.0 run -p xtask -- policy-report`

Future acceptance requires a dedicated deployment-provenance checker before
this spec may move to Accepted.
