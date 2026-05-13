# ripr Static Mutation-Exposure Lane

`ripr` is planned as an advisory PR-time static mutation-exposure lane for
`hl7v2-rs`.

## Doctrine

`ripr` is static mutation-exposure analysis.

It catches much of the same signal mutation testing catches: weak test or
oracle exposure. It catches that signal earlier and cheaper because it runs
statically and can run per PR.

Mutation testing remains the slower runtime backstop for what static analysis
cannot prove. `ripr` shifts mutation signal left; it does not make mutation
testing unnecessary.

## Planned Role

The initial lane should be advisory:

- run on pull requests touching Rust code, `xtask`, Cargo files, `ripr.toml`,
  or the `ripr` policy ledger;
- emit JSON, SARIF, and markdown artifacts;
- avoid branch-protection blocking until calibration data exists;
- use suppressions only through a policy ledger;
- preserve runtime mutation as targeted or release-time proof.

## CI Economics

At industrialized PR volume, broad always-on runtime mutation would become an
ordinary PR tax. `ripr` is the cheaper PR-time signal for mutation exposure.
It should identify weak oracle exposure early while targeted runtime mutation
continues to cover what static analysis cannot prove.

## Non-Goals

- Do not make `ripr` required by branch protection in the first lane.
- Do not use `ripr` as a reason to remove mutation testing.
- Do not put full runtime mutation on ordinary PRs.
- Do not hide skipped mutation lanes as passed.
- Do not add suppressions without ownership and review.

## Planned Artifacts

```text
.github/workflows/ripr.yml
ripr.toml
policy/ripr-suppressions.toml
target/ripr/ripr.json
target/ripr/ripr.sarif
target/ripr/ripr.md
```
