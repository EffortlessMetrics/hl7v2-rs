Feature: HTTP Server Mode
  As a system administrator
  I want to run hl7v2 as an HTTP service
  So that I can provide HL7 processing as a web API

  Background:
    Given the HTTP server is started on port 8080

  Scenario: Health check endpoint
    When I send GET request to "/health"
    Then the response status should be 200
    And the response should be "OK"

  Scenario: Readiness check endpoint
    When I send GET request to "/ready"
    Then the response status should be 200
    And the response should indicate ready state

  Scenario: Parse message via HTTP
    Given a valid HL7 ADT^A01 message
    When I POST the message to "/hl7/parse"
    Then the response status should be 200
    And the response Content-Type should be "application/json"
    And the response should contain parsed message JSON

  Scenario: Validate message via HTTP
    Given a valid HL7 message and profile name "adt_a01"
    When I POST to "/hl7/validate" with:
      """json
      {
        "message": "<hl7 message>",
        "profile_name": "adt_a01"
      }
      """
    Then the response status should be 200
    And the response should contain validation results

  Scenario: Handle invalid message gracefully
    Given malformed HL7 data
    When I POST to "/hl7/parse"
    Then the response status should be 400
    And the response should contain error details

  Scenario: Generate ACK via HTTP
    Given a valid HL7 message
    When I POST to "/hl7/ack" with code "AA"
    Then the response status should be 200
    And the response should contain an ACK message

  Scenario: Handle concurrent requests
    Given 100 concurrent parse requests
    When I send all requests in parallel
    Then all requests should complete successfully
    And response times should be under 100ms p95

  Scenario: Apply backpressure when overloaded
    Given the server queue capacity is 10
    When I send 20 requests simultaneously
    Then the first 10 should be accepted (200)
    And subsequent requests should receive 429 (Too Many Requests)
    And the 429 response should include Retry-After header

  Scenario: Authentication with Bearer token
    Given the server requires authentication
    When I send a request without Authorization header
    Then the response status should be 401
    When I send a request with valid Bearer token
    Then the response status should be 200

  Scenario: RBAC authorization
    Given user "analyst" has role "read-only"
    When user "analyst" attempts POST to "/hl7/parse"
    Then the request should succeed
    When user "analyst" attempts DELETE to "/admin/cache"
    Then the response status should be 403 (Forbidden)

  Scenario: PHI redaction in logs
    Given logging is configured with redact_phi=true
    When I parse a message containing patient name "John Doe"
    Then logs should not contain "John Doe"
    And logs should contain segment/field structure only

  Scenario: Request tracing with correlation ID
    When I send a request with header "X-Trace-ID: abc-123"
    Then the response should include header "X-Trace-ID: abc-123"
    And all log entries for this request should include trace_id="abc-123"

  Scenario: Streaming upload and response
    Given a file containing 1000 HL7 messages
    When I POST the file with Transfer-Encoding: chunked
    Then the server should parse incrementally
    And respond with NDJSON stream of results

  Scenario: Prometheus metrics endpoint
    Given the server has processed 100 messages
    When I send GET to "/metrics"
    Then the response should include Prometheus format metrics
    And hl7_messages_parsed_total should be 100
    And hl7_parse_duration_ms histogram should have values

  Scenario: Graceful shutdown
    Given the server is processing requests
    When I send SIGTERM signal
    Then the server should finish in-flight requests
    And the server should stop accepting new requests
    And the server should shut down within 30 seconds
