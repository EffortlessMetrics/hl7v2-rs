## Summary

<!-- Describe what this PR does and why. -->

## Assumptions

<!-- State any assumptions you made that reviewers should know. -->

## Checklist

- [ ] I agree to the [Contributor License Agreement](CLA.md) for this contribution.
- [ ] `cargo run -p xtask -- gate --check` passes locally.
- [ ] `cargo run -p xtask -- check-lint-policy` passes.
- [ ] `cargo run -p xtask -- check-no-panic-family` passes.
- [ ] `cargo run -p xtask -- check-file-policy` passes.

## CI Economics

<!-- Fill in when CI workflows are touched. -->

| Field                  | Value |
| ---------------------- | ----- |
| Default PR LEM impact  | <!-- e.g. +0 LEM (docs only) --> |
| Workflows touched      | <!-- list workflow files --> |
| Branch protection      | <!-- changed / unchanged --> |
| Failure mode caught    | <!-- what would break without this --> |
| Cheaper signal?        | <!-- was there a cheaper way? --> |
| Rollback path          | <!-- how to revert --> |

## Commands Run

```bash
# paste relevant commands here
```
