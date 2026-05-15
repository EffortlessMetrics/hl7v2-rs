# Support Tier Proof Map

This document maps product surfaces to support tiers and proof commands.

`docs/STATUS.md` remains the current feature and release status source of truth.
This map does not replace it. Specs and plans link here when a change affects
claim tier or proof expectations.

## Tier Definitions

| Tier | Meaning |
| --- | --- |
| Stable | Supported product behavior with repeatable local or hosted proof. |
| Beta | Supported for targeted use, but contract or coverage is still narrower than the stable surface. |
| Experimental | Available for proof and feedback, but not yet release-closed. |
| Blocked | Implementation or proof is present, but an external or release decision blocks completion. |
| Not released | No public release claim exists. |

## Support Map

| Surface | Tier | Proof |
| --- | --- | --- |
| Rust parse, validate, normalize, ACK, MLLP, and evidence models | Stable | `cargo test -p hl7v2 --all-features` |
| Evidence schemas | Stable | `cargo run -p xtask -- evidence-schema-check` |
| Cross-surface evidence parity contract | Accepted | [HL7V2-SPEC-0006](../specs/HL7V2-SPEC-0006-cross-surface-evidence-parity.md); surface-specific claims still require their own tests, schema checks, language install/import proof, or registry receipts |
| Dirty corpus compatibility proof | Stable core proof | `cargo test -p hl7v2 --lib --all-features dirty_real_world`; [dirty real-world corpus proof](../audits/real-world-corpus-proof-2026-05-14.md) |
| CLI evidence commands | Stable | `cargo test -p hl7v2-cli --test integration_tests` |
| CLI BDD workflows | Stable | `cargo test -p hl7v2-cli --test bdd_tests` |
| REST evidence sidecar | Stable | `cargo test -p hl7v2-server --test validate_endpoint_test`; `cargo test -p hl7v2-server --test bundle_endpoint_test`; `cargo test -p hl7v2-server --test replay_endpoint_test` |
| REST runtime contracts | Stable | `cargo test -p hl7v2-server --test http_runtime_contract_test` |
| gRPC evidence service | Beta | `cargo test -p hl7v2-server --test grpc_contract_tests`; `cargo test -p hl7v2-cli --test serve_grpc_contract_test`. Current gRPC evidence coverage includes parse, parse stream, validate, profile lint, validate-redacted, ACK, normalize, health, inline corpus summary, inline corpus fingerprint, and inline corpus diff. |
| PHI and quarantine sentinels | Stable | `cargo test -p hl7v2-e2e-tests security`; `cargo test -p hl7v2-server --test quarantine_output_hooks_test` |
| Python binding local wheel lane | Experimental | `python tests/python_smoke/smoke.py`; `python tests/python_smoke/evidence_workflow_guide.py` after a local wheel install |
| Python TestPyPI distribution (`hl7v2`) | Blocked | Issue [#563](https://github.com/EffortlessMetrics/hl7v2-rs/issues/563); requires TestPyPI upload and install-back receipt |
| Production PyPI distribution (`hl7v2`) | Not released | Requires same-commit TestPyPI proof, production PyPI upload, install-back from `https://pypi.org/simple/`, smoke proof, and receipt PR |
| TypeScript/npm package (`@effortlessmetrics/hl7v2`) | Not released | [HL7V2-SPEC-0005](../specs/HL7V2-SPEC-0005-npm-wasm-binding-package-model.md); requires future npm package review, install/import smoke, registry proof, and receipt |
| Primary Rust crates.io product graph | Stable | `cargo run -p xtask -- publish-plan --surface primary`; must report `hl7v2`, `hl7v2-server`, and `hl7v2-cli`. v1.5.0 release claims require readiness, dry-run, publish, and tag receipts. |
| Binding backend crates | Planned / governed | `cargo run -p xtask -- publish-plan --surface bindings`; ADR [HL7V2-ADR-0003](../adr/HL7V2-ADR-0003-publishable-binding-backend-crates.md); [binding-backend readiness audit](../audits/binding-backend-readiness-2026-05-14.md); [current-main dry-run refresh](../audits/publish-dry-run-v1.5.0-2026-05-14.md); [parity refresh](../audits/publish-dry-run-v1.5.0-2026-05-14-parity-refresh.md); [corpus refresh](../audits/publish-dry-run-v1.5.0-2026-05-14-corpus-refresh.md); [gRPC status refresh](../audits/publish-dry-run-v1.5.0-2026-05-14-grpc-status-refresh.md); [gRPC corpus evidence refresh](../audits/publish-dry-run-v1.5.0-2026-05-14-grpc-corpus-refresh.md); [numeric validation refresh](../audits/publish-dry-run-v1.5.0-2026-05-15-numeric-refresh.md); [gRPC profile lint refresh](../audits/publish-dry-run-v1.5.0-2026-05-15-grpc-profile-lint-refresh.md); [v1.5.0 release graph decision](../audits/v1.5.0-release-graph-decision-2026-05-14.md). #604-#614 proved classification, release-proof spec, dry-run tooling, publishable `hl7v2-python` metadata, npm/WASM package model, and surface guards only. The 2026-05-14 and 2026-05-15 refreshes proved the binding dry-run surface after #616-#618, after #621-#622, after #624-#625, after #627, after #629-#630, after #632-#633, and again after #635-#636. v1.5.0 selects `hl7v2-python` as binding backend infrastructure, but future publish proof must still show registry resolution, language install/import smoke, and release receipts. |
| RIPR evidence surface | Advisory | `cargo run -p xtask -- badges --check`; `cargo run -p xtask -- impacted-evidence --check`; [2026-05-15 calibration audit](../audits/ripr-calibration-2026-05-15.md). RIPR remains advisory static mutation-exposure proof and does not replace targeted runtime mutation. |

## Rules

- Do not copy this table into specs or plans. Link here for claim tier impact.
- Do not copy current release status here. Link to `docs/STATUS.md`.
- Do not claim TestPyPI or production PyPI success without upload and
  install-back receipts.
- Do not add `hl7v2-python` to the primary Rust product graph.
- If a binding backend crate becomes publishable, record it as binding
  infrastructure, not as the recommended Rust API.
- Do not treat binding-backend closeout receipts as registry publish receipts.
- Do not treat the binding-backend readiness audit as a crates.io, PyPI,
  TestPyPI, npm, tag, or GitHub release receipt.
- Do not use `hl7v2-rs` as the public npm SDK package.
- When a surface changes tier, update this map and the relevant proof receipt in
  the same PR.
