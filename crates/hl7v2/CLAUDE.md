# hl7v2

Canonical Rust library crate for the workspace.

Rust users should depend on this crate, not on retired implementation
microcrates and not on binding backend crates such as `hl7v2-python`.

## Build

```bash
cargo build -p hl7v2 --all-features
```

## Test

```bash
cargo test -p hl7v2 --all-features
```

## Lint

```bash
cargo clippy -p hl7v2 --all-targets --all-features -- -D warnings
```

## Role

`hl7v2` owns the Rust API for parsing, writing, validation, normalization,
ACK generation, MLLP framing, profiles, redaction, lifecycle metadata,
synthetic data, and evidence artifact helpers.

Keep shared HL7 semantics inside this crate as modules. Do not split parser,
model, redaction, MLLP, batch, or stream internals back into public Rust
microcrates without an explicit crate-boundary decision.

## Evidence

The CLI, server, Python binding, and future TypeScript package should preserve
the semantics exposed here for:

- parse and write behavior
- validation reports
- normalization and ACK behavior
- profile lint, explain, and test reports
- redaction receipts
- corpus summary, fingerprint, and diff artifacts
- bundle and replay helpers
- safe error and PHI sentinel behavior
- schema version handling

When changing shared evidence behavior, update the schema, parity, guide, and
receipt surfaces that claim that behavior.
