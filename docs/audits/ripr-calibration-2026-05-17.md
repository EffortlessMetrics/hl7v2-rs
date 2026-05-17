# RIPR Calibration Audit - 2026-05-17

This audit refreshes the advisory `ripr` static mutation-exposure calibration
after the evidence-parity and Python-proof PR traffic on 2026-05-17. It is a
signal-quality receipt only. It is not a branch-protection change, runtime
mutation result, release readiness refresh, crates.io publish receipt,
TestPyPI receipt, PyPI receipt, npm receipt, tag, or GitHub release receipt.

## Scope

| Field | Value |
| --- | --- |
| Repository line | post-v1.5.0 evidence-parity and Python-proof traffic |
| Base checkout | `9086b7c773016e6c32e5cf6baf24487ecd7a7a64` |
| Lane | Advisory `ripr` static mutation-exposure |
| Workflow | `.github/workflows/ripr.yml` |
| Artifact name | `ripr-pr-evidence` |
| Policy ledger | `policy/ripr-suppressions.toml` |
| Runtime mutation relationship | Targeted backstop, not replaced by `ripr` |

## Sampled Traffic

| Pull request | Workflow run | Observed signal |
| --- | --- | --- |
| #731, evidence parity acceptance runner | `25990753620` | One `no_static_path` severe gap, zero review comments, and `targeted` impacted-evidence routing. Useful as a routing signal for new orchestrator code, but not enough by itself to require runtime mutation on every similar PR. |
| #737, Python dirty evidence workflow smoke | `25997492068` | Zero severe gaps, zero review comments, and `fast_only` impacted-evidence routing across 20 changed files. Good evidence that normal test-heavy Python smoke additions do not automatically create a mutation tax. |
| #738, Python wheel dirty smoke workflow | `25998379659` | Zero severe gaps, zero review comments, and `fast_only` impacted-evidence routing across 3 changed files. Good advisory behavior for narrow CI/Python-smoke wiring. |
| #743, Python Wheels policy CRLF guard | `25998977833` | Zero severe gaps, zero review comments, and `fast_only` impacted-evidence routing for a one-file xtask policy fix. |
| #742, server validation helper refactor | `25999837574` | Fifteen `no_static_path` severe gaps, 3 line annotations, 7 summary-only recommendations, and `targeted` impacted-evidence routing. Useful for catching refactor seams in server validation plumbing. |
| #740, duplicated value generation refactor | `26000683148` | Zero severe gaps but 3 line annotations and 7 summary-only recommendations. `impacted-evidence` stayed `fast_only`, which is the right split: useful reviewer guidance without mutation escalation. |

The artifact sample was downloaded locally with `gh run download ... -n
ripr-pr-evidence` into a temporary directory and inspected from
`repo-exposure.json`, `summary.md`, `comments.json`, `comments.md`, and
`target/xtask/impacted-evidence/latest.md`.

## Cost And Latency Sample

The hosted lane was also sampled for runtime cost because a useful static
signal can still become an expensive default PR tax if it is slow enough at
industrialized volume.

| Pull request | Workflow run | Workflow elapsed | `ripr` job elapsed | Install `ripr` | Advisory evidence step |
| --- | --- | ---: | ---: | ---: | ---: |
| #731, evidence parity acceptance runner | `25990753620` | 1m48s | 1m46s | 1m12s | 28s |
| #737, Python dirty evidence workflow smoke | `25997492068` | 1m54s | 1m51s | 1m14s | 30s |
| #738, Python wheel dirty smoke workflow | `25998379659` | 1m53s | 1m51s | 1m15s | 29s |
| #743, Python Wheels policy CRLF guard | `25998977833` | 1m52s | 1m48s | 1m14s | 29s |
| #742, server validation helper refactor | `25999837574` | 1m54s | 1m51s | 1m15s | 30s |
| #740, duplicated value generation refactor | `26000683148` | 1m51s | 1m48s | 1m12s | 31s |
| #748, FTS count validation and profile-loader stabilization | `26004231616` | 1m54s | 1m50s | 1m13s | 28s |
| #747, ACK and batch unit coverage | `26004773809` | 1m56s | 1m53s | 1m16s | 31s |
| #751, profile loader cache coverage | `26005082252` | 1m55s | 1m53s | 1m13s | 29s |

Observed envelope for this sample:

- workflow elapsed: p50 1m54s, max 1m56s;
- `ripr` job elapsed: p50 1m51s, max 1m53s;
- advisory evidence step: p50 29s, max 31s;
- queue-to-start delay: 0s-3s in the sampled runs;
- most elapsed time is tool installation, not analysis.

This is acceptable for an advisory PR-time lane today, but it is not yet enough
evidence to make the lane branch-protection blocking. A future optimization
should focus on installation/cache behavior before adding default CI weight.

## Calibration Findings

The lane is still useful as advisory review evidence:

- Severe-gap routing is not merely theoretical. #731 and #742 both routed to
  `targeted` impacted evidence when `ripr_severe_gap = true`.
- Ordinary Python smoke, CI policy, and test-heavy parity PRs stayed on
  `fast_only` when severe static gaps were zero.
- Review guidance can be useful even when mutation routing stays fast-only.
  #740 produced line and summary guidance while leaving
  `requires_targeted_mutation = false`.
- `policy/ripr-suppressions.toml` did not need an active suppression for this
  sample. The sampled recommendations should stay review input, not source
  suppressions.

The lane is not ready to become branch-protection blocking:

- The same class of focused PR traffic can produce very different advisory
  shapes: zero severe gaps for CI/Python smoke changes, targeted routing for
  server refactor seams, and guidance-only output for helper refactors.
- `repo-exposure.json` and `comments.json` still represent different views.
  In #742, `repo-exposure.json` recorded zero `comments` and zero
  `summary_only`, while the review guidance artifact recorded 3 line comments
  and 7 summary-only recommendations. Gates must not use one artifact alone to
  claim there was no reviewer guidance.
- `ripr` still does not execute mutation. `impacted-evidence` routes work; it
  does not prove the runtime mutation backstop passed.

## Current Decision

Keep `ripr` advisory:

- run it on relevant PRs;
- publish the JSON, Markdown, annotation, badge, and impacted-evidence
  artifacts;
- use severe-gap and impacted-evidence signals as reviewer and
  mutation-routing input;
- keep runtime mutation as the targeted or release-time backstop;
- do not add branch-protection requirements for `ripr` yet.

## Follow-Ups

- Align or document the counter semantics between `repo-exposure.json`,
  `comments.json`, `summary.md`, and `comments.md`.
- Keep collecting hosted samples before deciding whether any `ripr` severity or
  routing result can become blocking.
- Do not promote `ripr` from advisory to soft-gate or required until all of the
  following are true: artifact counter semantics are aligned or explicitly
  specified; at least 25 hosted PR samples show stable low-noise routing;
  targeted-mutation escalation correlates with meaningful review or test risk;
  p95 workflow elapsed remains below 3 minutes after normal cache behavior; and
  the workflow does not add broad runtime mutation to ordinary PRs.
- Track whether caching or a pinned prebuilt tool path can reduce the current
  install-dominated runtime before considering stricter enforcement.
- Consider a focused test follow-up for server validation helper seams if the
  #742 guidance maps to meaningful untested behavior.

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
| `python -c "import pathlib,tomllib; tomllib.loads(pathlib.Path('.hl7v2/goals/active.toml').read_text())"` | pass |
| `cargo +1.95.0 run -p xtask -- badges --check` | pass |
| `cargo +1.95.0 run -p xtask -- impacted-evidence` | pass; generated the local impacted-evidence receipt for `--check` |
| `cargo +1.95.0 run -p xtask -- impacted-evidence --check` | pass |
| `cargo +1.95.0 run -p xtask -- check-doc-links` | pass; 201 Markdown files and 568 local links checked |
| `cargo +1.95.0 run -p xtask -- check-file-policy` | pass; 618 tracked/untracked non-ignored files checked |
| `git diff --check` | pass |
