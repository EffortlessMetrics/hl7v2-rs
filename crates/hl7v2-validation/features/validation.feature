Feature: HL7 v2 Message Validation
  As a healthcare data engineer
  I want to validate HL7 v2 messages against various rules
  So that I can ensure data quality and compliance

  Background:
    Given the validation engine is initialized

  # ============================================================================
  # Required Field Validation
  # ============================================================================

  Scenario: Validate message with all required fields present
    Given an ADT^A01 message with all required fields populated
    When I validate the message
    Then validation should succeed
    And there should be 0 errors

  Scenario: Detect missing required field PID.3
    Given an ADT^A01 message with PID.3 (Patient ID) missing
    When I validate the message
    Then validation should fail
    And there should be 1 error
    And the error code should be "MISSING_REQUIRED_FIELD"
    And the error should reference field "PID.3"

  Scenario: Detect missing required field PID.5
    Given an ADT^A01 message with PID.5 (Patient Name) missing
    When I validate the message
    Then validation should fail
    And there should be 1 error
    And the error should reference field "PID.5"

  Scenario: Detect multiple missing required fields
    Given an ADT^A01 message with PID.3 and PID.5 missing
    When I validate the message
    Then validation should fail
    And there should be 2 errors

  # ============================================================================
  # Data Type Validation
  # ============================================================================

  Scenario: Validate valid date format
    Given a message with PID.7 (birth date) = "19800101"
    When I validate the data type as "DT"
    Then validation should succeed for PID.7

  Scenario: Detect invalid date format
    Given a message with PID.7 (birth date) = "invalid"
    When I validate the data type as "DT"
    Then validation should fail
    And the error code should be "INVALID_DATA_TYPE"

  Scenario: Validate valid time format
    Given a message with a time field = "143052"
    When I validate the data type as "TM"
    Then validation should succeed

  Scenario: Detect invalid time format
    Given a message with a time field = "25:00:00"
    When I validate the data type as "TM"
    Then validation should fail

  Scenario: Validate valid timestamp format
    Given a message with MSH.7 = "20230101143052"
    When I validate the data type as "TS"
    Then validation should succeed

  Scenario: Validate numeric data type
    Given a message with OBX.5 = "123.45"
    When I validate the data type as "NM"
    Then validation should succeed

  Scenario: Detect non-numeric value in numeric field
    Given a message with OBX.5 = "not-a-number"
    When I validate the data type as "NM"
    Then validation should fail

  # ============================================================================
  # Field Length Validation
  # ============================================================================

  Scenario: Validate field within maximum length
    Given a profile with max length 20 for PID.3.1
    And a message with PID.3.1 = "12345678901234567890"
    When I validate the message
    Then validation should succeed

  Scenario: Detect field exceeding maximum length
    Given a profile with max length 10 for PID.3.1
    And a message with PID.3.1 = "12345678901"
    When I validate the message
    Then validation should fail
    And the error should indicate "exceeds maximum length"

  # ============================================================================
  # Code Value Validation
  # ============================================================================

  Scenario: Validate valid code from HL7 table
    Given a profile requiring PID.8 (Sex) to match table 0001
    And a message with PID.8 = "M"
    When I validate the message
    Then validation should succeed

  Scenario: Detect invalid code value
    Given a profile requiring PID.8 (Sex) to match table 0001
    And a message with PID.8 = "X"
    When I validate the message
    Then validation should fail
    And the error should list valid values M, F, O, U, A, N

  Scenario: Validate custom code table
    Given a profile with custom table "facility_codes" = ["FAC1", "FAC2", "FAC3"]
    And a message with MSH.4 = "FAC2"
    When I validate the message
    Then validation should succeed

  # ============================================================================
  # Cross-Field Validation
  # ============================================================================

  Scenario: Validate conditional requirement - inpatient requires room
    Given a profile with rule: "if PV1.2 = 'I' then PV1.3 is required"
    And a message with PV1.2 = "I" and PV1.3 = "ICU^101"
    When I validate the message
    Then validation should succeed

  Scenario: Detect missing conditional field
    Given a profile with rule: "if PV1.2 = 'I' then PV1.3 is required"
    And a message with PV1.2 = "I" but PV1.3 empty
    When I validate the message
    Then validation should fail
    And the error should reference the conditional rule

  Scenario: Validate temporal rule - birth date before message date
    Given a profile with rule: "PID.7 must be before MSH.7"
    And a message with PID.7 = "19800101" and MSH.7 = "20230101"
    When I validate the message
    Then validation should succeed

  Scenario: Detect birth date after message date
    Given a profile with rule: "PID.7 must be before MSH.7"
    And a message with PID.7 = "20250101" and MSH.7 = "20240101"
    When I validate the message
    Then validation should fail
    And the error should indicate "birth date after message timestamp"

  # ============================================================================
  # Severity Levels
  # ============================================================================

  Scenario: Error severity blocks validation
    Given a profile with:
      | field | constraint | severity |
      | PID.3 | required   | error    |
    And a message missing PID.3
    When I validate the message
    Then validation should fail
    And there should be 1 error

  Scenario: Warning severity does not block validation
    Given a profile with:
      | field | constraint | severity |
      | PID.6 | required   | warning  |
    And a message missing PID.6
    When I validate the message
    Then validation should succeed
    And there should be 1 warning

  Scenario: Mixed severity levels
    Given a profile with:
      | field | constraint | severity |
      | PID.3 | required   | error    |
      | PID.6 | required   | warning  |
    And a message missing both PID.3 and PID.6
    When I validate the message
    Then validation should fail
    And there should be 1 error and 1 warning

  # ============================================================================
  # Segment Order Validation
  # ============================================================================

  Scenario: Validate correct segment order
    Given an ADT^A01 message with segments in order: MSH, EVN, PID, PV1
    When I validate segment order
    Then validation should succeed

  Scenario: Detect incorrect segment order
    Given an ADT^A01 message with segments in order: MSH, PID, EVN, PV1
    When I validate segment order
    Then validation should fail
    And the error should indicate "EVN must appear before PID"

  # ============================================================================
  # Cardinality Validation
  # ============================================================================

  Scenario: Validate single occurrence segment
    Given a message with exactly 1 PID segment
    When I validate cardinality
    Then validation should succeed

  Scenario: Detect duplicate single-occurrence segment
    Given a message with 2 PID segments
    When I validate cardinality
    Then validation should fail
    And the error code should be "CARDINALITY_VIOLATION"

  Scenario: Validate multiple occurrence segment
    Given a message with 3 OBX segments
    When I validate cardinality allowing multiple OBX
    Then validation should succeed

  # ============================================================================
  # Checksum Validation
  # ============================================================================

  Scenario: Validate Luhn checksum - valid
    Given a profile with Luhn checksum validation for PID.3.1
    And a message with PID.3.1 = "79927398713"
    When I validate the message
    Then validation should succeed

  Scenario: Validate Luhn checksum - invalid
    Given a profile with Luhn checksum validation for PID.3.1
    And a message with PID.3.1 = "79927398710"
    When I validate the message
    Then validation should fail
    And the error code should be "CHECKSUM_VALIDATION_FAILED"

  # ============================================================================
  # Format Validation
  # ============================================================================

  Scenario: Validate phone number format
    Given a profile requiring phone format "+1-XXX-XXX-XXXX"
    And a message with PID.13 = "+1-555-123-4567"
    When I validate the message
    Then validation should succeed

  Scenario: Detect invalid phone format
    Given a profile requiring phone format "+1-XXX-XXX-XXXX"
    And a message with PID.13 = "5551234567"
    When I validate the message
    Then validation should fail

  Scenario: Validate email format
    Given a profile requiring valid email format
    And a message with PID.13 = "patient@example.com"
    When I validate the message
    Then validation should succeed

  Scenario: Detect invalid email format
    Given a profile requiring valid email format
    And a message with PID.13 = "not-an-email"
    When I validate the message
    Then validation should fail

  Scenario: Validate SSN format
    Given a profile requiring SSN format
    And a message with PID.19 = "123-45-6789"
    When I validate the message
    Then validation should succeed

  Scenario: Detect invalid SSN format
    Given a profile requiring SSN format
    And a message with PID.19 = "123456789"
    When I validate the message
    Then validation should succeed for format but may fail for display format

  # ============================================================================
  # Range Validation
  # ============================================================================

  Scenario: Validate numeric value within range
    Given a profile with range 4.0-11.0 for OBX.5
    And a message with OBX.5 = "7.5"
    When I validate the message
    Then validation should succeed

  Scenario: Detect value below range
    Given a profile with range 4.0-11.0 for OBX.5
    And a message with OBX.5 = "3.5"
    When I validate the message
    Then validation should fail
    And the error should indicate "below minimum"

  Scenario: Detect value above range
    Given a profile with range 4.0-11.0 for OBX.5
    And a message with OBX.5 = "12.0"
    When I validate the message
    Then validation should fail
    And the error should indicate "above maximum"

  # ============================================================================
  # Pattern Validation
  # ============================================================================

  Scenario: Validate regex pattern match
    Given a profile requiring PID.3.1 to match pattern "^MRN[0-9]{6}$"
    And a message with PID.3.1 = "MRN123456"
    When I validate the message
    Then validation should succeed

  Scenario: Detect pattern mismatch
    Given a profile requiring PID.3.1 to match pattern "^MRN[0-9]{6}$"
    And a message with PID.3.1 = "123456"
    When I validate the message
    Then validation should fail

  # ============================================================================
  # Batch Validation
  # ============================================================================

  Scenario: Validate batch of messages
    Given 3 valid ADT^A01 messages
    When I validate all messages
    Then all validations should succeed
    And the summary should show 3 valid, 0 invalid

  Scenario: Validate batch with some invalid messages
    Given 2 valid ADT^A01 messages and 1 invalid message
    When I validate all messages
    Then 2 validations should succeed
    And 1 validation should fail
    And the summary should show 2 valid, 1 invalid

  # ============================================================================
  # Edge Cases
  # ============================================================================

  Scenario: Handle empty message gracefully
    Given an empty message
    When I validate the message
    Then validation should fail
    And the error should indicate "empty message"

  Scenario: Handle missing MSH segment
    Given a message without MSH segment
    When I validate the message
    Then validation should fail
    And the error should indicate "missing MSH segment"

  Scenario: Handle unknown message type
    Given a message with type UNKNOWN^TYPE
    When I validate the message
    Then validation should fail
    And the error should indicate "unknown message type"

  Scenario: Validate message with special characters
    Given a message with PID.5.1 = "O'Brien"
    When I validate the message
    Then validation should succeed

  Scenario: Validate message with escape sequences
    Given a message with field containing escape sequence "\\F\\"
    When I validate the message
    Then validation should succeed

  # ============================================================================
  # Profile Handling
  # ============================================================================

  Scenario: Handle missing profile gracefully
    Given a message to validate against non-existent profile
    When I validate the message
    Then validation should fail
    And the error should indicate "profile not found"

  Scenario: Handle invalid profile
    Given an invalid profile with syntax errors
    When I load the profile
    Then loading should fail
    And the error should indicate profile syntax error

  Scenario: Validate with profile inheritance
    Given a parent profile with common constraints
    And a child profile with additional constraints
    When I validate a message against the child profile
    Then all constraints from both profiles should be enforced
