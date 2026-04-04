Feature: PHI Redaction in HL7 Messages
  As a healthcare developer
  I want to redact Protected Health Information from HL7 messages
  So that I can safely log and share messages in non-production environments

  Background:
    Given an HL7 message with PHI:
      """
      MSH|^~\&|HOSPITAL_ADT|MAIN_HOSPITAL|LAB_SYSTEM|LAB|20250128120000||ADT^A01^ADT_A01|MSG00001|P|2.5
      EVN|A01|20250128120000|||USER123
      PID|1||123456789^^^HOSPITAL^MRN||Doe^John^Michael^Jr^^L||19800115|M|||123 Main Street^^Springfield^IL^62701^USA^^^H||555-123-4567|555-987-6543||S|C|123456789|123-45-6789
      NK1|1|Smith^Jane^Marie|SPOUSE|||555-456-7890||E\C
      PV1|1|I|200^201^01|R|||DOC12345^Smith^John^M^^^MD|DOC67890^Jones^Mary^A^^^MD|||MED||||A|||1234567890|COMP_INS|||||
      OBX|1|ST|12345^Test^L||Positive||||||F
      """

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

  Scenario Outline: Different redaction strategies
    Given an HL7 message with PHI:
      """
      MSH|^~\&|HOSPITAL|FAC|20250128120000||ADT^A01|MSG00001|P|2.5
      PID|1||<patient_id>||<patient_name>||<dob>|M
      """
    And a redaction rule for path "<field_path>" with strategy "<strategy>"
    And I create a redaction engine with the configured rules
    When I apply the redaction
    Then the message should be successfully redacted
    And no error should occur

    Examples:
      | field_path | strategy | patient_id | patient_name | dob      |
      | PID.3      | hash     | 12345      | Doe^John     | 19800101 |
      | PID.5      | mask     | 67890      | Smith^Jane   | 19900101 |
      | PID.7      | replace  | 11111      | Brown^Bob    | 19700101 |
      | PID.3      | remove   | 99999      | Wilson^Alice | 19600101 |
