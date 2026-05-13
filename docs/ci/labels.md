# CI Labels

Labels on a pull request override the default routing decisions made by the PR Plan. Applying
a label triggers the corresponding expensive or deep validation lane without requiring CI
workflow changes.

## Label Inventory

| Label                  | Effect                                                          |
| ---------------------- | --------------------------------------------------------------- |
| `full-ci`              | Run all lanes including platform matrix, property tests, Python wheel, API contracts, coverage, benchmarks |
| `platform-matrix`      | Run Windows + macOS + MSRV + beta matrix                        |
| `property-tests`       | Run full property tests (`PROPTEST_CASES=1000`)                 |
| `benchmarks`           | Run benchmark suite                                             |
| `python`               | Run Python wheel smoke (`hl7v2` distribution / maturin)        |
| `api-contract`         | Run API / OpenAPI / gRPC contract deep checks                   |
| `release-check`        | Run publish dry-run and release smoke                           |
| `security-audit`       | Run `cargo deny` and dependency security audit                  |
| `coverage`             | Run coverage collection and upload                              |
| `ripr`                 | Run ripr advisory on non-Rust-diff PRs too                      |
| `ripr-waive`           | Waive a ripr soft-gate finding (after calibration)              |
| `ci-budget-ack`        | Acknowledge elevated LEM; suppress budget warnings              |
| `ci-budget-override`   | Override the hard LEM ceiling (>125 LEM)                        |
| `mutation`             | Run mutation testing (separate from ripr)                       |
| `evidence`             | Run evidence bundle replay and schema fixture validation         |

## Adding a Label

Labels are added in the GitHub UI on the PR sidebar. No CI file changes are needed.

## Label + Branch Protection

Labels do not change which jobs are *required* by branch protection. The `PR Gate Success`
check aggregates the required surface. Optional lanes triggered by labels are never blocking.

## Label Governance

Label usage is tracked in `policy/ci-lane-whitelist.toml` under the `labels` field of each
lane entry. Introducing a new label requires adding it to the whitelist and documenting it here.
