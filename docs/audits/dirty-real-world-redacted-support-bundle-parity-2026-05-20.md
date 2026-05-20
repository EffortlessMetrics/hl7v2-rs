# Dirty Real-World Redacted Support Bundle Parity Receipt

Date: 2026-05-20
Branch: `test/dirty-corpus-redacted-support-bundle`

## Scope

Added a synthetic already-redacted support bundle payload to the shared dirty
real-world after corpus. The fixture represents a `message.redacted.hl7`
artifact that downstream corpus tools may receive from an operator support
packet. It expands corpus proof without changing registry, release, deployment,
npm, TestPyPI, or PyPI claims.

## Fixture Added

| Fixture | Purpose |
| --- | --- |
| `test_data/dirty-real-world/after/redacted-support-bundle.hl7` | Keeps a sanitized support-bundle message parseable, fingerprintable, and visible through a synthetic `ZSB` support-bundle marker without raw PHI. |

## Expected Evidence Change

After test-generated MLLP fixtures are materialized, the shared dirty-corpus
after set contains eleven files. Eight messages parse successfully, three
fixtures remain intentional safe parse failures, and the diff from the before
corpus gains one additional file and one additional parsed message.

The parity assertions now require:

- `ADT^A01` appears twice in dirty-corpus message type counts.
- `ZSB` appears once in dirty-corpus segment counts.
- `MSH.3` has eight total occurrences across parsed after messages.
- `ZSB.1` has one total occurrence.
- `PID.3` records at least one text-shaped value, proving the redacted
  `hash:sha256:` identifier shape is included without exposing raw identifiers.

## Validation

```text
cargo +1.95.0 test -p hl7v2 --all-features dirty_real_world --locked
cargo +1.95.0 test -p hl7v2-cli --test integration_tests test_corpus_commands_share_dirty_real_world_fixture_categories --locked
cargo +1.95.0 test -p hl7v2-server --test corpus_endpoint_test test_corpus_endpoints_share_dirty_real_world_fixture_categories --locked
cargo +1.95.0 test -p hl7v2-server --test grpc_contract_tests test_grpc_corpus_commands_share_dirty_real_world_fixture_categories --locked
cargo +1.95.0 run -p xtask -- check-dirty-corpus-parity
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo +1.95.0 run -p xtask -- python-local-wheel-proof --root F:\cargo-target\hl7v2-python-proof-redacted-support --rust-toolchain 1.95.0
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
