# The spec/proposal system, fully explained

The system is a **repo source-of-truth stack**. Its central rule is:

> **Do not make every document do every job.**

Each artifact owns one kind of truth: **why**, **what**, **what decision**, **how**, **what now**, **what proves it**, and **what changed**.

The end result is a repo where a human, Codex, Droid, Claude, or CI can answer:

```text
Why are we doing this?
What exact behavior must be true?
What architecture decision did we make?
What PR-sized work comes next?
What is the active lane right now?
What proves the claim?
Which support tier changed?
Which policy ledgers changed?
What happened after merge?
```

---

## 1. The stack at a glance

```text
Roadmap
  -> Proposal / PRD
    -> Specs
      -> ADRs where needed
        -> Implementation plan
          -> Active goal manifest
            -> Issues / PRs
              -> Proof commands
              -> CI lanes
              -> support-tier updates
              -> policy receipts
                -> Closeout / handoff
```

Each layer narrows the previous one.

- A **roadmap** says direction.
- A **proposal** says why the initiative exists.
- A **spec** says the behavior contract.
- An **ADR** says the architecture decision committed.
- An **implementation plan** says PR sequence.
- An **active goal manifest** says what the agent is executing now.
- A **support-tier map** says what users may believe.
- A **policy ledger** says exceptions/rules/classifications/receipts.
- A **closeout** says what actually happened.

---

## 2. Why the system exists

The point is **repo-operational memory**.

Without this system, workers rely on stale chat context, old PR descriptions, ambiguous README claims, hidden CI costs, and unverified assumptions.

With this system, the repo itself tells workers what to do:

```text
.hl7v2/goals/active.toml
  -> linked implementation plan
    -> linked spec
      -> linked proposal
        -> linked support-tier and policy proof
```

---

## 3. Artifact types and ownership

### 3.1 Roadmap

**Owns:** release direction, milestone themes, high-level sequencing.

**Does not own:** acceptance tests, PR order, detailed implementation tasks.

### 3.2 Proposal / PRD

**Owns:** why the work exists.

Contains problem, value, alternatives, risks, success criteria, and the linked specs/ADRs/plan.

### 3.3 Spec

**Owns:** what behavior must be true.

Defines contract, non-goals, required evidence, test mapping, CI proof, and promotion rules.

### 3.4 ADR

**Owns:** durable architecture decisions.

Used only when decisions constrain future design/implementation.

### 3.5 Implementation plan

**Owns:** PR-sized sequencing.

Turns proposal/spec/ADR truth into concrete ordered work items.

### 3.6 Active goal manifest

**Owns:** what the agent/operator is executing now.

Machine-readable execution state should live under a Codex-specific goals namespace.

### 3.7 Support tiers

**Owns:** product claim → proof command mapping.

Stable claims must map to concrete proof commands.

### 3.8 Policy ledgers

**Own:** governed exceptions, package boundaries, CI lane policy, lint policy, file policy, and proof receipts.

### 3.9 Closeout / handoff

**Owns:** what actually happened after merge.

Captures landed changes, proof results, claim changes, policy deltas, and remaining work.

---

## 4. Directory layout

A mature repo typically includes:

```text
docs/
  proposals/
  specs/
  adr/
  audits/
  plans/
  status/
  handoffs/
.hl7v2/goals/
policy/
```

Use stable, repo-specific IDs (for example `HL7V2-SPEC-0001`) so CI and automation can validate artifact references.

---

## 5. Linking model

The system is link-driven:

- Roadmap item -> proposal.
- Proposal -> specs/ADRs/plan.
- Spec -> proposal + proof commands.
- ADR -> dependent specs.
- Plan -> proposal/spec/ADR IDs.
- Active goal -> plan work items.
- PR -> plan/spec/proposal.
- Closeout -> landed outcomes and proof.

Use consistent headers so CI can validate graph integrity:

```md
Status:
Owner:
Created:
Milestone:
Linked proposal:
Linked specs:
Linked ADRs:
Linked plan:
Linked issues:
Linked PRs:
Support-tier impact:
Policy impact:
```

---

## 6. Status lifecycle

Recommended statuses:

- Proposals/specs/ADRs: `draft`, `proposed`, `accepted`, `implemented`, `superseded`, `rejected`
- Plan items: `ready`, `active`, `blocked`, `done`, `superseded`
- Active goals: `active`, `paused`, `complete`, `archived`

---

## 7. Anti-duplication rule

Keep one source of truth per fact.

Examples:

- Claim stability -> `docs/status/SUPPORT_TIERS.md`
- CI lane intent/cost -> `policy/ci-lane-whitelist.toml`
- Package classification -> `policy/package-boundary.toml`
- Active agent work -> `.hl7v2/goals/active.toml`
- PR order -> `plans/<milestone>/implementation-plan.md`
- Initiative rationale -> `docs/proposals/...`
- Behavior contract -> `docs/specs/...`
- Durable architecture decisions -> `docs/adr/...`

---

## 8. Agent operating flow

1. Read active goal manifest.
2. Select next ready work item.
3. Read linked implementation plan item.
4. Read linked spec.
5. Read linked proposal for context.
6. Read linked ADRs when architecture is involved.
7. Ship one PR-sized change.
8. Update support tiers/policy ledgers only when claims/policy change.
9. Run listed proof commands.
10. Update goal manifest status.
11. Complete merge flow per repo policy.
12. Record closeout notes when lane completes.

Always verify named commands/lints/crates/workflows/features exist before relying on them.

---

## 9. CI enforcement

Recommended checks should use repo-local commands that actually exist. In this
repo, the source-of-truth stack is guarded by commands such as:

```text
cargo run -p xtask -- check-doc-links
cargo run -p xtask -- check-file-policy
cargo run -p xtask -- check-ci-lane-whitelist
cargo run -p xtask -- check-python-publish-policy
cargo run -p xtask -- check-evidence-parity
cargo run -p xtask -- policy-report
```

These checks enforce link integrity, file-policy coverage, CI lane policy,
Python release boundaries, evidence parity claims, and policy completeness.
Future document-artifact or goal-manifest checks should be added only after the
repo has concrete commands for them.

---

## 10. PR structure

PRs should include:

- Summary
- Links (proposal/spec/ADR/plan/issue)
- Scope and non-goals
- Support-tier impact
- Policy impact
- Proof commands
- Claim boundary
- Rollback

The **claim boundary** prevents narrow proof from being interpreted as broad capability.

---

## 11. Core principles

1. **One artifact, one kind of truth.**
2. **Specs are contracts, not queues.**
3. **Plans are PR-sized.**
4. **Claims must be proof-mapped.**
5. **Policy exceptions are ledgers, not vibes.**
6. **Agent state must be machine-readable.**
7. **Do not encode fake repo rules.**
8. **Verify specifics before acting.**

---

## 12. Minimal rollout order

1. Define the model and templates.
2. Add a document artifact ledger.
3. Add doc-artifact validation.
4. Add active goal manifest.
5. Add goal validation.
6. Add first proposal.
7. Add first spec.
8. Add support tiers.
9. Add package/CI/policy ledgers.
10. Wire CI checks (advisory first, then blocking as appropriate).

---

## 13. Simplest mental model

```text
Proposal = why.
Spec = what.
ADR = durable decision.
Plan = how.
Active goal = what Codex is doing now.
Support tiers = what users may believe.
Policy ledgers = exceptions and proof obligations.
CI = what proved it.
Closeout = what happened.
```

The system works when these layers are linked, validated, and not duplicating each other.
