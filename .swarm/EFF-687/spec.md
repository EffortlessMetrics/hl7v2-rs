# Spec: Regex Caching in hl7v2-validation (EFF-687)

**Issue:** [EFF-687](/EFF/issues/EFF-687)  
**Status:** Spec Complete  
**Branch:** EFF-687-regex-caching  
**Project:** hl7v2-rs

---

## Problem Statement

The `hl7v2-validation` crate compiles regex patterns from scratch on every validation call that uses the `matches_regex` operator. This is a severe performance bottleneck since regex compilation is computationally expensive.

### Current State (Lines 831, 382)

```rust
// Line 831 - compiles regex on every call
"matches_regex" => {
    if let (Some(l), Some(pat)) = (lhs, rhs_first) {
        // compile per-call for simplicity; optimize later with a cache if needed
        Regex::new(pat).map(|re| re.is_match(l)).unwrap_or(false)
    } else {
        false
    }
}
```

### Impact

- **Regex compilation is expensive**: Hundreds of microseconds to milliseconds
- **Called on every validation**: O(n) regex compilations for n field validations
- **Batch validation amplification**: 10,000 messages = 10,000 regex compilations

---

## Solution Overview

Implement a thread-safe, LRU-bounded regex cache using:
- `std::sync::OnceLock` for lazy initialization
- `std::sync::RwLock` for read-heavy concurrent access
- `lru::LruCache` for bounded storage with LRU eviction

---

## Requirements Summary

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-1 | Cache compiled Regex objects | High |
| FR-2 | Thread-safe concurrent access | High |
| FR-3 | Bounded cache (default 100 patterns) | Medium |
| FR-4 | Handle invalid patterns gracefully | High |
| NFR-1 | Backward compatible API | High |
| NFR-2 | No additional crate dependencies | Medium |

---

## Design Summary

### Cache Architecture

```rust
static REGEX_CACHE: OnceLock<RwLock<LruCache<String, Regex>>> = OnceLock::new();

fn get_or_compile_regex(pattern: &str) -> Option<Regex> {
    // 1. Check cache (read lock)
    // 2. If miss: compile regex (no lock)
    // 3. Store in cache (write lock)
    // 4. Return compiled regex
}
```

### Modified Code Locations

1. **Line 831**: Replace `Regex::new(pat)` with `get_or_compile_regex(pat)`
2. **Line 382**: Replace `Regex::new(pattern)` with `get_or_compile_regex(pattern)`

---

## BDD Scenarios

See [scenarios.md](./scenarios.md) for detailed test scenarios.

### Quick Summary

1. **Cache hit**: Second use of same pattern returns cached regex
2. **Cache miss**: First use of pattern compiles and stores
3. **Thread safety**: Concurrent access from 100 threads, no panic
4. **Invalid pattern**: Returns false consistently
5. **Cache eviction**: LRU removes oldest when capacity exceeded

---

## File Changes

| File | Change | Purpose |
|------|--------|---------|
| `crates/hl7v2-validation/src/lib.rs` | Add cache module | Regex cache implementation |
| `crates/hl7v2-validation/src/lib.rs` | Modify line 831 | Use cached regex |
| `crates/hl7v2-validation/src/lib.rs` | Modify lines 382-387 | Use cached regex |
| `crates/hl7v2-validation/Cargo.toml` | Add lru dep | LRU cache implementation |

---

## Verification Criteria

- [ ] Regex cache module exists and compiles
- [ ] `matches_regex` operator uses cache
- [ ] `matches_complex_pattern` function uses cache
- [ ] Thread safety: concurrent access works
- [ ] Cache bounded: doesn't grow beyond capacity
- [ ] Invalid patterns: handled gracefully
- [ ] Unit tests pass for cache operations
- [ ] Performance: 10x+ speedup for repeated patterns

---

## Out of Scope

- hl7v2-prof regex caching (separate issue)
- RegexSet optimization (future consideration)
- Cache statistics/metrics (future enhancement)
- Distributed caching (not needed)

---

## Next Owner

**Spec Verifier** - Verify spec completeness and testability.

---

## Related Issues

- [EFF-67](/EFF/issues/EFF-67) - Similar caching issue in profile parsing
- [EFF-688](/EFF/issues/EFF-688) - Batch parsing allocates String for every message
- [EFF-694](/EFF/issues/EFF-694) - Streaming parser allocates 1KB buffer on every read
