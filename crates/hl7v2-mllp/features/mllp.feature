Feature: MLLP (Minimal Lower Layer Protocol) Framing
  As an HL7 message transport layer
  I want to frame and unframe HL7 messages using MLLP
  So that messages can be reliably transmitted over TCP connections

  Scenario: Wrap a simple HL7 message
    Given an HL7 message "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128||ADT^A01|MSG001|P|2.5.1\r"
    When I wrap it with MLLP framing
    Then the first byte should be 0x0B
    And the second-to-last byte should be 0x1C
    And the last byte should be 0x0D
    And the wrapped length should be the message length plus 3

  Scenario: Unwrap a framed message
    Given an HL7 message "MSH|^~\\&|TEST\r"
    And the message is wrapped with MLLP framing
    When I unwrap the MLLP frame
    Then the unwrapped content should equal the original message

  Scenario: Roundtrip wrap then unwrap
    Given an HL7 message "MSH|^~\\&|App|Fac|Recv|RecvFac|20250128||ADT^A01|ABC123|P|2.5.1\rPID|1||12345\r"
    When I wrap it with MLLP framing
    And I unwrap the MLLP frame
    Then the unwrapped content should equal the original message

  Scenario: Detect MLLP framing on a framed message
    Given an HL7 message "MSH|^~\\&|TEST\r"
    And the message is wrapped with MLLP framing
    When I check if the data is MLLP framed
    Then the result should be true

  Scenario: Detect non-MLLP data
    Given raw bytes "MSH|^~\\&|TEST\r"
    When I check if the data is MLLP framed
    Then the result should be false

  Scenario: Missing start block error
    Given raw bytes "MSH|^~\\&|TEST\r"
    When I try to unwrap the data with checked unwrap
    Then the error should be MissingStartBlock

  Scenario: Missing end block error
    Given a byte sequence starting with 0x0B followed by "MSH|TEST"
    When I try to unwrap the data with checked unwrap
    Then the error should be MissingEndBlock

  Scenario: Empty message wrapping
    Given an empty message
    When I wrap it with MLLP framing
    Then the wrapped length should be 3
    And the first byte should be 0x0B
    And the second-to-last byte should be 0x1C
    And the last byte should be 0x0D

  Scenario: Find complete message length
    Given an HL7 message "MSH|^~\\&|TEST\r"
    And the message is wrapped with MLLP framing
    When I search for a complete MLLP message
    Then the found length should equal the wrapped data length

  Scenario: Frame iterator with single message
    Given an MLLP frame iterator
    And an HL7 message "MSH|^~\\&|SINGLE\r" wrapped with MLLP framing is added to the iterator
    When I extract the next message from the iterator
    Then the extracted message should be "MSH|^~\\&|SINGLE\r"
    And there should be no more messages in the iterator

  Scenario: Frame iterator with multiple messages
    Given an MLLP frame iterator
    And an HL7 message "MSH|^~\\&|FIRST\r" wrapped with MLLP framing is added to the iterator
    And an HL7 message "MSH|^~\\&|SECOND\r" wrapped with MLLP framing is added to the iterator
    When I extract the next message from the iterator
    Then the extracted message should be "MSH|^~\\&|FIRST\r"
    When I extract the next message from the iterator
    Then the extracted message should be "MSH|^~\\&|SECOND\r"
    And there should be no more messages in the iterator
