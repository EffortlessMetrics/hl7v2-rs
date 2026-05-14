# Dirty Real-World Corpus Proof

Date: 2026-05-14
Branch: `test/real-world-corpus-proof`
Result: Passed

This receipt records the first focused dirty-corpus compatibility proof for the
core corpus summary, fingerprint, and diff surfaces. It is a local test receipt,
not a registry, release, TestPyPI, PyPI, npm, tag, or GitHub release receipt.

## Scope

The proof adds a core library regression test that builds before/after corpus
directories with deliberately ugly but common interface data:

- Z-segments;
- blank and odd legacy MSH fields;
- MLLP-framed message bytes;
- a larger OBX-heavy message;
- malformed delimiter input that must be counted as a parse error without
  echoing the raw payload.

The test then verifies:

- corpus summary file, message, segment, message-type, byte, and parse-error
  counts;
- corpus fingerprint file, message, parse-error, and field-cardinality counts;
- corpus diff before/after deltas for file count, message count, parse errors,
  and OBX field cardinality;
- safe parse-error output that does not include the synthetic `MRN-DIRTY`
  payload marker.

## Validation

The local H: target directory hit disk pressure during broad test compilation,
so the passing Rust checks used an F: target directory and disabled incremental
compilation for the cargo-heavy commands.

| Command | Result |
| --- | --- |
| `CARGO_TARGET_DIR=F:\cargo-target\hl7v2-rs-real-world-corpus CARGO_INCREMENTAL=0 cargo +1.95.0 test -p hl7v2 --lib --all-features dirty_real_world` | pass; 1 targeted dirty-corpus test passed |
| `CARGO_TARGET_DIR=F:\cargo-target\hl7v2-rs-real-world-corpus CARGO_INCREMENTAL=0 cargo +1.95.0 test -p hl7v2 --lib --all-features summary_tests` | pass; 7 corpus summary/fingerprint/diff tests passed |
| `cargo +1.95.0 fmt --all -- --check` | pass |
| `CARGO_TARGET_DIR=F:\cargo-target\hl7v2-rs-real-world-corpus CARGO_INCREMENTAL=0 cargo +1.95.0 clippy -p hl7v2 --lib --all-features -- -D warnings` | pass |
| `git diff --check` | pass |

## Non-Claims

This proof does not claim:

- a public compatibility corpus exists;
- all real-world HL7 variants are covered;
- server, Python, or TypeScript corpus parity;
- any crates.io upload;
- any TestPyPI or PyPI upload;
- any npm package;
- any tag or GitHub release.

Future corpus work should expand this into named fixture categories and
cross-surface parity receipts after the stable Rust proof remains useful.
