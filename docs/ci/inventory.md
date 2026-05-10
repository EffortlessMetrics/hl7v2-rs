# CI Lane Inventory

Current lane inventory derived from `policy/ci-lane-whitelist.toml`.

Last updated: 2026-05-09 (PR 07: platform matrix routed off ordinary PRs)

## Active Lanes

| ID                        | Workflow              | Job              | Tier          | Default PR? | Blocking? | Base LEM | Owner           |
| ------------------------- | --------------------- | ---------------- | ------------- | :---------: | :-------: | -------: | --------------- |
| `fast_checks`             | `ci.yml`              | `fast`           | frontdoor     | yes         | yes       |       12 | core/build      |
| `standard_tests`          | `ci.yml`              | `standard`       | frontdoor     | yes         | yes       |       15 | core/test       |
| `msrv_smoke`              | `ci.yml`              | `msrv-smoke`     | compatibility | yes         | yes       |       12 | platform/compat |
| `matrix_tests`            | `ci.yml`              | `matrix-test`    | compatibility | **no**      | no        |       85 | platform/compat |
| `extended_property_tests` | `ci.yml`              | `extended`       | deep          | no          | no        |       20 | core/parser     |
| `benchmarks`              | `ci.yml`              | `benchmarks`     | deep          | no          | no        |       15 | performance     |
| `ci_success`              | `ci.yml`              | `ci-success`     | frontdoor     | yes         | yes       |        1 | release/ci      |
| `coverage`                | `coverage.yml`        | `coverage`       | deep          | no          | no        |       20 | release/ci      |
| `security`                | `security.yml`        | `*`              | deep          | no          | no        |        5 | release/ci      |
| `python_wheels`           | `python-wheels.yml`   | `wheel-smoke`    | deep          | no          | no        |       20 | binding/python  |
| `nightly`                 | `nightly.yml`         | `*`              | deep          | no          | no        |       25 | core/build      |
| `contracts`               | `contracts.yml`       | `*`              | deep          | no          | no        |       15 | api/contracts   |
| `publish`                 | `publish.yml`         | `publish`        | release       | no          | no        |       20 | release/ci      |

## Default PR LEM Estimate (after PR 07)

Ordinary PRs now run:

| Lane             | Base LEM |
| ---------------- | -------: |
| `fast_checks`    |       12 |
| `standard_tests` |       15 |
| `msrv_smoke`     |       12 |
| `ci_success`     |        1 |
| **Total**        |   **40** |

Most PRs target 35 LEM. The MSRV smoke check adds a 5 LEM compile-only overage
for a 40 LEM default path that prevents minimum-supported-Rust regressions.

## LEM Savings from PR 07

Before PR 07, ordinary PRs ran the full matrix (85 LEM):

| Before | After | Saved |
| -----: | ----: | ----: |
|    113 |    40 |    73 |

That is a 65% reduction in default PR LEM. On $0.008/LEM, each ordinary PR
now costs approximately $0.32 instead of $0.90.

## Label-Triggered Lanes

| Label              | Additionally triggers            |
| ------------------ | -------------------------------- |
| `platform-matrix`  | `matrix_tests` (85 LEM)          |
| `full-ci`          | all deep lanes + `matrix_tests`  |
| `release-check`    | `matrix_tests`, `security`, `publish` |
| `property-tests`   | `extended_property_tests`        |
| `python`           | `python_wheels`                  |
| `api-contract`     | `contracts`                      |
| `coverage`         | `coverage`                       |
| `benchmarks`       | `benchmarks`                     |

## Main Branch Lanes

On `push` to `main`, the following additional lanes run beyond default PRs:

- `matrix_tests` (full platform fan-out)
- `extended_property_tests` (PROPTEST_CASES=1000)
- `benchmarks`
- `coverage` (via coverage.yml)

## Scheduled Lanes

- `nightly` (via the `nightly.yml` schedule)

## Exceptions

No active exceptions. The `ci_exception_matrix_tests_default` exception was
retired in PR 07 when `matrix_tests` was moved off the ordinary PR path.
