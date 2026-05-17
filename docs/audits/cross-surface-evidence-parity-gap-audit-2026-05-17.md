# Cross-Surface Evidence Parity Gap Audit

Date: 2026-05-17
Base commit: `d17306661eee1c5bd328321c8dfd4061fe9f20ea`

This audit maps the remaining cross-surface evidence parity work after the
v1.5.0 Rust release, the package-boundary closeout, the first-use guides, the
dirty-corpus parity receipts, the Python local wheel proof, and the current
readiness refresh through #721.

This is a gap map. It does not add runtime behavior, does not change support
tiers, and does not claim new crates.io, TestPyPI, PyPI, npm, tag, GitHub
release, or install-back success.

## Source Evidence

| Source | What it proves |
| --- | --- |
| [HL7V2-SPEC-0006](../specs/HL7V2-SPEC-0006-cross-surface-evidence-parity.md) | Accepted parity contract and minimum proof rules. |
| [`policy/evidence-parity.toml`](../../policy/evidence-parity.toml) | Machine-readable current parity surface states, proof commands, fixture families, and known gaps. |
| [Support tier map](../status/SUPPORT_TIERS.md) | Current claim tier and proof command map. |
| [First-use guide](../guides/first-use-by-surface.md) | Current user-facing install and first evidence receipt paths. |
| [User journey acceptance proof](user-journey-acceptance-2026-05-15.md) | Rust, CLI, server, and local Python first-use evidence workflow proof. |
| [Public crates install smoke](public-crates-install-first-use-2026-05-16.md) | crates.io install-back for `hl7v2`, `hl7v2-cli`, and `hl7v2-server` v1.5.0. |
| [Python local wheel proof](python-local-wheel-proof-2026-05-15.md) | Local `hl7v2` wheel build, install, import, smoke, and evidence workflow proof. |
| [Python TestPyPI publish attempt](python-testpypi-publish-attempt-2026-05-17.md) | Publishing-mode wheel smoke passed, then TestPyPI upload failed at `invalid-publisher`. |
| [Dirty real-world shared fixture proof](dirty-real-world-corpus-shared-fixture-proof-2026-05-16.md) | Shared Rust/CLI dirty-corpus fixture categories and corpus parity proof. |
| [Dirty real-world server proof](dirty-real-world-server-corpus-parity-2026-05-16.md) | REST and gRPC dirty-corpus corpus parity proof. |
| [Dirty real-world Python proof](dirty-real-world-python-corpus-parity-2026-05-16.md) | Local Python dirty-corpus corpus parity proof. |
| [gRPC enhanced ACK receipt](grpc-enhanced-ack-parity-2026-05-16.md) | gRPC ACK parity for `AA`, `AE`, `AR`, `CA`, `CE`, and `CR`. |
| [v1.5.0 current-main readiness refresh](publish-dry-run-v1.5.0-2026-05-17-current-main-refresh.md) | Current-main package, policy, evidence, docs, registry, tag, and release-state checks without new uploads. |

## Current Gap Matrix

| Contract | Current proof state | Remaining gap | Next lane |
| --- | --- | --- | --- |
| parse / write | Rust, CLI, REST, gRPC, and local Python parse paths are claimed through existing tests and first-use receipts; write parity is exposed as canonical serialization or normalized output where the surface provides it. | REST and gRPC do not claim a standalone general write endpoint/RPC; public Python registry install-back is absent; TypeScript is unimplemented. | TestPyPI proof first; keep server write claims scoped to exposed endpoints; TypeScript/WASM later. |
| validate | Rust, CLI, REST, gRPC, and local Python validation helpers are covered by support-tier proof commands and local wheel proof. | Public Python registry install-back is absent; no future TypeScript validation surface exists. | TestPyPI proof, then Python public parity receipt. |
| normalize | Rust, CLI, REST/gRPC, and local Python normalization are documented as current parity surfaces. | Public Python registry install-back is absent; TypeScript is unimplemented. | Python public proof, then TypeScript plan. |
| ACK | Rust, gRPC, and local Python proof cover all six supported ACK codes; CLI and REST ACK coverage is useful but remains tied to the commands/endpoints and cases currently exposed in tests. | Public Python registry install-back is absent; TypeScript is unimplemented; any broader CLI/REST ACK code matrix should be proven before it is claimed. | Python public proof; add focused CLI/REST ACK matrix tests only if a future PR broadens those claims. |
| profile lint / explain / test | Rust, CLI, gRPC, and local Python helper proof are recorded; REST claims remain limited to exposed endpoints. | Public Python registry install-back is absent; TypeScript is unimplemented. | Python public proof, then parity acceptance suite. |
| redaction receipt | Rust, CLI, REST, gRPC `ValidateRedacted`, and local Python redaction helper proof exist. | Public Python registry install-back is absent; TypeScript is unimplemented. | Python public proof; keep PHI sentinel checks explicit. |
| quarantine output | REST and gRPC configured quarantine behavior is proved where exposed. | Python quarantine output is not a public package claim; TypeScript is unimplemented. | Do not claim Python quarantine unless a focused helper and smoke proof are added. |
| bundle / replay | Rust, CLI, REST, gRPC, and local Python helper proof exist for bundle/replay semantics; `cargo run -p xtask -- check-bundle-replay-parity` composes the Rust/CLI/REST/gRPC acceptance path. | Public Python registry install-back is absent; TypeScript is unimplemented; Python remains local-wheel proof until registry install-back exists. | Python public proof before promoting Python registry parity; keep the shared runner current as bundle/replay surfaces change. |
| corpus summary / fingerprint / diff | Rust core, CLI, REST, gRPC, and local Python dirty-corpus parity proof share `test_data/dirty-real-world/`; `cargo run -p xtask -- check-dirty-corpus-parity` composes the Rust/CLI/REST/gRPC acceptance path. | TypeScript is unimplemented; dirty-corpus proof is strongest for corpus commands, not every evidence workflow; Python remains local-wheel proof until registry install-back exists. | Extend dirty-corpus proof to validate/redact/bundle/replay only in focused test PRs. |
| safe error shape | Surface-specific tests and specs require safe diagnostics without raw PHI echo, and `cargo run -p xtask -- check-safe-error-phi-parity` now composes the fixture-backed Rust, CLI, REST, and gRPC checks. | Python remains local-wheel smoke until public registry install-back exists; TypeScript is unimplemented. | Keep the runner current as new surfaces claim safe-error parity; add Python public proof after TestPyPI/PyPI receipts. |
| `schema_version` behavior | Evidence schemas and surface-specific tests cover v1/v2 outputs where implemented, and `cargo run -p xtask -- check-schema-version-parity` composes the fixture-backed Rust, CLI, REST, and gRPC checks. | Python remains local-wheel smoke until public registry install-back exists; TypeScript is unimplemented. | Keep the runner current as new surfaces claim schema-version parity; add Python public proof after TestPyPI/PyPI receipts. |
| PHI sentinel behavior | PHI and quarantine sentinels are stable in support tiers, Python/local evidence receipts include PHI-safe checks, and `cargo run -p xtask -- check-safe-error-phi-parity` covers Rust, CLI, REST, and gRPC PHI fixture checks. | Python remains local-wheel smoke until public registry install-back exists; TypeScript is unimplemented. | Keep PHI sentinel proof explicit when adding new artifact families or language surfaces. |
| TypeScript / npm | Package identity is specified as `@effortlessmetrics/hl7v2`. | No npm package, WASM backend, pack/install/import proof, or parity fixtures exist. | Plan npm/WASM only after Python public proof and parity stabilization. |

## Implementation Queue

1. Finish external TestPyPI Trusted Publisher setup for public project `hl7v2`,
   then rerun `Python TestPyPI Proof` with `publish_to_testpypi=true`.
2. Record TestPyPI upload, install-back, `import hl7v2`, `smoke.py`, and
   `evidence_workflow_guide.py` proof before closing #563.
3. Decide production PyPI separately from TestPyPI.
4. Keep [`policy/evidence-parity.toml`](../../policy/evidence-parity.toml)
   current as the machine-readable parity manifest for proof commands, fixture
   families, supported surfaces, and known gaps.
5. Use `cargo run -p xtask -- check-evidence-parity-acceptance` as the default
   local Rust/CLI/REST/gRPC parity acceptance suite; use `--include-python`
   only when a local Python wheel is installed.
6. Keep `cargo run -p xtask -- check-safe-error-phi-parity` as the shared
   safe-error and PHI sentinel runner for Rust, CLI, REST, and gRPC surfaces;
   use `--include-python` only when a local Python wheel is installed.
7. Keep `cargo run -p xtask -- check-schema-version-parity` as the shared
   schema-version runner for Rust, CLI, REST, and gRPC surfaces; use
   `--include-python` only when a local Python wheel is installed.
8. Keep `cargo run -p xtask -- check-bundle-replay-parity` as the shared
   bundle/replay runner for Rust, CLI, REST, and gRPC surfaces; use
   `--include-python` only when a local Python wheel is installed.
9. Keep `cargo run -p xtask -- check-dirty-corpus-parity` as the shared
   dirty-corpus runner for Rust, CLI, REST, and gRPC corpus
   summary/fingerprint/diff proof; use `--include-python` only when a local
   Python wheel is installed.
10. Extend dirty-corpus proof beyond corpus summary/fingerprint/diff only where
   the next surface has a concrete user workflow, such as validate, redaction,
   bundle, or replay.
11. Keep gRPC as Beta until transport lifecycle and operational hardening
   catches up with the artifact semantics already covered for implemented RPCs.
12. Start npm/WASM implementation planning only after Python public proof and
   parity acceptance are stable.

## Boundaries

- No new crates.io upload.
- No new tag or GitHub release.
- No TestPyPI upload or install-back.
- No production PyPI upload or install-back.
- No npm package.
- No TypeScript implementation.
- No new runtime feature claim.
- No promotion of `hl7v2-python` as the recommended Rust API.
- No return to parser/model/redaction/MLLP implementation microcrates.

## Conclusion

The repo now has substantial cross-surface evidence parity, but the remaining
work is not "more readiness." The immediate blocker is public Python registry
proof. After that, the next technical product work should add a shared parity
manifest and acceptance suite so future Rust, CLI, server, Python, and
TypeScript claims are routed through the same fixtures and proof commands.
