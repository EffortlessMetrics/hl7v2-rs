# Repo source-of-truth system

This repo uses a linked source-of-truth stack. The rule is:

> Do not make every document do every job.

Separate why, what, decision, how, what now, and what proves it.

## Stack

```text
Roadmap
  -> Proposal / PRD
    -> Spec
      -> ADR where needed
        -> Implementation plan
          -> Active goal
            -> Issue / PR
              -> Proof command
              -> CI or receipt
              -> support-tier update
              -> policy-ledger update
```

A durable claim does not need every artifact, but it must have exactly one
owning source and a link path to proof. Small maintenance-only fixes may skip the
stack when the PR body says so and includes direct proof.

## Artifact roles

| Artifact | Owns | Does not own |
| --- | --- | --- |
| Roadmap | Release direction, milestone framing, lane names | Detailed PR queue, generated metrics, proof receipts |
| Proposal / PRD | Why, users, alternatives, risks, non-goals, success criteria | Behavior contract, PR sequence, completed proof |
| Spec | Required behavior, acceptance, examples, proof requirements | Product rationale, PR order, active queue |
| ADR | Durable architecture or operating decision and consequences | Task list, current metrics, implementation queue |
| Implementation plan | PR order, work items, dependencies, proof commands, rollback | Product strategy, durable decisions, generated status truth |
| Active goal | Current lane, machine-readable objective, ready work items, claim boundaries | Long prose, historical receipts, generated metrics |
| Issue / PR | Reviewable unit of work and discussion | Canonical status or architecture truth |
| Proof / receipt | Evidence that a command or external check ran | Future behavior requirements |
| Support tiers | Public claim proof and stable/advisory/experimental/blocked classification | Feature design or PR sequencing |
| Policy ledgers | Exceptions, CI/policy intent, owners, coverage, review dates | Broad architecture or campaign rationale |

## Canonical locations

| Question | Source of truth |
| --- | --- |
| Why are we doing this? | `docs/proposals/` |
| What must be true? | `docs/specs/` |
| What durable decision constrains the work? | `docs/adr/` |
| What PR lands next? | `plans/<lane>/implementation-plan.md` or lane-specific plan files |
| What is the agent actively executing? | `.hl7v2/goals/active.toml` |
| What proves a public claim? | `docs/status/SUPPORT_TIERS.md`, receipts, and CI |
| What exceptions exist? | `policy/*.toml` |
| What is current product status? | `docs/STATUS.md` and scoped status docs |

If two documents disagree, prefer the artifact that owns that kind of truth and
fix or retire the stale copy.

## Rules

1. One kind of truth per artifact.
2. One semantic artifact per PR unless the plan explicitly says otherwise.
3. Specs define behavior and proof; plans define sequencing and rollback.
4. Proposals explain why; ADRs record durable choices.
5. Active goals tell agents what to do now.
6. Generated status is updated by the named tool, not by hand.
7. Public claims require a support-tier row or equivalent proof pointer.
8. Policy exceptions require owner, reason, coverage, and review date.
9. Runtime/code PRs must link to the spec and plan item they implement.
10. Do not broaden a docs-only PR into behavior.

## Required front matter fields

Every proposal, spec, ADR, and implementation plan should include the fields
that apply to its artifact type. Use `n/a` where the field is not applicable.

```text
Status:
Owner:
Created:
Linked proposal:
Linked specs:
Linked ADRs:
Linked plan:
Linked issues:
Linked PRs:
Support-tier impact:
Policy impact:
```

Existing historical docs may use older headings such as `Proposal:` or `Spec:`;
new docs should prefer the linked-field form above.

## Agent workflow

Agents must:

1. Read repo instructions (`AGENTS.md`, `CLAUDE.md`, and any nested instruction
   files).
2. Read this file.
3. Read `.hl7v2/goals/active.toml`.
4. Read the linked implementation plan.
5. Read the linked proposal only for why.
6. Read the linked spec for acceptance.
7. Read linked ADRs for constraints.
8. Inspect `git status --short` for unrelated work.
9. Pick exactly one ready work item.
10. Implement only that work item.
11. Run the proof commands listed by the plan or active goal.
12. Update receipts/status/policy files only if the work item requires it.
13. Commit and open one focused PR.

If no ready work item is identifiable, do not invent one. Write a handoff or
stop with a clear blocker.

## Stop conditions

Stop and report instead of guessing when:

- `.hl7v2/goals/active.toml` is missing, stale, or contradictory;
- linked proposal/spec/ADR/plan files do not exist;
- no ready work item is available;
- proof commands cannot run and no substitute evidence is authorized;
- generated status is dirty or would need hand editing;
- unrelated staged changes exist;
- the requested work conflicts with an ADR;
- a public claim lacks support-tier proof;
- a policy exception would be added without owner, reason, `covered_by`, and
  `review_after`.

## Active goal lifecycle

### Activate

Create or update exactly one active manifest:

```text
.hl7v2/goals/active.toml
```

Use `status = "active"` when a lane is selected.

### Pause

Use `status = "paused"` with a `reason` when no lane is selected.

### Archive

Move old manifests to:

```text
.hl7v2/goals/archive/YYYY-MM-DD-<lane>.toml
```

Then create the new active manifest. Do not leave multiple active goals.

## Closeout format

At the end of a lane, create or update:

```text
plans/<lane>/closeout.md
```

Include:

- what shipped;
- proof commands and receipts;
- PRs and CI runs;
- generated status updates;
- support-tier and policy updates;
- what did not ship;
- deferred work;
- claim boundary;
- next-lane recommendation.

Closeout prevents the next agent from rediscovering already-finished work.

## Common failure modes

### Spec becomes a task list

Move PR order to `plans/<lane>/implementation-plan.md`; keep specs to behavior,
examples, and proof.

### Plan becomes product rationale

Move why and user pain to `docs/proposals/`; keep plans to work items,
dependencies, proof, rollback, and handoff.

### Active goal becomes prose

Keep `.hl7v2/goals/active.toml` machine-readable. Link to docs instead of
copying long tables or receipts.

### Generated status is hand-edited

Run the named generator/checker and commit the resulting files. If no generator
exists, record the gap instead of inventing generated truth.

### Support claims drift

Require `Support-tier impact:` on source-of-truth artifacts and a
`docs/status/SUPPORT_TIERS.md` row or equivalent proof pointer for public claims.

### Policy exceptions become silent debt

Every exception must have owner, reason, `covered_by`, `created`, and
`review_after`, plus `expires` when temporary.

### Mega PR

Split by semantic artifact or by one implementation work item. Do not combine a
new proposal, spec, ADR, plan, runtime behavior, and receipts unless an accepted
plan explicitly requires that bundle.

## What good looks like

A new contributor or agent can arrive cold and answer:

```text
What are we doing?
Why?
What must be true?
What decision constrains it?
What PR lands next?
What command proves it?
What may we claim?
What must we not claim?
```

If the repo answers those questions without chat history, the system is working.
