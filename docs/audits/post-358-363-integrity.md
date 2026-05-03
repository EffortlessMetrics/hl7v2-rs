# Post-358–363 Integrity Audit

## Scope
This audit covers the rapid succession of repairs and migrations performed in PR #358 through PR #363. The goal is to verify that the transition to **Rust 2024**, **MSRV 1.93**, and the introduction of new features (gRPC, Lifecycle) did not introduce semantic drift or weakened validation.

## Verified Main Baseline
- **CI Pipeline:** PASSED (All 10 jobs on Ubuntu, Windows, macOS).
- **Coverage Pipeline:** PASSED (LLVM-Cov).
- **Security Pipeline:** PASSED (TruffleHog, Semgrep, Trivy, Audit, Deny).
- **Local Gates:** PASSED (Rust 1.93.1).

## Rust 2024 / MSRV 1.93 Alignment
- Workspace correctly declares `edition = "2024"` and `rust-version = "1.93"`.
- Extensive adoption of `let chains` has been verified for correctness.
- `Cargo.lock` is synchronized and stable.

## Parser / MSH / Charset Semantics
- **MSH Handling:** Confirmed that `fields[0]` contains the encoding characters (`MSH-2`).
- **Charset Extraction:** Refactored to `let chains` while preserving indexing logic for `MSH-18` (index 17).
- **Correctness:** No truncation or segment skipping was found in the core parser logic.

## ACK Semantics
- Property tests were successfully modernized with `let chains`.
- Verified that all assertions (`prop_assert_eq!`) remain active and cover critical fields (Control ID, App fields, Version).

## gRPC Build and Service Surface
- **Build Determinism:** `hl7v2-server` now uses `protoc-bin-vendored` for cross-platform stability.
- **Defect Found & Fixed:** Identified and corrected incorrect field indices for `message_type` and `version` in the gRPC metadata extractor.
- **Contract Note:** While the service compiles and metadata extraction is now correct, full contract tests are still pending (scheduled for PR #365).

## Lifecycle Surface
- Verified basic metadata extraction logic.
- Field index for Control ID (`MSH-10`) is correctly set to `8`.
- Crate is currently in **Beta** status pending comprehensive domain transition tests.

## E2E Network Test Stabilization
- Replaced flaky atomic port counters with robust OS-assigned ports (`:0`).
- Implemented `oneshot` channel synchronization between server and client tasks.
- Eliminated hardcoded `sleep` calls, resulting in faster and more reliable CI runs.

## Security Workflow Repair
- TruffleHog configuration corrected to handle `push` and `pull_request` events with appropriate base/head ranges.
- Disallowed license regex adjusted to exclude documentation (`GEMINI.md`) and dependency metadata (`metadata.json`), eliminating false positives.

## Dependency Graph Notes
- Found minor duplication in `thiserror` (1.0 vs 2.0) due to deep dependency stacks in different library families. This is non-blocking but marked for future rationalization.
- PyO3 compatibility flag is now globally applied to ensure consistent bindings builds.

## Required Follow-up PRs
1. **PR #365:** gRPC Contract Coverage (High Priority).
2. **PR #366:** Lifecycle Domain Contracts.
3. **PR #367:** Profile Cache & Guard Proof-of-Stability.
