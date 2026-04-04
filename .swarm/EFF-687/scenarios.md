# BDD Scenarios: Regex Caching in hl7v2-validation (EFF-687)

**Issue:** [EFF-687](/EFF/issues/EFF-687)  
**Purpose:** Verification scenarios for regex caching implementation

---

## Scenario 1: Cache Hit - Second Use Returns Cached Regex

```gherkin
Given a regex pattern "^[A-Z]{3}$"
When matches_regex is called with the pattern for the first time
Then the regex should be compiled and stored in cache
When matches_regex is called with the same pattern a second time
Then the cached regex should be returned (no recompilation)
And the operation should be significantly faster
```

## Scenario 2: Cache Miss - First Use Compiles and Stores

```gherkin
Given a regex pattern "^[0-9]+$"
And the cache is empty
When matches_regex is called with the pattern
Then the regex should be compiled
And the compiled regex should be stored in cache
And the match operation should execute correctly
```

## Scenario 3: Thread Safety - Concurrent Access

```gherkin
Given 100 threads running concurrently
And all threads use the same regex pattern "test[0-9]+"
When each thread calls matches_regex 100 times
Then no panics should occur
And all operations should complete successfully
And the regex should be compiled only once
```

## Scenario 4: Invalid Pattern - Graceful Handling

```gherkin
Given an invalid regex pattern "[invalid("
When matches_regex is called with the invalid pattern
Then the operation should return false
And no panic should occur
When the same invalid pattern is used again
Then it should return false immediately (cached error)
```

## Scenario 5: Cache Eviction - LRU Behavior

```gherkin
Given a cache with capacity of 100 patterns
And the cache is filled with 100 different patterns
When a 101st unique pattern is added
Then the least recently used pattern should be evicted
And the new pattern should be stored
```

## Scenario 6: matches_complex_pattern Uses Cache

```gherkin
Given a value "ABC123" to validate
And patterns ["[A-Z]+", "[0-9]+"]
When matches_complex_pattern is called
Then both patterns should be compiled and cached
When matches_complex_pattern is called again with same patterns
Then both patterns should be retrieved from cache
```

## Scenario 7: Multiple Patterns Independent

```gherkin
Given patterns "^[A-Z]+$" and "^[0-9]+$"
When matches_regex is called with first pattern
Then only first pattern should be in cache
When matches_regex is called with second pattern
Then both patterns should be in cache (independent storage)
```

## Scenario 8: Cache Capacity Configurable

```gherkin
Given environment variable HL7V2_REGEX_CACHE_SIZE=50
When the application starts
Then the cache should be created with capacity 50
And should accept up to 50 patterns before eviction
```

## Scenario 9: Backward Compatibility

```gherkin
Given existing validation rules using matches_regex
When the caching implementation is deployed
Then all existing rules should work without modification
And validation results should be identical
```

## Scenario 10: Performance Improvement

```gherkin
Given a batch of 10000 validation calls
And all calls use the same regex pattern
When validated without caching (baseline)
And validated with caching (implementation)
Then the cached version should be at least 10x faster
```

## Scenario 11: Regex Clone Safety

```gherkin
Given a cached regex object
When the regex is cloned from cache
Then the clone should be independent and safe to use
And the original cached regex should remain valid
```

## Scenario 12: Pattern String as Cache Key

```gherkin
Given patterns "test" and "TEST" (different cases)
When both patterns are used
Then they should be stored as separate cache entries
Because pattern string is case-sensitive cache key
```

---

## Test Mapping

| Scenario | Test Function | File |
|----------|--------------|------|
| S1, S2 | `test_regex_cache_hit_miss` | `regex_cache_tests.rs` |
| S3 | `test_regex_cache_thread_safety` | `regex_cache_tests.rs` |
| S4 | `test_regex_cache_invalid_pattern` | `regex_cache_tests.rs` |
| S5 | `test_regex_cache_lru_eviction` | `regex_cache_tests.rs` |
| S6 | `test_matches_complex_pattern_caching` | `regex_cache_tests.rs` |
| S7 | `test_regex_cache_independent_patterns` | `regex_cache_tests.rs` |
| S8 | `test_regex_cache_configurable_capacity` | `regex_cache_tests.rs` |
| S9 | `test_backward_compatibility` | `integration_tests.rs` |
| S10 | `test_performance_improvement` | `benchmark_tests.rs` |
| S11 | `test_regex_clone_safety` | `regex_cache_tests.rs` |
| S12 | `test_pattern_case_sensitivity` | `regex_cache_tests.rs` |

---

## Edge Cases

| Edge Case | Expected Behavior |
|-----------|-------------------|
| Empty pattern string | Cache key is "", Regex::new("") returns error, cached as Invalid |
| Very long pattern (1KB+) | Stored normally, memory overhead acceptable |
| Unicode patterns | Handled correctly, cache key preserves Unicode |
| Pattern with special characters | Cache key is exact string, no escaping needed |
| Concurrent cache miss (same pattern) | May compile twice, both stored (acceptable race) |
| Cache write lock contention | Readers not blocked, only writers block each other |

---

**Owner:** Spec Verifier  
**Status:** Ready for verification
