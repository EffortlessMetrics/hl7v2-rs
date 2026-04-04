Feature: HL7 v2 JSON Serialization
  As an HL7 message processor
  I want to serialize HL7 messages to JSON format
  So that I can work with HL7 data in JSON-based systems

  Scenario: Convert a simple message to JSON
    Given a message with MSH and PID segments
    When I convert the message to JSON
    Then the JSON should be valid
    And the JSON should contain "meta" object
    And the JSON should contain "segments" array

  Scenario: Convert message to JSON string
    Given a message with MSH and PID segments
    When I convert the message to JSON string
    Then the result should be a valid JSON string

  Scenario: Convert message to pretty JSON string
    Given a message with MSH and PID segments
    When I convert the message to pretty JSON string
    Then the result should be formatted with indentation

  Scenario: JSON should include delimiter metadata
    Given a message with default delimiters
    When I convert the message to JSON
    Then the JSON should contain delimiter information
    And the field separator should be "|"
    And the component separator should be "^"

  Scenario: JSON should include charset information
    Given a message with charset specification
    When I convert the message to JSON
    Then the JSON should contain charset information

  Scenario: JSON should include segment IDs
    Given a message with MSH and PID segments
    When I convert the message to JSON
    Then the JSON should contain segment "MSH"
    And the JSON should contain segment "PID"

  Scenario: JSON should include field values
    Given a message with MSH and PID segments
    When I convert the message to JSON
    Then the JSON should contain field values from MSH
    And the JSON should contain field values from PID

  Scenario: JSON should handle empty fields
    Given a message with empty fields
    When I convert the message to JSON
    Then the JSON should represent empty fields correctly

  Scenario: JSON should handle null values
    Given a message with null values
    When I convert the message to JSON
    Then the JSON should represent null values correctly

  Scenario: JSON should handle field repetitions
    Given a message with field repetitions
    When I convert the message to JSON
    Then the JSON should represent repetitions as an array

  Scenario: JSON should handle components
    Given a message with components
    When I convert the message to JSON
    Then the JSON should represent components correctly

  Scenario: JSON should handle subcomponents
    Given a message with subcomponents
    When I convert the message to JSON
    Then the JSON should represent subcomponents correctly

  Scenario: JSON should handle escape sequences
    Given a message with escape sequences
    When I convert the message to JSON
    Then the JSON should handle escape sequences properly

  Scenario: JSON should handle special characters
    Given a message with special characters
    When I convert the message to JSON
    Then the JSON should handle special characters properly

  Scenario Outline: Convert different message types to JSON
    Given a <message_type> message
    When I convert the message to JSON
    Then the JSON should be valid
    And the JSON should contain the message type

    Examples:
      | message_type |
      | ADT^A01      |
      | ORU^R01      |
      | ORM^O01      |
      | DFT^P03      |

  Scenario: JSON should handle multiple segments
    Given a message with multiple segments
    When I convert the message to JSON
    Then the JSON should contain all segments

  Scenario: JSON should handle long field values
    Given a message with long field values
    When I convert the message to JSON
    Then the JSON should preserve long values

  Scenario: JSON should handle custom delimiters
    Given a message with custom delimiters "#$*@!"
    When I convert the message to JSON
    Then the JSON should reflect the custom delimiters

  Scenario: Convert empty message to JSON
    Given an empty message
    When I convert the message to JSON
    Then the JSON should be valid
    And the JSON should have empty segments array
