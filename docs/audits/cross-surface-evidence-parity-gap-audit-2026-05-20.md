# Cross-Surface Evidence Parity Gap Audit

Date: 2026-05-20
Base commit: `658f5c9825983713dec504cf0d6f416a89363df7`

This audit refreshes the current evidence-parity gap map after the source repo
landed the release, evidence, guide-smoke, deployment-provenance, and
public-registry proof scaffolding work that followed the 2026-05-18 audit.
It also records the current boundary before further swarm cutover work: source
main is clean and checked, but public Python registry proof and self-hosted
runner administration are still externally blocked.

It is a current-state map, not a release-readiness receipt. It does not add
runtime behavior, publish packages, tag a release, enable branch protection,
claim deployment success, or claim TestPyPI, PyPI, npm, or new crates.io
success.

## Source Evidence

| Source | What it proves |
| --- | --- |
| [HL7V2-SPEC-0006](../specs/HL7V2-SPEC-0006-cross-surface-evidence-parity.md) | Accepted parity contract and minimum proof rules. |
| [`policy/evidence-parity.toml`](../../policy/evidence-parity.toml) | Machine-readable current parity surface states, proof commands, fixture families, and known gaps. |
| [Support tier map](../status/SUPPORT_TIERS.md) | Current claim tier and proof command map. |
| [Full Evidence Receipt Path](../guides/full-evidence-receipt-path.md) | User-facing validate, redact, support-bundle, replay, and evidence summary path. |
| [First Use By Surface](../guides/first-use-by-surface.md) | Rust, CLI, server, and Python first-use routing with Python public-registry boundary. |
| [First 10 Minutes](../guides/first-10-minutes.md) | CLI onboarding path for doctor, validation, profile, corpus, support-bundle, and replay evidence. |
| [Vendor Upgrade Diff](../guides/vendor-upgrade-diff.md) | Before/after corpus summary, fingerprint, and diff workflow. |
| [Operator Error Guidance](../guides/operator-error-guidance.md) | Safe failure interpretation across representative CLI, REST, and gRPC surfaces. |
| [Safe Support Bundle](../guides/safe-support-bundle.md) | Operator support packet recipe and safe-sharing checklist expectations. |
| [Evidence Artifacts For Operators](../guides/evidence-artifacts-for-operators.md) | Artifact interpretation guide for reports, receipts, bundles, replay, and corpus evidence. |
| [Deploy Validation Sidecar](../guides/deploy-validation-sidecar.md) | Source-checkout sidecar smoke for HTTP validation deployment. |
| [User journey acceptance proof](user-journey-acceptance-2026-05-15.md) | Rust, CLI, server, and local Python first-use evidence workflow proof. |
| [Public crates install smoke](public-crates-install-first-use-2026-05-16.md) | crates.io install-back for `hl7v2`, `hl7v2-cli`, and `hl7v2-server` v1.5.0. |
| [Dirty real-world redacted support bundle proof](dirty-real-world-redacted-support-bundle-parity-2026-05-20.md) | Latest dirty-corpus fixture expansion and Rust/CLI/REST/gRPC/local Python parity counts. |
| [Dirty real-world vendor ORU proof](dirty-real-world-vendor-oru-null-text-parity-2026-05-18.md) | Vendor-shaped ORU null/text fixture expansion and Rust/CLI/REST/gRPC/local Python parity counts. |
| [Sidecar guide smoke](sidecar-guide-smoke-2026-05-18.md) | Ephemeral-loopback sidecar proof for guide deployment behavior. |
| [Python public registry proof command](python-public-registry-proof-command-2026-05-18.md) | Local install-back proof command for TestPyPI/PyPI after public upload succeeds. |
| [Python public registry workflow routing](python-public-registry-proof-workflow-routing-2026-05-18.md) | Hosted TestPyPI/PyPI install-back jobs route through the shared xtask proof command. |
| [Python registry proof parity boundary](python-registry-proof-parity-boundary-2026-05-18.md) | The parity manifest records blocked TestPyPI/PyPI install-back proof commands. |

## Current-State Verification

| Check | Result |
| --- | --- |
| Source branch | `main...origin/main`, clean before this audit refresh. |
| Source base commit | `658f5c9825983713dec504cf0d6f416a89363df7`. |
| Source main GitHub checks | Latest `CI`, `CI Policy`, `API Contracts`, `Coverage`, and `Security` runs for `658f5c9` completed successfully on 2026-05-20. |
| Public Python blocker | [#563](https://github.com/EffortlessMetrics/hl7v2-rs/issues/563) remains open for TestPyPI Trusted Publisher setup. |
| TestPyPI package endpoint | `https://test.pypi.org/pypi/hl7v2/json` returned `404` on 2026-05-20. |
| PyPI package endpoint | `https://pypi.org/pypi/hl7v2/json` returned `404` on 2026-05-20. |
| Swarm runner setup | `hl7v2-rs-swarm` has a green hosted `HL7v2 Rust Small` main run, but the self-hosted runner setup and branch protection remain blocked by runner-group/admin access tracked in `EffortlessMetrics/hl7v2-rs-swarm#5`. |

The swarm runner state is recorded here only to avoid confusing CI-migration
blockers with product parity blockers. It does not change the evidence parity
claim tiers for Rust, CLI, REST, gRPC, Python, or TypeScript.

## Current Gap Matrix

| Contract | Current proof state | Remaining gap | Next lane |
| --- | --- | --- | --- |
| parse / write | Rust, CLI, REST, gRPC, and local Python parse paths are covered by tests, first-use receipts, and local wheel proof; write parity is exposed as canonical serialization or normalized output where the surface provides it. | REST and gRPC do not claim a standalone general write endpoint/RPC; public Python registry install-back is absent; TypeScript is unimplemented. | TestPyPI proof first; keep server write claims scoped to exposed endpoints; TypeScript/WASM later. |
| validate | Rust, CLI, REST, gRPC, and local Python validation helpers are covered by support-tier proof commands, guide smokes, and local wheel proof. | Public Python registry install-back is absent; TypeScript is unimplemented. | TestPyPI proof, then Python public parity receipt. |
| normalize | Rust, CLI, REST/gRPC, and local Python normalization are documented as current parity surfaces. | Public Python registry install-back is absent; TypeScript is unimplemented. | Python public proof, then TypeScript plan. |
| ACK | Rust, CLI, REST, gRPC, and local Python proof cover the ACK cases currently exposed by each surface; gRPC has full six-code payload and parsed-shape contract coverage. | Public Python registry install-back is absent; TypeScript is unimplemented; any broader CLI/REST ACK matrix must be proven before it is claimed. | Python public proof; add focused CLI/REST ACK matrix tests only if a future PR broadens those claims. |
| profile lint / explain / test | Rust, CLI, gRPC, first-use guides, and local Python helper proof are recorded; REST claims remain limited to exposed endpoints. | Public Python registry install-back is absent; TypeScript is unimplemented. | Python public proof, then parity acceptance suite. |
| redaction / quarantine | Rust, CLI, REST, gRPC `ValidateRedacted`, safe support-bundle guide smoke, and local Python redaction helper proof exist. | Python quarantine output is not a public package claim; public Python registry install-back is absent; TypeScript is unimplemented. | Do not claim Python quarantine unless a focused helper and smoke proof are added. |
| bundle / replay | Rust, CLI, REST, gRPC, and local Python helper proof exist for bundle/replay semantics; `cargo run -p xtask -- check-bundle-replay-parity` composes the Rust/CLI/REST/gRPC acceptance path; first-use and support-bundle guide smokes exercise the operator path. | Public Python registry install-back is absent; TypeScript is unimplemented; Python remains local-wheel proof until registry install-back exists. | Python public proof before promoting Python registry parity; keep the shared runner current as bundle/replay surfaces change. |
| corpus summary / fingerprint / diff | Rust core, CLI, REST, gRPC, and local Python dirty-corpus parity proof share `test_data/dirty-real-world/`; `cargo run -p xtask -- check-dirty-corpus-parity` composes the Rust/CLI/REST/gRPC acceptance path and now includes operator guide coverage for vendor upgrade diff. | TypeScript is unimplemented; Python remains local-wheel proof until registry install-back exists. | Keep Python dirty workflow proof local-wheel-scoped until TestPyPI/PyPI install-back exists; TypeScript/WASM later. |
| safe error shape | `cargo run -p xtask -- check-safe-error-phi-parity` composes fixture-backed Rust, CLI, REST, and gRPC checks; `cargo run -p xtask -- check-operator-error-guidance-guide` proves representative user-facing REST, gRPC, and CLI safe-failure guidance. | Python remains local-wheel smoke until public registry install-back exists; TypeScript is unimplemented. | Keep the runner current as new surfaces claim safe-error parity; add Python public proof after TestPyPI/PyPI receipts. |
| `schema_version` behavior | Evidence schemas and surface-specific tests cover v1/v2 outputs where implemented, and `cargo run -p xtask -- check-schema-version-parity` composes the fixture-backed Rust, CLI, REST, and gRPC checks. | Python remains local-wheel smoke until public registry install-back exists; TypeScript is unimplemented. | Keep the runner current as new surfaces claim schema-version parity; add Python public proof after TestPyPI/PyPI receipts. |
| PHI sentinel behavior | PHI and quarantine sentinels are stable in support tiers, Python/local evidence receipts include PHI-safe checks, guide smokes include PHI-sentinel boundaries, and `cargo run -p xtask -- check-safe-error-phi-parity` covers Rust, CLI, REST, and gRPC PHI fixture checks. | Python remains local-wheel smoke until public registry install-back exists; TypeScript is unimplemented. | Keep PHI sentinel proof explicit when adding new artifact families or language surfaces. |
| TypeScript / npm | Package identity is specified as `@effortlessmetrics/hl7v2`. | No npm package, WASM backend, pack/install/import proof, or parity fixtures exist. | Plan npm/WASM only after Python public proof is resolved or deliberately parked. |

## Implementation Queue

1. Finish external TestPyPI Trusted Publisher setup for public project `hl7v2`,
   then rerun `Python TestPyPI Proof` with `publish_to_testpypi=true`.
2. Record TestPyPI upload, install-back, `import hl7v2`, `smoke.py`,
   `dirty_evidence_workflow.py`, and `evidence_workflow_guide.py` proof before
   closing #563.
3. Decide production PyPI separately from TestPyPI.
4. Keep [`policy/evidence-parity.toml`](../../policy/evidence-parity.toml)
   current as the machine-readable parity manifest for proof commands, fixture
   families, supported surfaces, and known gaps.
5. Use `cargo run -p xtask -- check-evidence-parity-acceptance` as the default
   local Rust/CLI/REST/gRPC parity acceptance suite. It includes the
   operator-error guidance smoke so representative REST, gRPC, and CLI
   safe-failure guidance cannot drift out of the aggregate proof. Use
   `--include-python` only when a local Python wheel is installed.
6. Keep the first-use and operator guide smokes current when docs or user
   workflows change: `check-first-use-guides`,
   `check-first-use-by-surface-guide`, `check-first-10-minutes-guide`,
   `check-vendor-upgrade-diff-guide`, `check-operator-error-guidance-guide`,
   `check-safe-support-bundle-guide`, `check-evidence-artifacts-guide`, and
   `check-sidecar-guide`.
7. Keep `cargo run -p xtask -- check-safe-error-phi-parity` as the shared
   safe-error and PHI sentinel runner for Rust, CLI, REST, and gRPC surfaces;
   use `--include-python` only when a local Python wheel is installed.
8. Keep `cargo run -p xtask -- check-schema-version-parity` as the shared
   schema-version runner for Rust, CLI, REST, and gRPC surfaces; use
   `--include-python` only when a local Python wheel is installed.
9. Keep `cargo run -p xtask -- check-bundle-replay-parity` as the shared
   bundle/replay runner for Rust, CLI, REST, and gRPC surfaces; use
   `--include-python` only when a local Python wheel is installed.
10. Keep `cargo run -p xtask -- check-dirty-corpus-parity` as the shared
    dirty-corpus runner for Rust, CLI, REST, and gRPC corpus
    summary/fingerprint/diff proof plus the CLI, REST, and gRPC
    validate/redact/bundle/replay workflow proof; use `--include-python` only
    when a local Python wheel is installed.
11. Keep the local-wheel Python dirty validate/redact/bundle/replay smoke
    current when dirty-corpus fixture semantics change.
12. Keep gRPC as Beta until transport lifecycle and operational hardening
    catches up with the artifact semantics already covered for implemented RPCs.
13. Keep the swarm runner migration separate from product parity claims until
    runner-group access, CX53/CX43 routing proof, and branch protection are
    actually configured and receipted.
14. Start npm/WASM implementation planning only after Python public proof is
    resolved or deliberately parked.

## Registry Boundary Check

Live package checks on 2026-05-20 still returned no public Python `hl7v2`
package:

| Check | Result |
| --- | --- |
| `https://test.pypi.org/pypi/hl7v2/json` | `404` |
| `https://pypi.org/pypi/hl7v2/json` | `404` |

Those `404` checks are not a substitute for Trusted Publisher setup. They only
confirm that this audit does not have a public Python upload/install-back
receipt to record.

## Boundaries

- No new crates.io upload.
- No new tag or GitHub release.
- No TestPyPI upload or install-back.
- No production PyPI upload or install-back.
- No token fallback.
- No `skip-existing`.
- No npm package.
- No TypeScript implementation.
- No new runtime feature claim.
- No deployment success claim.
- No branch protection or self-hosted runner success claim.
- No promotion of `hl7v2-python` as the recommended Rust API.
- No return to parser/model/redaction/MLLP implementation microcrates.

## Conclusion

The repo now has executable user-facing guide smokes, shared parity runners,
dirty-corpus parity, machine-readable Python registry proof boundaries, and a
clean separation between product parity blockers and swarm CI-administration
blockers. The immediate product blocker remains public Python registry proof.
Future surface claims should continue to route through the same fixtures and
proof commands rather than creating parallel truth.
