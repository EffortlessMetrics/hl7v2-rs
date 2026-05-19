# HL7V2-SPEC-0009: User Journey Acceptance

Status: Accepted
Date: 2026-05-19
Source-of-truth stack: [HL7V2-SPEC-0001](HL7V2-SPEC-0001-source-of-truth-stack.md)
Related parity spec: [HL7V2-SPEC-0006](HL7V2-SPEC-0006-cross-surface-evidence-parity.md)

## Contract

Any guide that presents a copy-paste evidence workflow must either be executable with an xtask smoke runner, or be explicitly marked conceptual/non-executable.

## Machine Rails

- `policy/user-journey-guides.toml`
- `cargo +1.95.0 run -p xtask -- check-first-use-guides`
- `cargo +1.95.0 run -p xtask -- check-first-10-minutes-guide`
- `cargo +1.95.0 run -p xtask -- check-first-use-by-surface-guide`
- `cargo +1.95.0 run -p xtask -- check-evidence-artifacts-guide`
- `cargo +1.95.0 run -p xtask -- check-safe-support-bundle-guide`
- `cargo +1.95.0 run -p xtask -- check-vendor-upgrade-diff-guide`
- `cargo +1.95.0 run -p xtask -- check-operator-error-guidance-guide`
- `cargo +1.95.0 run -p xtask -- check-sidecar-guide`
