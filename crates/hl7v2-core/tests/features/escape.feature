Feature: HL7 Escape Sequence Handling
  As an HL7 message processor
  I want to handle escape sequences
  So that special characters are properly encoded and decoded

  Scenario: Escape field separator
    Given a text containing the field separator "|"
    When I escape the text
    Then the escaped text should contain "\F\"

  Scenario: Escape component separator
    Given a text containing the component separator "^"
    When I escape the text
    Then the escaped text should contain "\S\"

  Scenario: Escape repetition separator
    Given a text containing the repetition separator "~"
    When I escape the text
    Then the escaped text should contain "\R\"

  Scenario: Unescape HL7 text
    Given an escaped HL7 text "\F\test\S\"
    When I unescape the text
    Then the result should be "|test^"

  Scenario: Roundtrip escape handling
    Given a text with special characters "|^~&"
    When I escape then unescape the text
    Then the result should be the original text

  Scenario: Detect text needing escaping
    Given a text with delimiter characters
    When I check if escaping is needed
    Then the result should indicate escaping is needed

  Scenario: Handle text without special characters
    Given plain text without special characters
    When I escape the text
    Then the text should remain unchanged