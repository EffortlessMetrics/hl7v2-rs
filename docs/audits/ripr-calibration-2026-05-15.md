# RIPR Calibration Audit - 2026-05-15

This audit calibrates the advisory `ripr` static mutation-exposure lane after
real pull request traffic. It is an evidence-usefulness receipt only. It is not
a branch-protection change, runtime mutation result, release readiness refresh,
crates.io publish receipt, TestPyPI receipt, PyPI receipt, npm receipt, tag, or
GitHub release receipt.

## Scope

| Field | Value |
| --- | --- |
| Repository line | v1.5.0 release candidate |
| Lane | Advisory `ripr` static mutation-exposure |
| Workflow | `.github/workflows/ripr.yml` |
| Artifact name | `ripr-pr-evidence` |
| Policy ledger | `policy/ripr-suppressions.toml` |
| Runtime mutation relationship | Targeted backstop, not replaced by `ripr` |

## Sampled Traffic

| Pull request | Workflow run | Observed signal |
| --- | --- | --- |
| #618, RIPR evidence surface | `25878290885` | Broad self-hosting/config signal: 116 `weakly_exposed`, 135 `reachable_unrevealed`, 689 `no_static_path`, 940 severe gaps, and `requires_targeted_mutation = true`. Useful for routing and review, too broad for blocking. |
| #629, gRPC corpus fingerprint parity | `25894522063` | Focused review guidance with 3 changed-line comments, 6 summary-only recommendations, 1 generated suppression, zero severe gaps, and `fast_only` impacted-evidence routing. |
| #630, gRPC corpus diff parity | `25895990745` | Focused review guidance with 3 changed-line comments, 3 summary-only recommendations, 4 generated suppressions, zero severe gaps, and `fast_only` impacted-evidence routing. |
| #633, finite numeric validation fix | `25899168227` | Focused review guidance with 2 changed-line comments, 1 summary-only recommendation, 7 generated suppressions, zero severe gaps, and `fast_only` impacted-evidence routing. |

## Calibration Findings

The lane is useful as advisory review evidence:

- #618 showed that broad verification/config changes can produce a severe-gap
  signal and route targeted mutation.
- #629, #630, and #633 produced review hints without escalating to targeted
  mutation.
- `impacted-evidence` kept ordinary focused code changes on the `fast_only`
  route when severe static gaps were zero.
- `policy/ripr-suppressions.toml` still has no active suppressions.

The lane is not ready to become branch-protection blocking:

- The initial self-hosting/config PR produced a large severe-gap count. That is
  useful for routing, but too broad for a required status without more traffic.
- The sampled artifacts show that `repo-exposure.json` summary counters and the
  generated review Markdown are not interchangeable. In #629, #630, and #633,
  Markdown review guidance existed while the repository-exposure JSON summary
  counters for comments and summary guidance were zero. Until those counter
  semantics are aligned or documented, gates must not use one artifact alone to
  claim there were no recommendations.
- The #633 finite numeric fix was ultimately caught by tests and CI. `ripr`
  provided review hints but did not route targeted mutation, which is correct
  for an advisory static signal with zero severe static gaps.

## Current Decision

Keep `ripr` advisory:

- run it on relevant PRs;
- publish the JSON, Markdown, annotation, badge, and impacted-evidence
  artifacts;
- use severe-gap and impacted-evidence signals as reviewer and mutation-routing
  input;
- keep runtime mutation as the targeted or release-time backstop;
- do not add branch-protection requirements for `ripr` yet.

## Follow-Ups

- Align or document the counter semantics between `repo-exposure.json`,
  `summary.md`, and `comments.md`.
- Keep collecting hosted samples before deciding whether any `ripr` severity or
  routing result can become blocking.
- Track cost and latency once there is enough PR traffic to calculate a useful
  envelope.
- Add dashboard or calibration metrics only after the artifact semantics are
  stable enough to avoid misleading pass/fail claims.

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
| `python -c "import tomllib; tomllib.load(open('.hl7v2/goals/active.toml','rb'))"` | pass |
| `cargo +1.95.0 run -p xtask -- badges --check` | pass |
| `cargo +1.95.0 run -p xtask -- impacted-evidence --check` | pass |
| `cargo +1.95.0 run -p xtask -- check-doc-links` | pass; 161 Markdown files and 376 local links checked |
| `cargo +1.95.0 run -p xtask -- check-file-policy` | pass; 520 tracked/untracked non-ignored files checked |
| `git diff --check` | pass |
