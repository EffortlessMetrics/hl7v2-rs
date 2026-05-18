# HL7v2-rs Roadmap

**Current status**: v1.5.0 Rust 1.95 quality-ratchet release is published for
`hl7v2`, `hl7v2-python`, `hl7v2-server`, and `hl7v2-cli`.

`hl7v2-python` is binding backend infrastructure for the public Python
distribution. It is not the recommended Rust API.

For detailed current feature state, support tiers, and receipts, use:

- [`docs/STATUS.md`](docs/STATUS.md)
- [`docs/status/SUPPORT_TIERS.md`](docs/status/SUPPORT_TIERS.md)
- [`policy/evidence-parity.toml`](policy/evidence-parity.toml)
- [`docs/guides/first-use-by-surface.md`](docs/guides/first-use-by-surface.md)

## Now

1. **Public Python proof**
   - Configure TestPyPI Trusted Publisher for project `hl7v2`.
   - Run `Python TestPyPI Proof` from `main` with `publish_to_testpypi=true`.
   - Record upload, install-back, `import hl7v2`, `smoke.py`, and
     `evidence_workflow_guide.py`.
   - Do not use token fallback or `skip-existing`.

2. **Evidence parity maintenance**
   - Keep Rust, CLI, REST/gRPC, and local Python wheel parity routed through the
     shared fixtures and `xtask` acceptance commands.
   - Keep public Python parity local-wheel-scoped until TestPyPI/PyPI
     install-back exists.
   - Keep gRPC Beta until transport lifecycle and operational hardening catch up
     with the artifact semantics already covered by contract tests.

3. **Operator workflow hardening**
   - Keep the safe support-bundle, sidecar, dirty-corpus, and artifact
     interpretation guides aligned with executable smoke tests.
   - Keep RIPR advisory while hosted calibration samples accumulate.

## Next

1. **Production PyPI decision**
   - Decide explicitly after same-commit TestPyPI proof exists.
   - Publish production PyPI only with upload and install-back receipts.

2. **Public Python parity promotion**
   - Promote Python evidence parity from local wheel proof to public package
     proof only after public registry install-back passes.

3. **Real-world corpus expansion**
   - Expand synthetic/redacted dirty HL7 fixtures for vendor-shaped ADT/ORU
     data, Z-segments, malformed delimiters, legacy timestamp formats, large
     OBX payloads, partial batches, and MLLP wrapper/failure traces.

## Later

1. **TypeScript and WASM**
   - Plan and implement only after Python public proof is resolved or
     deliberately parked.
   - Public npm package identity is `@effortlessmetrics/hl7v2`, not
     `hl7v2-rs`.
   - Rust backend crates such as `hl7v2-wasm` or `hl7v2-node` remain binding
     infrastructure, not a return to public parser/model/redaction microcrates.

2. **Broader deployment polish**
   - Continue making sidecar deployment, container smoke, and operator
     evidence handoff boring.
   - Keep expensive verification routed by risk instead of making it an
     ordinary PR tax.

## Boundaries

- Do not split parser, model, redaction, MLLP, batch, or stream internals back
  into public Rust crates.
- Do not claim TestPyPI, PyPI, npm, tag, GitHub release, or crates.io success
  without registry resolution or upload/install-back proof.
- Do not treat `hl7v2-python` crates.io publication as public Python package
  proof.
- Do not start npm/WASM implementation until Python public proof is resolved or
  explicitly parked.
