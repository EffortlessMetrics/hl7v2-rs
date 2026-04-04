Feature: Deterministic Message Generation
  As a test engineer
  I want to generate reproducible HL7 test messages
  So that I can create consistent test datasets

  # ============================================================================
  # Deterministic Generation
  # ============================================================================

  Scenario: Generate identical messages with same seed
    Given a simple ADT template
    And seed value 42
    When I generate a message
    And I generate another message with the same seed
    Then both messages should be byte-for-byte identical

  Scenario: Generate different messages with different seeds
    Given a template with dynamic values
    When I generate a message with seed 42
    And I generate a message with seed 1337
    Then the messages should differ

  # ============================================================================
  # Multiple Messages
  # ============================================================================

  Scenario: Generate multiple messages
    Given a simple ADT template
    When I generate 100 messages with seed 42
    Then I should receive 100 messages
    And all messages should be valid HL7

  Scenario: Generate zero messages
    Given a simple ADT template
    When I generate 0 messages with seed 42
    Then I should receive 0 messages

  # ============================================================================
  # Value Sources
  # ============================================================================

  Scenario: Generate with fixed values
    Given a template with PID.5 fixed to "Smith^John"
    When I generate a message with seed 42
    Then the generated message should be valid HL7

  Scenario: Generate with value lists
    Given a template with PID.8 from list "M,F,O"
    When I generate 30 messages with seed 42
    Then I should receive 30 messages
    And all PID.8 values should be from the list "M,F,O"

  Scenario: Generate with UUID values
    Given a template with PID.3 as UUID
    When I generate a message with seed 42
    Then the generated message should be valid HL7

  Scenario: Generate with numeric values
    Given a template with PID.3 as 6-digit numeric
    When I generate a message with seed 42
    Then the generated message should be valid HL7

  Scenario: Generate with date range values
    Given a template with PID.7 as date between "20200101" and "20251231"
    When I generate a message with seed 42
    Then the generated message should be valid HL7

  Scenario: Generate with gaussian distribution
    Given a template with OBX.5 as gaussian mean 100.0 stddev 10.0
    When I generate a message with seed 42
    Then the generated message should be valid HL7

  # ============================================================================
  # ACK Generation
  # ============================================================================

  Scenario: Generate ACK for valid message
    Given a valid HL7 message to acknowledge
    When I generate an ACK with code AA
    Then the ACK should have MSH and MSA segments
    And MSA.1 should be "AA"

  Scenario: Generate ACK with error
    Given a valid HL7 message to acknowledge
    When I generate an ACK with error code AE and text "Segment error"
    Then the ACK should have MSH, MSA, and ERR segments
    And MSA.1 should be "AE"

  # ============================================================================
  # Different Message Types
  # ============================================================================

  Scenario: Generate ORU^R01 message
    Given an ORU template with OBX segments
    When I generate a message with seed 42
    Then the generated message should contain segment "OBX"
    And the generated message should be valid HL7

  Scenario: Generate corpus with deterministic output
    Given a simple ADT template
    When I generate 50 messages with seed 42
    And I generate 50 messages again with seed 42
    Then both corpora should be identical

  # ============================================================================
  # Faker Integration
  # ============================================================================

  Scenario: Generate with faker names
    Given a faker with seed 42
    When I generate a patient name for gender "M"
    Then the name should contain a component separator
