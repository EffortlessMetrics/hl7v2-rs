# AGENTS.md

## Repo source-of-truth stack

This repo uses a linked source-of-truth stack:

```text
Roadmap -> Proposal -> Spec -> ADR -> Plan -> Active goal -> PR -> Proof
```

Read these before changing files:

1. `docs/reference/SPEC_SYSTEM.md`
2. `.hl7v2/goals/active.toml`
3. The linked implementation plan
4. The linked spec for the selected work item
5. Any linked ADRs

## Scope rule

Implement one work item per PR.

Docs-only artifacts should stay separate unless the selected plan item says
otherwise:

- proposal PRs explain why;
- spec PRs define behavior;
- ADR PRs record durable decisions;
- plan PRs define sequencing;
- active-goal PRs define current execution;
- runtime/code PRs link to the spec and plan item they implement.

Do not create a new lane or broaden a lane unless the user explicitly asks for
that source-of-truth change.

## Proof rule

Run the proof commands listed in the selected plan item and run
`git diff --check` before reporting success.

If a proof command cannot run, record:

- the exact command;
- why it was unavailable;
- substitute evidence, if any;
- whether the missing proof blocks merge.

Do not claim release, registry, TestPyPI, PyPI, npm, support-tier, or runtime
success without the corresponding receipt or support-tier proof pointer.

## Generated status rule

Do not hand-edit generated status. Run the generator/checker named by the plan
or stop and report that the generator is unavailable.

## Policy rule

If you add or broaden an exception, update the relevant `policy/*.toml` ledger
with owner, reason, `covered_by`, `created`, `review_after`, and expiry when the
exception is temporary.

## Stop conditions

Stop and report instead of guessing when:

- the active goal is missing or stale;
- linked specs, ADRs, or plans are missing;
- no ready work item is identifiable;
- proof commands cannot run;
- generated status is dirty;
- unrelated staged changes exist;
- requested work conflicts with an ADR;
- the task would mix proposal/spec/ADR/plan/runtime changes without an explicit plan item.
