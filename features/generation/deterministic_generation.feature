Feature: Deterministic Message Generation
  As a test engineer
  I want to generate reproducible HL7 test messages
  So that I can create consistent test datasets

  Background:
    Given the message generator is initialized

  Scenario: Generate identical messages with same seed
    Given a template "adt_a01.yaml"
    And seed value 42
    When I generate a message
    And I generate another message with the same seed
    Then both messages should be byte-for-byte identical

  Scenario: Generate different messages with different seeds
    Given a template "adt_a01.yaml"
    When I generate a message with seed 42
    And I generate a message with seed 1337
    Then the messages should differ

  Scenario: Generate with realistic data
    Given a template with realistic patient name generator
    When I generate 100 messages
    Then all patient names should be plausible
    And names should follow gender conventions
    And there should be variety in names

  Scenario: Generate corpus with manifest
    Given a template "adt_a01.yaml"
    And seed 42
    When I generate 100 messages
    Then a manifest.json file should be created
    And the manifest should include tool version
    And the manifest should include the seed value
    And the manifest should include template SHA-256
    And the manifest should list all 100 generated files with SHA-256 hashes

  Scenario: Verify corpus integrity
    Given a previously generated corpus with manifest
    When I run corpus verification
    Then all SHA-256 hashes should match
    And verification should succeed

  Scenario: Detect corpus tampering
    Given a corpus with manifest
    When I modify one message file
    And I run corpus verification
    Then verification should fail
    And the error should identify the tampered file

  Scenario: Generate with error injection
    Given a template with 10% error injection rate
    When I generate 100 messages
    Then approximately 10 messages should have intentional errors
    And errors should be realistic (invalid segment IDs, malformed fields)

  Scenario: Generate with statistical distributions
    Given a template with age distribution: normal(mean=45, std=15)
    When I generate 1000 messages
    Then the age values should follow a normal distribution
    And mean age should be approximately 45
    And standard deviation should be approximately 15

  Scenario: Generate with value lists
    Given a template with gender from list ["M", "F", "O"]
    When I generate 300 messages
    Then all gender values should be from the list
    And distribution should be roughly balanced

  Scenario: Generate ICD-10 codes
    Given a template using realistic ICD-10 code generator
    When I generate a message
    Then the diagnosis code should be a valid ICD-10 format
    And the code should be clinically plausible

  Scenario: Generate LOINC codes
    Given a template using LOINC code generator
    When I generate an ORU^R01 message
    Then OBX.3 should contain valid LOINC codes
    And codes should match the observation type

  Scenario: Generate addresses
    Given a template with US address generator
    When I generate a message
    Then PID.11 should have valid US address components
    And the ZIP code should match the state

  Scenario: Generate phone numbers
    Given a template with US phone number generator
    When I generate a message
    Then phone numbers should be in format "+1-XXX-XXX-XXXX"
    And area codes should be valid US area codes

  Scenario: Generate correlated fields
    Given a template with correlated BMI, height, and weight
    When I generate a message
    Then BMI should be consistent with height and weight

  Scenario: Generate with train/validation/test splits
    Given a template and seed 42
    When I generate a corpus with 80/10/10 split
    Then 80% of messages should be in train set
    And 10% should be in validation set
    And 10% should be in test set
    And the manifest should document the splits
