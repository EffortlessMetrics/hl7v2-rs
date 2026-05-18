# Plans

Plans define how accepted work lands. They own PR sequence, work items,
dependencies, proof commands, rollback notes, and closeout handoff.

Plans do not own product motivation, durable architecture decisions, public
support claims, generated status truth, or external release receipts. Link to
the proposal, spec, ADR, status document, policy ledger, or receipt that owns
that truth.

## Plan shape

A lane plan should identify:

- status, owner, linked proposal, linked specs, linked ADRs, and active goal;
- current factual baseline with links to status docs or receipts;
- one PR-sized work item per section;
- explicit non-goals;
- acceptance criteria;
- proof commands;
- rollback guidance.

## Agent rule

Agents should select exactly one ready work item from the active goal and linked
plan. If no ready work item exists, stop and report the blocker instead of
inventing work.

## Closeout

When a lane finishes, add or update `plans/<lane>/closeout.md` with what shipped,
proof, receipts, generated status updates, policy/support-tier impact, deferred
work, claim boundaries, and the recommended next lane.
