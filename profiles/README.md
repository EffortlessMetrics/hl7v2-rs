# HL7v2 Conformance Profiles

This directory contains the small, checked-in profiles used by the repository's
current CLI and library examples. Profiles are YAML contracts that describe the
message structure, required fields, value sets, and other validation rules for
an HL7v2 message.

## Start here

| Profile | Message shape | Use it for |
| --- | --- | --- |
| [`generic.yaml`](generic.yaml) | `GENERIC` | A minimal baseline for custom profiles |
| [`adt_a01.yaml`](adt_a01.yaml) | `ADT^A01` | Admit/visit notifications |
| [`oru_r01.yaml`](oru_r01.yaml) | `ORU^R01` | Observation and laboratory results |

The built-in ADT and ORU profiles inherit common constraints from
`generic.yaml`. The expanded message-type examples are grouped under
[`profiles/examples`](examples/), and the standalone reference fixtures under
[`examples/profiles`](../examples/profiles/) have a separate guide.

## Inspect and use a profile

From an installed CLI:

```bash
# Check the profile before using it as an interface contract
hl7v2-cli profile lint profiles/adt_a01.yaml

# Inspect its resolved structure and rules
hl7v2-cli profile explain profiles/adt_a01.yaml --format json

# Validate a message against it
hl7v2-cli val message.hl7 --profile profiles/adt_a01.yaml --report json
```

From a source checkout, prefix the same arguments with
`cargo run -q -p hl7v2-cli --`.

For the complete first-use flow, including sample generation, reports, and
evidence bundles, see [First Use By Surface](../docs/guides/first-use-by-surface.md).

## Create a custom profile

Copy the closest built-in profile, preserve the required `message_structure` and
`version` fields, then add the constraints required by the integration:

```bash
cp profiles/adt_a01.yaml profiles/my_adt_a01.yaml
hl7v2-cli profile lint profiles/my_adt_a01.yaml
```

Use field paths such as `PID.3` and `PID.5[1].1` for constraints. Keep business
rules explicit and document why an institution-specific rule exists. The
[`examples/profiles` guide](../examples/profiles/README.md) contains larger
examples of segment constraints, value sets, cross-field rules, and temporal
rules.

## Adding a profile

When adding a profile to this directory:

1. Keep the filename lowercase and descriptive, for example `adt_a08.yaml`.
2. Include a message structure and HL7 version.
3. Run the profile linter and the relevant CLI or library validation tests.
4. Add or update a nearby README entry so users can discover its message type.
