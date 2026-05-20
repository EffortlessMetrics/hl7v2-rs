# Dirty Real-World Legacy Timestamp Parity Receipt

Date: 2026-05-20
Branch: `test/dirty-corpus-legacy-timestamps`

## Scope

Added `test_data/dirty-real-world/after/legacy-timestamps.hl7` as a synthetic
dirty-corpus fixture for partial legacy timestamp shapes in MSH, EVN, and PID
fields. The fixture expands the shared after corpus without changing package,
registry, release, deployment, npm, TestPyPI, or PyPI claims.

## Fixture Added

| Fixture | Purpose |
| --- | --- |
| `test_data/dirty-real-world/after/legacy-timestamps.hl7` | Keeps partial legacy timestamp values parseable and visible in corpus value-shape evidence. |

## Expected Evidence Change

The shared dirty-corpus after set now contains ten files after test-generated
MLLP fixtures are materialized. Seven messages parse successfully, three
fixtures remain intentional safe parse failures, and the diff from the before
corpus gains one additional file and one additional parsed message.

The parity assertions now require:

- `ADT^A31` appears once in dirty-corpus message type counts.
- `MSH.3` has seven total occurrences across parsed after messages.
- `PID.7` records at least one numeric legacy timestamp-shaped value.

## Validation

```text
cargo +1.95.0 test -p hl7v2 --all-features dirty_real_world --locked
cargo +1.95.0 test -p hl7v2-cli --test integration_tests test_corpus_commands_share_dirty_real_world_fixture_categories --locked
cargo +1.95.0 test -p hl7v2-server --test corpus_endpoint_test test_corpus_endpoints_share_dirty_real_world_fixture_categories --locked
cargo +1.95.0 test -p hl7v2-server --test grpc_contract_tests test_grpc_corpus_commands_share_dirty_real_world_fixture_categories --locked
cargo +1.95.0 run -p xtask -- check-dirty-corpus-parity
python -m py_compile tests/python_smoke/smoke.py
cargo +1.95.0 fmt --all -- --check
cargo +1.95.0 run -p xtask -- check-doc-links
cargo +1.95.0 run -p xtask -- check-file-policy
cargo +1.95.0 run -p xtask -- check-evidence-parity
git diff --check
```

## Non-Claims

- No TestPyPI or PyPI upload/install-back proof.
- No npm package or TypeScript implementation.
- No new crates.io upload, tag, GitHub release, or deployment proof.
- No public Python parity promotion; Python remains local-wheel scoped until
  registry install-back succeeds.
