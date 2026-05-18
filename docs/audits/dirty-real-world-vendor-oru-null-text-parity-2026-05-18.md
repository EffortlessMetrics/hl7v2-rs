# Dirty Real-World Vendor ORU Null/Text Parity

Date: 2026-05-18
Branch: `test/dirty-corpus-vendor-oru-null`

## Scope

This receipt records a focused dirty real-world corpus expansion for a
synthetic vendor-shaped ORU result packet. The fixture adds narrative result
text, escaped delimiters, an NTE support note, and an explicit HL7 null
observation to the shared dirty corpus.

## Added Fixture

| Path | Purpose |
| --- | --- |
| `test_data/dirty-real-world/after/vendor-oru-null-text.hl7` | Exercises ORU narrative text, escaped delimiters, NTE notes, and explicit HL7 null observations while asserting the aggregate corpus summary, fingerprint, and diff shape without changing registry or release claims. |

## Expected Corpus Shape

The shared after corpus now contains:

- 9 files;
- 6 parsed messages;
- 3 parse failures;
- two `ORU^R01` messages;
- 22 `OBX` segments;
- one `NTE` segment;
- one explicit HL7 null shape on `OBX.5`;
- at least one text shape on `OBX.5`.

The before/after diff now records:

- file delta `7`;
- message delta `4`;
- parse-error delta `3`;
- `OBX.5` total-occurrence delta `17`;
- `OBX.5` null-count delta `1`;
- `OBX.5` text-count delta at least `1`.

## Proof Surfaces

| Surface | Proof |
| --- | --- |
| Rust core | `dirty_real_world` synthetic corpus test verifies updated summary, fingerprint, and diff counts. |
| CLI | `test_corpus_commands_share_dirty_real_world_fixture_categories` verifies command output for summary, fingerprint, and diff. |
| REST | `test_corpus_endpoints_share_dirty_real_world_fixture_categories` verifies v2 REST corpus reports. |
| gRPC | `test_grpc_corpus_commands_share_dirty_real_world_fixture_categories` verifies v2 gRPC corpus reports. |
| Python | `tests/python_smoke/smoke.py` keeps local-wheel dirty corpus expectations aligned with the shared fixture family. |

## Validation

| Command | Result |
| --- | --- |
| `cargo +1.95.0 test -p hl7v2 --lib --all-features dirty_real_world` | pass |
| `cargo +1.95.0 test -p hl7v2-cli --test integration_tests test_corpus_commands_share_dirty_real_world_fixture_categories --locked` | pass |
| `cargo +1.95.0 test -p hl7v2-server --test corpus_endpoint_test test_corpus_endpoints_share_dirty_real_world_fixture_categories --locked` | pass |
| `cargo +1.95.0 test -p hl7v2-server --test grpc_contract_tests test_grpc_corpus_commands_share_dirty_real_world_fixture_categories --locked` | pass |

The PR validation also ran the composed dirty-corpus parity, Python local-wheel,
Clippy, docs, policy, badge, impacted-evidence, formatting, and diff checks
recorded in the PR body.

## Non-Claims

- No TestPyPI upload.
- No TestPyPI install-back.
- No PyPI upload.
- No PyPI install-back.
- No npm package.
- No crates.io upload.
- No tag or GitHub release.
- No public Python registry proof.
