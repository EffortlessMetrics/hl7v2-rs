# ADR-0004: Schema-Driven Validation

**Status**: Accepted

**Date**: 2025-11-19

**Deciders**: Architecture team

**Technical Story**: HL7v2 message validation requirements differ across facilities, message types, and HL7 versions. A static, hardcoded validation approach cannot accommodate the diversity of real-world deployments without frequent code changes and redeployments.

## Context

HL7v2 messages vary widely between implementations. Different healthcare facilities, message types (ADT, ORU, ORM, etc.), and HL7 versions (2.3 through 2.9) impose different validation requirements. For example:

- An ADT^A01 admission message requires patient class (`PV1.2`) and patient ID (`PID.3`), while a generic message may only require `PID.3`.
- An ORU^R01 result message must validate that `OBX.5` contains a numeric value when `OBX.2` is `NM`, a conditional rule that does not apply to other message types.
- Different facilities may accept different administrative sex codes (`F`, `M`, `O`, `U`, `A`, `N`) or restrict them to a subset.

The validation engine must handle:

1. **Data type validation** across 13 HL7 types (ST, ID, DT, TM, TS, NM, SI, TX, FT, IS, PN, CX, HD)
2. **Format validation** for domain-specific patterns (phone numbers, email addresses, SSN)
3. **Checksum validation** for identifiers (Luhn, Mod10)
4. **Temporal validation** with precision-aware timestamp comparison (year through second granularity)
5. **Cross-field validation** where the validity of one field depends on another field's value
6. **Contextual validation** where rules activate based on message context (e.g., patient class)
7. **Value set validation** against HL7 tables and custom code sets

Hardcoding all of these rules for every message type and facility combination is not sustainable. We need a way to define validation requirements externally and compose them through reuse.

## Decision

We will implement schema-driven validation using a two-layer architecture:

1. **`hl7v2-validation` crate** -- A rule-based validation engine providing the runtime evaluation primitives: data type checkers, format validators, checksum algorithms, temporal comparisons, and a cross-field rule evaluator (`check_rule_condition`).

2. **`hl7v2-prof` crate** -- A profile definition and loading layer that reads YAML profile files, supports profile inheritance via a `parent` field, merges parent and child profiles, and drives the validation engine.

Profiles are defined in YAML and validated against a JSON Schema (`schemas/profile/profile-v1.schema.json`). Profile inheritance allows a base profile (e.g., `generic.yaml`) to define common rules that specialized profiles (e.g., `adt_a01.yaml`, `oru_r01.yaml`) extend and override.

**Rationale:**

1. **Separation of concerns** -- Validation logic (Rust code) is decoupled from validation policy (YAML configuration), allowing policy changes without recompilation.
2. **Composability** -- Profile inheritance enables a layered approach: organization-wide base rules, message-type-specific rules, and facility-specific overrides.
3. **Auditability** -- YAML profiles are human-readable, version-controllable, and reviewable by compliance teams who may not read Rust.
4. **Extensibility** -- New constraint types, rule types, and validation modes can be added to the engine without changing existing profiles.

## Consequences

### Positive

- **Zero-code policy changes**: Operators can modify validation behavior by editing YAML profiles without rebuilding the application.
- **Profile reuse**: The inheritance system (`parent` field) eliminates duplication across related message types. Both `adt_a01.yaml` and `oru_r01.yaml` inherit from `generic.yaml`.
- **Rich validation**: 14 condition operators (`Eq`, `Ne`, `Gt`, `Lt`, `Ge`, `Le`, `In`, `Contains`, `Exists`, `Missing`, `MatchesRegex`, `IsDate`, `Before`, `WithinRange`) cover the full range of HL7v2 validation needs.
- **Type safety**: The validation engine uses Rust's type system (`Severity`, `Issue`, `ConditionOperator`, `RuleCondition`, `RuleAction` structs) to prevent malformed rules at compile time.
- **Remote loading**: The `ProfileLoader` supports loading profiles from HTTP/HTTPS URLs with ETag-based caching, enabling centralized profile management.
- **Schema-validated configuration**: The JSON Schema for profiles catches structural errors before the Rust code ever sees them.

### Negative

- **Runtime errors possible**: Invalid field paths or malformed regex patterns in profiles are only caught at validation time, not at profile load time.
- **YAML complexity**: Profiles with many cross-field rules, temporal rules, and contextual rules can become large and hard to review.
- **Performance overhead**: Each validation run parses field paths, compiles regex patterns, and resolves cross-field references dynamically. This is acceptable for healthcare message rates but adds latency compared to compiled validators.
- **Two validation systems**: Having both `hl7v2-validation` (programmatic) and `hl7v2-prof` (declarative) means developers must understand both layers.

### Neutral

- **YAML as format**: YAML was chosen over JSON for readability and over TOML for nested structure support. This is a stylistic preference.
- **Profile merge semantics**: Child profiles override parent constraints on the same path and parent rules with the same ID. This is intuitive but may surprise users expecting additive merging.

## Alternatives Considered

### Alternative 1: Hardcoded Validation Rules

**Pros:**
- Compile-time type safety for all rules
- Maximum performance with no parsing overhead
- Simpler codebase with no YAML/JSON Schema dependencies

**Cons:**
- Every validation change requires a code change and redeployment
- Rules for different facilities cannot coexist without feature flags or conditional compilation
- Non-developer stakeholders (compliance officers, interface analysts) cannot review or modify rules

**Why not chosen:**
Healthcare environments require frequent, facility-specific validation adjustments. Recompiling and redeploying for each change is operationally unacceptable.

### Alternative 2: XSD/XML Schema Validation

**Pros:**
- Industry-standard schema language with tooling support
- Well-understood by healthcare IT teams familiar with HL7v3/CDA
- Mature ecosystem of validators

**Cons:**
- HL7v2 is not XML-based; mapping pipe-delimited messages to XML adds a conversion layer
- XSD cannot express cross-field validation rules (e.g., "if PV1.2 = I then PV1.3 is required")
- Verbose syntax increases maintenance burden
- Poor fit for temporal validation and checksum verification

**Why not chosen:**
Impedance mismatch between HL7v2's pipe-delimited format and XML schema validation. Cross-field rules and temporal logic would still need a separate engine.

### Alternative 3: Custom Domain-Specific Language (DSL)

**Pros:**
- Could be tailored precisely to HL7v2 validation semantics
- Potentially more expressive than YAML for complex rules
- Could support type checking and static analysis

**Cons:**
- Requires designing, implementing, and documenting a new language
- Learning curve for users who must learn yet another syntax
- Parser and interpreter maintenance burden
- No existing tooling (syntax highlighting, linting, IDE support)

**Why not chosen:**
The development cost of a custom DSL is disproportionate to the benefit. YAML profiles with a JSON Schema provide adequate expressiveness with zero language design effort.

### Alternative 4: Schematron-Style Assertions

**Pros:**
- Pattern-based assertions are natural for validation
- Can express complex cross-field rules using XPath-like expressions
- Established standard (ISO/IEC 19757-3)

**Cons:**
- Designed for XML documents, not pipe-delimited HL7v2
- Would require an XPath-like expression language for HL7 paths
- Limited ecosystem in Rust
- Heavier runtime than YAML profile evaluation

**Why not chosen:**
Similar impedance mismatch as XSD. The HL7v2 path syntax (`PID.5[1].1`) already provides field addressing; wrapping it in Schematron patterns adds complexity without benefit.

## Implementation Notes

### Profile Structure

Profiles are defined in YAML with the following top-level fields:

```yaml
message_structure: "ADT_A01"
version: "2.5.1"
parent: "GENERIC"              # Optional: inherit from parent profile
message_type: "ADT^A01^ADT_A01"
segments:
  - id: "MSH"
  - id: "EVN"
  - id: "PID"
  - id: "PV1"
constraints:
  - path: "PID.3"
    required: true
  - path: "PV1.2"
    required: true
valuesets:
  - path: "PID.8"
    name: "HL70001"
    codes: ["F", "M", "O", "U", "A", "N"]
lengths:
  - path: "PID.5[1].1"
    max: 80
```

### Validation Engine Architecture

The `hl7v2-validation` crate exposes a `Validator` trait and a `check_rule_condition` function:

```rust
pub trait Validator {
    fn validate(&self, msg: &Message) -> ValidationResult;
}

pub fn check_rule_condition(msg: &Message, condition: &RuleCondition) -> bool {
    let lhs = get_nonempty(msg, &condition.field);
    match condition.operator.as_str() {
        "eq" | "ne" | "contains" | "in" | "matches_regex"
        | "exists" | "not_exists" | "is_date" | "before"
        | "within_range" => { /* evaluate */ }
        _ => false,
    }
}
```

### Profile Inheritance and Merging

The `merge_profiles` function in `hl7v2-prof` combines parent and child profiles with child-wins semantics:

```rust
pub fn merge_profiles(parent: Profile, child: Profile) -> Profile {
    Profile {
        message_structure: child.message_structure,
        version: child.version,
        segments: merge_segment_specs(parent.segments, child.segments),
        constraints: merge_constraints(parent.constraints, child.constraints),
        // ... additional fields merged similarly
    }
}
```

Child constraints override parent constraints on the same path. Child rules override parent rules with the same ID. Segments are merged by union (no duplicates by segment ID).

### Conditional Validation

Profiles support conditional constraints using the `when` clause:

```yaml
constraints:
  - path: "OBX.5"
    when:
      eq: ["OBX.2", "NM"]
    pattern: "^-?[0-9]+(\\.[0-9]+)?$"
```

### Profile Loading

The `ProfileLoader` supports local files and remote URLs with LRU caching and ETag-based conditional requests:

```rust
let loader = ProfileLoader::builder()
    .cache_size(100)
    .timeout(Duration::from_secs(30))
    .build();

let result = loader.load("profiles/adt_a01.yaml").await?;
let result = loader.load("https://profiles.example.com/oru_r01.yaml").await?;
```

### JSON Schema Validation

Profiles are validated against `schemas/profile/profile-v1.schema.json` which defines:
- Required fields: `message_structure`, `version`
- Constraint types: `required`, `length`, `pattern`, `table`, `data_type`
- Rule types: `requires`, `prohibits`, `validates`, `temporal`, `contextual`
- Severity levels: `error`, `warning`, `info`
- HL7 version enumeration: 2.3 through 2.9

## References

- [HL7 Version 2 Standard](https://www.hl7.org/implement/standards/product_brief.cfm?product_id=185) -- Message structure and data type definitions
- [HL7 Conformance Profile](https://www.hl7.org/documentcenter/public/standards/V2/V2.9_Conformance_Profile.pdf) -- HL7's own conformance profiling specification
- [JSON Schema Draft-07](https://json-schema.org/specification-links.html#draft-7) -- Schema language used for profile validation
- [serde_yaml](https://docs.rs/serde_yaml/) -- YAML deserialization for profile loading
- [Luhn Algorithm](https://en.wikipedia.org/wiki/Luhn_algorithm) -- Checksum algorithm used for identifier validation
- Internal: `crates/hl7v2-validation/src/lib.rs` -- Validation engine implementation
- Internal: `crates/hl7v2-prof/src/model.rs` -- Profile data model
- Internal: `crates/hl7v2-prof/src/merge.rs` -- Profile inheritance merging
- Internal: `crates/hl7v2-prof/src/loader.rs` -- Profile loading with caching
- Internal: `schemas/profile/profile-v1.schema.json` -- Profile JSON Schema
- Internal: `profiles/generic.yaml`, `profiles/adt_a01.yaml`, `profiles/oru_r01.yaml` -- Example profiles
