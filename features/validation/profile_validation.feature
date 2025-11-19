Feature: Profile-Based Validation
  As a data quality engineer
  I want to validate HL7 messages against conformance profiles
  So that I can ensure data meets organizational standards

  Background:
    Given the validation engine is initialized

  Scenario: Validate required fields
    Given a profile requiring PID.3 (Patient ID) and PID.5 (Patient Name)
    And an HL7 ADT^A01 message with PID.3 populated but PID.5 empty
    When I validate the message against the profile
    Then validation should fail
    And there should be 1 error
    And the error code should be "V_RequiredField"
    And the error should reference field "PID.5"

  Scenario: Validate field length constraints
    Given a profile with max length 10 for PID.3
    And an HL7 message with PID.3 = "12345678901" (11 characters)
    When I validate the message
    Then validation should fail
    And the error should indicate "exceeds maximum length"

  Scenario: Validate against HL7 table
    Given a profile requiring PID.8 (Sex) to match table 0001
    And an HL7 message with PID.8 = "X" (invalid value)
    When I validate the message
    Then validation should fail
    And the error should list valid values from table 0001

  Scenario: Validate cross-field rules
    Given a profile with rule: "if PV1.2 = 'I' then PV1.3 is required"
    And an HL7 message with PV1.2 = "I" (inpatient) but PV1.3 empty
    When I validate the message
    Then validation should fail
    And the error should reference the conditional rule

  Scenario: Validate data type constraints
    Given a profile requiring PID.3 to be CX data type
    And an HL7 message with PID.3 = "123^^^FACILITY^MR"
    When I validate the message
    Then validation should succeed for PID.3

  Scenario: Temporal validation rules
    Given a profile with rule: "PID.7 (birth date) must be before MSH.7 (message timestamp)"
    And an HL7 message with PID.7 = "20250101" and MSH.7 = "20240101"
    When I validate the message
    Then validation should fail
    And the error should indicate "birth date after message timestamp"

  Scenario: Pattern validation with regex
    Given a profile requiring SSN to match pattern "^[0-9]{3}-[0-9]{2}-[0-9]{4}$"
    And an HL7 message with SSN = "123-45-6789"
    When I validate the message
    Then validation should succeed

  Scenario: Validate with profile inheritance
    Given a parent profile "base_adt.yaml" with common ADT constraints
    And a child profile "adt_a01.yaml" inheriting from base with additional constraints
    When I validate an ADT^A01 message against the child profile
    Then all constraints from both parent and child should be enforced

  Scenario: Profile cycle detection
    Given profile "A.yaml" with parent "B.yaml"
    And profile "B.yaml" with parent "A.yaml"
    When I attempt to load profile "A.yaml"
    Then loading should fail with error "E_Profile_Cycle"
    And the error should show the cycle chain "A -> B -> A"

  Scenario: Severity levels (error vs warning)
    Given a profile with:
      | field | constraint | severity |
      | PID.3 | required   | error    |
      | PID.6 | required   | warning  |
    And an HL7 message missing both PID.3 and PID.6
    When I validate the message
    Then there should be 1 error and 1 warning
    And validation should fail (due to error severity)

  Scenario: Custom table validation
    Given a profile with custom table "gender_codes" = ["M", "F", "O", "U"]
    And an HL7 message with PID.8 = "M"
    When I validate the message
    Then validation should succeed

  Scenario: Validate advanced data types
    Given a profile requiring OBX.5 to be NM (numeric) type
    And an HL7 message with OBX.5 = "123.45"
    When I validate the message
    Then validation should succeed

  Scenario: Validate checksum (Luhn algorithm)
    Given a profile with Luhn checksum validation for PID.3
    And an HL7 message with PID.3 = "79927398713" (valid Luhn)
    When I validate the message
    Then validation should succeed

  Scenario: Validate phone number format
    Given a profile requiring phone numbers in format "+1-XXX-XXX-XXXX"
    And an HL7 message with phone = "+1-555-123-4567"
    When I validate the message
    Then validation should succeed

  Scenario: Export validation report as JSON
    Given a profile with multiple constraints
    And an HL7 message with multiple validation failures
    When I validate and generate a JSON report
    Then the report should be valid against error-v1.schema.json
    And the report should include all violations with locations
