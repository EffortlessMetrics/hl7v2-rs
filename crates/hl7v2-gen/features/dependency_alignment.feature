Feature: Workspace Dependency Alignment
  As a maintainer
  I want hl7v2-gen to keep using the workspace tokio dependency
  So that EFF-1136 does not regress

  # ============================================================================
  # EFF-1136: Tokio Dependency Alignment
  # ============================================================================

  @eff-1136 @red-test
  Scenario: hl7v2-gen tokio uses workspace version
    Given the hl7v2-gen crate Cargo.toml
    When I check the tokio dependency
    Then it should use workspace = true
    And it should NOT have a hardcoded version

  @eff-1136 @red-test @regression-prevention
  Scenario: Reverting tokio to hardcoded version fails
    Given the hl7v2-gen crate Cargo.toml
    When the tokio dependency has a hardcoded version like "1.49.0"
    Then the workspace alignment test should fail
    And the error should mention "EFF-1136 REGRESSION"
