# HL7V2-SPEC-0008: Deployment Provenance

Status: Accepted
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

- `cargo +1.95.0 run -p xtask -- gate --check`
- `cargo +1.95.0 run -p xtask -- check-deployment-provenance`
- `cargo +1.95.0 run -p xtask -- check-file-policy`
- `cargo +1.95.0 run -p xtask -- policy-report`

The dedicated deployment-provenance checker scans checked-in deployment
examples for floating image tags, untagged image references, and hl7v2 image
tags that drift from the workspace version. The full gate runs the same checker
so deployment example drift cannot bypass the ordinary policy stack. Local
Compose examples may use `hl7v2-server:local`; production/provenance examples
should move to digest references when a release image is published and
receipted.

The checked-in Kubernetes deployment remains a version-tagged example so local
and internal smoke deployments can follow the workspace version. Production
provenance receipts should render
`infrastructure/k8s/deployment.digest.example.yaml` with a registry image
digest from the release image receipt before applying it. The digest example is
also scanned by `check-deployment-provenance`, and its placeholder is accepted
only because it requires an explicit digest-pinned image reference.
