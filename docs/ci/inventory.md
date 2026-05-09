# CI Lane Inventory

Current lane inventory derived from `policy/ci-lane-whitelist.toml`.

Last updated: 2026-05-09

## Active Lanes

| ID                        | Workflow              | Job              | Tier          | Default PR? | Blocking? | Base LEM | Owner           |
| ------------------------- | --------------------- | ---------------- | ------------- | :---------: | :-------: | -------: | --------------- |
| `fast_checks`             | `ci.yml`              | `fast`           | frontdoor     | yes         | yes       |       12 | core/build      |
| `standard_tests`          | `ci.yml`              | `standard`       | frontdoor     | yes         | yes       |       15 | core/test       |
| `matrix_tests`            | `ci.yml`              | `matrix-test`    | compatibility | yes*        | yes       |       85 | platform/compat |
| `extended_property_tests` | `ci.yml`              | `extended`       | deep          | no          | no        |       20 | core/parser     |
| `benchmarks`              | `ci.yml`              | `benchmarks`     | deep          | no          | no        |       15 | performance     |
| `ci_success`              | `ci.yml`              | `ci-success`     | frontdoor     | yes         | yes       |        1 | release/ci      |
| `coverage`                | `coverage.yml`        | `coverage`       | deep          | no          | no        |       20 | release/ci      |
| `security`                | `security.yml`        | `security`       | deep          | no          | no        |        5 | release/ci      |
| `python_wheels`           | `python-wheels.yml`   | `build`          | deep          | no          | no        |       20 | binding/python  |
| `nightly`                 | `nightly.yml`         | `nightly`        | deep          | no          | no        |       25 | core/build      |
| `contracts`               | `contracts.yml`       | `contracts`      | deep          | no          | no        |       15 | api/contracts   |
| `publish`                 | `publish.yml`         | `publish`        | release       | no          | no        |       20 | release/ci      |

*`matrix_tests` is `default_pr = true` with an active exception (`ci_exception_matrix_tests_default`).
 This is the primary target of PR 07 (route platform matrix to main and labels).

## Default PR LEM Estimate

With the current configuration, every ordinary PR runs:

| Lane             | Base LEM |
| ---------------- | -------: |
| `fast_checks`    |       12 |
| `standard_tests` |       15 |
| `matrix_tests`   |       85 |
| `ci_success`     |        1 |
| **Total**        |  **113** |

After PR 07 (matrix routing), the ordinary PR estimate drops to:

| Lane             | Base LEM |
| ---------------- | -------: |
| `fast_checks`    |       12 |
| `standard_tests` |       15 |
| `ci_success`     |        1 |
| **Total**        |   **28** |

This brings ordinary PRs within the preferred 25 LEM target and well under the 35 LEM limit.

## Exceptions

| Exception ID                           | Lane           | Expires    | Reason              |
| -------------------------------------- | -------------- | ---------- | ------------------- |
| `ci_exception_matrix_tests_default`    | `matrix_tests` | 2026-08-09 | Pending PR 07 route |
