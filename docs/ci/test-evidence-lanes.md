# Test Evidence Lanes

`hl7v2-rs` keeps verification deep by routing proof to the risk surface that
needs it. Ordinary PRs should get fast policy, lint, unit, doc, and targeted
test feedback. Expensive runtime lanes remain available for labels, main,
nightly, release readiness, and high-risk changes.

## Lane Split

| Lane | Default PR? | Evidence |
| --- | :---: | --- |
| Format, lint, no-panic, file policy | yes | `xtask gate`, `check-lint-policy`, `check-no-panic-family`, `check-file-policy` |
| Unit and doc tests | yes | `cargo test --lib`, `cargo test --doc` |
| Standard integration and BDD tests | yes | CLI/server integration and BDD job logs |
| MSRV smoke | yes | Current declared MSRV compile check |
| Platform matrix | no | main, merge queue, `platform-matrix`, `full-ci`, or `release-check` |
| Extended property tests | no | main, dispatch, `property-tests`, `full-ci`, or `release-check` |
| Benchmarks | no | main, dispatch, `benchmarks`, `full-ci`, or `release-check` |
| Coverage | no | main, dispatch, `coverage`, or `full-ci` |
| Contracts | no | OpenAPI, proto, schema, and evidence validation |
| Python wheels and publish proof | no | Python-lane workflows and receipts |
| `ripr` static exposure | advisory | JSON, SARIF, markdown, badge, and impacted-evidence artifacts; see the [2026-05-18 calibration audit](../audits/ripr-calibration-2026-05-18.md) |
| Runtime mutation | targeted advisory | high-risk path changes, `mutation`, `full-ci`, `release-check`, nightly, and release readiness |

## Rust 1.95 Target

The Rust 1.95 / 1.5.0 rollout should keep default PR cost bounded while adding
better receipts:

- Rust 1.95 MSRV smoke remains a default compatibility proof after the MSRV PR.
- `ripr` is a cheap advisory PR-time static mutation-exposure signal.
- Runtime mutation remains targeted by risk pack, label, nightly, or release
  readiness.
- Release readiness owns the broadest proof bundle before `1.5.0`.

## Runtime Mutation Routing

Runtime mutation is not a default required PR tax. The targeted mutation
workflow plans against changed files, then runs `cargo-mutants` only when a PR
touches a high-risk HL7 surface, carries the `mutation`, `full-ci`, or
`release-check` label, or is invoked manually.

The targeted lane uploads the changed-file plan and mutation artifacts. It is
review input for PRs; nightly and release-readiness lanes remain the broader
runtime backstop.

## High-Risk HL7 Surfaces

The following surfaces justify targeted deeper proof:

- message parser and delimiter handling;
- MLLP framing;
- profile validation;
- safe-analysis redaction;
- evidence bundle and replay hashes;
- schema and evidence artifact contracts;
- server auth and rate-limit policy;
- Python binding parity;
- release and publish behavior.

## Related Docs

- [Cost and Verification Policy](cost-and-verification-policy.md)
- [Verification Ladder](verification-ladder.md)
- [CI Lane Inventory](inventory.md)
- [ripr](ripr.md)
