Feature: Basic HL7 Message Parsing
  As a healthcare system integrator
  I want to parse HL7 v2 messages reliably
  So that I can extract and process clinical data

  Background:
    Given the hl7v2 parser is initialized

  Scenario: Parse simple ADT message with standard delimiters
    Given an HL7 ADT^A01 message with standard delimiters:
      """
      MSH|^~\&|SENDER|FACILITY|RECEIVER|FACILITY|20231119||ADT^A01|123456|P|2.5
      PID|1||MRN123^^^FACILITY^MR||DOE^JOHN^A||19800101|M|||123 MAIN ST^^CITY^ST^12345
      PV1|1|I|WARD^ROOM^BED||||DOCTOR^ATTENDING^A|||SUR||||ADM|A0|1234567890
      """
    When I parse the message
    Then the parsing should succeed
    And the message should have 3 segments
    And segment 1 should be "MSH"
    And segment 2 should be "PID"
    And segment 3 should be "PV1"
    And the message delimiters should be:
      | field     | \|  |
      | component | ^   |
      | repetition| ~   |
      | escape    | \\  |
      | subcomp   | &   |

  Scenario: Parse message with custom delimiters
    Given an HL7 message with custom field delimiter "|" and component delimiter "~":
      """
      MSH|~\&^|SENDER|FACILITY|RECEIVER|FACILITY|20231119||ORU^R01|654321|P|2.5
      """
    When I parse the message
    Then the parsing should succeed
    And the field delimiter should be "|"
    And the component delimiter should be "~"

  Scenario: Handle truncated message gracefully
    Given a truncated HL7 message:
      """
      MSH|^~\&|SENDER|FACILITY|RECEIVER
      """
    When I attempt to parse the message
    Then parsing should fail with error code "P_Truncated"
    And the error should include byte offset information
    And the error should suggest checking message completeness

  Scenario: Reject message without MSH segment
    Given an invalid message without MSH:
      """
      PID|1||123456
      """
    When I attempt to parse the message
    Then parsing should fail with error code "P_MissingMSH"
    And the error message should contain "must start with MSH"

  Scenario: Parse MLLP-framed message
    Given an MLLP-framed HL7 message
    When I parse the message with MLLP framing
    Then the parsing should succeed
    And the MLLP frame should be removed
    And the message content should be valid HL7

  Scenario: Parse batch message with multiple messages
    Given an HL7 batch with BHS and BTS:
      """
      BHS|^~\&|SENDER|||20231119|BATCH001
      MSH|^~\&|SENDER||RECEIVER||20231119||ADT^A01|1|P|2.5
      PID|1||123
      MSH|^~\&|SENDER||RECEIVER||20231119||ADT^A01|2|P|2.5
      PID|1||456
      BTS|2
      """
    When I parse the batch
    Then batch parsing should succeed
    And the batch should contain 2 messages
    And message 1 should have patient ID "123"
    And message 2 should have patient ID "456"

  Scenario: Detect delimiter configuration from MSH
    Given an HL7 message
    When I parse the MSH segment
    Then the parser should detect delimiters from MSH.1 and MSH.2
    And subsequent fields should be parsed with detected delimiters

  Scenario Outline: Parse various message types
    Given an HL7 <message_type> message
    When I parse the message
    Then the parsing should succeed
    And the message structure should be <message_type>

    Examples:
      | message_type |
      | ADT^A01      |
      | ADT^A04      |
      | ORU^R01      |
      | ORM^O01      |
      | RDE^O11      |
      | SIU^S12      |

  Scenario: Handle escape sequences correctly
    Given an HL7 message with escape sequences:
      """
      MSH|^~\&|SENDER|||20231119||ADT^A01|1|P|2.5
      PID|1||123||DOE\S\JOHN||19800101
      """
    When I parse the message
    Then the parsing should succeed
    And PID.5 should be unescaped to "DOE^JOHN"

  Scenario: Handle empty vs null fields
    Given an HL7 message with various field presence patterns:
      """
      MSH|^~\&|SENDER|||20231119||ADT^A01|1|P|2.5
      PID|1||||DOE^JOHN|""|19800101
      """
    When I parse the message
    Then field PID.2 should be "missing"
    And field PID.3 should be "empty"
    And field PID.4 should be "empty"
    And field PID.5 should be "value"
    And field PID.6 should be "null"
    And field PID.7 should be "value"

  Scenario: Round-trip parsing and serialization
    Given a valid HL7 message
    When I parse the message
    And I serialize it back to HL7
    And I parse the serialized message
    Then the two parsed messages should be semantically identical
