Feature: HL7 v2 Batch Message Handling
  As an HL7 message processor
  I want to parse and handle batch messages containing multiple HL7 messages
  So that I can process multiple messages efficiently in a single transmission

  Scenario: Parse a single batch (BHS/BTS only)
    Given a batch with BHS and BTS containing 2 messages
    When I parse the batch
    Then the batch type should be Single
    And the batch should contain 2 messages
    And batch message 1 should have patient ID "123456"
    And batch message 2 should have patient ID "789012"

  Scenario: Parse a file batch (FHS/BHS/FTS/BTS)
    Given a file batch with FHS, BHS, BTS, and FTS containing 3 messages
    When I parse the batch
    Then the batch type should be File
    And the batch should contain 3 messages
    And the batch name should be "BATCH001"

  Scenario: Parse batch with custom delimiters
    Given a batch with custom delimiters "#$*@!"
    When I parse the batch
    Then the batch should parse successfully
    And the delimiters should be "#$*@!"

  Scenario: Parse batch with nested batches
    Given a file batch with 2 nested batches
    When I parse the batch
    Then the batch type should be File
    And the batch should contain nested batches

  Scenario: Extract batch metadata from BHS
    Given a batch with BHS containing metadata
    When I parse the batch
    Then the sending application should be "SendingApp"
    And the sending facility should be "SendingFac"
    And the receiving application should be "ReceivingApp"
    And the receiving facility should be "ReceivingFac"
    And the batch name should be "BATCH001"
    And the batch comment should be "Test batch"

  Scenario: Extract batch metadata from FHS
    Given a file batch with FHS containing metadata
    When I parse the batch
    Then the sending application should be "FileSender"
    And the sending facility should be "FileFacility"
    And the receiving application should be "FileReceiver"
    And the receiving facility should be "FileFacility"
    And the file creation time should be present

  Scenario: Validate batch count in BTS matches actual messages
    Given a batch with BTS count of 2 and 2 messages
    When I parse the batch
    Then the batch message count should be 2
    And the BTS count should match the actual message count

  Scenario: Validate batch count in FTS matches actual messages
    Given a file batch with FTS count of 3 and 3 messages
    When I parse the batch
    Then the file message count should be 3
    And the FTS count should match the actual message count

  Scenario: Handle batch with mismatched count
    Given a batch with BTS count of 3 but only 2 messages
    When I parse the batch
    Then the batch should have count mismatch error

  Scenario: Handle batch without BHS segment
    Given invalid batch data without BHS
    When I attempt to parse the batch
    Then an error should be returned
    And the error should indicate missing BHS segment

  Scenario: Handle batch without BTS segment
    Given invalid batch data without BTS
    When I attempt to parse the batch
    Then an error should be returned
    And the error should indicate missing BTS segment

  Scenario: Handle file batch without FHS segment
    Given invalid file batch data without FHS
    When I attempt to parse the batch
    Then an error should be returned
    And the error should indicate missing FHS segment

  Scenario: Handle file batch without FTS segment
    Given invalid file batch data without FTS
    When I attempt to parse the batch
    Then an error should be returned
    And the error should indicate missing FTS segment

  Scenario: Parse empty batch
    Given a batch with BHS and BTS but no messages
    When I parse the batch
    Then the batch should contain 0 messages
    And the batch should be valid

  Scenario: Parse batch with security field
    Given a batch with BHS security field set to "SECURE"
    When I parse the batch
    Then the batch security should be "SECURE"

  Scenario: Parse batch with trailer comment
    Given a batch with BTS comment "End of batch"
    When I parse the batch
    Then the trailer comment should be "End of batch"

  Scenario Outline: Parse batches with different message types
    Given a batch containing <message_type> messages
    When I parse the batch
    Then the batch should parse successfully
    And each message should be of type <message_type>

    Examples:
      | message_type |
      | ADT^A01      |
      | ORU^R01      |
      | ORM^O01      |
      | DFT^P03      |
