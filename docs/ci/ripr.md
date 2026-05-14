# ripr Static Mutation-Exposure Lane

`ripr` is an advisory PR-time static mutation-exposure lane for `hl7v2-rs`.

## Doctrine

`ripr` is static mutation-exposure analysis.

It catches much of the same signal mutation testing catches: weak test or
oracle exposure. It catches that signal earlier and cheaper because it runs
statically and can run per PR.

Mutation testing remains the slower runtime backstop for what static analysis
cannot prove. `ripr` shifts mutation signal left; it does not make mutation
testing unnecessary.

## Current Role

The initial lane is advisory:

- run on pull requests touching Rust code, `xtask`, Cargo files, `ripr.toml`,
  or the `ripr` policy ledger;
- emit portable PR evidence, review guidance, annotation, summary, and
  impacted-evidence artifacts;
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

## Artifacts

```text
.github/workflows/ripr.yml
ripr.toml
policy/ripr-suppressions.toml
target/ripr/pr/repo-exposure.json
target/ripr/pr/repo-exposure.md
target/ripr/pr/summary.md
target/ripr/review/comments.json
target/ripr/review/comments.md
target/ripr/review/annotations.txt
target/xtask/badges/ripr.json
target/xtask/badges/ripr-plus.json
target/xtask/impacted-evidence/latest.json
target/xtask/impacted-evidence/latest.md
```

The workflow installs `ripr` 0.5.0 with Cargo, runs the repo-local `xtask`
verification commands, emits a stable PR evidence summary, and uploads the
artifacts as `ripr-pr-evidence`. The lane remains advisory: findings route
review and mutation decisions, while contract/tooling drift is the failure mode
to repair.
