# AGENTS.md

## Repo source-of-truth stack

This repo uses a linked source-of-truth stack:

```text
Roadmap -> Proposal -> Spec -> ADR -> Plan -> Active goal -> PR -> Proof
```

Read these before changing files:

1. `docs/reference/SPEC_SYSTEM.md`
2. `.hl7v2/goals/active.toml`
3. The linked plan
4. The linked spec for the selected work item
5. Linked ADRs

## Scope rule

Implement one work item per PR.

Docs-only artifacts are separate PRs unless an accepted plan says otherwise:

- proposal PRs explain why;
- spec PRs define behavior;
- ADR PRs record durable decisions;
- plan PRs define sequencing;
- active goal PRs define current execution.

Runtime/code PRs must link to the spec and plan item they implement.

## Proof rule

Run the proof commands listed in the plan item or active goal.

If a proof command cannot run, record:

- command;
- reason unavailable;
- substitute evidence, if any;
- whether this blocks merge.

No public claim is done without proof or an explicit unavailable-proof note.

## Generated status rule

Do not hand-edit generated status. Run the generator/checker named in the plan or
active goal, and commit the generated result only when the work item requires it.

## Policy rule

If you add an exception, add or update the relevant `policy/*.toml` ledger with:

- owner;
- reason;
- `covered_by`;
- `created`;
- `review_after`;
- `expires`, when temporary.

## Stop conditions

Stop and report instead of guessing when:

- the active goal is missing, stale, or contradictory;
- linked specs, ADRs, or plans are missing;
- no ready work item is available;
- proof commands cannot run and no substitute evidence is authorized;
- generated status differs from committed status;
- the requested work conflicts with an ADR;
- unrelated staged changes exist;
- a public claim lacks support-tier proof.

## Completion rule

A PR is ready only when:

- the intended artifact or code change exists;
- linked docs are updated when required;
- proof commands have run or are explicitly marked unavailable;
- claim boundaries are respected;
- `git diff --check` passes.
