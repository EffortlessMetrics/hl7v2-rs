# Documentation Index

This directory contains current operating documentation, release receipts, and
historical project records. For live behavior and package-surface truth, start
with the current sources below before reading older planning documents.

## Current Sources

| Need | Start here |
| --- | --- |
| Current release and feature status | [STATUS.md](STATUS.md) |
| Current roadmap and next work | [../ROADMAP.md](../ROADMAP.md) |
| Current blockers and issue map | [ISSUES_AND_NEXT_STEPS.md](ISSUES_AND_NEXT_STEPS.md) |
| Support tiers and proof commands | [status/SUPPORT_TIERS.md](status/SUPPORT_TIERS.md) |
| Contributor workflow | [../CONTRIBUTING.md](../CONTRIBUTING.md), [../DEVELOPMENT.md](../DEVELOPMENT.md) |
| Task-focused evidence workflows | [guides/README.md](guides/README.md) |
| Machine-readable evidence artifacts | [contracts/evidence-contract-index.md](contracts/evidence-contract-index.md), [contracts/evidence-artifact-compatibility-policy.md](contracts/evidence-artifact-compatibility-policy.md) |
| Evidence artifact semantics and provenance | [guides/evidence-artifacts-for-operators.md](guides/evidence-artifacts-for-operators.md), [architecture/evidence-artifacts.md](architecture/evidence-artifacts.md), [architecture/evidence-provenance-versioning.md](architecture/evidence-provenance-versioning.md) |
| JSON schemas | [../schemas/README.md](../schemas/README.md) |
| Current Rust module and package surface | [architecture/module-map.md](architecture/module-map.md) |
| HTTP and gRPC API usage | [API_GUIDE.md](API_GUIDE.md) |
| CI and release validation lanes | [CI_PIPELINE.md](CI_PIPELINE.md) |
| Release process | [../RELEASE_PROCESS.md](../RELEASE_PROCESS.md) |
| Lint, file, and panic-family policies | [CLIPPY_POLICY.md](CLIPPY_POLICY.md), [FILE_POLICY.md](FILE_POLICY.md), [NO_PANIC_POLICY.md](NO_PANIC_POLICY.md) |
| Rust 1.95 / 1.5.0 rollout map | [development/RUST_1_95_ROLLOUT.md](development/RUST_1_95_ROLLOUT.md) |
| Policy allowlist map | [POLICY_ALLOWLISTS.md](POLICY_ALLOWLISTS.md) |

## How Docs Fit Together

Use the smallest document type that owns the claim:

| Document type | Job |
| --- | --- |
| Proposal / PRD | Explain why a campaign exists, who it serves, and what success means. |
| Spec | Define required behavior, contracts, acceptance criteria, and proof. |
| ADR | Record a durable architecture decision and its consequences. |
| Implementation plan | Sequence PR-sized work, rollback, and validation commands. |
| Active goal | Record current agent execution state, blockers, and next work. |
| Status / support | State current product claims and the proof behind them. |
| Policy TOML | Hold exception, CI, lint, file, and package ledgers. |
| Audit / handoff | Preserve what happened, what was validated, and what remains open. |

Start new governance work in [proposals/](proposals/), define durable
requirements in [specs/](specs/), use [adr/](adr/) only for architecture
decisions, sequence execution in a scoped `plans/<milestone>/` directory when
a durable implementation plan is needed, and keep the current active state in
[../.hl7v2/goals/](../.hl7v2/goals/). The historical
[../plans/1.4.1/](../plans/1.4.1/) directory is closed and should not be used
as the default target for new work. Current feature status still lives in
[STATUS.md](STATUS.md); do not duplicate it in proposal, spec, plan, or receipt
documents.

## Evidence Guides

| Guide | Workflow |
| --- | --- |
| [First Use By Surface](guides/first-use-by-surface.md) | Choose the first Rust, CLI, server, or Python receipt without learning the workspace layout. |
| [Full Evidence Receipt Path](guides/full-evidence-receipt-path.md) | Validate, redact, bundle, replay, and interpret one message across the current Rust, CLI, server, and local Python proof paths. |
| [First 10 Minutes](guides/first-10-minutes.md) | Install, diagnose, validate, summarize, bundle, and replay. |
| [Evidence Artifacts For Operators](guides/evidence-artifacts-for-operators.md) | Interpret reports, receipts, bundles, replay output, corpus artifacts, PHI posture, and safe sharing limits. |
| [Operator Error Guidance](guides/operator-error-guidance.md) | Interpret parse, validation, redaction, bundle, replay, server, and Python failures without exposing PHI. |
| [Vendor Upgrade Diff](guides/vendor-upgrade-diff.md) | Compare before/after corpora and interpret drift. |
| [Safe Support Bundle](guides/safe-support-bundle.md) | Redact and package replayable support evidence. |
| [Deploy Validation Sidecar](guides/deploy-validation-sidecar.md) | Run `hl7v2-server` as an edge guard. |
| [Python Evidence Workflow](guides/python-evidence-workflow.md) | Use the Python binding for validation reports, corpus diffs, redaction, bundles, and replay. |
| [Python TestPyPI Release Proof](guides/python-testpypi-release-proof.md) | Prove the separate Python packaging lane without changing the primary Rust product graph. |

## Release And Proof Receipts

| Receipt | Use for |
| --- | --- |
| [v1.4.0 Evidence Contracts release notes](releases/v1.4.0-evidence-contracts.md) | Published release scope and user-facing changes. |
| [v1.4.0 objective audit](audits/v1.4.0-objective-completion-audit.md) | Release-snapshot prompt-to-artifact map for the evidence-layer objective and remaining boundaries. |
| [Final source-tree gap audit](audits/current-source-tree-evidence-objective-gap-audit.md) | Current package-state receipt after the broad local evidence-lane workbench was split and merged. |
| [v1.4.0 publish dry-run receipt](audits/publish-dry-run-v1.4.0-2026-05-09.md) | Package verification before upload. |
| [v1.4.0 publish receipt](audits/publish-v1.4.0-2026-05-09.md) | Dependency-ordered crates.io publication proof. |
| [v1.5.0 Rust 1.95 release notes](releases/v1.5.0-rust-1.95-quality-ratchet.md) | Published Rust 1.95 quality-ratchet release scope; the dedicated publish receipt owns registry proof. |
| [v1.5.0 release readiness](release/1.5.0-readiness.md) | Receipt home for Rust 1.95 / 1.5.0 readiness workflow results. |
| [v1.5.0 publish dry-run receipt](audits/publish-dry-run-v1.5.0-2026-05-13.md) | Non-publishing crates.io dry-run proof for the v1.5.0 Rust graph. |
| [v1.5.0 parity refresh dry-run receipt](audits/publish-dry-run-v1.5.0-2026-05-14-parity-refresh.md) | Current-main non-publishing proof after gRPC corpus summary parity and the cross-surface evidence parity spec. |
| [v1.5.0 corpus refresh dry-run receipt](audits/publish-dry-run-v1.5.0-2026-05-14-corpus-refresh.md) | Current-main non-publishing proof after the first-use guide and dirty real-world corpus proof. |
| [v1.5.0 gRPC status refresh dry-run receipt](audits/publish-dry-run-v1.5.0-2026-05-14-grpc-status-refresh.md) | Current-main non-publishing proof after the gRPC serve-mode status sync. |
| [v1.5.0 gRPC bundle refresh dry-run receipt](audits/publish-dry-run-v1.5.0-2026-05-15-grpc-bundle-refresh.md) | Current-main non-publishing proof after gRPC configured-root evidence bundle creation parity. |
| [v1.5.0 package registry state audit](audits/package-registry-state-2026-05-15.md) | Live pre-release registry state after the selected v1.5.0 graph and latest readiness refresh. |
| [v1.5.0 objective completion audit](audits/v1.5.0-objective-completion-audit-2026-05-15.md) | Prompt-to-artifact map for the active lane, including completed evidence and remaining release/Python/npm blockers. |
| [v1.5.0 post-release objective audit](audits/v1.5.0-objective-completion-audit-2026-05-15-post-release.md) | Current-state prompt-to-artifact map after the crates.io release, tag, GitHub release, local Python wheel proof, and hosted non-publishing Python workflow proof. |
| [v1.5.0 post-SRP readiness refresh](audits/publish-dry-run-v1.5.0-2026-05-16-post-srp-refresh.md) | Current-main post-release package, policy, evidence, and docs proof after the focused SRP module split train. |
| [v1.5.0 dirty-corpus parity readiness refresh](audits/publish-dry-run-v1.5.0-2026-05-16-dirty-corpus-parity-refresh.md) | Current-main post-release package, policy, evidence, and docs proof after shared Rust/CLI/server/Python dirty-corpus parity. |
| [v1.5.0 dirty-corpus evidence workflow readiness refresh](audits/publish-dry-run-v1.5.0-2026-05-17-dirty-corpus-evidence-workflow-refresh.md) | Current-main non-publishing proof after REST and gRPC dirty validate-redacted/bundle/replay workflows landed through #734. |
| [v1.5.0 refactor cleanup readiness refresh](audits/publish-dry-run-v1.5.0-2026-05-17-refactor-refresh.md) | Current-main non-publishing proof after the Python wheel dirty evidence smoke, Windows policy guard, server validation-helper refactor, and synthetic value/CLI help cleanup landed through #740/#742/#743/#738. |
| [v1.5.0 normalization and ACK readiness refresh](audits/publish-dry-run-v1.5.0-2026-05-16-normalization-ack-refresh.md) | Current-main post-release package, policy, evidence, and docs proof after normalization and CLI ACK parity. |
| [v1.5.0 gRPC enhanced ACK readiness refresh](audits/publish-dry-run-v1.5.0-2026-05-16-grpc-enhanced-ack-refresh.md) | Current-main post-release package, policy, evidence, and docs proof after gRPC `GenerateAck` parity for all six supported ACK codes. |
| [Dirty real-world corpus proof](audits/real-world-corpus-proof-2026-05-14.md) | Focused proof that core corpus summary, fingerprint, and diff handle Z-segments, odd MSH fields, MLLP bytes, large OBX messages, malformed delimiters, and safe parse-error output. |
| [Dirty real-world shared fixture proof](audits/dirty-real-world-corpus-shared-fixture-proof-2026-05-16.md) | Shared fixture categories and Rust/CLI corpus parity proof for dirty interface data. |
| [Dirty real-world server corpus parity proof](audits/dirty-real-world-server-corpus-parity-2026-05-16.md) | REST and gRPC corpus parity proof using the shared dirty real-world fixture categories. |
| [Dirty real-world Python corpus parity proof](audits/dirty-real-world-python-corpus-parity-2026-05-16.md) | Local Python wheel corpus parity proof using the shared dirty real-world fixture categories. |
| [Dirty real-world Python evidence workflow proof](audits/dirty-real-world-python-evidence-workflow-2026-05-17.md) | Local Python wheel validate/redact/bundle/replay proof using the shared dirty Z-segment fixture. |
| [Dirty real-world odd MSH metadata parity proof](audits/dirty-real-world-odd-msh-parity-2026-05-18.md) | Rust, CLI, REST, and gRPC corpus parity proof for componentized MSH sender/receiver metadata. |
| [Dirty real-world vendor ORU null/text parity proof](audits/dirty-real-world-vendor-oru-null-text-parity-2026-05-18.md) | Rust, CLI, REST, gRPC, and local Python corpus parity proof for narrative ORU text, escaped delimiters, NTE notes, and explicit HL7 null observations. |
| [User journey acceptance proof](audits/user-journey-acceptance-2026-05-15.md) | First-use acceptance map for Rust, CLI, server, and Python evidence workflows. |
| [First use by surface guide smoke](audits/first-use-by-surface-guide-smoke-2026-05-18.md) | Executable `xtask check-first-use-by-surface-guide` proof for the Rust, CLI, and server first-use routing guide. |
| [First 10 Minutes guide smoke](audits/first-10-minutes-guide-smoke-2026-05-18.md) | Executable `xtask check-first-10-minutes-guide` proof for the job-first CLI onboarding guide. |
| [First-use guide smoke](audits/first-use-guide-smoke-2026-05-18.md) | Executable `xtask check-first-use-guides` proof for the documented full evidence receipt path. |
| [Vendor upgrade diff guide smoke](audits/vendor-upgrade-diff-guide-smoke-2026-05-18.md) | Executable `xtask check-vendor-upgrade-diff-guide` proof for the before/after corpus drift guide. |
| [Operator error guidance guide smoke](audits/operator-error-guidance-guide-smoke-2026-05-18.md) | Executable `xtask check-operator-error-guidance-guide` proof for representative REST and CLI safe-failure guidance. |
| [Safe support-bundle guide smoke](audits/safe-support-bundle-guide-smoke-2026-05-18.md) | Executable `xtask check-safe-support-bundle-guide` proof for the operator support packet recipe. |
| [Support bundle safe-sharing checklist receipt](audits/support-bundle-safe-sharing-checklist-2026-05-18.md) | CLI, core/Python, REST, and gRPC bundle proof for generated `SAFE-SHARING.md` checklists and legacy replay compatibility. |
| [Sidecar guide smoke](audits/sidecar-guide-smoke-2026-05-18.md) | Executable `xtask check-sidecar-guide` proof for the HTTP deployment sidecar guide. |
| [Evidence artifact guide smoke](audits/evidence-artifacts-guide-smoke-2026-05-18.md) | Executable `xtask check-evidence-artifacts-guide` proof for the operator artifact interpretation guide. |
| [Public crates install and first-use smoke](audits/public-crates-install-first-use-2026-05-16.md) | Repeatable crates.io install-back smoke for `hl7v2`, `hl7v2-cli`, and `hl7v2-server` v1.5.0 first-use paths. |
| [gRPC enhanced ACK parity receipt](audits/grpc-enhanced-ack-parity-2026-05-16.md) | gRPC `GenerateAck` parity for all six supported ACK codes: `AA`, `AE`, `AR`, `CA`, `CE`, and `CR`. |
| [Cross-surface evidence parity spec](specs/HL7V2-SPEC-0006-cross-surface-evidence-parity.md) | Contract map for Rust, CLI, REST/gRPC, Python, and future TypeScript evidence semantics. |
| [Cross-surface evidence parity manifest](../policy/evidence-parity.toml) | Machine-readable current parity states, proof commands, fixture families, and known gaps. |
| [Cross-surface evidence parity gap audit](audits/cross-surface-evidence-parity-gap-audit-2026-05-20.md) | Current implementation gap map after guide smokes, dirty-corpus parity, Python registry proof-command routing, the blocked public-registry proof boundary, and the swarm runner administration boundary. |
| [Python local wheel proof](audits/python-local-wheel-proof-2026-05-15.md) | Current-main non-publishing wheel build, fresh-venv install, import smoke, and Python evidence workflow proof for `hl7v2`. |
| [Python local wheel proof command](audits/python-local-wheel-proof-command-2026-05-17.md) | Self-contained `xtask python-local-wheel-proof` receipt proving local wheel build/install/import, Python evidence helpers, and Python-included parity acceptance. |
| [Python public registry proof command](audits/python-public-registry-proof-command-2026-05-18.md) | `xtask python-public-registry-proof` command surface for TestPyPI/PyPI install-back reproduction after public upload succeeds; not a registry success receipt by itself. |
| [Python Trusted Publisher diagnostics](audits/python-trusted-publisher-diagnostics-2026-05-19.md) | Current GitHub environment and public registry visibility check after the workflow OIDC claim diagnostic rail landed; not a TestPyPI/PyPI upload or install-back receipt. |
| [Python TestPyPI OIDC diagnostic](audits/python-testpypi-oidc-diagnostic-2026-05-19.md) | Hosted no-upload diagnostic run proving the GitHub `testpypi` OIDC identity matches the expected Trusted Publisher subject; not a TestPyPI upload or install-back receipt. |
| [Python TestPyPI non-publish proof](audits/python-testpypi-nonpublish-proof-2026-05-16.md) | Current hosted non-publishing Python wheel, import, smoke, and evidence workflow proof for public package `hl7v2`. |
| [Python TestPyPI publish attempt refresh](audits/python-testpypi-publish-attempt-2026-05-17.md) | Latest publishing-mode proof attempt on current `main`; wheel smoke passed, upload is still blocked by TestPyPI Trusted Publishing setup. |
| [Python TestPyPI publish attempt](audits/python-testpypi-publish-attempt-2026-05-10.md) | Publishing-mode proof attempt; wheel smoke passed, upload is blocked by TestPyPI Trusted Publishing setup. |

## Current Boundaries

- `docs/STATUS.md` is the current-state source of truth.
- `Cargo.toml`, `rust-toolchain.toml`, `clippy.toml`, and
  `policy/clippy-lints.toml` own the active Rust 1.95 MSRV/toolchain state.
  `docs/development/RUST_1_95_ROLLOUT.md` remains the current rollout map for
  the remaining 1.5.0 quality-ratchet lane.
- The v1.4.0 objective audit is a release-snapshot receipt, not proof that the
  full long-range evidence-layer objective is finished.
- The public Python distribution is `hl7v2`, built from the `hl7v2-python`
  binding backend lane, until a production PyPI release is intentionally proven
  and executed.
- gRPC coverage is useful but still narrower than the full HTTP evidence
  surface. It now includes inline corpus summary, fingerprint, diff,
  configured-root evidence bundle creation and replay parity, and configured
  quarantine output through `ValidateRedacted`.
  Use
  [`HL7V2-SPEC-0006`](specs/HL7V2-SPEC-0006-cross-surface-evidence-parity.md)
  for the cross-surface parity contract,
  [`policy/evidence-parity.toml`](../policy/evidence-parity.toml) for current
  parity proof state, and use `docs/API_GUIDE.md` and `docs/STATUS.md` for
  current endpoint claims.

## Historical Documents

These documents are retained for traceability. They should not override the
current package surface, module map, status document, evidence schemas, or
guides. Some historical receipts preserve links or paths to retired crate
folders as evidence of the state they recorded; use the current sources above
for live navigation.

| Historical document | Use for |
| --- | --- |
| [TESTING_ARCHITECTURE.md](TESTING_ARCHITECTURE.md) | Historical testing architecture narrative. Examples are updated where practical, but the rollout story predates the crate collapse. |
| [TESTING_ANALYSIS.md](TESTING_ANALYSIS.md) and [TESTING_SUMMARY.md](TESTING_SUMMARY.md) | Dated testing snapshots from the former microcrate topology. |
| [MICROCRATE_ANALYSIS.md](MICROCRATE_ANALYSIS.md) | Historical analysis of the retired microcrate structure. |
| [TASK_COMPLETION_SUMMARY.md](TASK_COMPLETION_SUMMARY.md) | Earlier documentation alignment receipt. |
| [../TESTING.md](../TESTING.md) | Historical root testing guide; current gates live in `DEVELOPMENT.md` and `docs/CI_PIPELINE.md`. |
| [../SESSION_SUMMARY.md](../SESSION_SUMMARY.md) | Historical session receipt from 2025-11-19. |
| [plans/](plans/) and [audits/](audits/) | Historical plans and verification receipts. |

## Package Surface

The current primary Rust product surface is:

- `hl7v2`
- `hl7v2-server`
- `hl7v2-cli`

The public Python distribution is `hl7v2`, built from the `hl7v2-python`
maturin backend lane. `hl7v2-python` is a binding backend crate, not the
recommended Rust API. Historical old microcrate names may exist on crates.io,
but they are not the product surface for new code.
