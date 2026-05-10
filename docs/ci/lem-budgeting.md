# LEM Budgeting

## What is LEM?

LEM (Linux-Equivalent Minute) is the unit of CI cost in this repository. One LEM equals one
GitHub Actions minute on `ubuntu-latest`.

Runner multipliers convert other platform costs:

| Runner              | Multiplier | 1 wall-minute costs |
| ------------------- | ---------: | ------------------: |
| `ubuntu-latest`     |        1.0 |              1 LEM  |
| `windows-latest`    |        2.0 |              2 LEM  |
| `macos-latest`      |       10.0 |             10 LEM  |
| Python wheel build  |        2.0 |              2 LEM  |
| Docker build        |        6.0 |              6 LEM  |
| External AI review  |        4.0 |              4 LEM  |

## Budget Tiers

| Tier      |  LEM range | Behavior                                                         |
| --------- | ---------: | ---------------------------------------------------------------- |
| Green     |       0–35 | No action                                                        |
| Warning   |      36–75 | PR summary warning; no block                                     |
| High warn |     76–125 | Stronger warning; reviewer should confirm surface                |
| Hard ceil |       >125 | Fails unless `full-ci` or `ci-budget-override` label is present  |

## How LEM Is Estimated

The PR Plan job (`pr-plan.yml`) classifies the diff against risk packs defined in
`policy/ci-risk-packs.toml` and computes a forecast:

```text
estimated_lem = sum(lane.base_lem × runner_multiplier for each triggered lane)
```

Cache hits reduce wall-clock time. The static `base_lem` for each lane is a pessimistic
estimate assuming no cache hit. Once actuals accumulate (`ci-actuals.json`), the system
transitions to learned estimates:

```text
estimate = max(static_floor, p50_recent_actual × 1.15)
warning  = p90_recent_actual
hard plan = p95_recent_actual
```

## Actuals

The `ci-actuals.json` artifact records:

- estimated LEM (from PR Plan)
- actual seconds per job
- actual LEM (wall seconds × runner multiplier / 60)
- cache hit status
- risk packs matched

After 1–2 weeks of actuals, learned estimates replace static floors.

## Enforcement Timeline

| Phase       | Status                    |
| ----------- | ------------------------- |
| Now         | Advisory only             |
| After PR 14 | Actuals collected         |
| After PR 15 | Soft warnings at 36/76    |
| After PR 15 | Hard ceiling at 125       |
| After PR 17 | Learned estimates replace  |

Hard enforcement requires labels `full-ci` or `ci-budget-override` to bypass.
