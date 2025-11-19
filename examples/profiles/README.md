# HL7v2 Conformance Profile Examples

This directory contains reference conformance profiles demonstrating the capabilities of the hl7v2-prof validation engine.

## Quick Start

### Validating a Message Against a Profile

```bash
# Using the CLI (when available)
hl7v2-cli validate --profile examples/profiles/ADT_A01.yaml --message message.hl7

# Using the HTTP API
curl -X POST http://localhost:8080/hl7/validate \
  -H "Content-Type: application/json" \
  -H "X-API-Key: your-api-key" \
  -d '{
    "message": "MSH|^~\\&|...",
    "profile": "ADT_A01"
  }'
```

### Using in Rust Code

```rust
use hl7v2_prof::{load_profile, validate};
use hl7v2_core::parse;

// Load profile
let profile_yaml = std::fs::read_to_string("examples/profiles/ADT_A01.yaml")?;
let profile = load_profile(&profile_yaml)?;

// Parse message
let message_bytes = std::fs::read("message.hl7")?;
let message = parse(&message_bytes)?;

// Validate
let issues = validate(&message, &profile);

for issue in &issues {
    println!("{}: {} - {}", issue.severity, issue.code, issue.detail);
}
```

## Available Profiles

### minimal.yaml - Minimal Profile

**Purpose**: Basic structural validation only
**Use Cases**:
- Starting template for custom profiles
- Minimal overhead validation
- Testing basic HL7v2 syntax

**Validates**:
- MSH segment presence
- Basic HL7v2 delimiter syntax
- Segment structure

**Does NOT Validate**:
- Field-level content
- Value sets
- Business rules

---

### ADT_A01.yaml - Admit/Visit Notification

**Purpose**: Patient admission messages
**HL7 Version**: 2.5.1
**Message Type**: ADT^A01

**Use Cases**:
- Inpatient admission
- Outpatient registration with admission
- Emergency department admission
- Transfer from another facility

**Key Validations**:
- **Required Demographics**: Patient ID, name, birth date
- **Admission Details**: Patient class, admit date/time
- **HL7 Tables**: Administrative sex, marital status, patient class
- **Business Rules**:
  - Admission date must be after birth date
  - Discharge date must be after admission date
  - Birth date must be in the past
  - Admission date within reasonable range (≤150 years after birth)

**Segment Structure**:
```
MSH (required)
EVN (required)
PID (required)
[PD1]
[{ROL}]
[{NK1}]
PV1 (required)
[PV2]
[{DB1}]
[{OBX}]
[{AL1}]
[{DG1}]
[DRG]
[{PR1}]
```

---

### ADT_A04.yaml - Register a Patient

**Purpose**: Patient registration without admission
**HL7 Version**: 2.5.1
**Message Type**: ADT^A04

**Use Cases**:
- Outpatient clinic registration
- Pre-admission registration
- Emergency department registration (no admit)
- Patient demographic updates

**Key Validations**:
- **Required Demographics**: Patient ID, name
- **Optional Demographics**: Birth date, address, phone (flexible for registration)
- **HL7 Tables**: Administrative sex, patient class
- **Business Rules**:
  - Registration date after birth date (if birth date known)
  - Recommends contact info (address or phone)

**Differences from ADT_A01**:
- Birth date optional (may not be known at registration)
- More flexible contact information requirements
- Designed for quick registration workflows

**Segment Structure**:
```
MSH (required)
EVN (required)
PID (required)
[PD1]
[{ROL}]
[{NK1}]
PV1 (required)
[PV2]
[{DB1}]
[{OBX}]
[{AL1}]
[{DG1}]
[DRG]
[{PR1}]
```

---

### ORU_R01.yaml - Observation Results

**Purpose**: Lab results, diagnostic reports
**HL7 Version**: 2.5.1
**Message Type**: ORU^R01

**Use Cases**:
- Laboratory results (chemistry, hematology, microbiology)
- Radiology reports
- Cardiology results (EKG, echocardiogram)
- Pathology reports
- Point-of-care testing

**Key Validations**:
- **Patient Identity**: Patient ID, name (critical for safety)
- **Order Details** (OBR):
  - Filler order number (tracking)
  - Universal service ID (test identification)
  - Collection date/time
  - Result status (F=Final, P=Preliminary, etc.)
- **Result Details** (OBX):
  - Value type (NM, ST, TX, CE, etc.)
  - Observation identifier (LOINC recommended)
  - Observation value (the actual result)
  - Units (for numeric results)
  - Result status
- **HL7 Tables**: Administrative sex, value type, result status
- **Business Rules**:
  - Numeric results should include units
  - OBX status ≤ OBR status (individual results not more final than order)
  - Observation time after collection time
  - Collection time not in future
  - Results within reasonable timeframe (≤1 year)

**Segment Structure**:
```
MSH (required)
PID (required)
[PD1]
[PV1 [PV2]]
{
  [ORC]
  OBR (required)
  [{NTE}]
  [{SPM}]
  {
    OBX (required)
    [{NTE}]
  }
}
```

## Profile Features Demonstrated

### 1. Segment Constraints

```yaml
segments:
  - id: "PID"
    description: "Patient Identification"
    required: true      # Must be present
    max_uses: 1        # Can only appear once (-1 = unlimited)
```

### 2. MSH Field Validation

```yaml
msh_constraints:
  - field: "MSH.9.1"   # Message type
    required: true
    values: ["ADT"]    # Must be ADT
```

### 3. Field-Level Constraints

```yaml
field_constraints:
  - path: "PID.3"           # Patient ID
    required: true
    description: "Patient identifier required"

  - path: "PID.7"           # Birth date
    required: true
    datatype: "TS"          # Must be timestamp format
```

### 4. HL7 Table Validation

```yaml
hl7_tables:
  - id: "HL70001"
    name: "Administrative Sex"
    version: "2.5.1"
    codes:
      - code: "M"
        display: "Male"
      - code: "F"
        display: "Female"

valuesets:
  - path: "PID.8"
    name: "Administrative Sex"
    hl7_table: "HL70001"  # Link field to table
```

### 5. Cross-Field Rules

**Assert Mode** (condition must be true):
```yaml
cross_field_rules:
  - id: "admit-after-birth"
    description: "Admission must be after birth"
    validation_mode: "assert"
    conditions:
      - field: "PV1.44"
        operator: "after"
        value: "PID.7"
```

**Conditional Mode** (if-then logic):
```yaml
cross_field_rules:
  - id: "numeric-needs-units"
    description: "Numeric results need units"
    validation_mode: "conditional"
    conditions:
      - field: "OBX.2"
        operator: "eq"
        value: "NM"
    actions:
      - field: "OBX.6"
        action: "validate"
        message: "Units required for numeric results"
```

### 6. Temporal Rules

```yaml
temporal_rules:
  - id: "birth-in-past"
    description: "Birth date must be in past"
    before: "NOW"
    after: "PID.7"
    allow_equal: false
    tolerance: "1h"  # Clock skew tolerance
```

### 7. Security Guardrails

```yaml
expression_guardrails:
  max_depth: 10                  # Prevent deeply nested expressions
  max_length: 1000              # Prevent excessively long expressions
  allow_custom_scripts: false   # Disable custom script execution
```

## Customizing Profiles

### Adding Custom Value Sets

```yaml
valuesets:
  - path: "PV1.3"  # Assigned patient location
    name: "Hospital Locations"
    codes:
      - "ICU"
      - "ER"
      - "OR1"
      - "OR2"
      - "WARD3"
```

### Adding Institution-Specific Rules

```yaml
cross_field_rules:
  - id: "insurance-required-for-inpatient"
    description: "Inpatient admissions require insurance"
    validation_mode: "conditional"
    conditions:
      - field: "PV1.2"
        operator: "eq"
        value: "I"  # Inpatient
    actions:
      - field: "IN1"  # Insurance segment
        action: "require"
        message: "Insurance required for inpatient"
```

### Extending Existing Profiles

Start with a base profile and add your requirements:

```bash
cp examples/profiles/ADT_A01.yaml my_institution_ADT_A01.yaml
# Edit to add institution-specific rules
```

## Validation Workflow

1. **Parse Message**: Convert HL7v2 wire format to internal structure
2. **Load Profile**: Parse YAML conformance profile
3. **Validate Structure**: Check segment presence and order
4. **Validate Fields**: Check required fields and datatypes
5. **Validate Value Sets**: Check coded values against tables
6. **Validate Cross-Field Rules**: Check business logic
7. **Validate Temporal Rules**: Check date/time relationships
8. **Return Issues**: List of errors, warnings, and info messages

## Issue Severity Levels

- **Error**: Message violates conformance profile, should be rejected
- **Warning**: Message has issues but may be processable
- **Info**: Informational messages, recommendations

## Best Practices

### 1. Start Minimal, Add Incrementally

Begin with `minimal.yaml` and add validation rules as needed:
- Start with required segments
- Add field-level validation
- Add value sets for coded fields
- Add business rules last

### 2. Use HL7 Tables

Leverage standard HL7 tables instead of custom code lists:
- Ensures interoperability
- Reduces maintenance
- Clear semantics

### 3. Document Business Rules

Always include clear descriptions:
```yaml
cross_field_rules:
  - id: "clear-identifier"
    description: "Why this rule exists and what it checks"
```

### 4. Test with Real Messages

Validate profiles against actual messages from your integration partners:
- Catches edge cases
- Validates assumptions
- Ensures compatibility

### 5. Version Your Profiles

Track profile versions alongside HL7 versions:
```yaml
message_structure: "ADT_A01"
version: "2.5.1"
description: "Institution XYZ ADT_A01 v1.2.0"
```

### 6. Use Validation Modes Appropriately

- **Assert mode**: For mandatory business rules
- **Conditional mode**: For recommendations and complex logic

## Common Patterns

### Patient Identity Validation

```yaml
field_constraints:
  - path: "PID.3"
    required: true
    description: "At least one patient ID required"

cross_field_rules:
  - id: "mrn-format"
    description: "MRN must match institution format"
    validation_mode: "assert"
    conditions:
      - field: "PID.3.5"
        operator: "matches_regex"
        value: "^MRN-[0-9]{7}$"
```

### Result Safety Checks

```yaml
cross_field_rules:
  - id: "critical-results-flagged"
    description: "Critical values must be flagged"
    validation_mode: "conditional"
    conditions:
      - field: "OBX.8"
        operator: "in"
        values: ["LL", "HH", "<", ">"]
    actions:
      - field: "NTE"
        action: "require"
        message: "Critical values require comment"
```

### Temporal Safety

```yaml
temporal_rules:
  - id: "procedure-during-visit"
    description: "Procedure must occur during visit"
    before: "PV1.45"  # Discharge
    after: "PR1.5"    # Procedure date
    allow_equal: true
```

## Troubleshooting

### Profile Won't Load

- Check YAML syntax (indentation, quotes)
- Validate field paths (e.g., "PID.3" not "PID-3")
- Ensure required fields are present

### Too Many False Positives

- Review field `required` settings
- Check value sets for completeness
- Consider using `conditional` mode instead of `assert`

### Missing Validation Errors

- Verify segment is in `segments` list
- Check field path correctness
- Ensure cross-field rules use correct operators

## Additional Resources

- [HL7 Version 2.5.1 Specification](http://www.hl7.org/)
- [LOINC Codes](https://loinc.org/) for observation identifiers
- [SNOMED CT](https://www.snomed.org/) for clinical terminology
- Project documentation in `/docs`
- API specification in `/schemas/openapi/hl7v2-api.yaml`

## Contributing

To contribute new example profiles:

1. Base on real-world use cases
2. Include comprehensive comments
3. Test against actual messages
4. Document key features
5. Submit PR with examples

## License

These example profiles are provided as reference implementations under the project license.
