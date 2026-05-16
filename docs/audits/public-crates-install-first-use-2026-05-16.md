# Public Crates Install And First-Use Smoke

Date: 2026-05-16

Scope: repeatable public install-back proof for the already-published v1.5.0
Rust library, CLI, and server crates after the current-main evidence parity
refresh. This receipt does not upload or publish any package.

## Summary

The checked-in smoke script installed the public crates.io artifacts into a
disposable scratch directory and exercised the first-use paths a new user would
run:

| Surface | Proof |
| --- | --- |
| Rust library | `cargo new`, `cargo add hl7v2@1.5.0`, parse, validate, normalize, and ACK |
| CLI | `cargo install hl7v2-cli --version 1.5.0`, then `hl7v2-cli doctor --format json` |
| Server | `cargo install hl7v2-server --version 1.5.0`, `--print-config`, start on a local loopback port, then `/ready` |

The script removes its scratch directory on success.

## Command

```powershell
python tests/public_crates_smoke/smoke.py --version 1.5.0
```

The run used the default scratch parent:

```text
F:\cargo-target
```

and created this temporary directory:

```text
F:\cargo-target\hl7v2-rs-public-crates-smoke-c53dc3e6
```

The directory was removed after the successful run.

## Observed Result

```text
scratch=F:\cargo-target\hl7v2-rs-public-crates-smoke-c53dc3e6
+ cargo +1.95.0 new rust-smoke --bin
+ cargo +1.95.0 add hl7v2@1.5.0
+ cargo +1.95.0 add serde_json
+ cargo +1.95.0 run --quiet
+ cargo +1.95.0 install hl7v2-cli --version 1.5.0 --root F:\cargo-target\hl7v2-rs-public-crates-smoke-c53dc3e6\cli-install
+ F:\cargo-target\hl7v2-rs-public-crates-smoke-c53dc3e6\cli-install\bin\hl7v2-cli.exe doctor --format json
+ cargo +1.95.0 install hl7v2-server --version 1.5.0 --root F:\cargo-target\hl7v2-rs-public-crates-smoke-c53dc3e6\server-install
+ F:\cargo-target\hl7v2-rs-public-crates-smoke-c53dc3e6\server-install\bin\hl7v2-server.exe --print-config
{
  "cli": "pass",
  "npm_registry": "not tested",
  "python_registry": "not tested",
  "rust_library": "pass",
  "server": "pass",
  "version": "1.5.0"
}
removed scratch=F:\cargo-target\hl7v2-rs-public-crates-smoke-c53dc3e6
```

## What The Smoke Checks

- `hl7v2@1.5.0` resolves from crates.io in a clean scratch binary project.
- A minimal Rust program can parse an HL7 message, validate it with an inline
  profile, normalize it, and generate an `AA` ACK with the source control ID.
- `hl7v2-cli@1.5.0` installs from crates.io and `doctor --format json` reports
  version `1.5.0`.
- `hl7v2-server@1.5.0` installs from crates.io, prints configuration, starts on
  a loopback port, and returns HTTP `200` from `/ready` with version `1.5.0`.

## Non-Claims

- No crates.io upload occurred.
- No new tag or GitHub release was created.
- No TestPyPI upload occurred.
- No PyPI upload occurred.
- No public Python `hl7v2` install-back was run.
- No npm package was tested or published.
- This does not make `hl7v2-python` the recommended Rust API.
