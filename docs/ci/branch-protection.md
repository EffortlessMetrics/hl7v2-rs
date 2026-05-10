# Branch Protection

## Current State

Branch protection on `main` currently requires these individual jobs from `ci.yml`:

- `Fast Checks`
- `Standard Tests`
- `Matrix Tests`
- `CI Success`

This coupling means:

1. Adding a new required check requires a branch protection change.
2. Temporarily skipping an optional lane can block merges if branch protection references it.
3. Windows/macOS matrix jobs gating every PR inflates per-PR cost (85 LEM for `matrix_tests`).

## Target State (after PR 16)

Branch protection will require one check:

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

1. PRs 01–15 land and `PR Gate Success` accumulates a run history on real PRs.
2. PR 16 updates branch protection to require `PR Gate Success` and removes the direct
   requirements for `Fast Checks`, `Standard Tests`, `Matrix Tests`, and `CI Success`.
3. `ci.yml` jobs remain available but are no longer branch-protection-required after PR 16.

## Why Not Change Branch Protection Now?

`PR Gate Success` should accumulate at least several successful runs before it becomes the
required check. Changing branch protection before that creates a risk of a stale or
misconfigured gate blocking merges.

## Label Impact on Branch Protection

Labels never change which checks are *required*. The `PR Gate Success` check aggregates the
required surface. Optional lanes triggered by labels (`full-ci`, `platform-matrix`, etc.) are
never blocking.
