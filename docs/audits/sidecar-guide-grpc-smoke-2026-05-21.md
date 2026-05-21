# Sidecar Guide gRPC Smoke Receipt

Date: 2026-05-21
Branch: `test/sidecar-guide-grpc-smoke`
Scope: extend the executable source-checkout sidecar guide proof to cover the
documented gRPC transport and dirty evidence workflow path.

## Purpose

`docs/guides/deploy-validation-sidecar.md` includes an optional gRPC smoke with
`grpcurl`. The source-checkout guide command now backs that user path with
targeted Rust contract tests instead of requiring `grpcurl` to be installed:

```text
cargo +1.95.0 run -p xtask -- check-sidecar-guide
```

The command still proves the HTTP sidecar recipe from the existing receipt, and
now also runs:

```text
cargo test -p hl7v2-server --test grpc_contract_tests --locked test_grpc_transport_server_serves_health_check
cargo test -p hl7v2-server --test grpc_contract_tests --locked test_grpc_dirty_real_world_validate_redact_bundle_replay_workflow
```

Those tests prove that the typed gRPC server can serve the health check and
that the gRPC dirty real-world path covers validate-redacted, evidence bundle
creation, and replay over the shared dirty fixture family.

## Non-Claims

- This receipt does not prove a deployed gRPC service outside the local
  source-checkout contract tests.
- This receipt does not require or prove `grpcurl` availability.
- This receipt does not upload to TestPyPI or PyPI.
- This receipt does not prove `pip install hl7v2` from a public Python
  registry.
- This receipt does not publish or prove an npm package.
- This receipt does not create a new crates.io, tag, or GitHub release claim.
- This receipt does not promote `hl7v2-python` as the recommended Rust API.

## Validation

```text
cargo +1.95.0 run -p xtask -- check-sidecar-guide
cargo +1.95.0 test -p xtask check_sidecar_guide --locked
cargo +1.95.0 fmt --all -- --check
cargo +1.95.0 run -p xtask -- check-doc-links
cargo +1.95.0 run -p xtask -- check-file-policy
git diff --check
```
