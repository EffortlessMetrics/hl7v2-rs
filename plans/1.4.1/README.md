# v1.4.1 Plans

This directory preserves the historical PR-sized plan for the v1.4.1
source-of-truth and Python proof lane.

Status: closed as a standalone release plan. The source-of-truth stack landed,
and the release train moved forward through the Rust 1.95 / v1.5.0 quality
ratchet. Current release and package truth now lives in
[`docs/STATUS.md`](../../docs/STATUS.md),
[`docs/status/SUPPORT_TIERS.md`](../../docs/status/SUPPORT_TIERS.md), and the
v1.5.0 audit receipts linked from those files.

Plans should describe:

- linked proposals, specs, and ADRs;
- PR-sized work items;
- production delta and non-goals;
- proof commands;
- rollback or closeout steps.

Plans do not define the product contract. Link to specs for behavior, ADRs for
durable decisions, `.hl7v2/goals/active.toml` for current execution state, and
audits or handoffs for completed proof. If this historical plan conflicts with
current release receipts, the current status/support docs and receipts win.
