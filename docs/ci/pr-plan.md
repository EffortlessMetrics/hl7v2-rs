# PR Plan

The PR Plan is an advisory workflow that classifies the diff for a pull request against the
risk packs defined in `policy/ci-risk-packs.toml` and estimates the Linux-Equivalent Minute
(LEM) cost of the triggered CI lanes.

## What It Does

1. Computes the list of changed files (`git diff --name-only base...head`).
2. Matches changed files against each risk pack's `paths` globs.
3. Determines which lanes are triggered by the matched packs.
4. Applies label overrides to add additional lanes.
5. Estimates total LEM from lane `base_lem` and runner multipliers.
6. Writes a summary to the GitHub Actions step summary.
7. Writes a JSON artifact `ci-plan.json` for downstream jobs.
8. Exports `docs_only`, `estimated_lem`, `tier`, and `matched_packs` outputs.

## Outputs

| Output           | Description                                                     |
| ---------------- | --------------------------------------------------------------- |
| `docs_only`      | `"true"` if all changes are documentation only                  |
| `estimated_lem`  | Estimated LEM as a number string                                |
| `tier`           | One of `green`, `ok`, `warning`, `high-warning`, `over-ceiling` |
| `matched_packs`  | Comma-separated list of matched risk pack names                 |

## LEM Tiers

| Tier           | LEM range | Effect                                          |
| -------------- | --------: | ----------------------------------------------- |
| `green`        |      0–25 | No action                                       |
| `ok`           |     26–35 | No action                                       |
| `warning`      |     36–75 | Warning in step summary                         |
| `high-warning` |    76–125 | Strong warning in step summary                  |
| `over-ceiling` |      >125 | Error note; add `full-ci` or `ci-budget-override` to proceed |

The LEM ceiling is enforced in PR 15 (soft budget guard). At this stage, the plan is
advisory only — it reports cost but does not block.

## Script

```bash
python3 scripts/ci/pr-plan.py \
  --base origin/main \
  --head HEAD \
  --labels "label1,label2" \
  --json-out target/ci/ci-plan.json \
  --github-summary "$GITHUB_STEP_SUMMARY"
```

## Risk Packs

Risk packs are defined in `policy/ci-risk-packs.toml`. Each pack maps file path globs to a
set of lanes. When a changed file matches a pack's paths, the pack's `lanes` are triggered.
Labels can additionally trigger `deep_lanes`.

See `docs/ci/cost-and-verification-policy.md` for the routing doctrine.

## Artifact

The `ci-plan.json` artifact is uploaded with a 7-day retention. Downstream jobs can
reference it to make routing decisions. Schema:

```json
{
  "schema_version": 1,
  "base": "origin/main",
  "head": "abc123",
  "labels": ["property-tests"],
  "changed_files_count": 12,
  "classification": {
    "matched_packs": ["core_parser"],
    "docs_only": false,
    "triggered_lanes": ["fast_checks", "standard_tests", "ripr_advisory"],
    "triggered_deep": ["extended_property_tests"],
    "label_triggered": ["extended_property_tests"]
  },
  "estimate": {
    "total_lem": 27.0,
    "breakdown": [...]
  },
  "tier": "ok"
}
```
