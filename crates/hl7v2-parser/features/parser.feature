Feature: HL7 v2 Message Parsing
  As an HL7 message processor
  I want to parse HL7 v2 messages from raw bytes
  So that I can extract structured healthcare data

  # ---------- Basic parsing ----------

  Scenario: Parse a simple ADT^A01 message
    Given a valid ADT^A01 message with MSH and PID segments
    When I parse the message
    Then parsing should succeed
    And the message should have 2 segments
    And segment 1 should be "MSH"
    And segment 2 should be "PID"

  Scenario: Parse a message with multiple segments
    Given a message with MSH, PID, PV1, and OBX segments
    When I parse the message
    Then parsing should succeed
    And the message should have 4 segments
    And segment 1 should be "MSH"
    And segment 2 should be "PID"
    And segment 3 should be "PV1"
    And segment 4 should be "OBX"

  # ---------- MSH field extraction ----------

  Scenario: Extract MSH sending application
    Given a valid ADT^A01 message with MSH and PID segments
    When I parse the message
    Then MSH.3 should be "SendingApp"

  Scenario: Extract MSH sending and receiving facilities
    Given a valid ADT^A01 message with MSH and PID segments
    When I parse the message
    Then MSH.4 should be "SendingFac"
    And MSH.5 should be "ReceivingApp"
    And MSH.6 should be "ReceivingFac"

  Scenario: Extract MSH message type components
    Given a valid ADT^A01 message with MSH and PID segments
    When I parse the message
    Then MSH.9.1 should be "ADT"
    And MSH.9.2 should be "A01"

  Scenario: Extract MSH control ID and processing ID
    Given a valid ADT^A01 message with MSH and PID segments
    When I parse the message
    Then MSH.10 should be "ABC123"
    And MSH.11 should be "P"

  # ---------- Custom delimiters ----------

  Scenario: Parse message with custom delimiters
    Given a message with custom delimiters "#$*@!"
    When I parse the message
    Then parsing should succeed
    And the field delimiter should be "#"
    And the component delimiter should be "$"
    And the repetition delimiter should be "*"
    And the escape delimiter should be "@"
    And the subcomponent delimiter should be "!"

  # ---------- Field repetitions ----------

  Scenario: Parse message with field repetitions
    Given a message with repeated patient names "Doe^John~Smith^Jane"
    When I parse the message
    Then PID.5[1].1 should be "Doe"
    And PID.5[1].2 should be "John"
    And PID.5[2].1 should be "Smith"
    And PID.5[2].2 should be "Jane"

  # ---------- Components and subcomponents ----------

  Scenario: Parse message with components
    Given a message with patient ID "123456^^^HOSP^MR"
    When I parse the message
    Then PID.3.1 should be "123456"
    And PID.3.4 should be "HOSP"
    And PID.3.5 should be "MR"

  Scenario: Parse message with subcomponents
    Given a message with subcomponent value "MainId&SubId" in PID-3
    When I parse the message
    Then PID.3.1 should contain subcomponents "MainId" and "SubId"

  # ---------- MLLP framing ----------

  Scenario: Parse MLLP-framed message
    Given an MLLP-framed ADT^A01 message
    When I parse the MLLP message
    Then parsing should succeed
    And the message should have 2 segments
    And MSH.9.1 should be "ADT"
    And MSH.9.2 should be "A01"

  # ---------- Error handling ----------

  Scenario: Handle empty input
    Given an empty byte input
    When I attempt to parse the message
    Then parsing should fail
    And the error should indicate an invalid segment

  Scenario: Handle missing MSH segment
    Given a message starting with "PID" instead of MSH
    When I attempt to parse the message
    Then parsing should fail
    And the error should indicate an invalid segment

  Scenario: Handle non-UTF8 input
    Given a byte sequence with invalid UTF-8
    When I attempt to parse the message
    Then parsing should fail
    And the error should indicate an invalid charset

  # ---------- Escape sequences ----------

  Scenario: Parse message with escape sequences
    Given a message with escape sequence "\F\" in a field value
    When I parse the message
    Then parsing should succeed
    And the unescaped field value should contain "|"

  # ---------- Segment count validation ----------

  Scenario: Validate segment count for a large message
    Given a message with 10 OBX segments
    When I parse the message
    Then parsing should succeed
    And the message should have 12 segments

  # ---------- HL7 version ----------

  Scenario: Parse HL7 v2.5.1 message
    Given a message with HL7 version "2.5.1"
    When I parse the message
    Then MSH.12 should be "2.5.1"

  Scenario: Parse HL7 v2.3 message
    Given a message with HL7 version "2.3"
    When I parse the message
    Then MSH.12 should be "2.3"

  # ---------- Batch parsing ----------

  Scenario: Parse batch with BHS header
    Given a batch message with BHS, 2 MSH messages, and BTS
    When I parse the batch
    Then batch parsing should succeed
    And the batch should contain 2 messages

  Scenario: Parse file batch with FHS header
    Given a file batch with FHS, BHS, 1 MSH message, BTS, and FTS
    When I parse the file batch
    Then file batch parsing should succeed
    And the file batch should contain 1 batch
