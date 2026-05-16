# Dirty Real-World Python Corpus Parity Proof

Date: 2026-05-16
Branch: `test/python-dirty-corpus-parity`
Result: Passed locally

This receipt extends the shared dirty-corpus fixture proof to the local Python
`hl7v2` wheel smoke lane. It reuses `test_data/dirty-real-world/`, the same
fixture family already used by the Rust core, CLI, REST, and gRPC corpus
proofs, and verifies that Python corpus summary, fingerprint, and diff helpers
preserve the same evidence semantics.

This is a local source-checkout and wheel proof. It is not a registry, release,
TestPyPI, PyPI, npm, tag, or GitHub release receipt.

## Fixture Coverage

| Category | Fixture source | Surface proved here |
| --- | --- | --- |
| Z-segment | `test_data/dirty-real-world/before/z-segment.hl7`, `test_data/dirty-real-world/after/z-segment.hl7` | Python `corpus_summary`, `corpus_fingerprint`, `corpus_diff` |
| Large OBX expansion | `test_data/dirty-real-world/before/large-obx.hl7`, `test_data/dirty-real-world/after/large-obx.hl7` | Python `corpus_summary`, `corpus_fingerprint`, `corpus_diff` |
| Legacy encoding declaration | `test_data/dirty-real-world/after/legacy-encoding.hl7` | Python `corpus_summary`, `corpus_fingerprint`, `corpus_diff` |
| Malformed delimiters | `test_data/dirty-real-world/after/malformed-delimiters.hl7` | Python `corpus_summary`, `corpus_fingerprint`, `corpus_diff` |
| Partial batch | `test_data/dirty-real-world/after/partial-batch.hl7` | Python `corpus_summary`, `corpus_fingerprint`, `corpus_diff` |
| MLLP source | `test_data/dirty-real-world/sources/mllp-source.hl7` | Python smoke wraps it with MLLP framing before adding it to the after corpus |

The Python smoke materializes the shared text fixtures into temporary before and
after corpus directories, normalizes line endings to HL7 segment terminators,
and generates the MLLP-framed message at runtime. The smoke does not print full
dirty-corpus reports on failure.

## Proof

The local Python proof verifies:

- corpus summary file, message, parse-error, message-type, and segment counts;
- corpus fingerprint file, message, parse-error, and field-cardinality counts;
- corpus diff file, message, parse-error, and OBX field-cardinality deltas;
- schema-versioned v2 Python corpus outputs;
- safe parse-error output that does not include the synthetic `MRN-DIRTY`
  marker.

## Validation

The passing local checks used an external build root to keep repository
artifacts out of the worktree:

```text
F:\cargo-target\hl7v2-rs-python-dirty-corpus-parity
```

| Command | Result |
| --- | --- |
| `cargo +1.95.0 test -p hl7v2-python --locked` | pass |
| `python -m maturin build --release --out F:\cargo-target\hl7v2-rs-python-dirty-corpus-parity\dist --target-dir F:\cargo-target\hl7v2-rs-python-dirty-corpus-parity` | pass |
| `python -m venv F:\cargo-target\hl7v2-rs-python-dirty-corpus-parity\venv` | pass |
| `F:\cargo-target\hl7v2-rs-python-dirty-corpus-parity\venv\Scripts\python.exe -m pip install --force-reinstall F:\cargo-target\hl7v2-rs-python-dirty-corpus-parity\dist\hl7v2-1.5.0-cp314-cp314-win_amd64.whl` | pass |
| `F:\cargo-target\hl7v2-rs-python-dirty-corpus-parity\venv\Scripts\python.exe tests\python_smoke\smoke.py` | pass; includes shared dirty-corpus summary/fingerprint/diff parity |
| `F:\cargo-target\hl7v2-rs-python-dirty-corpus-parity\venv\Scripts\python.exe tests\python_smoke\evidence_workflow_guide.py` | pass |
| `cargo +1.95.0 run -p xtask -- check-doc-links` | pass; 185 Markdown files and 477 local links checked |
| `cargo +1.95.0 run -p xtask -- check-file-policy` | pass; 593 tracked/untracked non-ignored files checked |
| `cargo +1.95.0 run -p xtask -- check-python-publish-policy` | pass |
| `cargo +1.95.0 run -p xtask -- badges --check` | pass |
| `cargo +1.95.0 run -p xtask -- impacted-evidence --check` | pass |
| `git diff --check` | pass |

## Non-Claims

This proof does not claim:

- all real-world HL7 variants are covered;
- a public compatibility corpus exists;
- TypeScript dirty-corpus parity;
- any crates.io upload;
- any TestPyPI or PyPI upload;
- any TestPyPI or PyPI install-back;
- any npm package;
- any tag or GitHub release.

Future TypeScript parity work should reuse `test_data/dirty-real-world/` or
explicitly explain why a TypeScript-specific fixture is required.
