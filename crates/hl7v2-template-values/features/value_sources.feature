Feature: Template Value Source Generation
  As an HL7 template generator
  I want to generate values from template value sources
  So that message content remains predictable for fixed and randomized paths

  Scenario: Generate a fixed value source
    Given a fixed value source of "ALICE"
    When I generate the value
    Then the generated value should be "ALICE"

  Scenario: Generate a random value from a list
    Given a from-value source with options "red", "green", "blue"
    When I generate the value
    Then the generated value should be one of "red", "green", or "blue"

  Scenario: Generate an injected error value source
    Given an injected invalid segment id value source
    When I attempt to generate the value
    Then the generation should fail
