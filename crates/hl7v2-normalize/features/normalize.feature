Feature: HL7 v2 Message Normalization
  As an HL7 message processor
  I want to normalize HL7 messages to a consistent format
  So that I can ensure consistent message structure across systems

  Scenario: Normalize a simple ADT^A01 message
    Given a valid ADT^A01 message
    When I normalize the message
    Then the normalized message should be valid HL7
    And the normalized message should start with "MSH|"

  Scenario: Normalize message with custom delimiters to canonical
    Given a message with custom delimiters "#$*@!"
    When I normalize the message with canonical delimiters
    Then the normalized message should use canonical delimiters "|^~\\&"

  Scenario: Normalize message preserving custom delimiters
    Given a message with custom delimiters "#$*@!"
    When I normalize the message without canonical delimiters
    Then the normalized message should preserve the custom delimiters

  Scenario: Normalize message with irregular spacing
    Given a message with irregular spacing
    When I normalize the message
    Then the normalized message should have consistent spacing

  Scenario: Normalize message with extra segments
    Given a message with extra segments
    When I normalize the message
    Then the normalized message should contain all segments

  Scenario: Normalize message with escape sequences
    Given a message containing escape sequences
    When I normalize the message
    Then the normalized message should preserve escape sequences

  Scenario: Normalize message with field repetitions
    Given a message with field repetitions
    When I normalize the message
    Then the normalized message should preserve repetitions

  Scenario: Normalize message with components
    Given a message with components
    When I normalize the message
    Then the normalized message should preserve components

  Scenario: Normalize message with subcomponents
    Given a message with subcomponents
    When I normalize the message
    Then the normalized message should preserve subcomponents

  Scenario: Normalize message with null values
    Given a message with null values
    When I normalize the message
    Then the normalized message should preserve null values

  Scenario: Normalize message with empty fields
    Given a message with empty fields
    When I normalize the message
    Then the normalized message should preserve empty fields

  Scenario: Normalize message with charset specification
    Given a message with charset specification
    When I normalize the message
    Then the normalized message should preserve charset

  Scenario: Normalize invalid message
    Given an invalid HL7 message
    When I attempt to normalize the message
    Then normalization should fail
    And an error should be returned

  Scenario: Normalize message without MSH segment
    Given a message without MSH segment
    When I attempt to normalize the message
    Then normalization should fail

  Scenario: Normalize message with malformed delimiters
    Given a message with malformed delimiters
    When I attempt to normalize the message
    Then normalization should fail

  Scenario Outline: Normalize different message types
    Given a <message_type> message
    When I normalize the message
    Then the normalized message should be valid HL7
    And the normalized message should contain "<message_type>"

    Examples:
      | message_type |
      | ADT^A01      |
      | ORU^R01      |
      | ORM^O01      |
      | DFT^P03      |

  Scenario: Normalize message with special characters
    Given a message with special characters
    When I normalize the message
    Then the normalized message should preserve special characters

  Scenario: Normalize message with long field values
    Given a message with long field values
    When I normalize the message
    Then the normalized message should preserve long values

  Scenario: Normalize message to canonical format
    Given a message with non-canonical delimiters
    When I normalize the message with canonical delimiters
    Then the output should start with "MSH|^~\\&|"
    And the field separator should be "|"
    And the component separator should be "^"
    And the repetition separator should be "~"
    And the escape character should be "\\"
    And the subcomponent separator should be "&"
