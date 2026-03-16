Feature: Enhanced HL7v2 Validation Scenarios
  As a healthcare developer
  I want to validate HL7 messages with various rule types
  So that I can ensure message integrity and compliance

  Background:
    Given the hl7v2-validation library is available

  # ==========================================================================
  # Data Type Validation Scenarios
  # ==========================================================================

  Scenario: Validate valid string data type
    When I validate "hello world" as string data type
    Then the validation should pass

  Scenario: Validate invalid numeric as string
    When I validate "123ABC" as string data type
    Then the validation should fail

  Scenario: Validate valid date data type
    When I validate "20230101" as date data type
    Then the validation should pass

  Scenario: Validate invalid date data type
    When I validate "20231301" as date data type
    Then the validation should fail

  # ==========================================================================
  # Validation Result Scenarios
  # ==========================================================================

  Scenario: Create error validation result
    Given a validation error with code "101"
    When I create the error result
    Then the result should have severity "E"
    And the result should have code "101"
    And the result should have empty path
    And the result should have detail "Test error"

  Scenario: Create warning validation result
    Given a validation warning with code "W01"
    When I create the warning result with path "PID.5.1"
    Then the result should have severity "W"
    And the result should have code "W01"
    And the result should have path "PID.5.1"
    And the result should have detail "Test warning"

  # ==========================================================================
  # Composite Validation Scenarios
  # ==========================================================================

  Scenario: Validate multiple fields pass
    Given I have validations for PID.5 (string) and PID.7 (date)
    When I validate "John Doe" as PID.5 and "19800101" as PID.7
    Then both validations should pass

  Scenario: Validate mixed success/failure
    Given I have validations for PID.5 (string) and PID.8 (numeric)
    When I validate "John Doe" as PID.5 and "ABC" as PID.8
    Then PID.5 validation should pass
    And PID.8 validation should fail
