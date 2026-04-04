Feature: PHI Redaction in HL7 Messages
  As a healthcare developer
  I want to redact Protected Health Information from HL7 messages
  So that I can safely log and share messages in non-production environments

  Background:
    Given a realistic HL7 message with PHI

  Scenario: Apply common PHI redaction rules
    Given a redaction engine with common PHI rules
    When I apply redaction rules
    Then the message should be successfully redacted
    And the patient identifier should be hashed
    And the patient name should be masked
    And the address should be removed
    And the phone number should be masked
    And an audit log should be generated

  Scenario: Apply HIPAA Safe Harbor rules
    Given a redaction engine with HIPAA Safe Harbor rules
    When I apply redaction rules
    Then the message should be successfully redacted
    And the SSN should be hashed
    And the patient name should be masked
    And the date of birth should be replaced
    And an audit log should be generated

  Scenario: Redacted message remains valid HL7
    Given a redaction engine with common PHI rules
    When I apply redaction rules
    Then the redacted message should be valid HL7
    And the message structure should be preserved

  Scenario: Custom redaction rule for specific field
    Given a redaction rule for path "PID.3" with strategy "hash"
    And a redaction rule for path "PID.5" with strategy "mask"
    And I create a redaction engine with the configured rules
    When I apply the redaction
    Then the patient identifier should be hashed
    And the patient name should be masked
    And the audit log should contain 2 entries

  Scenario: Empty message handling
    Given an empty HL7 message
    And a redaction engine with common PHI rules
    When I apply redaction rules
    Then the message should be successfully redacted
    And no error should occur

  Scenario: Message with no PHI
    Given an HL7 message with no PHI
    And a redaction engine with common PHI rules
    When I apply redaction rules
    Then the message should be successfully redacted
    And an audit log should be generated
    And the message structure should be preserved
