# Dirty Real-World Server Corpus Parity Proof

Date: 2026-05-16
Branch: `test/server-dirty-corpus-parity`
Result: Passed locally

This receipt extends the shared dirty-corpus fixture proof to the server REST and
gRPC inline corpus surfaces. It reuses `test_data/dirty-real-world/`, the same
fixture family already used by the Rust core and CLI corpus proofs, and verifies
that server-side corpus summary, fingerprint, and diff outputs preserve the same
evidence semantics.

This is not a registry, release, TestPyPI, PyPI, npm, tag, or GitHub release
receipt.

## Fixture Coverage

| Category | Fixture source | Surface proved here |
| --- | --- | --- |
| Z-segment | `test_data/dirty-real-world/before/z-segment.hl7`, `test_data/dirty-real-world/after/z-segment.hl7` | REST corpus endpoints, gRPC corpus RPCs |
| Large OBX expansion | `test_data/dirty-real-world/before/large-obx.hl7`, `test_data/dirty-real-world/after/large-obx.hl7` | REST corpus endpoints, gRPC corpus RPCs |
| Legacy encoding declaration | `test_data/dirty-real-world/after/legacy-encoding.hl7` | REST corpus endpoints, gRPC corpus RPCs |
| Malformed delimiters | `test_data/dirty-real-world/after/malformed-delimiters.hl7` | REST corpus endpoints, gRPC corpus RPCs |
| Partial batch | `test_data/dirty-real-world/after/partial-batch.hl7` | REST corpus endpoints, gRPC corpus RPCs |
| MLLP source | `test_data/dirty-real-world/sources/mllp-source.hl7` | REST and gRPC tests wrap it with MLLP framing before submitting inline messages |

The server surfaces accept inline messages rather than filesystem corpus paths.
The tests materialize the shared text fixtures into inline request payloads,
normalize line endings to HL7 segment terminators, and generate the MLLP-framed
message at runtime.

## Proof

The REST and gRPC proofs verify:

- corpus summary file, message, parse-error, message-type, and segment counts;
- corpus fingerprint file, message, parse-error, and field-cardinality counts;
- corpus diff file, message, parse-error, and OBX field-cardinality deltas;
- schema-versioned v2 REST and gRPC corpus outputs;
- safe parse-error output that does not include the synthetic `MRN-DIRTY`
  marker.

## Validation

The passing local checks used an external target directory to keep repository
artifacts out of the worktree:

```text
F:\cargo-target\hl7v2-rs-server-dirty-corpus-parity
```

| Command | Result |
| --- | --- |
| `cargo +1.95.0 fmt --all -- --check` | pass |
| `cargo +1.95.0 test -p hl7v2-server --test corpus_endpoint_test` | pass; 7 REST corpus endpoint tests passed |
| `cargo +1.95.0 test -p hl7v2-server --test grpc_contract_tests test_grpc_corpus` | pass; 10 gRPC corpus contract tests passed |
| `cargo +1.95.0 clippy -p hl7v2-server --test corpus_endpoint_test --test grpc_contract_tests --all-features -- -D warnings` | pass |
| `cargo +1.95.0 run -p xtask -- check-doc-links` | pass; 184 Markdown files and 473 local links checked |
| `cargo +1.95.0 run -p xtask -- check-file-policy` | pass; 592 tracked/untracked non-ignored files checked |
| `cargo +1.95.0 run -p xtask -- evidence-schema-check` | pass; 33 evidence fixtures validated |
| `git diff --check` | pass |

## Non-Claims

This proof does not claim:

- all real-world HL7 variants are covered;
- a public compatibility corpus exists;
- Python or TypeScript dirty-corpus parity from this server receipt alone;
- any crates.io upload;
- any TestPyPI or PyPI upload;
- any npm package;
- any tag or GitHub release.

The local Python wheel smoke lane now reuses `test_data/dirty-real-world/`.
See
[`dirty-real-world-python-corpus-parity-2026-05-16.md`](dirty-real-world-python-corpus-parity-2026-05-16.md).
