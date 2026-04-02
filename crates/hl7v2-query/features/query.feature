Feature: HL7 v2 Path-based Field Query
  As an HL7 message processor
  I want to query message fields using path notation
  So that I can extract specific data from HL7 messages

  Scenario: Query a simple field value
    Given a message with MSH and PID segments
    When I query the path "PID.5.1"
    Then the result should be "Doe"

  Scenario: Query a field without component specification
    Given a message with MSH and PID segments
    When I query the path "PID.5"
    Then the result should be "Doe^John"

  Scenario: Query a field with repetition
    Given a message with a field containing 2 repetitions
    When I query the path "PID.5[1].1"
    Then the result should be "Doe"
    When I query the path "PID.5[2].1"
    Then the result should be "Smith"

  Scenario: Query MSH field
    Given a message with MSH and PID segments
    When I query the path "MSH.9.1"
    Then the result should be "ADT"
    When I query the path "MSH.9.2"
    Then the result should be "A01"

  Scenario: Query non-existent segment
    Given a message with MSH and PID segments
    When I query the path "OBX.5.1"
    Then the result should be None

  Scenario: Query non-existent field
    Given a message with MSH and PID segments
    When I query the path "PID.99.1"
    Then the result should be None

  Scenario: Query non-existent component
    Given a message with MSH and PID segments
    When I query the path "PID.5.99"
    Then the result should be None

  Scenario: Query empty field
    Given a message with an empty field
    When I query the path "PID.2"
    Then the result should be None

  Scenario: Query field with null value
    Given a message with a null field value
    When I query the path "PID.2"
    Then the result should be None

  Scenario: Query field with multiple components
    Given a message with a field containing multiple components
    When I query the path "PID.3.1"
    Then the result should be "123456"
    When I query the path "PID.3.4"
    Then the result should be "MR"

  Scenario: Query field with subcomponents
    Given a message with a field containing subcomponents
    When I query the path "PID.5.1.1"
    Then the result should be "Doe"

  Scenario: Query presence of existing field with value
    Given a message with MSH and PID segments
    When I query the presence of "PID.5.1"
    Then the presence should be Value

  Scenario: Query presence of existing empty field
    Given a message with an empty field
    When I query the presence of "PID.2"
    Then the presence should be Empty

  Scenario: Query presence of null field
    Given a message with a null field value
    When I query the presence of "PID.2"
    Then the presence should be Null

  Scenario: Query presence of missing field
    Given a message with MSH and PID segments
    When I query the presence of "PID.99.1"
    Then the presence should be Missing

  Scenario: Query with invalid path format
    Given a message with MSH and PID segments
    When I query the path "INVALID_PATH"
    Then the result should be None

  Scenario: Query field with custom delimiters
    Given a message with custom delimiters "#$*@!"
    When I query the path "PID.5.1"
    Then the result should be "Doe"

  Scenario Outline: Query different message types
    Given a <message_type> message
    When I query the path "MSH.9.1"
    Then the result should be "<message_code>"

    Examples:
      | message_type | message_code |
      | ADT^A01      | ADT          |
      | ORU^R01      | ORU          |
      | ORM^O01      | ORM          |
      | DFT^P03      | DFT          |

  Scenario: Query field with special characters
    Given a message with special characters in field values
    When I query the path "PID.5.1"
    Then the result should contain the special characters

  Scenario: Query field with escape sequences
    Given a message with escape sequences in field values
    When I query the path "PID.5.1"
    Then the result should have escape sequences decoded

  Scenario: Query field with leading/trailing whitespace
    Given a message with whitespace in field values
    When I query the path "PID.5.1"
    Then the result should preserve the whitespace
