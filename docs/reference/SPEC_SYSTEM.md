# Repo source-of-truth system

This repo uses a linked source-of-truth stack so humans and agents can find the
one place that owns each kind of durable truth.

## Stack

```text
Roadmap
  -> Proposal
    -> Spec
      -> ADR where needed
        -> Implementation plan
          -> Active goal
            -> Issue / PR
              -> Proof
```

The rule is: do not make every document do every job. Separate why, what,
decision, how, what now, and what proves it.

## Artifact roles

| Artifact | Owns | Does not own |
| --- | --- | --- |
| Roadmap | Release direction, milestone framing, high-level lanes | PR queue, generated metrics, proof receipts |
| Proposal | Why the work exists, users, alternatives, success criteria | Behavior contract, detailed PR sequence, current metric state |
| Spec | Required behavior, acceptance examples, proof requirements | Product rationale, PR order, active queue |
| ADR | Durable architecture or operating decisions | Task lists, live status, implementation queue |
| Implementation plan | PR-sized work order, dependencies, proof commands, rollback | Product rationale, durable architecture, generated status truth |
| Active goal | Current machine-readable objective, ready work items, claim boundaries | Long prose, generated metrics, durable decisions |
| Support tiers | Public support claims and proof pointers | Feature design, campaign rationale |
| Policy ledgers | Exceptions, CI/policy intent, coverage, owner, review date | Broad architecture or product strategy |
| Audit / receipt | What happened and what was proven | Future behavior requirements |

## Canonical locations

| Question | Source of truth |
| --- | --- |
| Why are we doing this? | `docs/proposals/` |
| What must be true? | `docs/specs/` |
| What architecture decision did we make? | `docs/adr/` |
| What PR lands next? | `plans/<lane>/implementation-plan.md` |
| What is the agent actively executing? | `.hl7v2/goals/active.toml` |
| What proves a public claim? | `docs/status/SUPPORT_TIERS.md`, receipts, and CI |
| What exceptions exist? | `policy/*.toml` |

Current product status still lives in `docs/STATUS.md`; do not duplicate it in
proposal, spec, ADR, plan, or active-goal prose.

## Rules

1. One kind of truth per artifact.
2. One semantic artifact per PR unless the selected plan item explicitly says otherwise.
3. Proposals explain why; specs define behavior; ADRs record durable decisions.
4. Plans define sequencing and proof commands; active goals define current execution.
5. Generated status is updated by tools, not by hand.
6. Public claims require a support-tier row, receipt, or equivalent proof pointer.
7. Policy exceptions require owner, reason, coverage, and review date.
8. Proof commands must be run or marked unavailable with an explicit reason and merge impact.

## Required metadata

Use the repository's established casing and fields, but every durable proposal,
spec, ADR, and plan should declare the applicable form of:

- `Status:`
- `Owner:` or equivalent accountable owner when known
- `Created:` / `Date:` when known
- linked proposal, specs, ADRs, plan, issues, and PRs when applicable
- support-tier impact
- policy impact

Use `n/a` when a field is intentionally not applicable.

## Agent workflow

Agents must:

1. read root repo instructions such as `AGENTS.md` or `CLAUDE.md`;
2. read this file;
3. read `.hl7v2/goals/active.toml`;
4. read the linked implementation plan;
5. read the linked proposal only for why;
6. read the linked spec for acceptance and proof;
7. read linked ADRs for constraints;
8. inspect git status for unrelated work;
9. pick exactly one ready work item;
10. implement only that work item;
11. run the listed proof commands plus `git diff --check`;
12. update status, receipts, and policy ledgers only when the work item requires it.

If there is no ready work item, do not invent one. Stop and report the missing
rail or create a handoff only when explicitly asked.

## Stop conditions

Stop instead of guessing when:

- the active goal is missing, stale, or contradictory;
- linked proposal, spec, ADR, or plan files do not exist;
- the selected work item lacks proof commands;
- proof commands cannot run and no substitute evidence is authorized;
- generated status differs from committed status;
- unrelated staged changes exist;
- the requested work conflicts with an ADR;
- a public claim lacks support-tier proof;
- the requested change would mix proposal/spec/ADR/plan/runtime work without an explicit plan item.

## Active goal lifecycle

`.hl7v2/goals/active.toml` is the single active execution manifest. A paused
state should use `status = "paused"` with a reason. Archive replaced manifests
under `.hl7v2/goals/archive/` before creating a new active manifest.

## Closeout

At the end of a lane, write a closeout or audit that records:

- what shipped;
- proof commands and receipts;
- PRs and CI runs;
- support-tier or policy updates;
- what did not ship;
- deferred work;
- claim boundaries;
- next-lane recommendations.

Closeout prevents future humans and agents from rediscovering old work.
