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
| CLI evidence commands | Stable | `cargo test -p hl7v2-cli --test integration_tests` |
| CLI BDD workflows | Stable | `cargo test -p hl7v2-cli --test bdd_tests` |
| REST evidence sidecar | Stable | `cargo test -p hl7v2-server --test validate_endpoint_test`; `cargo test -p hl7v2-server --test bundle_endpoint_test`; `cargo test -p hl7v2-server --test replay_endpoint_test` |
| REST runtime contracts | Stable | `cargo test -p hl7v2-server --test http_runtime_contract_test` |
| gRPC evidence service | Beta | `cargo test -p hl7v2-server --test grpc_contract_tests`; `cargo test -p hl7v2-cli --test serve_grpc_contract_test` |
| PHI and quarantine sentinels | Stable | `cargo test -p hl7v2-e2e-tests security`; `cargo test -p hl7v2-server --test quarantine_output_hooks_test` |
| Python binding local wheel lane | Experimental | `python tests/python_smoke/smoke.py`; `python tests/python_smoke/evidence_workflow_guide.py` after a local wheel install |
| Python TestPyPI distribution (`hl7v2`) | Blocked | Issue [#563](https://github.com/EffortlessMetrics/hl7v2-rs/issues/563); requires TestPyPI upload and install-back receipt |
| Production PyPI distribution (`hl7v2`) | Not released | Requires same-commit TestPyPI proof, production PyPI upload, install-back from `https://pypi.org/simple/`, smoke proof, and receipt PR |
| Primary Rust crates.io product graph | Stable | `cargo run -p xtask -- publish-plan --surface primary`; must report `hl7v2`, `hl7v2-server`, and `hl7v2-cli`. v1.5.0 release claims require readiness, dry-run, publish, and tag receipts. |
| Binding backend crates | Planned / governed | `cargo run -p xtask -- publish-plan --surface bindings`; ADR [HL7V2-ADR-0003](../adr/HL7V2-ADR-0003-publishable-binding-backend-crates.md); #604-#608 closeout proved classification and metadata framing only. Future publish proof must show package metadata, dry-run, and language install/import smoke. |

## Rules

- Do not copy this table into specs or plans. Link here for claim tier impact.
- Do not copy current release status here. Link to `docs/STATUS.md`.
- Do not claim TestPyPI or production PyPI success without upload and
  install-back receipts.
- Do not add `hl7v2-python` to the primary Rust product graph.
- If a binding backend crate becomes publishable, record it as binding
  infrastructure, not as the recommended Rust API.
- Do not treat binding-backend closeout receipts as registry publish receipts.
- When a surface changes tier, update this map and the relevant proof receipt in
  the same PR.
