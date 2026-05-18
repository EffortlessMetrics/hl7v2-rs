# Dirty Real-World MLLP Failure Parity Proof

Date: 2026-05-18
Branch: `test/dirty-corpus-mllp-failure-trace`
Result: Passed locally

This receipt records the shared dirty-corpus MLLP failure trace added after the
initial Rust, CLI, REST, gRPC, and local Python dirty-corpus parity proofs. The
change keeps `test_data/dirty-real-world/` text-only by deriving generated MLLP
inputs from `test_data/dirty-real-world/sources/mllp-source.hl7` at test time.

This is not a crates.io, TestPyPI, PyPI, npm, tag, or GitHub release receipt.
It does not claim public Python registry install-back proof.

## Fixture Shape

| Generated fixture | Source | Expected result |
| --- | --- | --- |
| `mllp-framed.hl7` | `sources/mllp-source.hl7` wrapped with MLLP start/end bytes | Parses as the existing valid generated MLLP message. |
| `mllp-truncated.hl7` | `sources/mllp-source.hl7` wrapped with the final MLLP byte removed | Counts as a safe parse failure for an incomplete MLLP transfer. |

The shared after-corpus now has seven files, four parsed messages, and three
parse failures. The parse failures remain safe to report and include:

- `malformed-delimiters.hl7`
- `mllp-truncated.hl7`
- `partial-batch.hl7`

## Surfaces

| Surface | Proof |
| --- | --- |
| Rust core | `dirty_real_world` synthetic corpus test verifies summary, fingerprint, diff, safe parse-error paths, and PHI sentinel behavior. |
| CLI | `test_corpus_commands_share_dirty_real_world_fixture_categories` verifies `corpus summarize`, `corpus fingerprint`, and `corpus diff` JSON output. |
| REST | `test_corpus_endpoints_share_dirty_real_world_fixture_categories` verifies inline corpus summary, fingerprint, and diff endpoint output. |
| gRPC | `test_grpc_corpus_commands_share_dirty_real_world_fixture_categories` verifies inline corpus summary, fingerprint, and diff RPC output. |
| Python local wheel | `python-local-wheel-proof` installs a local `hl7v2` wheel into a scratch virtual environment and runs the Python smoke/evidence scripts with the updated dirty-corpus expectations. |

## Validation

All commands used scratch target directories under `F:\cargo-target` so the
repository worktree did not retain Cargo build artifacts.

| Command | Result |
| --- | --- |
| `cargo +1.95.0 fmt --all -- --check` | pass |
| `cargo +1.95.0 test -p hl7v2 --lib --all-features --locked dirty_real_world` | pass |
| `cargo +1.95.0 test -p hl7v2-cli --test integration_tests --locked test_corpus_commands_share_dirty_real_world_fixture_categories` | pass |
| `cargo +1.95.0 test -p hl7v2-server --test corpus_endpoint_test --locked test_corpus_endpoints_share_dirty_real_world_fixture_categories` | pass |
| `cargo +1.95.0 test -p hl7v2-server --test grpc_contract_tests --locked test_grpc_corpus_commands_share_dirty_real_world_fixture_categories` | pass |
| `cargo +1.95.0 run -p xtask -- check-dirty-corpus-parity` | pass; Rust, CLI, REST, and gRPC dirty-corpus proof passed. Python local-wheel smoke was skipped by this command because `--include-python` was not used. |
| `PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo +1.95.0 run -p xtask -- python-local-wheel-proof --root F:\cargo-target\hl7v2-python-proof-mllp-dirty --rust-toolchain 1.95.0` | pass; local wheel installed, `import hl7v2` succeeded, and `smoke.py`, `evidence_workflow_guide.py`, and `dirty_evidence_workflow.py` passed. |
| `cargo +1.95.0 run -p xtask -- badges --check` | pass; badge endpoints are current after regenerating `badges/ripr.json`. |
| `cargo +1.95.0 run -p xtask -- check-doc-links` | pass; 205 Markdown files and 610 local links checked. |
| `cargo +1.95.0 run -p xtask -- check-file-policy` | pass; 622 tracked/untracked non-ignored files checked. |
| `git diff --check` | pass |

## Non-Claims

This proof does not claim:

- all malformed MLLP network failures are covered;
- public Python `hl7v2` TestPyPI or PyPI availability;
- TestPyPI or PyPI upload/install-back success;
- npm package availability;
- a new crates.io publish, tag, or GitHub release;
- that `hl7v2-python` is the recommended Rust API.

The public Python package remains blocked on the separate TestPyPI Trusted
Publisher setup and upload/install-back proof tracked by #563.
