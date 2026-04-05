Feature: Workspace Dependency Alignment
  As a maintainer
  I want all workspace crates to use workspace = true for shared dependencies
  So that we prevent version drift and ensure consistent dependency versions

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

  # ============================================================================
  # Workspace Dependency Consistency
  # ============================================================================

  @workspace-alignment @red-test
  Scenario: All workspace crates use workspace = true for tokio
    Given all workspace crates
    When I check the tokio dependency in each crate
    Then every crate should use workspace = true
    And no crate should have a hardcoded tokio version

  @workspace-alignment @red-test
  Scenario: All workspace crates use workspace = true for serde
    Given all workspace crates
    When I check the serde dependency in each crate
    Then every crate should use workspace = true
    And no crate should have a hardcoded serde version

  @workspace-alignment @red-test
  Scenario: All workspace crates use workspace = true for chrono
    Given all workspace crates
    When I check the chrono dependency in each crate
    Then every crate should use workspace = true
    And no crate should have a hardcoded chrono version

  @workspace-alignment @red-test
  Scenario: No workspace-managed dependencies have hardcoded versions in any crate
    Given the workspace root Cargo.toml defines managed dependencies
    And all workspace member crates
    When I check all dependencies in all crates
    Then no crate should have hardcoded versions for managed dependencies
    And the test should list all violations if any exist

  # ============================================================================
  # Dev Dependencies
  # ============================================================================

  @workspace-alignment @red-test
  Scenario: Dev dependencies also use workspace = true
    Given all workspace crates
    When I check dev-dependencies in each crate
    Then workspace-managed dev-dependencies should use workspace = true
    And no dev-dependency should have a hardcoded version for managed deps

  # ============================================================================
  # Dependency Drift Prevention
  # ============================================================================

  @drift-prevention @red-test
  Scenario: Adding hardcoded version causes test failure
    Given a workspace crate using workspace = true for tokio
    When a developer changes tokio to version "1.60.0"
    Then the workspace alignment tests should fail
    And the error message should indicate the specific crate and dependency

  @drift-prevention @red-test
  Scenario: New crate must use workspace dependencies
    Given a newly scaffolded workspace crate
    When it has dependencies that are workspace-managed
    Then it must use workspace = true for those dependencies
    And the build should fail if hardcoded versions are used
