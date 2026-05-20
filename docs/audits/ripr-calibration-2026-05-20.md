# RIPR Calibration Audit - 2026-05-20

This audit refreshes the advisory `ripr` static mutation-exposure calibration
after the dirty-corpus, CLI/gRPC limit, and evidence-parity support-map PR
traffic on 2026-05-20. It is a signal-quality receipt only. It is not a
branch-protection change, runtime mutation result, release readiness refresh,
crates.io publish receipt, TestPyPI receipt, PyPI receipt, npm receipt, tag, or
GitHub release receipt.

## Scope

| Field | Value |
| --- | --- |
| Repository line | post-v1.5.0 dirty-corpus and evidence-parity guard traffic |
| Base checkout | `e0301226506cdf26dad4f991ec53efe6dff8e851` |
| Lane | Advisory `ripr` static mutation-exposure |
| Workflow | `.github/workflows/ripr.yml` |
| Artifact name | `ripr-pr-evidence` |
| Policy ledger | `policy/ripr-suppressions.toml` |
| Runtime mutation relationship | Targeted backstop, not replaced by `ripr` |

## Sampled Traffic

| Pull request | Workflow run | Observed signal |
| --- | --- | --- |
| #848, CLI/gRPC max body limit proof | `26156923501` | Hosted `ripr` passed. The PR changed two files, produced 10 summary-only recommendations, reported zero severe gaps, and did not route targeted mutation. The summary-only notes were review hints, while the max-body behavior remained owned by targeted CLI/gRPC tests. |
| #849, dirty legacy timestamp proof | `26159546424` | Hosted `ripr` passed. The PR changed nine files, produced 10 summary-only recommendations, reported zero severe gaps, and did not route targeted mutation. This was useful advisory signal for fixture and corpus-stat changes without becoming a release blocker. |
| #850, redacted support bundle corpus proof | `26162641373` | Hosted `ripr` passed. The PR changed 13 files, produced 10 summary-only recommendations, reported zero severe gaps, and did not route targeted mutation. The large summary-only artifact reinforced that dirty-corpus fixture expansion still needs human review, not an automatic block. |
| #852, evidence parity support-map guard | `26166665858` | Hosted `ripr` passed. The PR changed two files, produced zero summary-only recommendations, reported zero severe gaps, and did not route targeted mutation. This was useful negative evidence for a focused guard/doc-link repair. |

## Cost And Latency Sample

| Pull request | Workflow run | Workflow elapsed | `ripr` job elapsed | Install `ripr` | Advisory evidence step |
| --- | --- | ---: | ---: | ---: | ---: |
| #848, CLI/gRPC max body limit proof | `26156923501` | 1m57s | 1m52s | 1m13s | 29s |
| #849, dirty legacy timestamp proof | `26159546424` | 1m58s | 1m54s | 1m13s | 31s |
| #850, redacted support bundle corpus proof | `26162641373` | 2m08s | 2m02s | 1m18s | 34s |
| #852, evidence parity support-map guard | `26166665858` | 2m02s | 1m57s | 1m18s | 31s |

Observed envelope for this sample:

- workflow elapsed: 1m57s-2m08s;
- `ripr` job elapsed: 1m52s-2m02s;
- advisory evidence step: 29s-34s;
- most elapsed time is still tool installation, not analysis.

This remains acceptable for an advisory PR-time lane. It is still not enough
data to make the lane branch-protection blocking, to promote severe-gap output
to a required status, or to add broad runtime mutation to ordinary PRs.

## Calibration Findings

The lane remains useful as advisory review evidence:

- Recent dirty-corpus PRs produced zero severe gaps but did surface summary-only
  recommendations around static evidence boundaries. That is useful reviewer
  context for fixture, corpus-stat, and support-bundle changes.
- The same dirty-corpus PRs show why `ripr` should stay advisory. Fixture-count
  and receipt updates can produce many recommendations even when the real proof
  belongs to targeted corpus, CLI, REST, gRPC, and Python-local smoke tests.
- The support-map guard PR produced no recommendations, which is useful
  negative evidence for a narrow documentation/xtask guard update.
- None of the sampled runs set `requires_targeted_mutation` or
  `ripr_severe_gap`. No suppression was needed.

The lane is not ready to become branch-protection blocking:

- The sample is still skewed toward proof, fixture, and guard PRs.
- Summary-only recommendations can be noisy on fixture-expansion diffs because
  the changed lines are often generated counts or receipt text rather than
  executable seams.
- `ripr` still does not execute mutation. `impacted-evidence` routes work; it
  does not prove the runtime mutation backstop passed.

## Current Decision

Keep `ripr` advisory:

- run it on relevant PRs;
- publish JSON, Markdown, annotation, badge, review, and impacted-evidence
  artifacts;
- use severe-gap, annotation, summary-only, and impacted-evidence output as
  reviewer and mutation-routing input;
- keep runtime mutation as the targeted or release-time backstop;
- do not add branch-protection requirements for `ripr` yet.

## Follow-Ups

- Keep collecting hosted samples before deciding whether any `ripr` severity,
  annotation, or routing result can become blocking.
- Add calibration metrics or dashboards only after artifact semantics and
  counter meanings are stable enough that the numbers are not misleading.
- Do not promote `ripr` from advisory to soft-gate or required until at least
  25 hosted PR samples show stable low-noise routing, targeted-mutation
  escalation correlates with meaningful review or test risk, p95 workflow
  elapsed stays below 3 minutes under normal cache behavior, and the workflow
  still avoids broad runtime mutation on ordinary PRs.
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
