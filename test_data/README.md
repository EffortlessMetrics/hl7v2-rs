# Test Data Fixtures

This directory contains synthetic HL7 v2 fixtures used by unit tests,
integration tests, evidence parity checks, examples, and first-use guides. The
fixtures are intentionally small and reviewable. Do not add real patient data,
live interface exports, secrets, or vendor payloads here.

For user-facing evidence workflows, start with the guide fixtures and the dirty
real-world corpus:

| Path | Purpose |
| --- | --- |
| `valid_message.hl7` | Valid ADT-style message used by validation, guide, and smoke paths. |
| `invalid_message.hl7` | Parseable ADT-style message with validation issues for error, quarantine, and support-bundle proof. |
| `test_message.hl7`, `test_message2.hl7`, `test.hl7` | General parser and CLI sample messages. |
| `ack_message.hl7` | ACK sample for ACK parsing and writer paths. |
| `custom_delimiters.hl7` | Message using non-standard delimiters for delimiter parsing and normalization checks. |
| `test_mllp.hl7` | MLLP-framed sample for transport and CLI smoke coverage. |
| `dirty-real-world/` | Synthetic vendor-shaped corpus for dirty HL7 compatibility, corpus summary, fingerprint, diff, redaction, bundle, and replay proof. |

## Message Fixture Families

| Fixture family | Files | Used for |
| --- | --- | --- |
| Valid ADT samples | `valid_message.hl7`, `correct_test.hl7`, `proper_test.hl7`, `working_test.hl7`, `simple_test.hl7` | Baseline parsing, writing, validation, examples, and guide smokes. |
| Invalid or edge samples | `invalid_message.hl7`, `non_canonical.hl7` | Safe diagnostics, validation failures, delimiter normalization, and support-bundle flows. |
| Encoding samples | `ascii_test.hl7`, `utf8_test.hl7`, `test_utf8.hl7`, `test_utf8_fixed.hl7`, `test_utf8_proper.hl7` | ASCII and UTF-8 parser/writer behavior. |
| Normalization samples | `normalized.hl7`, `custom_delimiters.hl7` | Canonical delimiter output and parse-after-write checks. |
| MLLP samples | `test_mllp.hl7` | MLLP wrapper, unwrapping, and transfer-framing tests. |
| ACK samples | `ack_message.hl7` | ACK parsing, ACK metadata, and response writer checks. |

## Profile And Template Fixtures

| File | Purpose |
| --- | --- |
| `test_profile.yaml` | Minimal ADT profile for validation tests. |
| `test_template.yaml` | Static ADT message template. |
| `dynamic_template.yaml` | Template with generated value rules. |
| `advanced_template.yaml`, `realistic_template.yaml` | Broader synthetic message templates for examples and generator coverage. |

## Evidence And Security Fixtures

| Path | Purpose |
| --- | --- |
| `evidence/schema-version-parity.json` | Shared schema-version parity fixture used by Rust, CLI, server, and Python proof paths. |
| `security/safe-error-phi-parity.json` | Shared safe-error and PHI sentinel fixture. |
| `dirty-real-world/README.md` | Detailed inventory for the synthetic dirty corpus categories. |

The dirty corpus covers Z-segments, large OBX payloads, legacy encoding
metadata, odd MSH metadata, vendor ORU null/text results, already-redacted
support bundle payloads, malformed delimiters, partial batch input, and
generated MLLP frames. Keep additions synthetic or redacted, and update
`dirty-real-world/README.md` when a new category is added.

## Adding Fixtures

When adding a fixture:

1. Keep the payload synthetic or fully redacted.
2. Prefer extending an existing family before creating a new one.
3. Document the purpose in this README or the nested corpus README.
4. Add or update the test, guide, or parity command that consumes it.
5. Do not add PHI, credentials, local filesystem paths, or real interface
   partner identifiers.

Useful checks for fixture-backed evidence claims:

```bash
cargo +1.95.0 run -p xtask -- check-dirty-corpus-parity
cargo +1.95.0 run -p xtask -- check-safe-error-phi-parity
cargo +1.95.0 run -p xtask -- check-schema-version-parity
cargo +1.95.0 run -p xtask -- check-doc-links
```

Public Python registry proof is separate from these source-checkout fixtures.
A local fixture pass does not prove TestPyPI or PyPI upload/install-back.
