# Post-371-381 Contract Integrity Audit

## Scope

This audit covers the contract and schema stabilization wave ending with PR #381, plus the immediate runtime proof from PR #382 and PR #383 where it affects contract truth. It is an audit-only deliverable: no Rust code, workflow, schema, OpenAPI, or proto changes are made here.

The audit asks whether the current contract surface is strict, representative, and safe to build on, or whether it is only easier to keep green.

## Verdict

PR #381 repaired the most load-bearing workflow defect: the AJV invocations are now syntactically valid, load `ajv-formats`, and the config schema step no longer ends in `|| true`. The API Contracts workflow is therefore a real strict signal for the files it actually checks.

That signal is not yet complete enough to treat as full runtime truth. The remaining issues are mostly coverage and documentation gaps: the workflow's pull-request path filter misses workflow-only edits, Buf lint ignores are undocumented, config validation has no checked-in `config/*.toml` sample, OpenAPI auth prose overstates the authenticated surface, and proto comments do not fully match gRPC validation input behavior.

No concrete defect found in this audit was fixed in this PR. The defects below should be handled as follow-up work, with the runtime/API truth pass as the natural next code-changing lane.

## Findings

| Area | Finding | Evidence | Disposition |
| --- | --- | --- | --- |
| API Contracts workflow | Strict for current commands, but the PR trigger only watches `api/**`; workflow-only PR edits do not automatically run API Contracts. | `.github/workflows/contracts.yml` includes `.github/workflows/contracts.yml` under `push.paths`, but not under `pull_request.paths`. | Follow-up defect: add the workflow path to PR triggers or document a manual required check for workflow-only contract changes. |
| AJV invocation | Profile and config validation now use continued commands, `-c ajv-formats`, and no advisory `|| true`. | `Validate Profile Schemas` and `Validate Config Schema` both call `ajv validate -c ajv-formats ... --spec=draft7`. | Accept as strict for matched data files. |
| AJV coverage | Config schema validation currently targets `config/*.toml`, but there is no checked-in `config/` sample tree in the workspace. | `rg --files profiles schemas config` finds schemas and profiles, and reports no `config` path. | Follow-up defect: add a representative config fixture or change the contract to fail when no config sample is present. |
| Schema compilation | The workflow compiles every `schemas/**/*.schema.json` with `ajv compile -c ajv-formats`. | `Validate Test Data` loops over all schema files. | Accept as schema-syntax coverage, not instance coverage. |
| Buf lint | Buf uses `STANDARD` but ignores several naming and directory rules. | `api/proto/buf.yaml` ignores `RPC_REQUEST_RESPONSE_UNIQUE`, request/response standard names, `PACKAGE_DIRECTORY_MATCH`, and `ENUM_VALUE_PREFIX`. | Follow-up defect: document why each ignore is intentional, or remove ignores that only preserve legacy layout. |
| OpenAPI routes | OpenAPI and HTTP runtime agree on the currently documented HTTP routes: `/health`, `/ready`, `/metrics`, `/hl7/parse`, and `/hl7/validate`. | `api/openapi/hl7v2-api-v1.yaml` documents those paths; `crates/hl7v2-server/src/routes.rs` registers the same set. | Accept for current HTTP surface. |
| HTTP parity | HTTP has no `/hl7/ack` or `/hl7/normalize`, while gRPC exposes `GenerateAck` and `Normalize`. | `routes.rs` nests only `/hl7/parse` and `/hl7/validate`; proto and gRPC implementation expose the ACK and normalization RPCs. | Runtime/API truth pass should decide whether to implement HTTP parity or keep those operations out of OpenAPI. |
| Auth truth | Runtime applies `X-API-Key` auth only to `/hl7/*` routes when an API key is configured; health, readiness, metrics, docs, and OpenAPI YAML are public. OpenAPI path-level security matches `/hl7/*`, but the top-level description says all endpoints require an API key. | `routes.rs` applies auth middleware only to `api_routes`; OpenAPI security appears on `/hl7/parse` and `/hl7/validate`, while the description says "All endpoints require API Key authentication". | Follow-up defect: align the prose with public health/metrics behavior or change runtime policy. |
| CORS | Runtime still allows any origin, method, and header. | `build_cors_layer()` uses `CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any)`. | Follow-up defect for the runtime/API truth pass: decide whether CORS is intentionally open or config-driven. |
| gRPC implementation | Proto and Rust implementation now have behavior proof for Parse, Validate, GenerateAck, Normalize, HealthCheck, and explicit ParseStream unsupported behavior. | `crates/hl7v2-server/tests/grpc_contract_tests.rs` covers those RPCs after PR #383. | Accept as current gRPC behavior proof. |
| ParseStream | ParseStream is declared in proto but intentionally unimplemented in Rust. | `parse_stream` returns `Status::unimplemented("Streaming parse not yet implemented")`; tests assert the unsupported status. | Accept only because the unsupported behavior is explicit and tested. |
| Validate profile input | Proto says `ValidateRequest.profile` is a "Profile name or URL", but Rust treats it as inline profile YAML. | `api/proto/hl7v2/v1/hl7v2.proto` comment says name or URL; `grpc.rs` calls `hl7v2_prof::load_profile(&req.profile)`. | Follow-up defect: either update proto/API wording to inline YAML or implement name/URL loading. |
| JSON schema relaxations | Some relaxations appear domain-supported, but others look tooling-driven and should be revisited now that `ajv-formats` is loaded. | `GENERIC` matches checked-in profile inheritance; widened segment path patterns allow alphanumeric segment IDs such as profile paths with `PV1`. Removed `format: uri` from config URL fields is no longer justified by missing format support. | Follow-up defect: restore URI formats for URL-like config fields unless a domain reason exists for arbitrary strings. |
| Release claims | Existing release audit wording says the repo is green, tested, and ready for v1.3.0 release, but current contract evidence still has known follow-up defects. | `docs/audits/final-release-integrity.md` states all workflows are green and the project is ready for v1.3.0. | Follow-up documentation pass should distinguish green CI, behavior-tested surfaces, and publish-ready surfaces. |

## Schema Relaxation Notes

The profile schema changes are mixed:

- `message_structure: GENERIC` is consistent with the checked-in `profiles/generic.yaml` and child profiles that use `parent: "GENERIC"`.
- Alphanumeric path patterns are compatible with real HL7 segment IDs such as `PV1`.
- Removing `constraint_type` from required fields aligns with the Rust `Constraint` model, which uses booleans and typed constraint structures instead of a required `constraint_type` field.
- Removing `format: uri` from `profiles.remote.sources[].url` and `telemetry.endpoint` looks like a tooling workaround from the pre-#381 AJV state. With `ajv-formats` now loaded, those URL-like fields should regain URI validation unless the config model intentionally accepts non-URI identifiers.

## Required Follow-ups

1. Add `.github/workflows/contracts.yml` to the API Contracts pull-request path filter.
2. Add or require a representative `config/*.toml` fixture so config schema validation is not only theoretical.
3. Document or reduce the Buf lint ignores in `api/proto/buf.yaml`.
4. Align OpenAPI authentication prose with runtime public routes, especially `/metrics`.
5. Decide HTTP `/hl7/ack` and `/hl7/normalize` parity in the runtime/API truth pass.
6. Decide whether CORS remains intentionally open or becomes config-driven.
7. Align `ValidateRequest.profile` documentation with inline YAML behavior, or implement profile name/URL loading.
8. Revisit removed URI formats now that `ajv-formats` is part of the strict contract workflow.

## Stop Condition

The contract surface is safe to build on only after the strict workflow stays green and the follow-up defects above are either fixed or explicitly documented as intentional product choices. Until then, "green" means the current checks pass; it does not yet mean the HTTP, gRPC, schema, and release surfaces are fully contract-complete.
