# CI Lane Whitelist

The lane whitelist is the governed map of every CI job in this repository.

## Files

| File                                    | Purpose                                                |
| --------------------------------------- | ------------------------------------------------------ |
| `policy/ci-lane-whitelist.toml`         | One entry per CI job; required fields for each         |
| `policy/ci-whitelist-exceptions.toml`   | Temporary exceptions for policy violations             |
| `policy/ci-budget.toml`                 | LEM budget tiers and runner multipliers                |
| `policy/ci-risk-packs.toml`             | Diff path → lane routing table                         |

## Lane Entry Fields

Each `[[lane]]` entry in `ci-lane-whitelist.toml` must provide:

| Field                 | Required | Description                                                |
| --------------------- | :------: | ---------------------------------------------------------- |
| `id`                  | yes      | Unique identifier; matches xtask check output              |
| `workflow`            | yes      | Relative path to the `.github/workflows/*.yml` file        |
| `job`                 | yes      | GitHub Actions job id, or `*` for a governed aggregate workflow lane |
| `display_name`        | yes      | Human-readable name                                        |
| `kind`                | yes      | One of: `rust-policy`, `rust-integration`, `platform`, `property`, `performance`, `summary`, `coverage`, `security`, `python`, `api-contract`, `release`, `rust-nightly`, `static-analysis`, `mutation` |
| `tier`                | yes      | One of: `frontdoor`, `compatibility`, `advisory`, `deep`, `release` |
| `default_pr`          | yes      | Whether this lane runs on every PR by default              |
| `blocking`            | yes      | Whether failure blocks the PR                              |
| `runner`              | yes      | Runner type from `ci-budget.toml` `[runner_multipliers]`   |
| `base_lem`            | yes      | Static LEM estimate (pessimistic, no cache)                |
| `owner`               | yes      | Team owner path (e.g. `core/build`, `platform/compat`)     |
| `intent`              | yes      | What the lane proves                                       |
| `failure_mode`        | yes      | What breaks if this lane is skipped                        |
| `proof_obligation`    | yes      | Specific commands or checks this lane runs                 |
| `evidence`            | yes      | Artifact names or signals produced                         |
| `allowed_triggers`    | yes      | List of valid trigger events                               |
| `duplicate_of`        | yes      | IDs of lanes this overlaps (empty list if none)            |
| `review_after`        | yes      | Date to re-evaluate this lane                              |
| `expires`             | yes      | Date by which this lane entry must be reviewed             |
| `expensive`           | no       | `true` if runner cost is elevated                          |
| `default_pr_exception`| no       | ID in `ci-whitelist-exceptions.toml` allowing expensive default |
| `labels`              | no       | Labels that activate this lane                             |

## Checker

```bash
cargo run -p xtask -- check-ci-lane-whitelist
```

The checker verifies:
- Every checked-in workflow has an explicit top-level `permissions:` block.
- Every governed lane points at an existing workflow and job id, or uses `*` for an aggregate workflow lane.
- `default_pr = true` + `expensive = true` requires a valid exception.
- No expired exceptions remain.
- Required fields are present.
- `duplicate_of` references valid lane IDs.
- Windows/macOS/Python/Docker runners have the correct multiplier.

The checker is advisory for branch-protection selection. The source repository
currently requires `Fast Checks`; its intended normalized target is
`PR Gate Success`. The swarm repository uses a separate routed target,
`HL7v2 Rust Small Result`, only after the routed CX53, CX43 fallback, and
GitHub-hosted fallback proofs are complete.

## Inventory

See `docs/ci/inventory.md` for the current lane inventory table.
