Feature: HL7 v2 Field Path Parsing
  As an HL7 message processor
  I want to parse field path strings into structured Path objects
  So that I can validate and manipulate field paths

  Scenario: Parse a simple field path
    Given the path string "PID.5"
    When I parse the path
    Then the segment should be "PID"
    And the field should be 5
    And the repetition should be None
    And the component should be None
    And the subcomponent should be None

  Scenario: Parse a path with component
    Given the path string "PID.5.1"
    When I parse the path
    Then the segment should be "PID"
    And the field should be 5
    And the component should be 1
    And the subcomponent should be None

  Scenario: Parse a path with repetition
    Given the path string "PID.5[2]"
    When I parse the path
    Then the segment should be "PID"
    And the field should be 5
    And the repetition should be 2
    And the component should be None
    And the subcomponent should be None

  Scenario: Parse a path with repetition and component
    Given the path string "PID.5[2].1"
    When I parse the path
    Then the segment should be "PID"
    And the field should be 5
    And the repetition should be 2
    And the component should be 1
    And the subcomponent should be None

  Scenario: Parse a path with subcomponent
    Given the path string "PID.5.1.1"
    When I parse the path
    Then the segment should be "PID"
    And the field should be 5
    And the component should be 1
    And the subcomponent should be 1

  Scenario: Parse a path with all components
    Given the path string "PID.5[2].1.1"
    When I parse the path
    Then the segment should be "PID"
    And the field should be 5
    And the repetition should be 2
    And the component should be 1
    And the subcomponent should be 1

  Scenario: Parse MSH field path
    Given the path string "MSH.9.1"
    When I parse the path
    Then the segment should be "MSH"
    And the field should be 9
    And the component should be 1

  Scenario: Parse path with lowercase segment
    Given the path string "pid.5.1"
    When I parse the path
    Then the segment should be "PID"
    And the field should be 5
    And the component should be 1

  Scenario: Parse path with mixed case segment
    Given the path string "PiD.5.1"
    When I parse the path
    Then the segment should be "PID"
    And the field should be 5
    And the component should be 1

  Scenario: Parse path with invalid format
    Given the path string "INVALID"
    When I attempt to parse the path
    Then parsing should fail
    And the error should indicate invalid format

  Scenario: Parse path with invalid segment ID
    Given the path string "123.5.1"
    When I attempt to parse the path
    Then parsing should fail
    And the error should indicate invalid segment ID

  Scenario: Parse path with invalid field number
    Given the path string "PID.abc.1"
    When I attempt to parse the path
    Then parsing should fail
    And the error should indicate invalid field number

  Scenario: Parse path with invalid component number
    Given the path string "PID.5.abc"
    When I attempt to parse the path
    Then parsing should fail
    And the error should indicate invalid component number

  Scenario: Parse path with invalid repetition index
    Given the path string "PID.5[abc]"
    When I attempt to parse the path
    Then parsing should fail
    And the error should indicate invalid repetition index

  Scenario: Parse path with empty segment
    Given the path string ".5.1"
    When I attempt to parse the path
    Then parsing should fail

  Scenario: Parse path with zero field number
    Given the path string "PID.0.1"
    When I parse the path
    Then the field should be 0

  Scenario: Format path to string
    Given a parsed path with segment "PID" field 5 component 1
    When I format the path to string
    Then the result should be "PID.5.1"

  Scenario: Format path with repetition to string
    Given a parsed path with segment "PID" field 5 repetition 2 component 1
    When I format the path to string
    Then the result should be "PID.5[2].1"

  Scenario: Format path with subcomponent to string
    Given a parsed path with segment "PID" field 5 component 1 subcomponent 1
    When I format the path to string
    Then the result should be "PID.5.1.1"

  Scenario Outline: Parse various segment IDs
    Given the path string "<segment>.5.1"
    When I parse the path
    Then the segment should be "<expected_segment>"
    And the field should be 5
    And the component should be 1

    Examples:
      | segment   | expected_segment |
      | MSH       | MSH             |
      | PID       | PID             |
      | OBX       | OBX             |
      | ORC       | ORC             |
      | NK1       | NK1             |

  Scenario: Parse path with large field numbers
    Given the path string "PID.999.1"
    When I parse the path
    Then the field should be 999
    And the component should be 1

  Scenario: Parse path with large component numbers
    Given the path string "PID.5.999"
    When I parse the path
    Then the field should be 5
    And the component should be 999
