# Dirty Real-World Odd MSH Metadata Parity Proof

Date: 2026-05-18
Branch: `test/dirty-corpus-weird-msh`
Result: Passed locally for targeted Rust, CLI, REST, and gRPC checks

This receipt records a new shared dirty-corpus fixture for componentized MSH
sender, facility, receiver, and routing metadata. The fixture is synthetic and
redacted, but it represents a common interface-engine shape: MSH fields can
carry namespace-style components rather than simple application names.

This is not a crates.io, TestPyPI, PyPI, npm, tag, or GitHub release receipt.
It does not claim public Python registry install-back proof.

## Fixture Shape

| Fixture | Category | Expected result |
| --- | --- | --- |
| `test_data/dirty-real-world/after/weird-msh-fields.hl7` | Odd MSH metadata | Parses as an `ADT^A03` message and contributes MSH field-cardinality evidence without adding a parse-failure bucket. |

The shared after-corpus now has eight files, five parsed messages, and three
parse failures after tests generate the valid and truncated MLLP fixtures. The
parse-failure set remains:

- `malformed-delimiters.hl7`
- `mllp-truncated.hl7`
- `partial-batch.hl7`

## Surfaces

| Surface | Proof |
| --- | --- |
| Rust core | `dirty_real_world` synthetic corpus test verifies updated summary, fingerprint, diff, safe parse-error paths, `ADT^A03`, and MSH.3 cardinality. |
| CLI | `test_corpus_commands_share_dirty_real_world_fixture_categories` verifies `corpus summarize`, `corpus fingerprint`, and `corpus diff` JSON output over the updated corpus. |
| REST | `test_corpus_endpoints_share_dirty_real_world_fixture_categories` verifies inline corpus summary, fingerprint, and diff endpoint output over the updated corpus. |
| gRPC | `test_grpc_corpus_commands_share_dirty_real_world_fixture_categories` verifies inline corpus summary, fingerprint, and diff RPC output over the updated corpus. |
| Python local wheel | `python-local-wheel-proof` installs a local `hl7v2` wheel into a scratch virtual environment and runs the Python smoke/evidence scripts with the updated dirty-corpus expectations. Public Python registry proof remains blocked by #563 until TestPyPI upload/install-back succeeds. |

## Validation

The targeted checks used an external Cargo target directory:

```text
F:\cargo-target\hl7v2-rs-dirty-msh
```

| Command | Result |
| --- | --- |
| `cargo +1.95.0 fmt --all -- --check` | pass |
| `cargo +1.95.0 test -p hl7v2 --lib --all-features dirty_real_world` | pass |
| `cargo +1.95.0 test -p hl7v2-cli --test integration_tests test_corpus_commands_share_dirty_real_world_fixture_categories --locked` | pass |
| `cargo +1.95.0 test -p hl7v2-server --test corpus_endpoint_test test_corpus_endpoints_share_dirty_real_world_fixture_categories --locked` | pass |
| `cargo +1.95.0 test -p hl7v2-server --test grpc_contract_tests test_grpc_corpus_commands_share_dirty_real_world_fixture_categories --locked` | pass |
| `cargo +1.95.0 run -p xtask -- check-dirty-corpus-parity` | pass; Rust, CLI, REST, and gRPC dirty-corpus proof passed. Python local-wheel smoke was skipped by this command because `--include-python` was not used. |
| `PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo +1.95.0 run -p xtask -- python-local-wheel-proof --root F:\cargo-target\hl7v2-python-proof-dirty-msh --rust-toolchain 1.95.0` | pass; local wheel installed, `import hl7v2` succeeded, and `smoke.py`, `evidence_workflow_guide.py`, and `dirty_evidence_workflow.py` passed. |
| `cargo +1.95.0 clippy -p hl7v2 --lib -p hl7v2-cli -p hl7v2-server --all-targets --all-features --locked -- -D warnings` | pass |
| `cargo +1.95.0 run -p xtask -- check-doc-links` | pass; 211 Markdown files and 623 local links checked. |
| `cargo +1.95.0 run -p xtask -- check-file-policy` | pass; 630 tracked/untracked non-ignored files checked. |
| `cargo +1.95.0 run -p xtask -- badges --check` | pass; badge endpoints are current. |
| `cargo +1.95.0 run -p xtask -- impacted-evidence` | pass; impacted evidence artifacts generated under `target/xtask/impacted-evidence/`. |
| `cargo +1.95.0 run -p xtask -- impacted-evidence --check` | pass; impacted evidence artifacts are current. |
| `git diff --check` | pass |

## Non-Claims

This proof does not claim:

- all odd MSH namespace or routing variants are covered;
- public Python `hl7v2` TestPyPI or PyPI availability;
- TestPyPI or PyPI upload/install-back success;
- npm package availability;
- a new crates.io publish, tag, or GitHub release;
- that `hl7v2-python` is the recommended Rust API.

The public Python package remains blocked on the separate TestPyPI Trusted
Publisher setup and upload/install-back proof tracked by #563.
