# EFF-1194: gRPC Server Implementation - BDD Scenarios

## Feature: gRPC Parse RPC

### Scenario: Parse a valid HL7 message
```gherkin
Given the gRPC server is running on port 9090
And a valid HL7 ADT^A01 message
When I send a ParseRequest with the message bytes
Then I receive a ParseResponse with success=true
And the response contains the parsed Message structure
And the metadata includes message type "ADT^A01"
```

### Scenario: Parse an MLLP-framed message
```gherkin
Given the gRPC server is running
And an HL7 message wrapped in MLLP framing (0x0B ... 0x1C 0x0D)
When I send a ParseRequest with mllp_framed=true
Then I receive a successful ParseResponse
And the MLLP framing is correctly stripped
```

### Scenario: Parse with invalid message
```gherkin
Given the gRPC server is running
And an invalid/corrupted HL7 message
When I send a ParseRequest
Then I receive a ParseResponse with success=false
And the errors array contains at least one error with INVALID_MESSAGE code
```

### Scenario: Parse with strict mode
```gherkin
Given the gRPC server is running
And an HL7 message with version 2.3
When I send a ParseRequest with strict=true and expected_version="2.5"
Then I receive a ParseResponse with success=false
And the errors indicate version mismatch
```

---

## Feature: gRPC Validate RPC

### Scenario: Validate message against profile
```gherkin
Given the gRPC server is running
And a valid ADT^A01 message
And a registered "adt-a01-basic" profile
When I send a ValidateRequest with profile="adt-a01-basic"
Then I receive a ValidateResponse with valid=true
And the errors array is empty
```

### Scenario: Validation fails with errors
```gherkin
Given the gRPC server is running
And an ADT^A01 message missing required PID-3 (Patient ID)
And a strict "adt-a01-full" profile
When I send a ValidateRequest with profile="adt-a01-full"
Then I receive a ValidateResponse with valid=false
And the errors array contains a REQUIRED_FIELD_MISSING error for PID-3
```

### Scenario: Validation with warnings
```gherkin
Given the gRPC server is running
And an ADT^A01 message with extra optional segments
And a basic profile that doesn't use those segments
When I send a ValidateRequest with fail_on_warning=false
Then I receive a ValidateResponse with valid=true
And the warnings array contains UNEXPECTED_SEGMENT entries
```

---

## Feature: gRPC GenerateAck RPC

### Scenario: Generate Application Accept (AA) ACK
```gherkin
Given the gRPC server is running
And a received HL7 message with control ID "MSG001"
When I send an AckRequest with code=AA
Then I receive an AckResponse
And the ack_message contains an ACK segment with MSA-1="AA"
And the MSA-2 matches the original control ID "MSG001"
```

### Scenario: Generate Application Error (AE) ACK
```gherkin
Given the gRPC server is running
And a received HL7 message that failed processing
When I send an AckRequest with code=AE and error_message="Invalid patient ID"
Then I receive an AckResponse
And the ack_message contains MSA-1="AE"
And the ERR segment contains the error details
```

### Scenario: Generate Application Reject (AR) ACK
```gherkin
Given the gRPC server is running
And a completely unsupported message type
When I send an AckRequest with code=AR
Then I receive an AckResponse
And the ack_message contains MSA-1="AR"
```

---

## Feature: gRPC Normalize RPC

### Scenario: Normalize message with canonical delimiters
```gherkin
Given the gRPC server is running
And an HL7 message using non-standard delimiters
When I send a NormalizeRequest with canonical_delimiters=true
Then I receive a NormalizeResponse
And the normalized message uses standard |^~\& delimiters
```

### Scenario: Normalize and add MLLP framing
```gherkin
Given the gRPC server is running
And a raw HL7 message without framing
When I send a NormalizeRequest with mllp_frame=true
Then I receive a NormalizeResponse
And the normalized message is wrapped in MLLP framing bytes
```

---

## Feature: gRPC ParseStream RPC (Bidirectional Streaming)

### Scenario: Stream multiple messages
```gherkin
Given the gRPC server is running
And a stream of 100 ParseRequests
When I open a bidirectional ParseStream
And send all 100 requests
Then I receive exactly 100 ParseResponses
And each response corresponds to its request
And all responses are received within 5 seconds
```

### Scenario: Streaming with backpressure
```gherkin
Given the gRPC server is running
And a client that sends messages faster than they can be processed
When I open a ParseStream and flood with 10000 requests
Then the server applies HTTP/2 flow control backpressure
And no server-side buffer overflow occurs
And memory usage remains stable
```

### Scenario: Streaming error handling
```gherkin
Given the gRPC server is running
And a stream containing valid and invalid messages
When I send a ParseStream with mixed valid/invalid messages
Then valid messages return success responses
And invalid messages return error responses
And the stream remains open for subsequent messages
```

---

## Feature: gRPC HealthCheck RPC

### Scenario: Server is healthy
```gherkin
Given the gRPC server is running and fully initialized
When I send a HealthCheckRequest
Then I receive a HealthCheckResponse with status=SERVING
And the version field matches the crate version
And the uptime_seconds is greater than 0
```

### Scenario: Server not yet ready
```gherkin
Given the gRPC server is starting up
When I send a HealthCheckRequest during initialization
Then I receive a HealthCheckResponse with status=NOT_SERVING
```

---

## Feature: gRPC Server Integration

### Scenario: Server starts on configured port
```gherkin
Given the gRPC feature is enabled
When I run "hl7v2 serve --mode grpc --port 9090"
Then the server starts successfully
And binds to port 9090
And logs "Starting gRPC server on 0.0.0.0:9090"
```

### Scenario: Server graceful shutdown
```gherkin
Given the gRPC server is running with active connections
When a SIGINT (Ctrl+C) is received
Then the server stops accepting new connections
And waits up to 30 seconds for in-flight requests to complete
Then shuts down gracefully
```

### Scenario: Tower middleware integration
```gherkin
Given the gRPC server is running with tracing enabled
When I send any RPC request
Then the request is logged with trace_id
And metrics are recorded for the RPC
And rate limiting is applied per configuration
```

---

## Feature: gRPC Client Integration

### Scenario: tonic client can connect
```gherkin
Given the gRPC server is running
And a Rust tonic client is configured
When the client connects to the server
Then the connection succeeds
And the client can call the Parse RPC
```

### Scenario: Cross-language Go client
```gherkin
Given the gRPC server is running
And a Go client generated from hl7v2.proto
When the Go client connects and calls HealthCheck
Then the call succeeds
And the response contains valid health status
```

---

## Feature: Error Handling

### Scenario: Invalid proto request
```gherkin
Given the gRPC server is running
When I send a malformed ParseRequest (missing required fields)
Then I receive a gRPC INVALID_ARGUMENT status
And the error message indicates the missing field
```

### Scenario: Server-side error
```gherkin
Given the gRPC server is running
When an internal error occurs during message processing
Then I receive a gRPC INTERNAL status
And the error message does not expose sensitive details
And the trace_id is included for debugging
```

### Scenario: Deadline exceeded
```gherkin
Given the gRPC server is running
And a request with a 100ms deadline
When the processing takes longer than 100ms
Then I receive a gRPC DEADLINE_EXCEEDED status
And the server cancels the in-flight processing
```
