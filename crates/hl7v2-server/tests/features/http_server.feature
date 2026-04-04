Feature: HTTP Server Mode
  As a system administrator
  I want to run hl7v2 as an HTTP service
  So that I can provide HL7 processing as a web API

  # ============================================================================
  # Health & Readiness
  # ============================================================================

  Scenario: Health check endpoint
    Given the test server is running
    When I send GET request to "/health"
    Then the response status should be 200
    And the response should contain "healthy"

  Scenario: Readiness check endpoint
    Given the test server is running
    When I send GET request to "/ready"
    Then the response status should be 200
    And the response should contain "ready"

  # ============================================================================
  # Parse Endpoint
  # ============================================================================

  Scenario: Parse valid ADT^A01 message via HTTP
    Given the test server is running
    And a valid HL7 ADT^A01 message payload
    When I POST the message to "/hl7/parse"
    Then the response status should be 200
    And the response Content-Type should contain "application/json"
    And the response should contain "ADT"

  Scenario: Parse valid ORU^R01 message via HTTP
    Given the test server is running
    And a valid HL7 ORU^R01 message payload
    When I POST the message to "/hl7/parse"
    Then the response status should be 200
    And the response should contain "ORU"

  Scenario: Handle invalid message gracefully
    Given the test server is running
    And a malformed HL7 message payload
    When I POST the message to "/hl7/parse"
    Then the response status should be 400

  Scenario: Handle invalid JSON gracefully
    Given the test server is running
    And an invalid JSON payload
    When I POST raw body to "/hl7/parse"
    Then the response status should be 400

  Scenario: Parse response contains metadata
    Given the test server is running
    And a valid HL7 ADT^A01 message payload
    When I POST the message to "/hl7/parse"
    Then the response status should be 200
    And the response should contain "metadata"
    And the response should contain "message_type"
    And the response should contain "version"

  # ============================================================================
  # Validate Endpoint
  # ============================================================================

  Scenario: Validate message via HTTP
    Given the test server is running
    And a valid HL7 ADT^A01 message payload with profile
    When I POST to "/hl7/validate"
    Then the response status should be 200
    And the response should contain "valid"

  # ============================================================================
  # Error Handling
  # ============================================================================

  Scenario: GET method not allowed on parse endpoint
    Given the test server is running
    When I send GET request to "/hl7/parse"
    Then the response status should be 405

  Scenario: POST to nonexistent endpoint returns 404
    Given the test server is running
    When I POST raw body to "/nonexistent"
    Then the response status should be 404

  # ============================================================================
  # Metrics
  # ============================================================================

  Scenario: Prometheus metrics endpoint
    Given the test server is running
    When I send GET request to "/metrics"
    Then the response status should be 200

  # ============================================================================
  # Authentication
  # ============================================================================

  Scenario: Unauthenticated request to protected server
    Given the test server is running with API key "test-secret"
    And a valid HL7 ADT^A01 message payload
    When I POST without credentials to "/hl7/parse"
    Then the response status should be 401

  Scenario: Authenticated request to protected server
    Given the test server is running with API key "test-secret"
    And a valid HL7 ADT^A01 message payload
    When I POST with API key "test-secret" to "/hl7/parse"
    Then the response status should be 200

  # ============================================================================
  # Response Format
  # ============================================================================

  Scenario: Health check includes version
    Given the test server is running
    When I send GET request to "/health"
    Then the response status should be 200
    And the response should contain "version"

  Scenario: Health check includes uptime
    Given the test server is running
    When I send GET request to "/health"
    Then the response status should be 200
    And the response should contain "uptime_seconds"

  Scenario: Error response has error code
    Given the test server is running
    And a malformed HL7 message payload
    When I POST the message to "/hl7/parse"
    Then the response status should be 400
    And the response should contain "code"
