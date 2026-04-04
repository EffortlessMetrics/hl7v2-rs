Feature: HL7 v2 Message Writer/Serializer
  As an HL7 message processor
  I want to serialize message structures to HL7 format
  So that I can transmit or store HL7 messages in their standard format

  Scenario: Write a simple MSH segment
    Given a message with only an MSH segment
    When I write the message to bytes
    Then the output should start with "MSH|"
    And the output should end with a carriage return

  Scenario: Write a message with MSH and PID segments
    Given a message with MSH and PID segments
    When I write the message to bytes
    Then the output should contain "MSH|"
    And the output should contain "PID|"
    And the segments should be separated by carriage returns

  Scenario: Write message with custom delimiters
    Given a message with custom delimiters "#$*@!"
    When I write the message to bytes
    Then the output should use the custom delimiters

  Scenario: Write message with field repetitions
    Given a message with a field containing repetitions
    When I write the message to bytes
    Then the repetitions should be separated by tilde "~"

  Scenario: Write message with components
    Given a message with a field containing components
    When I write the message to bytes
    Then the components should be separated by caret "^"

  Scenario: Write message with subcomponents
    Given a message with a component containing subcomponents
    When I write the message to bytes
    Then the subcomponents should be separated by ampersand "&"

  Scenario: Write message with escape sequences
    Given a message containing characters that need escaping
    When I write the message to bytes
    Then the special characters should be properly escaped

  Scenario: Write message with MLLP framing
    Given a message with MSH and PID segments
    When I write the message with MLLP framing
    Then the output should start with MLLP start block
    And the output should end with MLLP end block
    And the message content should be between the blocks

  Scenario: Write empty message
    Given an empty message with default delimiters
    When I write the message to bytes
    Then the output should be valid HL7 format

  Scenario: Write message with empty fields
    Given a message with empty fields
    When I write the message to bytes
    Then empty fields should be represented as consecutive delimiters

  Scenario: Write message with null values
    Given a message with explicit null values
    When I write the message to bytes
    Then null values should be represented as double quotes '""'

  Scenario: Write message preserves delimiters
    Given a message with custom delimiters "#$*@!"
    When I write the message to bytes
    Then the MSH segment should contain the delimiters in field 2

  Scenario: Write message with multiple PID segments
    Given a message with MSH and multiple PID segments
    When I write the message to bytes
    Then all PID segments should be present in the output

  Scenario: Write message with charset specification
    Given a message with charset specification
    When I write the message to bytes
    Then the charset should be present in MSH-18

  Scenario Outline: Write messages with different message types
    Given a message of type <message_type>
    When I write the message to bytes
    Then the output should contain the message type

    Examples:
      | message_type |
      | ADT^A01      |
      | ORU^R01      |
      | ORM^O01      |
      | DFT^P03      |

  Scenario: Write message with long field values
    Given a message with long field values
    When I write the message to bytes
    Then the long values should be preserved

  Scenario: Write message with special characters
    Given a message containing special characters in field values
    When I write the message to bytes
    Then the special characters should be preserved or properly escaped
