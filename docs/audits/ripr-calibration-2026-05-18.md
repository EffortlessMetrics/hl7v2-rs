# RIPR Calibration Audit - 2026-05-18

This audit refreshes the advisory `ripr` static mutation-exposure calibration
after the hosted PR traffic for #765 and #766. It is a signal-quality receipt
only. It is not a branch-protection change, runtime mutation result, release
readiness refresh, crates.io publish receipt, TestPyPI receipt, PyPI receipt,
npm receipt, tag, or GitHub release receipt.

## Scope

| Field | Value |
| --- | --- |
| Repository line | post-v1.5.0 Python proof and CLI support-bundle traffic |
| Base checkout | `cd7226c774e0950c086d656f56fef0001685c786` |
| Lane | Advisory `ripr` static mutation-exposure |
| Workflow | `.github/workflows/ripr.yml` |
| Artifact name | `ripr-pr-evidence` |
| Policy ledger | `policy/ripr-suppressions.toml` |
| Runtime mutation relationship | Targeted backstop, not replaced by `ripr` |

## Sampled Traffic

| Pull request | Workflow run | Observed signal |
| --- | --- | --- |
| #765, Python parity proof guard test portability | `26012095682` | Hosted `ripr` passed on a narrow Windows-newline test fix. This was useful negative evidence: a one-test portability repair did not route extra mutation work or require suppressions. |
| #766, CLI support-bundle replay script fix | `26013515211` | Hosted `ripr` found checked-in repository artifact drift: `badges/ripr.json` was stale and needed regeneration. This was a tooling freshness failure, not evidence that the support-bundle behavior was unsafe. |
| #766, CLI support-bundle replay script fix after badge refresh | `26013813028` | Hosted `ripr` passed after `badges/ripr.json` was regenerated. Local RIPR, review-comment, summary, annotation, and impacted-evidence generation/checks also passed. Advisory annotations still highlighted changed replay-command/script seams, but the targeted CLI tests and PHI-sentinel assertions owned the behavioral proof. |

## Cost And Latency Sample

| Pull request | Workflow run | Workflow elapsed | `ripr` job elapsed | Install `ripr` | Advisory evidence step |
| --- | --- | ---: | ---: | ---: | ---: |
| #765, Python parity proof guard test portability | `26012095682` | 1m54s | 1m51s | 1m13s | 30s |
| #766, support-bundle replay scripts, stale badge | `26013515211` | 1m43s | 1m40s | 1m15s | 18s before failure |
| #766, support-bundle replay scripts, refreshed badge | `26013813028` | 1m59s | 1m56s | 1m13s | 30s |

Observed envelope for this small sample:

- workflow elapsed: 1m43s-1m59s;
- `ripr` job elapsed: 1m40s-1m56s;
- advisory evidence step: 18s-30s;
- most elapsed time is still tool installation, not analysis.

This is acceptable for an advisory PR-time lane. It is still not enough data to
make the lane branch-protection blocking or to add broad runtime mutation to
ordinary PRs.

## Calibration Findings

The lane remains useful as advisory review evidence:

- The stale badge failure in #766 caught checked-in artifact drift before merge.
  That is a real contract-freshness signal for the repo's evidence surfaces.
- The same #766 traffic also showed the boundary: a generated badge mismatch
  is not itself a product-behavior defect. The fix was to regenerate the
  checked-in badge artifact, then rely on targeted CLI tests for replay-script
  behavior and PHI-sentinel coverage.
- Narrow portability and test-guard fixes, such as #765, stayed cheap and did
  not create new suppression pressure.
- Advisory annotations over changed replay-command/script seams were useful
  reviewer input, but they were not release-blocking and did not replace
  targeted tests.

The lane is not ready to become branch-protection blocking:

- The sample is still small and skewed toward narrow PRs.
- The stale-badge failure mode is important, but it is a generated-artifact
  freshness gate rather than static proof that changed product code is weakly
  exposed.
- `ripr` still does not execute mutation. `impacted-evidence` routes work; it
  does not prove the runtime mutation backstop passed.

## Current Decision

Keep `ripr` advisory:

- run it on relevant PRs;
- publish JSON, Markdown, annotation, badge, review, and impacted-evidence
  artifacts;
- use stale artifact failures as repository evidence-drift fixes;
- use severe-gap, annotation, and impacted-evidence signals as reviewer and
  mutation-routing input;
- keep runtime mutation as the targeted or release-time backstop;
- do not add branch-protection requirements for `ripr` yet.

## Follow-Ups

- Document or automate when `badges/ripr.json` must be regenerated so future
  PRs see the stale-badge signal as actionable artifact drift, not a confusing
  product failure.
- Keep collecting hosted samples before deciding whether any `ripr` severity,
  annotation, or routing result can become blocking.
- Do not promote `ripr` from advisory to soft-gate or required until artifact
  semantics are specified, at least 25 hosted PR samples show stable low-noise
  routing, targeted-mutation escalation correlates with meaningful review or
  test risk, p95 workflow elapsed stays below 3 minutes under normal cache
  behavior, and the workflow still avoids broad runtime mutation on ordinary
  PRs.
- Continue focusing performance improvements on installation/cache behavior
  before adding any default CI weight.

## Non-Claims

- No branch-protection rule was changed.
- No required check was added.
- No runtime mutation was run by this audit.
- No `ripr` finding was treated as release-blocking.
- No crates.io, TestPyPI, PyPI, npm, tag, or GitHub release action was run.

## Validation

This audit PR was validated with:

| Command | Result |
| --- | --- |
| `cargo +1.95.0 run -p xtask -- badges --check` | pass |
| `cargo +1.95.0 run -p xtask -- impacted-evidence` | pass; generated the local impacted-evidence receipt for `--check` |
| `cargo +1.95.0 run -p xtask -- impacted-evidence --check` | pass |
| `cargo +1.95.0 run -p xtask -- check-doc-links` | pass |
| `cargo +1.95.0 run -p xtask -- check-file-policy` | pass |
| `git diff --check` | pass |
