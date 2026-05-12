# Active Goals

This directory holds machine-readable current execution state for hl7v2-rs
campaigns.

`active.toml` should tell an agent:

- the active campaign and objective;
- accepted proposal, spec, ADR, and plan links;
- the next PR-sized work item;
- proof commands;
- blockers and explicit non-goals;
- what must not be touched.

Do not use active goals as historical receipts. Move completed campaign state to
`archive/` or a closeout audit when the proof trail is durable.
