Feature: MLLP Framing Protocol
  As an HL7 message processor
  I want to handle MLLP framed messages
  So that I can communicate over TCP/IP connections

  Scenario: Wrap message with MLLP framing
    Given an HL7 message
    When I wrap it with MLLP framing
    Then the result should start with VT character
    And the result should end with FS CR characters

  Scenario: Unwrap MLLP framed message
    Given an MLLP framed message
    When I unwrap the MLLP framing
    Then I should get the original HL7 message
    And the message should be valid

  Scenario: Detect MLLP framed message
    Given an MLLP framed message
    When I check if it is MLLP framed
    Then the result should be true

  Scenario: Handle non-MLLP data
    Given raw non-MLLP data
    When I check if it is MLLP framed
    Then the result should be false

  Scenario: Find complete MLLP message in buffer
    Given a buffer containing an MLLP message
    When I search for a complete message
    Then I should find the message boundaries
    And I should get the message content