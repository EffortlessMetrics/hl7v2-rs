# ADR-011: Cross-Field Rule Semantics

## Status

**ACCEPTED** - Implementation complete

## Context

Cross-field rules in hl7v2-prof currently have ambiguous semantics that cause test failures and unclear behavior. The issue surfaces in `test_temporal_before_with_same_date_partial_precision` where a "before" condition is expected to generate validation errors when NOT met, but the current implementation only executes actions when conditions ARE met.

### Current Implementation

```yaml
cross_field_rules:
  - id: "date-before-date"
    description: "PV1 date should be before ORC date"
    conditions:
      - field: "PV1.10"
        operator: "before"
        value: "ORC.4"
    actions: []
```

Current behavior:
- Conditions are preconditions for actions
- When conditions are TRUE → execute actions
- When conditions are FALSE → do nothing
- Empty actions → no validation errors ever

### Problem

There are two competing interpretations:

1. **Actions-based** (current): Conditions determine when to execute actions
   - Flexible but requires non-empty actions to generate errors
   - Natural for "if X then require Y" logic

2. **Validation-based** (test expects): Conditions are assertions that should fail
   - Simpler for basic validation rules
   - Natural for "X must be before Y" logic

## Decision

We will support **both** interpretations through a `validation_mode` field:

```yaml
cross_field_rules:
  - id: "date-validation"
    description: "PV1 date should be before ORC date"
    validation_mode: "assert"  # NEW: "assert" or "conditional" (default)
    conditions:
      - field: "PV1.10"
        operator: "before"
        value: "ORC.4"
    # For "assert" mode, actions are optional
    # If omitted, a generic validation error is generated when conditions fail
```

### Modes

#### 1. Conditional Mode (default, backward compatible)
- **Semantics**: "If conditions are met, execute actions"
- **Use case**: "If patient is pediatric, require guardian info"
- **Behavior**:
  - Conditions TRUE + actions defined → execute actions
  - Conditions TRUE + no actions → no-op
  - Conditions FALSE → no-op

#### 2. Assert Mode (new)
- **Semantics**: "Conditions must be true, fail otherwise"
- **Use case**: "Admission date must be after birth date"
- **Behavior**:
  - Conditions TRUE → no-op (validation passes)
  - Conditions FALSE → generate validation error

### Schema Changes

```rust
pub struct CrossFieldRule {
    pub id: String,
    pub description: String,
    /// Validation mode: "conditional" (default) or "assert"
    #[serde(default = "default_validation_mode")]
    pub validation_mode: String,
    pub conditions: Vec<RuleCondition>,
    pub actions: Vec<RuleAction>,
}

fn default_validation_mode() -> String {
    "conditional".to_string()
}
```

## Implementation Plan

### Phase 1: Add validation_mode field
```rust
// 1. Update CrossFieldRule struct
// 2. Update validation logic in validate_cross_field_rule()
// 3. Add tests for both modes
```

### Phase 2: Update failing test
```yaml
# Change test to use assert mode
cross_field_rules:
  - id: "date-before-date"
    description: "PV1 date should be before ORC date"
    validation_mode: "assert"
    conditions:
      - field: "PV1.10"
        operator: "before"
        value: "ORC.4"
```

### Phase 3: Documentation
- Update profile schema documentation
- Add examples for both modes
- Migration guide for existing profiles

## Consequences

### Positive
- ✅ Clear semantics for both use cases
- ✅ Backward compatible (default is current behavior)
- ✅ Fixes test without breaking existing profiles
- ✅ More intuitive for simple validation rules
- ✅ Maintains flexibility for complex conditional logic

### Negative
- ⚠️ Adds complexity with two modes
- ⚠️ Requires documentation and examples
- ⚠️ Migration effort for profiles using workarounds

### Neutral
- Profile authors need to understand both modes
- Test suite grows to cover both modes

## Alternatives Considered

### Alternative 1: Always treat conditions as assertions
- **Rejected**: Breaks backward compatibility
- **Rejected**: Loses flexibility for conditional validation

### Alternative 2: Always require non-empty actions
- **Rejected**: Verbose for simple validations
- **Rejected**: Confusing error messages for "assert" use cases

### Alternative 3: Separate rule types (AssertRule vs ConditionalRule)
- **Rejected**: More complex schema
- **Rejected**: Harder to migrate existing profiles

## References

- Test: `test_temporal_before_with_same_date_partial_precision`
- Code: `crates/hl7v2-prof/src/lib.rs::validate_cross_field_rule`
- Related: HL7 conformance profile specification

## Implementation Status

- [x] Schema changes (added validation_mode field to CrossFieldRule)
- [x] Validation logic updated (validate_cross_field_rule handles both modes)
- [x] Tests added for assert mode (test_temporal_before_with_same_date_partial_precision)
- [x] Tests passing for conditional mode (existing tests maintain backward compatibility)
- [ ] Documentation updated (pending)
- [ ] Migration guide written (pending)
- [x] Failing test fixed (7/7 tests passing)

## Timeline

- **Proposed**: 2025-11-19
- **Accepted**: 2025-11-19
- **Implemented**: 2025-11-19

---

**Author**: Claude
**Reviewers**: [TBD]
**Last Updated**: 2025-11-19
