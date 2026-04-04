Feature: HL7 v2 CLI Operations
  As a healthcare integration developer
  I want to use the hl7v2-cli tool to process HL7 messages
  So that I can parse, validate, and generate HL7 v2 messages efficiently

  Background:
    Given the hl7v2-cli binary is available

  # =========================================================================
  # Parse Command Scenarios
  # =========================================================================

  Scenario: Parse a valid HL7 file
    Given a valid HL7 ADT^A01 message file
    When I run "parse" with the file path
    Then the command should succeed
    And the output should be valid JSON
    And the output should contain segment "MSH"

  Scenario: Parse with JSON formatting
    Given a valid HL7 ADT^A01 message file
    When I run "parse" with the file path and "--json" flag
    Then the command should succeed
    And the output should be formatted JSON

  Scenario: Parse with summary output
    Given a valid HL7 ADT^A01 message file
    When I run "parse" with the file path and "--summary" flag
    Then the command should succeed
    And the output should contain "Parse Summary"
    And the output should contain "Segments:"
    And the output should contain "File size:"

  Scenario: Parse MLLP-framed message
    Given an MLLP-framed HL7 message file
    When I run "parse" with the file path and "--mllp" flag
    Then the command should succeed
    And the output should be valid JSON
    And the output should contain segment "MSH"

  Scenario: Parse non-existent file
    Given a non-existent file path
    When I run "parse" with the file path
    Then the command should fail
    And the error output should contain "Error"

  Scenario: Parse invalid HL7 content
    Given a file with invalid HL7 content
    When I run "parse" with the file path
    Then the command should fail

  # =========================================================================
  # Normalize Command Scenarios
  # =========================================================================

  Scenario: Normalize HL7 message to file
    Given a valid HL7 ADT^A01 message file
    And an output file path
    When I run "norm" with input and output paths
    Then the command should succeed
    And the output file should exist
    And the output file should be valid HL7

  Scenario: Normalize to stdout
    Given a valid HL7 ADT^A01 message file
    When I run "norm" with the file path
    Then the command should succeed
    And the output should contain "MSH"

  Scenario: Normalize with MLLP output
    Given a valid HL7 ADT^A01 message file
    When I run "norm" with the file path and "--mllp-out" flag
    Then the command should succeed
    And the output should start with MLLP start block

  Scenario: Normalize with summary
    Given a valid HL7 ADT^A01 message file
    And an output file path
    When I run "norm" with input, output paths and "--summary" flag
    Then the command should succeed
    And the output should contain "Normalize Summary"

  # =========================================================================
  # Validate Command Scenarios
  # =========================================================================

  Scenario: Validate with matching profile
    Given a valid HL7 ADT^A01 message file
    And a minimal validation profile
    When I run "val" with the file and profile paths
    Then the command should succeed
    And the output should contain "Validation passed"

  Scenario: Validate with strict profile
    Given a valid HL7 ADT^A01 message file
    And a strict validation profile requiring ZZ1 segment
    When I run "val" with the file and profile paths
    Then the command should succeed

  Scenario: Validate with detailed output
    Given a valid HL7 ADT^A01 message file
    And a minimal validation profile
    When I run "val" with "--detailed" flag
    Then the command should succeed

  Scenario: Validate with summary
    Given a valid HL7 ADT^A01 message file
    And a minimal validation profile
    When I run "val" with "--summary" flag
    Then the command should succeed
    And the output should contain "Validation Summary"

  Scenario: Validate MLLP input
    Given an MLLP-framed HL7 message file
    And a minimal validation profile
    When I run "val" with "--mllp" flag
    Then the command should succeed
    And the output should contain "Validation passed"

  # =========================================================================
  # ACK Generation Scenarios
  # =========================================================================

  Scenario: Generate acceptance ACK
    Given a valid HL7 ADT^A01 message file
    When I run "ack" with the file path and code "AA"
    Then the command should succeed
    And the output should contain "MSA"
    And the output should be a valid HL7 ACK message

  Scenario: Generate error ACK
    Given a valid HL7 ADT^A01 message file
    When I run "ack" with the file path and code "AE"
    Then the command should succeed
    And the output should contain "MSA"

  Scenario: Generate reject ACK
    Given a valid HL7 ADT^A01 message file
    When I run "ack" with the file path and code "AR"
    Then the command should succeed
    And the output should contain "MSA"

  Scenario: Generate ACK with MLLP output
    Given a valid HL7 ADT^A01 message file
    When I run "ack" with "--mllp-out" flag and code "AA"
    Then the command should succeed
    And the output should start with MLLP start block

  Scenario: Generate ACK with summary
    Given a valid HL7 ADT^A01 message file
    When I run "ack" with "--summary" flag and code "AA"
    Then the command should succeed
    And the output should contain "ACK Generation Summary"

  Scenario: Generate ACK in original mode
    Given a valid HL7 ADT^A01 message file
    When I run "ack" with "--mode original" and code "AA"
    Then the command should succeed

  Scenario: Generate ACK in enhanced mode
    Given a valid HL7 ADT^A01 message file
    When I run "ack" with "--mode enhanced" and code "CA"
    Then the command should succeed

  # =========================================================================
  # Generate Command Scenarios
  # =========================================================================

  Scenario: Generate single message
    Given a valid template YAML file
    And an output directory
    When I run "gen" with the template and output paths
    Then the command should succeed
    And the output directory should contain generated messages

  Scenario: Generate multiple messages
    Given a valid template YAML file
    And an output directory
    When I run "gen" with count 5
    Then the command should succeed
    And the output directory should contain 5 messages

  Scenario: Generate with statistics
    Given a valid template YAML file
    And an output directory
    When I run "gen" with "--stats" flag
    Then the command should succeed
    And the output should contain generation statistics

  # =========================================================================
  # Help and Version Scenarios
  # =========================================================================

  Scenario: Display help
    When I run the command with "--help"
    Then the command should succeed
    And the output should contain "HL7 v2 parser"

  Scenario: Display version
    When I run the command with "--version"
    Then the command should succeed
    And the output should contain "hl7v2"

  Scenario: Display parse help
    When I run "parse" with "--help"
    Then the command should succeed
    And the output should contain "Parse HL7 v2 message"

  Scenario: Display validate help
    When I run "val" with "--help"
    Then the command should succeed
    And the output should contain "Validate"

  Scenario: Display ACK help
    When I run "ack" with "--help"
    Then the command should succeed
    And the output should contain "Generate ACK"

  # =========================================================================
  # Error Handling Scenarios
  # =========================================================================

  Scenario: Handle missing input file
    Given a non-existent file path
    When I run "parse" with the file path
    Then the command should fail with non-zero exit code
    And the error should indicate file not found

  Scenario: Handle invalid command
    When I run an invalid command
    Then the command should fail
    And the error should indicate unknown command

  Scenario: Handle missing required argument
    When I run "parse" without arguments
    Then the command should fail

  Scenario: Handle invalid ACK code
    Given a valid HL7 ADT^A01 message file
    When I run "ack" with invalid code "INVALID"
    Then the command should fail

  # =========================================================================
  # Edge Cases
  # =========================================================================

  Scenario: Handle empty file
    Given an empty file
    When I run "parse" with the file path
    Then the command should fail

  Scenario: Handle UTF-8 content
    Given a file with UTF-8 HL7 content
    When I run "parse" with the file path
    Then the command should succeed

  Scenario: Handle large message
    Given a file with a large HL7 message
    When I run "parse" with the file path
    Then the command should succeed
    And the output should be valid JSON
