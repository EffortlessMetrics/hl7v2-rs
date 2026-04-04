Feature: HL7 v2 ACK (Acknowledgment) Message Generation
  As an HL7 message processor
  I want to generate acknowledgment messages in response to received HL7 messages
  So that I can confirm receipt and processing status of messages

  Scenario: Generate AA (Application Accept) acknowledgment
    Given a valid ADT^A01 message with message ID "MSG001"
    When I generate an ACK with code AA
    Then the ACK message should have 2 segments
    And the first segment should be MSH
    And the second segment should be MSA
    And MSA.1 should be "AA"
    And MSA.2 should be "MSG001"

  Scenario: Generate AE (Application Error) acknowledgment
    Given a valid ORU^R01 message with message ID "MSG002"
    When I generate an ACK with code AE
    Then MSA.1 should be "AE"
    And MSA.2 should be "MSG002"

  Scenario: Generate AR (Application Reject) acknowledgment
    Given a valid ADT^A04 message with message ID "MSG003"
    When I generate an ACK with code AR
    Then MSA.1 should be "AR"
    And MSA.2 should be "MSG003"

  Scenario: Generate CA (Commit Accept) acknowledgment
    Given a valid ORM^O01 message with message ID "MSG004"
    When I generate an ACK with code CA
    Then MSA.1 should be "CA"
    And MSA.2 should be "MSG004"

  Scenario: Generate CE (Commit Error) acknowledgment
    Given a valid ADT^A01 message with message ID "MSG005"
    When I generate an ACK with code CE
    Then MSA.1 should be "CE"
    And MSA.2 should be "MSG005"

  Scenario: Generate CR (Commit Reject) acknowledgment
    Given a valid ORU^R01 message with message ID "MSG006"
    When I generate an ACK with code CR
    Then MSA.1 should be "CR"
    And MSA.2 should be "MSG006"

  Scenario: ACK preserves original message delimiters
    Given an HL7 message with custom delimiters "#$*@!"
    When I generate an ACK with code AA
    Then the ACK should use the same delimiters
    And the delimiters should be "#$*@!"

  Scenario: ACK swaps sending and receiving applications
    Given a message from "SendingApp" to "ReceivingApp"
    When I generate an ACK with code AA
    Then MSH.3 should be "ReceivingApp"
    And MSH.5 should be "SendingApp"

  Scenario: ACK swaps sending and receiving facilities
    Given a message from "SendingFac" to "ReceivingFac"
    When I generate an ACK with code AA
    Then MSH.4 should be "ReceivingFac"
    And MSH.6 should be "SendingFac"

  Scenario: ACK message type is ACK
    Given a valid ADT^A01 message
    When I generate an ACK with code AA
    Then MSH.9.1 should be "ACK"

  Scenario Outline: Generate ACK for different message types
    Given a valid <message_type> message with message ID "MSG010"
    When I generate an ACK with code AA
    Then MSH.9.1 should be "ACK"
    And MSA.1 should be "AA"
    And MSA.2 should be "MSG010"

    Examples:
      | message_type |
      | ADT^A01      |
      | ADT^A04      |
      | ORU^R01      |
      | ORM^O01      |
      | DFT^P03      |
