# Support Bundle Safe-Sharing Checklist Receipt

Date: 2026-05-18
Branch: `cli/support-bundle-safe-sharing`

## Scope

This receipt records the operator support-bundle polish that adds a generated
`SAFE-SHARING.md` checklist to CLI, Rust/Python core, REST, and gRPC evidence
bundles.

The checklist is advisory human text. It is manifest-hashed in newly generated
bundles, but replay remains compatible with older bundle manifests that do not
contain the checklist artifact.

## Changed Surfaces

- CLI `bundle` and `support-bundle` write `SAFE-SHARING.md`.
- Core `write_safe_analysis_bundle*` writes `SAFE-SHARING.md`, which covers
  Python local-wheel and shared Rust bundle paths.
- Server bundle and quarantine full-bundle writers include `SAFE-SHARING.md`.
- Evidence bundle manifest schemas accept the `safe_sharing_checklist` role.
- Operator guides and fixtures list the new generated artifact.

## Non-Claims

- No TestPyPI upload occurred.
- No PyPI upload occurred.
- No npm package exists.
- This is not a production Python public-registry install-back proof.
- This does not change the `hl7v2-python` binding-backend boundary.

## Validation

Passed:

```text
cargo +1.95.0 fmt --all -- --check
cargo +1.95.0 test -p hl7v2 evidence::tests::bundle_and_replay_keep_phi_out_of_reports --all-features --locked
cargo +1.95.0 test -p hl7v2 evidence::tests::replay_accepts_legacy_bundle_without_safe_sharing_checklist --all-features --locked
cargo +1.95.0 test -p hl7v2-cli test_bundle_writes_redacted_replayable_evidence_artifacts --test integration_tests --locked
cargo +1.95.0 test -p hl7v2-cli test_replay_accepts_legacy_bundle_without_safe_sharing_checklist --test integration_tests --locked
cargo +1.95.0 test -p hl7v2-server test_bundle_endpoint_writes_redacted_evidence_bundle --test bundle_endpoint_test --locked
cargo +1.95.0 test -p hl7v2-server test_grpc_validate_redacted_writes_quarantine_for_failed_validation --test grpc_contract_tests --locked
cargo +1.95.0 test -p hl7v2-server test_grpc_create_evidence_bundle_writes_redacted_bundle_and_v2_artifacts --test grpc_contract_tests --locked
cargo +1.95.0 test -p hl7v2-server test_quarantine_hook_writes_bundle_for_failed_redacted_validation --test quarantine_output_hooks_test --locked
cargo +1.95.0 test -p xtask --locked
cargo +1.95.0 clippy -p hl7v2 -p hl7v2-cli -p hl7v2-server --all-targets --locked -- -D warnings
cargo +1.95.0 run -p xtask -- check-safe-support-bundle-guide
cargo +1.95.0 run -p xtask -- check-evidence-artifacts-guide
cargo +1.95.0 run -p xtask -- check-first-use-guides
cargo +1.95.0 run -p xtask -- check-sidecar-guide
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo +1.95.0 run -p xtask -- python-local-wheel-proof
cargo +1.95.0 run -p xtask -- evidence-schema-check
cargo +1.95.0 run -p xtask -- check-doc-links
cargo +1.95.0 run -p xtask -- check-file-policy
cargo +1.95.0 run -p xtask -- check-lint-policy
cargo +1.95.0 run -p xtask -- check-no-panic-family
cargo +1.95.0 run -p xtask -- check-python-publish-policy
cargo +1.95.0 run -p xtask -- policy-report
cargo +1.95.0 run -p xtask -- badges --check
cargo +1.95.0 run -p xtask -- impacted-evidence --check
python -c "import tomllib, pathlib; tomllib.loads(pathlib.Path('.hl7v2/goals/active.toml').read_text(encoding='utf-8'))"
```

Notes:

- `badges/ripr.json` was regenerated before `badges --check`.
- `target/xtask/impacted-evidence/latest.json` and `.md` were regenerated
  before `impacted-evidence --check`.
- The successful `python-local-wheel-proof` remains local-wheel proof only.
