# Branch Protection

## Current State

For `EffortlessMetrics/hl7v2-rs`, live GitHub branch protection on `main`
currently requires one status check from `ci.yml`:

- `Fast Checks`

That keeps the source/release mirror mergeable while the broader frontdoor and
swarm routing surfaces burn in. It also means this document should not be used
as proof that `Standard Tests`, `Matrix Tests`, `CI Success`, or `PR Gate
Success` are currently required by branch protection.

For `EffortlessMetrics/hl7v2-rs-swarm`, branch protection is intentionally not
enabled yet. The swarm cutover plan is governed in the swarm repository by
`docs/ops/swarm-development.md`: enable branch protection only after CX53, CX43
fallback, and GitHub-hosted fallback are all proven, and require only
`HL7v2 Rust Small Result`.

The source-repo coupling risk remains:

1. Adding a new required check requires a branch protection change.
2. Temporarily skipping an optional lane can block merges if branch protection references it.
3. Promoting Windows/macOS matrix jobs to required on every PR would inflate
   per-PR cost (85 LEM for `matrix_tests`).

## Source Target State

The intended source-repo target is one normalized check:

```
PR Gate Success
```

The `PR Gate Success` job in `.github/workflows/pr-gate.yml` aggregates the required surface
and passes if:

- Rust PR: `rust` job passed, `docs` job skipped.
- Docs-only PR: `docs` job passed, `rust` job skipped.
- Merge group: `rust` job passed (plan is skipped).

This decouples branch protection from individual lane churn. Optional lanes (`ripr`, coverage,
Python wheel, full matrix, property tests, API contracts) can be added or removed without
touching branch protection.

## Migration Steps

1. `PR Gate Success` accumulates a run history on real PRs.
2. A deliberate admin change updates source branch protection to require
   `PR Gate Success` instead of `Fast Checks`.
3. `ci.yml` jobs remain available, but source branch protection no longer points
   at individual implementation jobs.

The swarm repository has a separate target: `HL7v2 Rust Small Result` after the
self-hosted and hosted fallback routes are all proven.

## Why Not Change Branch Protection Now?

`PR Gate Success` should accumulate several successful runs before it becomes
the source required check. Changing branch protection before that creates a risk
of a stale or misconfigured gate blocking merges.

For the swarm repository, changing branch protection before CX53, CX43 fallback,
and GitHub-hosted fallback are all proven creates the same risk. Keep the swarm
required check unset until those proofs exist.

## Label Impact on Branch Protection

Labels never change which checks are *required*. The `PR Gate Success` check aggregates the
required surface. Optional lanes triggered by labels (`full-ci`, `platform-matrix`, etc.) are
never blocking.
