# Release Integrity Status

## Scope
Current verification status for the `hl7v2-rs` workspace after the contract, HTTP runtime, gRPC proof, and schema workflow repairs through merge commit `6de37e0`.

This document is not a v1.3.0 release approval. It separates green CI, behavior-tested surfaces, and crates.io publish readiness.

## Verified Success Baseline
- **CI / Coverage / Security / API Contracts**: All main branch workflows passed on `6de37e0`.
- **Rust 2024 / MSRV 1.93**: Enforced by the workspace and CI matrix.
- **HTTP Runtime Contracts**: OpenAPI and runtime agree for `/hl7/parse`, `/hl7/validate`, `/hl7/ack`, and `/hl7/normalize`.
- **gRPC Contract Tests**: Unary Parse, Validate, GenerateAck, Normalize, and HealthCheck behavior is covered; ParseStream is explicitly unsupported.
- **Schema Contracts**: Profile schemas, converted config fixtures, and schema compilation run strictly with `ajv-formats` and draft7.

## Dependency Health
- **Rationalized**: Upgraded `thiserror` to 2.0 and `tokio` to 1.50.
- **Consolidated**: Duplicate versions of `tonic`, `prost`, and `base64` have been resolved.
- **Security Workflow**: Current main Security workflow is green.

## Publish Readiness
- **Publish Order**: `cargo run -p xtask -- publish-plan` resolves 30 publishable crates.
- **Dry-run Proof**: Direct `cargo publish --dry-run --locked` passed for crates 1-16 in publish order, ending with `hl7v2-core`; workspace-patched dry-run verification passed for crates 17-30.
- **Current Stop Condition**: Direct dry-run stopped at crate 17, `hl7v2`, because `hl7v2-core` is not yet present in the crates.io index. This is a registry-state boundary, not a source compilation failure.
- **Higher-level Crates**: Package verification is proven by `cargo run -p xtask -- publish-dry-run --from hl7v2 --workspace-patches`; direct public-registry dry-run still requires the real dependency publish sequence.

## Documentation Accuracy
- **README / STATUS / API Guide**: Distinguish tested runtime surfaces from partial publish readiness.
- **OpenAPI**: Current HTTP contract lives at `api/openapi/hl7v2-api-v1.yaml`.
- **Release Claims**: "Green" means current workflows passed. "Tested" means contract/runtime tests exist for the named surface. "Package-verified" means every publishable crate has a dry-run verification path. "Published" remains false until crates.io upload actually runs.

## Conclusion
The repository has a green, behavior-tested, and package-verified main branch. The next release-readiness step is the actual dependency-ordered crates.io publish sequence documented by `cargo run -p xtask -- publish-plan`.
