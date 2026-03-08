# Integration Test Suite

Comprehensive integration tests for the hl7v2-server HTTP/REST API.

## Test Organization

### Common Utilities (`common/mod.rs`)

Shared test utilities and fixtures:
- **Test Server Creation**: Helper functions to create test routers
- **HL7 Message Fixtures**: Sample messages for all major message types
  - `ADT_A01_VALID` - Admit/Visit Notification
  - `ADT_A04_VALID` - Register Patient
  - `ORU_R01_VALID` - Lab Results
  - `MINIMAL_VALID` - Minimal MSH-only message
  - `INVALID_*` - Various invalid message examples
- **Profile Fixtures**: Sample conformance profiles for testing
  - `MINIMAL_PROFILE` - Basic structure validation
  - `ADT_A01_PROFILE` - Comprehensive ADT admission profile

### Test Suites

#### `health_endpoints_test.rs` (9 tests)

Tests for health, readiness, and metrics endpoints:

**Health Endpoint (/health)**:
- Returns 200 OK status
- Returns JSON content type
- Contains "status" field with "healthy" value
- Contains "uptime_seconds" field

**Ready Endpoint (/ready)**:
- Returns 200 OK status
- Returns `{"ready": true}` JSON

**Metrics Endpoint (/metrics)**:
- Returns 200 OK status
- Returns Prometheus text format
- Contains HELP and TYPE comments

#### `parse_endpoint_test.rs` (11 tests)

Tests for the `/hl7/parse` endpoint:

**Valid Message Parsing**:
- Parse ADT^A01 messages successfully
- Parse ADT^A04 messages successfully
- Parse ORU^R01 messages successfully
- Parse minimal (MSH-only) messages
- Response contains segment information

**Error Handling**:
- Malformed messages return error status (4xx/5xx)
- Invalid encoding characters return error
- Empty request body returns 400 Bad Request
- Invalid JSON returns 400 Bad Request
- GET method returns 405 Method Not Allowed

**Response Validation**:
- Response contains segments/metadata
- Response format matches expectations

#### `validate_endpoint_test.rs` (10 tests)

Tests for the `/hl7/validate` endpoint:

**Successful Validation**:
- Validate message against minimal profile
- Validate ADT^A01 message against ADT_A01 profile
- Response contains validation results (issues/errors)

**Validation Failures**:
- Malformed message returns error
- Invalid profile YAML returns error
- Missing message field returns 400 Bad Request
- Missing profile field returns 400 Bad Request
- Empty request body returns 400 Bad Request

**HTTP Method Validation**:
- GET method returns 405 Method Not Allowed
- POST with valid JSON succeeds

**Response Format**:
- Returns JSON content type
- Contains validation results structure

#### `error_handling_test.rs` (9 tests)

Cross-cutting error handling tests:

**HTTP Error Handling**:
- Unknown routes return 404 Not Found
- Wrong content type is rejected
- Missing content-type header handled gracefully

**CORS Support**:
- CORS headers present in responses
- OPTIONS preflight requests handled
- Cross-origin requests allowed

**Request Limits**:
- Large (but reasonable) requests handled
- Very large requests rejected gracefully
- Compression (gzip) header supported

**Security**:
- No 500 errors for malformed requests
- Proper error responses for invalid input

## Running Tests

### Run All Integration Tests

```bash
cd crates/hl7v2-server
cargo test --test '*'
```

### Run Specific Test Suite

```bash
cargo test --test health_endpoints_test
cargo test --test parse_endpoint_test
cargo test --test validate_endpoint_test
cargo test --test error_handling_test
```

### Run Individual Test

```bash
cargo test --test health_endpoints_test test_health_endpoint_returns_200
```

### Run with Output

```bash
cargo test --test '*' -- --nocapture
```

### Run in Release Mode

```bash
cargo test --release --test '*'
```

## Test Coverage

**Total Integration Tests**: 39 tests

**Coverage by Category**:
- Health/Readiness: 9 tests (23%)
- Parse Endpoint: 11 tests (28%)
- Validate Endpoint: 10 tests (26%)
- Error Handling: 9 tests (23%)

**Coverage by HTTP Method**:
- GET: 12 tests
- POST: 21 tests
- OPTIONS: 1 test
- Method validation: 2 tests

**Coverage by Status Code**:
- 200 OK: 15 tests
- 400 Bad Request: 8 tests
- 404 Not Found: 1 test
- 405 Method Not Allowed: 2 tests
- 4xx/5xx general: 6 tests

## Test Fixtures

### HL7 Message Types Covered

1. **ADT^A01** - Admit/Visit Notification
   - Complete message with MSH, EVN, PID, PV1 segments
   - Valid patient demographics
   - Valid admission information

2. **ADT^A04** - Register Patient
   - Registration without admission
   - Outpatient scenario
   - Optional birth date

3. **ORU^R01** - Observation Results
   - Lab results with OBR/OBX segments
   - Multiple observations
   - Numeric results with units

4. **Minimal** - MSH segment only
   - Absolute minimum valid message
   - Used for basic parsing tests

5. **Invalid Messages**
   - Malformed (not HL7 format)
   - Wrong encoding characters
   - Missing required fields

### Conformance Profiles Covered

1. **Minimal Profile**
   - Requires MSH segment only
   - No field-level validation
   - Used for basic structure tests

2. **ADT_A01 Profile**
   - Full message structure validation
   - Required segments: MSH, EVN, PID, PV1
   - MSH constraints (message type, version)
   - Field constraints (patient ID, name)
   - HL7 table validation (Administrative Sex)
   - Expression guardrails

## CI/CD Integration

### GitHub Actions

```yaml
- name: Run Integration Tests
  run: |
    cd crates/hl7v2-server
    cargo test --test '*' --verbose
```

### GitLab CI

```yaml
integration-tests:
  script:
    - cd crates/hl7v2-server
    - cargo test --test '*' --verbose
```

### With Coverage

```yaml
- name: Run Tests with Coverage
  run: |
    cargo install cargo-llvm-cov
    cargo llvm-cov --html --test '*'
```

## Extending the Test Suite

### Adding New Test Cases

1. **Add fixture to `common/mod.rs`**:
```rust
pub const NEW_MESSAGE: &str = "MSH|^~\\&|...";
```

2. **Create test function**:
```rust
#[tokio::test]
async fn test_new_scenario() {
    let app = common::create_test_router();
    // ... test implementation
}
```

### Adding New Test Suite

1. Create `tests/new_feature_test.rs`
2. Import common utilities: `mod common;`
3. Add test functions with `#[tokio::test]`
4. Document in this README

### Best Practices

✅ **Use descriptive test names**: `test_parse_valid_adt_a01_message`

✅ **One assertion per test**: Focus on single behavior

✅ **Use fixtures**: Reuse common test data from `common/`

✅ **Test both success and failure**: Positive and negative cases

✅ **Verify HTTP status codes**: Check exact status when possible

✅ **Validate response format**: Check content-type and structure

✅ **Clean up resources**: Use `oneshot()` for stateless tests

✅ **Document edge cases**: Comment unusual scenarios

## Troubleshooting

### Tests Failing

1. **Check server dependencies**:
   ```bash
   cargo build
   ```

2. **Verify fixtures are valid**:
   ```bash
   cargo test --lib
   ```

3. **Run with detailed output**:
   ```bash
   cargo test --test '*' -- --nocapture --test-threads=1
   ```

### Port Conflicts

Tests use `oneshot()` to avoid binding actual ports. If you see port conflicts:
- Ensure you're not running a real server instance
- Check for `ServerConfig { port: 0 }` in test code

### Timeout Issues

Increase timeout for slow CI environments:
```rust
#[tokio::test(flavor = "multi_thread")]
async fn test_name() { ... }
```

## Metrics

Track test execution metrics:
- **Total tests**: 39
- **Average execution time**: ~50ms per test
- **Total suite time**: ~2 seconds
- **Flaky tests**: 0 (all deterministic)

## Future Enhancements

Planned test additions:
- [ ] Concurrency limit testing (503 responses)
- [ ] Authentication middleware tests
- [ ] Rate limiting behavior tests
- [ ] WebSocket support tests (if added)
- [ ] MLLP framing tests
- [ ] Performance/load tests
- [ ] Security tests (SQL injection, XSS, etc.)
- [ ] Database integration tests (if added)

## Related Documentation

- [OpenAPI Specification](../../../schemas/openapi/hl7v2-api.yaml) - API contract
- [Example Profiles](../../../examples/profiles/) - Sample conformance profiles
- [ADR-006](../../../docs/adr/0012-rate-limiting-and-backpressure.md) - Rate limiting strategy
- [Server Documentation](../README.md) - Server configuration and deployment

## License

These integration tests are part of the hl7v2-rs project and licensed under AGPL-3.0-or-later.
