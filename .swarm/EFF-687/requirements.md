# Requirements: Regex Caching in hl7v2-validation (EFF-687)

**Issue:** [EFF-687](/EFF/issues/EFF-687)  
**Status:** Requirements Defined  
**Last Updated:** 2026-04-04

---

## Problem Statement

The `hl7v2-validation` crate compiles regex patterns from scratch on every validation call that uses the `matches_regex` operator. Regex compilation is computationally expensive (hundreds of microseconds to milliseconds) and should be cached.

### Affected Locations

1. **Primary**: `crates/hl7v2-validation/src/lib.rs` line 831
2. **Secondary**: `crates/hl7v2-validation/src/lib.rs` line 382 (`matches_complex_pattern`)

---

## Functional Requirements

### FR-1: Regex Cache for matches_regex Operator
- **FR-1.1:** The system SHALL cache compiled `Regex` objects to avoid recompilation
- **FR-1.2:** The cache key SHALL be the pattern string
- **FR-1.3:** The cache SHALL be thread-safe for concurrent access
- **FR-1.4:** The system SHALL handle invalid regex patterns gracefully (return false, not panic)

### FR-2: Regex Cache for matches_complex_pattern Function
- **FR-2.1:** The `matches_complex_pattern` function SHALL cache compiled `Regex` objects
- **FR-2.2:** Each pattern in the patterns slice SHALL be cached independently
- **FR-2.3:** Cache behavior SHALL be consistent with FR-1 requirements

### FR-3: Cache Performance
- **FR-3.1:** Cached regex lookups SHALL be O(1) average case
- **FR-3.2:** First compilation of a pattern MAY be slower (cache miss)
- **FR-3.3:** Subsequent lookups of the same pattern SHALL be significantly faster than recompilation

### FR-4: Cache Lifecycle
- **FR-4.1:** The cache SHALL persist for the lifetime of the process
- **FR-4.2:** The cache SHALL NOT grow unbounded (memory protection)
- **FR-4.3:** Cache eviction policy MAY use LRU or fixed capacity limit

---

## Non-Functional Requirements

### NFR-1: Thread Safety
- **NFR-1.1:** The cache SHALL be safe for concurrent reads from multiple threads
- **NFR-1.2:** The cache SHALL handle concurrent writes safely
- **NFR-1.3:** Lock contention SHALL be minimized (prefer RwLock over Mutex if appropriate)

### NFR-2: Memory Efficiency
- **NFR-2.1:** The cache SHALL have a bounded maximum size
- **NFR-2.2:** Default cache capacity SHALL be 100 entries (configurable)
- **NFR-2.3:** Memory overhead per cached entry SHALL be minimal (pattern string + Regex object)

### NFR-3: Backward Compatibility
- **NFR-3.1:** The public API SHALL remain unchanged
- **NFR-3.2:** Existing validation rules SHALL work without modification
- **NFR-3.3:** Behavior SHALL be identical to non-cached version (aside from performance)

### NFR-4: Observability
- **NFR-4.1:** Cache hits and misses MAY be instrumented for debugging
- **NFR-4.2:** Cache statistics MAY be exposed via optional debug interface

---

## Verification Criteria

| Req ID | Test Method | Success Criteria |
|--------|-------------|------------------|
| FR-1.1 | Unit test | Same pattern used twice only compiles once |
| FR-1.2 | Unit test | Different patterns result in separate cache entries |
| FR-1.3 | Concurrency test | 100 threads accessing cache simultaneously without panic |
| FR-1.4 | Unit test | Invalid pattern returns false, doesn't panic |
| FR-2.1 | Unit test | matches_complex_pattern caches individual patterns |
| NFR-2.1 | Load test | Cache does not exceed configured capacity |
| NFR-3.1 | API test | Public function signatures unchanged |

---

## Out of Scope (Future Work)

The following are NOT requirements for this implementation:

1. **hl7v2-prof regex caching** - Only hl7v2-validation crate in scope
2. **RegexSet optimization** - May be considered for matches_complex_pattern later
3. **Distributed/shared cache** - Process-local cache only
4. **Cache persistence across restarts** - In-memory only
5. **Custom cache eviction policies** - LRU or fixed capacity is sufficient

---

## References

- Issue: [EFF-687](/EFF/issues/EFF-687)
- Code location: `crates/hl7v2-validation/src/lib.rs` lines 831, 382
- Related: [EFF-67](/EFF/issues/EFF-67) (similar caching issue in profile parsing)
