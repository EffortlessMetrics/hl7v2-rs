# Sidecar Guide Smoke Receipt

Date: 2026-05-18
Branch: `test/sidecar-guide-smoke`
Scope: executable source-checkout proof for the HTTP deployment sidecar guide.

## Purpose

This receipt records the guide-level command that backs
`docs/guides/deploy-validation-sidecar.md` with an executable HTTP sidecar
smoke:

```text
cargo +1.95.0 run -p xtask -- check-sidecar-guide
```

The command proves the local, non-registry sidecar path:

- writes the guide's `target/hl7v2-sidecar/server.toml`;
- writes the guide's fail-closed safe-analysis policy;
- builds `hl7v2-server`;
- verifies `hl7v2-server --print-config` reports sanitized configuration
  without leaking the API key value;
- chooses an ephemeral loopback port for the executable proof, while the manual
  guide keeps `127.0.0.1:18080` as a copy/paste example;
- verifies the generated config reports that exact ephemeral bind address;
- still fails closed when a selected bind address is occupied instead of
  passing against a stale sidecar;
- starts a local HTTP sidecar on the selected loopback address;
- runs `tests/server_smoke/smoke.py` against that sidecar with the guide API
  key and URL;
- runs `tests/server_smoke/guide_quarantine.py` against the same sidecar to
  prove the guide's invalid-message validate-redacted quarantine path,
  `ack-policy` rejection, corpus diff, metrics output, and PHI sentinels;
- shuts down the spawned sidecar after the smoke completes.

The server smoke covers health, readiness, redacted validation, bundle, replay,
corpus diff, guide-specific invalid validation/quarantine, ACK policy, metrics,
and PHI-sentinel checks against the running sidecar.

## Non-Claims

- This receipt does not upload to TestPyPI or PyPI.
- This receipt does not prove `pip install hl7v2` from a public Python
  registry.
- This receipt does not publish or prove an npm package.
- This receipt does not create a new crates.io, tag, or GitHub release claim.
- This receipt does not promote `hl7v2-python` as the recommended Rust API.
- This receipt does not prove gRPC or Docker sidecar deployment; those remain
  covered by their dedicated tests and hosted smoke workflows.

## Validation

```text
cargo +1.95.0 fmt --all -- --check
cargo +1.95.0 clippy -p xtask --all-targets --locked -- -D warnings
cargo +1.95.0 test -p xtask check_sidecar_guide --locked
cargo +1.95.0 test -p xtask sidecar_guide_config_uses_selected_port --locked
cargo +1.95.0 test -p xtask ensure_tcp_port_available_rejects_bound_address --locked
cargo +1.95.0 test -p xtask --locked
cargo +1.95.0 run -p xtask -- check-sidecar-guide
cargo +1.95.0 run -p xtask -- check-doc-links
cargo +1.95.0 run -p xtask -- check-file-policy
cargo +1.95.0 run -p xtask -- check-no-panic-family
cargo +1.95.0 run -p xtask -- check-lint-policy
cargo +1.95.0 run -p xtask -- badges --check
cargo +1.95.0 run -p xtask -- impacted-evidence
cargo +1.95.0 run -p xtask -- impacted-evidence --check
git diff --check
```
