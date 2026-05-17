# gRPC Enhanced ACK Parity Receipt

Date: 2026-05-16
Branch: `grpc/enhanced-ack-parity`
Scope: gRPC `GenerateAck` proto/API/runtime parity for all six supported ACK
codes.

## Summary

The gRPC `GenerateAck` surface now accepts the same ACK code set already
supported by Rust, CLI, REST, and local Python helper surfaces:

```text
AA AE AR CA CE CR
```

This closes the prior gRPC-specific commit-code gap without changing the
primary Rust product graph, Python public package state, TestPyPI/PyPI state,
npm state, tag state, or GitHub release state.

## Contract Delta

Updated proto enum values:

| Enum | Meaning |
| --- | --- |
| `ACK_CODE_AA` | Application Accept |
| `ACK_CODE_AE` | Application Error |
| `ACK_CODE_AR` | Application Reject |
| `ACK_CODE_CA` | Commit Accept |
| `ACK_CODE_CE` | Commit Error |
| `ACK_CODE_CR` | Commit Reject |

The server maps every gRPC enum value to the corresponding canonical
`hl7v2::AckCode`, then returns an ACK payload and parsed ACK message for
verification.

## Validation

```text
cargo +1.95.0 test -p hl7v2-server --test grpc_contract_tests test_grpc_generate_ack_maps_codes_and_preserves_control_id --locked
cargo +1.95.0 test -p hl7v2-server --test grpc_contract_tests --locked
cargo +1.95.0 test -p hl7v2-server --test proto_packaging_test --locked
cargo +1.95.0 test -p hl7v2-cli --test serve_grpc_contract_test --locked
npm exec --yes -- @bufbuild/buf lint api/proto
cargo +1.95.0 run -p xtask -- check-doc-links
cargo +1.95.0 run -p xtask -- check-file-policy
cargo +1.95.0 fmt --all -- --check
python -c "import pathlib,tomllib; tomllib.loads(pathlib.Path('.hl7v2/goals/active.toml').read_text()); print('active.toml ok')"
git diff --check
```

Observed result:

```text
Focused gRPC ACK contract test: passed
Full gRPC contract test: passed
Packaged proto contract test: passed
CLI gRPC serve contract test: passed
buf proto lint: passed
doc links: passed
file policy: passed
rustfmt check: passed
active.toml parse: passed
diff whitespace: passed
```

## Non-Claims

- No crates.io upload.
- No TestPyPI upload.
- No PyPI upload.
- No npm package.
- No tag or GitHub release.
- No production Python package install-back.
- No change to `hl7v2-python` as binding backend infrastructure.
