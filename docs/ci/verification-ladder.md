# Verification Ladder

Each layer below answers a different question. Routing means deciding which layers are needed
for a given diff, not which layers exist.

## Ladder

| Layer                                         | Default PR?        | Purpose                                                        |
| --------------------------------------------- | :----------------: | -------------------------------------------------------------- |
| fmt / lint policy / no-panic / file policy    | yes                | Code-shape and governance; catches suppressions and debt drift  |
| `cargo check` / clippy on changed surface     | yes                | Rust correctness and idiomatic shape                           |
| Unit tests                                    | yes                | Parser / model / writer behavior at function level             |
| Doc tests                                     | yes                | Documentation examples stay executable                         |
| Targeted integration tests                    | yes (risk-routed)  | CLI / server / API contract behavior for touched surfaces      |
| `ripr` advisory                               | advisory           | Oracle-gap detection (static, non-blocking)                    |
| Limited property tests                        | risk-routed        | Parser / profile invariants for high-risk diffs                |
| Full property tests                           | main / label       | Broad randomized confidence (PROPTEST_CASES=1000)              |
| Python wheel smoke                            | Python risk / label| Binding behavior (PyO3 / maturin)                             |
| OpenAPI / gRPC contract deep checks           | API risk / release | Runtime / schema parity                                        |
| Release / publish dry-run                     | release risk / main| Package integrity before crates.io publish                    |
| Coverage                                      | main / label       | Execution surface evidence                                     |
| Benchmarks                                    | main / label       | Performance regression tracking                               |
| Platform matrix (Windows / macOS / MSRV)      | label / main       | Platform and toolchain compatibility                          |
| Mutation / ripr soft-gate                     | after calibration  | High-confidence oracle-gap acknowledgement                    |

## Layer Ownership

| Layer                | Owner           |
| -------------------- | --------------- |
| fmt / lint policy    | core/build      |
| clippy               | core/build      |
| unit / doc tests     | core/test       |
| integration tests    | core/test       |
| ripr advisory        | core/parser     |
| property tests       | core/parser     |
| Python wheel         | binding/python  |
| API / gRPC contracts | api/contracts   |
| Release / publish    | release/ci      |
| Coverage             | release/ci      |
| Benchmarks           | performance     |
| Platform matrix      | platform/compat |

## Failure Mode per Layer

| Layer                 | If skipped, what breaks?                                              |
| --------------------- | --------------------------------------------------------------------- |
| fmt / lint policy     | Governance drift reaches main; suppression ledger diverges           |
| clippy                | Bad Rust shape, idiomatic debt accumulates silently                  |
| unit tests            | Parser / model regressions reach main                                |
| integration tests     | CLI / server contract breaks reach main undetected                   |
| ripr advisory         | Oracle gaps in production Rust code go unnoticed (non-blocking)     |
| property tests        | Parser invariant violations survive randomized input                 |
| Python wheel          | Binding API breaks reach PyPI or pip-installable artifact            |
| API contracts         | gRPC / OpenAPI drift: clients break at runtime                       |
| release checks        | Bad package metadata or publish graph reaches crates.io              |
| platform matrix       | Platform-specific failures ship undetected                           |

## Evidence per Layer

| Layer                 | Evidence artifact                                               |
| --------------------- | --------------------------------------------------------------- |
| fmt / lint policy     | Job logs; `policy/` TOML ledgers                               |
| clippy                | Job logs                                                        |
| unit tests            | Job logs; JUnit XML (after PR 13)                              |
| integration tests     | Job logs; JUnit XML (after PR 13)                              |
| ripr advisory         | `target/ripr/ripr.json`, `ripr.sarif` artifact                 |
| property tests        | Job logs; proptest regression files                             |
| Python wheel          | `dist/*.whl` artifact; smoke test logs                         |
| API contracts         | Generated schema diff; contract test logs                      |
| release checks        | `cargo package` logs; publish dry-run output                   |
| coverage              | `coverage.json`, `lcov.info`, Codecov dashboard                |
| benchmarks            | `benchmark-results.txt` artifact                               |
| LEM actuals           | `ci-actuals.json` artifact (after PR 14)                       |

## Related Docs

- [Test Evidence Lanes](test-evidence-lanes.md)
- [ripr Static Mutation-Exposure Lane](ripr.md)
- [Cost and Verification Policy](cost-and-verification-policy.md)
