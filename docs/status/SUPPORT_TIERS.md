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
| First-use user journeys | Stable where released; Python registry blocked | Rust: `cargo test -p hl7v2 --test user_journey --all-features`; CLI: `cargo test -p hl7v2-cli --test integration_tests journey_cli_validate_redact_bundle_replay_produces_shareable_receipts`; server: `tests/server_smoke/smoke.py` against a running sidecar; Python: `tests/python_smoke/smoke.py` and `tests/python_smoke/evidence_workflow_guide.py` after local wheel install. See the [user journey acceptance proof](../audits/user-journey-acceptance-2026-05-15.md). |
| Dirty corpus compatibility proof | Stable Rust core, CLI, REST, gRPC, and local Python wheel proof; TypeScript not released | `cargo test -p hl7v2 --lib --all-features dirty_real_world`; `cargo test -p hl7v2-cli --test integration_tests test_corpus_commands_share_dirty_real_world_fixture_categories`; `cargo test -p hl7v2-server --test corpus_endpoint_test test_corpus_endpoints_share_dirty_real_world_fixture_categories`; `cargo test -p hl7v2-server --test grpc_contract_tests test_grpc_corpus_commands_share_dirty_real_world_fixture_categories`; `tests/python_smoke/smoke.py` after local wheel install; [dirty real-world corpus proof](../audits/real-world-corpus-proof-2026-05-14.md); [shared fixture proof](../audits/dirty-real-world-corpus-shared-fixture-proof-2026-05-16.md); [server fixture proof](../audits/dirty-real-world-server-corpus-parity-2026-05-16.md); [Python fixture proof](../audits/dirty-real-world-python-corpus-parity-2026-05-16.md); [dirty-corpus parity readiness refresh](../audits/publish-dry-run-v1.5.0-2026-05-16-dirty-corpus-parity-refresh.md) |
| CLI evidence commands | Stable | `cargo test -p hl7v2-cli --test integration_tests` |
| CLI BDD workflows | Stable | `cargo test -p hl7v2-cli --test bdd_tests` |
| REST evidence sidecar | Stable | `cargo test -p hl7v2-server --test validate_endpoint_test`; `cargo test -p hl7v2-server --test bundle_endpoint_test`; `cargo test -p hl7v2-server --test replay_endpoint_test` |
| REST runtime contracts | Stable | `cargo test -p hl7v2-server --test http_runtime_contract_test` |
| gRPC evidence service | Beta | `cargo test -p hl7v2-server --test grpc_contract_tests`; `cargo test -p hl7v2-cli --test serve_grpc_contract_test`. Current gRPC evidence coverage includes parse, parse stream, validate, profile lint, profile explain, profile fixture test, validate-redacted with configured quarantine output, configured-root evidence bundle creation and replay, ACK payload/parsed-shape contract coverage, normalize, health, inline corpus summary, inline corpus fingerprint, and inline corpus diff. The beta tier reflects transport lifecycle and operational hardening, not a missing evidence-artifact parity claim for those RPCs. Enhanced ACK commit-code parity for gRPC requires a future proto/API/runtime change. |
| PHI and quarantine sentinels | Stable | `cargo test -p hl7v2-e2e-tests security`; `cargo test -p hl7v2-server --test quarantine_output_hooks_test` |
| Python binding local wheel lane | Experimental | `python tests/python_smoke/smoke.py`; `python tests/python_smoke/evidence_workflow_guide.py` after a local wheel install. Local helper proof covers parse, JSON export, normalize, default and enhanced ACK codes, generated fixtures, profile lint/explain/test, validate, corpus summary/fingerprint/diff, redaction, bundle, and replay surfaces; see the [Python local wheel proof](../audits/python-local-wheel-proof-2026-05-15.md) and [evidence parity readiness refresh](../audits/publish-dry-run-v1.5.0-2026-05-16-evidence-parity-refresh.md). It is not a TestPyPI or PyPI release claim. |
| Python TestPyPI distribution (`hl7v2`) | Blocked | Hosted non-publishing wheel/import/evidence workflow proof passed on current `main` after the SRP refactor wave; see the [Python TestPyPI non-publish proof](../audits/python-testpypi-nonpublish-proof-2026-05-16.md). Issue [#563](https://github.com/EffortlessMetrics/hl7v2-rs/issues/563) remains the external Trusted Publisher blocker; TestPyPI success still requires upload and install-back receipt. |
| Production PyPI distribution (`hl7v2`) | Not released | Requires same-commit TestPyPI proof, production PyPI upload, install-back from `https://pypi.org/simple/`, smoke proof, and receipt PR |
| TypeScript/npm package (`@effortlessmetrics/hl7v2`) | Not released | [HL7V2-SPEC-0005](../specs/HL7V2-SPEC-0005-npm-wasm-binding-package-model.md); requires future npm package review, install/import smoke, registry proof, and receipt |
| Primary Rust crates.io product graph | Stable | `cargo run -p xtask -- publish-plan --surface primary`; must report `hl7v2`, `hl7v2-server`, and `hl7v2-cli`. v1.5.0 is published and install-backed for the selected Rust graph; see the [v1.5.0 publish receipt](../audits/publish-v1.5.0-2026-05-15.md). |
| Binding backend crates | Published / governed | `cargo run -p xtask -- publish-plan --surface bindings`; ADR [HL7V2-ADR-0003](../adr/HL7V2-ADR-0003-publishable-binding-backend-crates.md); [binding-backend readiness audit](../audits/binding-backend-readiness-2026-05-14.md); [current-main dry-run refresh](../audits/publish-dry-run-v1.5.0-2026-05-14.md); [parity refresh](../audits/publish-dry-run-v1.5.0-2026-05-14-parity-refresh.md); [corpus refresh](../audits/publish-dry-run-v1.5.0-2026-05-14-corpus-refresh.md); [gRPC status refresh](../audits/publish-dry-run-v1.5.0-2026-05-14-grpc-status-refresh.md); [gRPC corpus evidence refresh](../audits/publish-dry-run-v1.5.0-2026-05-14-grpc-corpus-refresh.md); [numeric validation refresh](../audits/publish-dry-run-v1.5.0-2026-05-15-numeric-refresh.md); [gRPC profile lint refresh](../audits/publish-dry-run-v1.5.0-2026-05-15-grpc-profile-lint-refresh.md); [gRPC profile explain refresh](../audits/publish-dry-run-v1.5.0-2026-05-15-grpc-profile-explain-refresh.md); [gRPC profile test refresh](../audits/publish-dry-run-v1.5.0-2026-05-15-grpc-profile-test-refresh.md); [gRPC bundle creation refresh](../audits/publish-dry-run-v1.5.0-2026-05-15-grpc-bundle-refresh.md); [gRPC replay refresh](../audits/publish-dry-run-v1.5.0-2026-05-15-grpc-replay-refresh.md); [gRPC quarantine refresh](../audits/publish-dry-run-v1.5.0-2026-05-15-grpc-quarantine-refresh.md); [parity documentation refresh](../audits/publish-dry-run-v1.5.0-2026-05-15-parity-doc-refresh.md); [post-SRP refresh](../audits/publish-dry-run-v1.5.0-2026-05-16-post-srp-refresh.md); [dirty-corpus parity refresh](../audits/publish-dry-run-v1.5.0-2026-05-16-dirty-corpus-parity-refresh.md); [normalization and ACK parity refresh](../audits/publish-dry-run-v1.5.0-2026-05-16-normalization-ack-refresh.md); [evidence parity refresh](../audits/publish-dry-run-v1.5.0-2026-05-16-evidence-parity-refresh.md); [v1.5.0 release graph decision](../audits/v1.5.0-release-graph-decision-2026-05-14.md); [v1.5.0 publish receipt](../audits/publish-v1.5.0-2026-05-15.md). #604-#614 proved classification, release-proof spec, dry-run tooling, publishable `hl7v2-python` metadata, npm/WASM package model, and surface guards. The refreshes proved the binding dry-run surface through #702. v1.5.0 published `hl7v2-python` as binding backend infrastructure; this is not a public Python `hl7v2` PyPI/TestPyPI receipt and not the recommended Rust API. |
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
