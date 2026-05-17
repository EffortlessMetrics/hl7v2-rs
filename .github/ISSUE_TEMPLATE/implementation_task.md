---
name: Implementation Task
about: Create a scoped implementation, evidence, or release-readiness task
title: "[TASK] "
labels: needs-triage
assignees: ''

---

**Lane**: [release / evidence parity / Python proof / server / docs / policy]
**Surface**: [Rust / CLI / server / Python / binding backend / docs / CI]
**Story Points**: [1/2/3/5/8/13/21]
**Effort Estimate**: [Hours]

## Description

Clear description of what needs to be done.

## Why

Why is this important? Link to:

- proposal/spec/ADR/plan when applicable
- docs/STATUS.md current-state entry
- evidence contract or support-tier row when applicable

## Requirements

List what must be done:

- [ ] Requirement 1
- [ ] Requirement 2
- [ ] Focused tests or proof receipt
- [ ] Documentation updated when user-facing behavior changes
- [ ] Policy/evidence receipts updated when claims change

## Acceptance Criteria

How do we know this is done?

- [ ] All requirements met
- [ ] Relevant tests or proof commands pass
- [ ] No release, registry, TestPyPI, PyPI, or npm success is claimed without receipt
- [ ] docs/STATUS.md or support-tier docs updated if current product state changes
- [ ] Binding backend work is not promoted as the recommended Rust API

## Technical Notes

Implementation details, gotchas, dependencies:

- Depends on #123
- Uses pattern from src/xyz.rs
- Consider X and Y approaches

## Testing Strategy

How to test this:

1. Unit or integration test cases needed
2. Evidence or policy command needed
3. Registry/install-back proof if package state changes

## Definition of Done

Checklist before marking complete:

- [ ] Review completed
- [ ] Tests passing
- [ ] Required policy/doc gates pass
- [ ] Documentation updated when needed
- [ ] Merged to main
- [ ] Post-merge main checks reviewed
- [ ] Scratch worktrees and cargo targets cleaned up

## References

- [docs/STATUS.md](../../docs/STATUS.md)
- [docs/status/SUPPORT_TIERS.md](../../docs/status/SUPPORT_TIERS.md)
- [docs/README.md](../../docs/README.md)

## Links

- Design doc: [Link to proposal/spec/ADR/plan if applicable]
- Related issues: #xxx, #yyy
- Blocking issues: [If any]
- Blocked by: [If any]
