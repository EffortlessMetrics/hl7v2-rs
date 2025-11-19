# ADR-006: Rate Limiting and Backpressure Strategy

**Date**: 2025-11-19
**Status**: ACCEPTED
**Author**: Claude (AI Assistant)
**Deciders**: Project Team

## Context

The HL7v2 HTTP/REST API server needs protection against resource exhaustion from excessive concurrent requests. There are two related but distinct approaches to this problem:

1. **Rate Limiting**: Controls the *rate* of requests (e.g., 100 requests/second)
2. **Backpressure/Concurrency Limiting**: Controls the *number* of concurrent requests being processed

### Requirements

- Protect server resources from exhaustion
- Prevent cascading failures in distributed systems
- Provide predictable performance under load
- Simple to configure and reason about
- Minimal complexity and dependencies
- Production-ready for healthcare workloads

### Options Considered

#### Option 1: Rate Limiting with tower_governor

**Implementation**: `tower_governor` crate with token bucket algorithm

**Pros**:
- True rate limiting (requests/second)
- Familiar to many developers
- Per-client rate limiting support
- Token bucket algorithm is well-understood

**Cons**:
- Complex generic type system difficult to integrate with Axum
- Requires additional state management for distributed scenarios
- Rate limits (requests/second) don't directly correlate with resource usage
- Additional dependency with less mature ecosystem
- Configuration complexity (burst size, replenish rate, etc.)

**Investigation Result**: During implementation, encountered multiple generic type errors that made integration with Axum's middleware stack overly complex:

```rust
error[E0107]: struct takes 2 generic arguments but 1 generic argument was supplied
error[E0308]: mismatched types - expected `Arc<GovernorConfig>`, found `&mut GovernorConfig`
```

#### Option 2: Concurrency Limiting with tower::limit::ConcurrencyLimitLayer

**Implementation**: Tower's built-in `ConcurrencyLimitLayer`

**Pros**:
- Simple, single-purpose API
- Part of Tower's mature middleware ecosystem
- Directly controls resource usage (active requests)
- Clean integration with Axum
- No additional dependencies (already using Tower)
- Easy to reason about: "max N concurrent requests"
- Provides backpressure via 503 responses

**Cons**:
- Not true rate limiting (doesn't limit requests/second)
- Global limit rather than per-client
- May need additional rate limiting for public APIs

#### Option 3: Hybrid Approach

**Implementation**: Both `tower_governor` for rate limiting AND `ConcurrencyLimitLayer` for backpressure

**Pros**:
- Comprehensive protection
- Rate limiting for bursty traffic
- Concurrency limiting for resource protection

**Cons**:
- Increased complexity
- Two different mental models to maintain
- Potential for confusion about which limit is being hit
- Overhead of two middleware layers

## Decision

**We will use `tower::limit::ConcurrencyLimitLayer` for backpressure** as the primary request limiting mechanism.

### Rationale

1. **Simplicity**: Concurrency limiting directly maps to resource usage. If the server can handle 100 concurrent requests, set the limit to 100. No need to calculate requests/second based on average processing time.

2. **Resource Protection**: What matters for preventing resource exhaustion is *concurrent* load, not *rate*. A burst of 1000 requests/second that each take 1ms is less problematic than 10 requests/second that each take 30 seconds.

3. **Clean Integration**: Tower's `ConcurrencyLimitLayer` integrates seamlessly with Axum's middleware stack with zero type system complexity.

4. **Backpressure Semantics**: Returns HTTP 503 (Service Unavailable) when at capacity, signaling to clients to retry later. This is the correct semantic for "server is at capacity."

5. **Healthcare Context**: HL7v2 message processing workloads are typically:
   - Internal/trusted networks (not public internet)
   - Known integration partners
   - Predictable load patterns
   - More concerned with reliability than preventing abuse

6. **Future Extensibility**: If per-client rate limiting becomes necessary (e.g., multi-tenant SaaS), we can add it as a separate layer without removing concurrency limiting.

### Implementation

```rust
use tower::limit::ConcurrencyLimitLayer;

pub fn create_concurrency_limit_layer() -> ConcurrencyLimitLayer {
    ConcurrencyLimitLayer::new(100)  // Max 100 concurrent requests
}

// In router configuration:
Router::new()
    .route("/health", get(health_handler))
    .route("/hl7/parse", post(parse_handler))
    .with_state(state)
    .layer(create_concurrency_limit_layer())  // Outermost layer
```

### Configuration Guidelines

**Determining the Concurrency Limit**:

1. **Start with Benchmarks**: Run load tests to determine max sustainable concurrent requests
   - Monitor CPU usage, memory, response times
   - Find the point where latency degrades significantly

2. **Add Headroom**: Set limit 20-30% below max capacity
   - Example: If system handles 150 concurrent requests before degradation, set limit to 100-120

3. **Consider Downstream Dependencies**:
   - Database connection pool size
   - External API rate limits
   - Memory constraints for message parsing

4. **Monitor and Adjust**:
   - Track 503 response rate via Prometheus metrics
   - If frequent 503s, either increase limit or scale horizontally
   - If never hitting limit, may be over-provisioned

**Environment-Specific Limits**:

```rust
let max_concurrent = std::env::var("HL7V2_MAX_CONCURRENT")
    .ok()
    .and_then(|v| v.parse::<usize>().ok())
    .unwrap_or(100);  // Default 100

ConcurrencyLimitLayer::new(max_concurrent)
```

## Consequences

### Positive

✅ **Simpler Mental Model**: "Allow 100 concurrent requests" is easier to understand than "allow 1000 requests/second with burst of 50"

✅ **Direct Resource Mapping**: Concurrency limit directly correlates with memory/CPU usage

✅ **Correct Semantics**: HTTP 503 properly signals capacity issues

✅ **No Additional Dependencies**: Uses existing Tower infrastructure

✅ **Easy Testing**: Can easily test capacity limits with concurrent requests

✅ **Prometheus Integration**: Metrics already track concurrent requests and 503 responses

### Negative

⚠️ **Not True Rate Limiting**: Doesn't prevent 10,000 requests/second if each completes quickly

⚠️ **Global Limit**: Not per-client (could be addressed with future enhancement)

⚠️ **No Burst Handling**: Doesn't have sophisticated token bucket burst semantics

### Mitigations

**If per-client rate limiting needed**:
- Add `tower-governor` or similar as separate layer for specific routes
- Use API gateway (nginx, Envoy) for rate limiting
- Implement custom middleware for simple per-key limiting

**If true rate limiting needed**:
- For public APIs, use API gateway (recommended)
- For simple cases, implement sliding window counter
- Re-evaluate `tower-governor` when type system issues resolved

**Monitoring**:
```rust
metrics::counter!("hl7v2_requests_rejected_concurrency_limit").increment(1);
```

## Alternatives Not Chosen

### Token Bucket Rate Limiter (tower_governor)

**Why Not**: Complex type system, doesn't directly map to resource usage, rate limiting is less important than concurrency limiting for internal healthcare APIs

### Leaky Bucket Algorithm

**Why Not**: Similar complexity to token bucket, same fundamental issue of rate vs. concurrency

### No Limiting

**Why Not**: Unacceptable risk of resource exhaustion and cascading failures

## Related Decisions

- **ADR-003**: Prometheus metrics tracks `hl7v2_requests_total` with status codes, enables monitoring of 503s
- **ADR-005**: Cross-field rule semantics - similar philosophy of choosing simpler, more direct solution

## References

- [Tower Limit Documentation](https://docs.rs/tower/latest/tower/limit/index.html)
- [HTTP 503 Service Unavailable](https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/503)
- [Backpressure in Distributed Systems](https://mechanical-sympathy.blogspot.com/2012/05/apply-back-pressure-when-overloaded.html)
- [Rate Limiting vs Load Shedding](https://aws.amazon.com/builders-library/using-load-shedding-to-avoid-overload/)

## Revision History

- **2025-11-19**: Initial version - ACCEPTED
  - Chose `tower::limit::ConcurrencyLimitLayer`
  - Documented rationale and implementation
  - Provided configuration guidelines
  - Default limit: 100 concurrent requests
  - Environment variable: `HL7V2_MAX_CONCURRENT`
