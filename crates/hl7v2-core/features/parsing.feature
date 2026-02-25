Feature: HL7 v2 Message Parsing
  As an HL7 message processor
  I want to parse HL7 v2 messages
  So that I can extract and validate healthcare data

  Scenario: Parse a simple ADT^A01 message
    Given a valid HL7 ADT^A01 message
    When I parse the message
    Then the message should have 2 segments
    And the first segment should be MSH
    And the second segment should be PID
    And MSH.9.1 should be "ADT"
    And MSH.9.2 should be "A01"

  Scenario: Parse message with custom delimiters
    Given an HL7 message with custom delimiters
    When I parse the message
    Then the delimiters should be detected correctly
    And the message should parse successfully

  Scenario: Parse message with escape sequences
    Given an HL7 message containing escape sequences
    When I parse the message
    Then the escape sequences should be decoded
    And the field values should be unescaped

  Scenario: Parse MLLP framed message
    Given an MLLP framed HL7 message
    When I parse the MLLP message
    Then the MLLP framing should be removed
    And the message should parse successfully

  Scenario: Handle invalid message gracefully
    Given an invalid HL7 message
    When I attempt to parse the message
    Then an error should be returned
    And the error should indicate the problem

  Scenario: Parse message with field repetitions
    Given an HL7 message with repeated fields
    When I parse the message
    Then I can access the first repetition
    And I can access the second repetition
    And missing repetitions return None