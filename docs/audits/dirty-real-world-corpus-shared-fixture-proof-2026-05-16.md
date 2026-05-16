# Dirty Real-World Corpus Shared Fixture Proof

Date: 2026-05-16
Branch: `test/shared-dirty-corpus-parity`
Result: Passed locally

This receipt records the first shared dirty-corpus fixture set for Rust core and
CLI corpus evidence commands. It upgrades the original inline core-only proof
into named fixture categories under `test_data/dirty-real-world/` so future
server, Python, and TypeScript parity work can use the same corpus shapes
without reverse-engineering a unit test.

This is not a registry, release, TestPyPI, PyPI, npm, tag, or GitHub release
receipt.

## Fixture Categories

| Category | Fixture source | Surface proved here |
| --- | --- | --- |
| Z-segment | `test_data/dirty-real-world/before/z-segment.hl7`, `test_data/dirty-real-world/after/z-segment.hl7` | Rust core, CLI |
| Large OBX expansion | `test_data/dirty-real-world/before/large-obx.hl7`, `test_data/dirty-real-world/after/large-obx.hl7` | Rust core, CLI |
| Legacy encoding declaration | `test_data/dirty-real-world/after/legacy-encoding.hl7` | Rust core, CLI |
| Malformed delimiters | `test_data/dirty-real-world/after/malformed-delimiters.hl7` | Rust core, CLI |
| Partial batch | `test_data/dirty-real-world/after/partial-batch.hl7` | Rust core, CLI |
| MLLP source | `test_data/dirty-real-world/sources/mllp-source.hl7` | Rust core, CLI after tests wrap it with MLLP framing |

The fixture files are stored as reviewable text. Tests normalize line endings to
HL7 segment terminators and generate the MLLP-framed input at runtime.

## Proof

The Rust core proof verifies:

- summary file, message, parse-error, message-type, segment, and byte counts;
- fingerprint file, message, parse-error, and field-cardinality counts;
- diff file, message, parse-error, and OBX field-cardinality deltas;
- safe parse-error output that does not include the synthetic `MRN-DIRTY`
  marker.

The CLI proof runs `corpus summarize`, `corpus fingerprint`, and `corpus diff`
against the same materialized fixture set and verifies the JSON output shape for
the same counts and deltas.

## Validation

The passing local checks used an external target directory to keep repository
artifacts out of the worktree:

```text
F:\cargo-target\hl7v2-rs-dirty-corpus-parity
```

| Command | Result |
| --- | --- |
| `cargo +1.95.0 fmt --all -- --check` | pass |
| `cargo +1.95.0 test -p hl7v2 --lib --all-features dirty_real_world` | pass; 1 targeted dirty-corpus test passed |
| `cargo +1.95.0 test -p hl7v2 --lib --all-features summary_tests` | pass; 7 synthetic corpus summary tests passed |
| `cargo +1.95.0 test -p hl7v2-cli --test integration_tests test_corpus_commands_share_dirty_real_world_fixture_categories` | pass; 1 targeted CLI corpus fixture test passed |
| `cargo +1.95.0 test -p hl7v2-cli --test integration_tests` | pass; 169 CLI integration tests passed |
| `cargo +1.95.0 clippy -p hl7v2 --lib --all-features -- -D warnings` | pass |
| `cargo +1.95.0 clippy -p hl7v2-cli --test integration_tests --all-features -- -D warnings` | pass |
| `cargo +1.95.0 run -p xtask -- evidence-schema-check` | pass; 33 evidence fixtures validated |
| `cargo +1.95.0 run -p xtask -- check-doc-links` | pass; 183 Markdown files and 470 local links checked |
| `cargo +1.95.0 run -p xtask -- check-file-policy` | pass; 591 tracked or untracked non-ignored files checked |
| `git diff --check` | pass |

## Non-Claims

This proof does not claim:

- all real-world HL7 variants are covered;
- a public compatibility corpus exists;
- REST, gRPC, Python, or TypeScript dirty-corpus parity;
- any crates.io upload;
- any TestPyPI or PyPI upload;
- any npm package;
- any tag or GitHub release.

Future parity work should reuse `test_data/dirty-real-world/` for server and
Python receipts before extending the fixture family again.
