# Evidence Artifact Compatibility Policy

This policy defines what downstream users can rely on when they consume
HL7v2 evidence JSON from Rust, CLI, server, Python, and future TypeScript
surfaces. It is the contract-level companion to the
[Evidence Contract Index](evidence-contract-index.md) and the versioning
details in
[Evidence Provenance And Versioning](../architecture/evidence-provenance-versioning.md).

The short rule is:

```text
schemas define artifact shape
fixtures prove representative output
producer docs define how to request it
receipts prove the current release emitted it
```

## Scope

This policy covers machine-readable evidence artifacts, including:

- doctor reports;
- validation reports;
- profile lint, explain, and test reports;
- corpus summary, fingerprint, and diff reports;
- safe-analysis redaction outputs;
- redaction receipts;
- quarantine output summaries;
- evidence bundle summaries, manifests, environments, and field-path traces;
- evidence replay reports;
- safe error artifacts where they are exposed as JSON.

It does not define parser semantics, validation rule behavior, ACK policy,
redaction policy matching, or corpus diff algorithms. Those are owned by their
feature specs, schemas, tests, and release receipts.

## Stability Classes

| Class | Meaning | Examples |
| --- | --- | --- |
| Stable contract field | Consumers may parse and assert this field across patch releases for the same schema version. | Required schema fields, enum values documented in schemas, `schema_version`, `tool_name`, `tool_version`, bundle-relative artifact paths, SHA-256 hashes. |
| Stable behavior | Consumers may rely on the behavior when the relevant support tier is Stable and a proof command covers it. | v1 default output, explicit v2 opt-in, manifest hash replay checks, unsupported schema-version rejection. |
| Advisory context | Useful for humans or triage, but consumers should not make brittle automation decisions on exact wording or ordering. | Human-readable issue messages, display labels, help text, sorted-but-nonsemantic diagnostic lists. |
| Private implementation detail | Not part of the artifact contract even if visible in code. | Rust struct layout, helper function names, temporary target paths, internal fixture builders. |

If a field is not present in a checked-in schema or documented producer output,
it is not stable just because a current implementation happens to emit it.

## Semver Rules

Evidence artifacts are product contracts. Treat these as semver-relevant
changes:

- removing a stable field from an existing schema version;
- renaming a stable field;
- changing a field type, unit, or nullability;
- changing an enum value or status string that consumers are expected to parse;
- making an optional field required in the same schema version;
- changing default output from v1 to v2;
- adding raw HL7, local absolute paths, raw server bundle IDs, API keys, tokens,
  environment variables, or raw policy paths to an artifact;
- weakening replay, redaction, quarantine, PHI sentinel, or safe-error
  behavior while preserving the same claim tier.

Allowed patch-level changes include:

- adding a new schema version while preserving older schemas and producer
  opt-ins;
- adding an optional field to a new schema version;
- improving human-readable wording when machine-readable codes and paths remain
  stable;
- adding new producer surfaces for an existing artifact when the existing
  surface output does not change;
- adding fixtures, examples, or receipts that prove already-documented
  behavior.

When in doubt, add a new schema version and keep the old producer path working.

## Schema-Version Rules

Evidence schema versions are explicit JSON contract versions. They are not the
same as domain algorithm versions such as `fingerprint_version`,
`diff_version`, `bundle_version`, `quarantine_version`, or `replay_version`.

Rules:

- v1 remains the default output shape unless a release explicitly changes the
  default.
- v2 and later shapes are explicit opt-ins until their release notes say
  otherwise.
- Producers must reject unsupported requested schema versions with safe
  diagnostics.
- Consumers should reject unknown schema versions unless they have an explicit
  compatibility mode.
- A producer may emit v1 artifacts inside a v2-capable bundle when the request
  did not opt into v2 bundle artifacts.
- A bundle may contain multiple artifact families, but each artifact's schema
  version is interpreted independently.
- Newer bundles may include manifest-hashed advisory files such as
  `SAFE-SHARING.md`. Replay must continue to accept older manifests that lack
  advisory support text as long as required evidence artifacts still verify.

## Stable Versus Advisory Fields

Stable fields are the machine-readable fields in the JSON schemas and the
producer contract. Consumers may use them for CI gates, support automation, and
release receipts.

Advisory fields and values are for humans. Do not build hard automation around:

- exact English diagnostic messages;
- order of diagnostics when the schema does not define ordering as semantic;
- profile display labels such as `ValidationReport.profile`;
- local command examples embedded in bundle environment files;
- support notes or safe-sharing text;
- tool help text.

When consumers need stable identity, prefer hashes, schema versions, issue
codes, artifact roles, bundle-relative paths, and replay check statuses.

## PHI And Shareability Rules

No compatibility change may broaden PHI exposure without an explicit security
review and release note.

Stable evidence artifacts should avoid:

- raw HL7 payloads unless the artifact is explicitly a redacted HL7 output;
- raw dropped values in redaction receipts;
- raw profile YAML in malformed-profile diagnostics;
- raw configured filesystem roots;
- raw server bundle IDs;
- API keys, tokens, environment variables, or host-specific secrets.

Safe to share does not mean universally PHI-free. Redaction receipts prove the
configured policy ran. Replay reports prove bundle reproducibility. Neither is
a blanket authorization to disclose the packet.

## Producer Obligations

Every producer surface that claims an artifact contract must provide:

- a documented way to request the artifact and schema version;
- at least one checked fixture or integration test for the output shape;
- schema validation when a JSON schema exists;
- PHI/safe-error coverage when the artifact could expose sensitive data;
- a receipt or support-tier entry for release, registry, or public package
  claims.

Producer-specific notes:

- CLI commands may write artifacts to stdout or `--output`, but stderr remains
  for diagnostics.
- Server REST and gRPC surfaces must not read caller-supplied filesystem paths
  for corpus inputs and must operate bundle/replay paths under configured
  roots.
- Python helper proof from a local wheel is not TestPyPI or PyPI proof.
- Binding backend crate publication is not proof that the public language
  package was uploaded or install-backed.
- Future TypeScript artifacts must map to the same schemas or explicitly define
  a new schema version before claiming parity.

## Consumer Guidance

Consumers should:

- validate artifacts against `schemas/evidence/*-v*.schema.json`;
- parse `schema_version` before interpreting versioned fields;
- rely on issue codes, artifact roles, hashes, and replay statuses over prose;
- treat missing optional fields as missing context, not as success or failure;
- treat explicit `null` as known-not-applicable when the schema allows it;
- fail closed on unknown schema versions unless a migration path is documented;
- keep raw HL7 out of tickets and prefer replayable redacted bundles.

## Required Proof

The compatibility policy is enforced by the same artifact proof rails as the
contract index:

```bash
cargo run -p xtask -- evidence-schema-check
cargo run -p xtask -- check-evidence-parity
cargo run -p xtask -- check-evidence-parity-acceptance
```

Surface-specific claims still need their own proof, such as CLI integration
tests, server contract tests, Python wheel install/import proof, public package
install-back, or registry resolution receipts.

## Non-Goals

- No new runtime behavior.
- No new schema version by itself.
- No TestPyPI, PyPI, npm, tag, GitHub release, or registry claim.
- No change to the primary Rust product graph or binding backend graph.
- No return to parser/model/redaction/MLLP implementation microcrates.

## Related References

- [Evidence Contract Index](evidence-contract-index.md)
- [Evidence artifacts for operators](../guides/evidence-artifacts-for-operators.md)
- [Evidence Provenance And Versioning](../architecture/evidence-provenance-versioning.md)
- [Cross-Surface Evidence Parity Spec](../specs/HL7V2-SPEC-0006-cross-surface-evidence-parity.md)
- [Schema README](../../schemas/README.md)
