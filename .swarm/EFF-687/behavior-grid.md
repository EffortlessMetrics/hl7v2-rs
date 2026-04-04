# Behavior Grid: Regex Caching in hl7v2-validation (EFF-687)

**Purpose:** Reference matrix for implementers and testers. Defines expected behavior for all caching scenarios.

---

## Cache Behavior Matrix

| Scenario | Input | Cache State Before | Cache State After | Expected Result | Performance |
|----------|-------|-------------------|-------------------|-----------------|-------------|
| S1: Cache hit | Pattern "[A-Z]+" | Contains "[A-Z]+" | Unchanged | Match using cached regex | Fast (O(1) lookup) |
| S2: Cache miss | Pattern "[0-9]+" | Empty | Contains "[0-9]+" | Match, store in cache | Slow (compile + O(1) store) |
| S3: Thread safe | 100 threads, same pattern | Empty | Contains pattern | All succeed, 1 compile | Scalable |
| S4: Invalid pattern | Pattern "[invalid(" | Empty | Contains key with None | Returns false | Fast (cached error) |
| S5: LRU eviction | 101st pattern, capacity 100 | Full (100 entries) | Full (newest 100) | Oldest evicted | O(1) eviction |
| S6: Complex pattern | patterns ["[A-Z]+", "[0-9]+"] | Empty | Contains both | All patterns cached | Per-pattern compile |
| S7: Independent storage | Pattern A, then Pattern B | Contains A | Contains A+B | Separate entries | Per-pattern cost |
| S8: Configurable | env HL7V2_REGEX_CACHE_SIZE=50 | N/A | Capacity 50 | Accepts 50 patterns | Same behavior |
| S9: Backward compat | Existing validation rule | N/A | N/A | Same result | Improved speed |
| S10: Performance | 10,000 calls, same pattern | Contains pattern | Unchanged | All fast | 10x+ speedup |

---

## Regex Operation Matrix

| Operation | Line | Before | After | Cache Interaction |
|-----------|------|--------|-------|-------------------|
| matches_regex operator | 831 | `Regex::new(pat)` | `get_or_compile_regex(pat)` | Read/write cache |
| matches_complex_pattern | 382 | `Regex::new(pattern)` | `get_or_compile_regex(pattern)` | Read/write cache per pattern |

---

## Thread Safety Matrix

| Scenario | Threads | Read Operations | Write Operations | Expected Behavior |
|----------|---------|-----------------|------------------|-------------------|
| Concurrent reads | 100 | 100 simultaneous | 0 | All succeed, RwLock allows concurrent reads |
| Concurrent writes (different patterns) | 10 | 0 | 10 simultaneous | Writers serialized, all succeed |
| Read + Write race | 2 | 1 | 1 | Reader or writer goes first, no panic |
| Cache miss storm | 100 | 0 | 100 (same pattern) | 1-100 compilations, at least 1 cached |

---

## Error Handling Matrix

| Input | Regex::new Result | Cache Action | Return Value |
|-------|-------------------|--------------|--------------|
| Valid pattern "[A-Z]+" | Ok(regex) | Store regex in cache | Some(regex) |
| Invalid pattern "[broken(" | Err(e) | Store None in cache | None |
| Empty pattern "" | Ok(regex) or Err | Store result | Some or None |
| Unicode pattern "\p{L}+" | Ok(regex) | Store regex in cache | Some(regex) |

---

## Input/Output Examples

### Example 1: Cache Hit

**Input:**
```rust
// First call - cache miss
matches_regex("ABC", "^[A-Z]+$");

// Second call - cache hit
matches_regex("XYZ", "^[A-Z]+$");
```

**Expected Execution:**
```
Call 1: Cache miss
  - Read lock: miss
  - Compile: Regex::new("^[A-Z]+$") -> Ok(regex)
  - Write lock: store ("^[A-Z]+$", Some(regex))
  - Use: regex.is_match("ABC") -> true
  - Time: ~500µs

Call 2: Cache hit
  - Read lock: hit
  - Use: cached_regex.is_match("XYZ") -> true
  - Time: ~1µs
```

### Example 2: Invalid Pattern

**Input:**
```rust
matches_regex("test", "[invalid(");
matches_regex("test", "[invalid(");  // Second call
```

**Expected Execution:**
```
Call 1: Invalid pattern
  - Read lock: miss
  - Compile: Regex::new("[invalid(") -> Err
  - Write lock: store ("[invalid(", None)
  - Return: false

Call 2: Cached invalid
  - Read lock: hit (cached None)
  - Return: false (no recompilation)
```

### Example 3: LRU Eviction

**Input:**
```rust
// Fill cache to capacity (100)
for i in 0..100 {
    matches_regex("test", &format!("pattern{}", i));
}

// Add 101st pattern (evicts pattern0)
matches_regex("test", "pattern100");

// Use evicted pattern (recompile)
matches_regex("test", "pattern0");
```

**Expected Execution:**
```
After 100 patterns: Cache full with pattern0..pattern99
Call 101: pattern100 added
  - LRU evicts pattern0
  - Cache now contains pattern1..pattern100
Call 102: pattern0 (evicted)
  - Cache miss
  - Recompile pattern0
  - Cache evicts pattern1
```

---

## Edge Case Handling

| Edge Case | Detection | Behavior |
|-----------|-----------|----------|
| Empty pattern | `pattern.is_empty()` | Cache key "", compile result cached |
| Long pattern | `pattern.len() > 1000` | Stored normally (memory bounded by cache limit) |
| Special chars | Pattern contains `\n`, `\t` | Handled as literal string in cache key |
| Unicode | Pattern contains `\u{1F600}` | Preserved in cache key |
| Lock poisoned | RwLock poisoned from panic | Return compiled regex directly (degraded mode) |

---

## Test File Mappings

| Behavior Grid Scenario | Test Function | File |
|------------------------|--------------|------|
| S1, S2 | `test_cache_hit_miss` | `regex_cache_tests.rs` |
| S3 | `test_concurrent_access` | `regex_cache_tests.rs` |
| S4 | `test_invalid_pattern_handling` | `regex_cache_tests.rs` |
| S5 | `test_lru_eviction` | `regex_cache_tests.rs` |
| S6 | `test_complex_pattern_caching` | `regex_cache_tests.rs` |
| S7 | `test_independent_pattern_storage` | `regex_cache_tests.rs` |
| S8 | `test_configurable_capacity` | `regex_cache_tests.rs` |
| S9 | `test_backward_compatibility` | `validation_tests.rs` |
| S10 | `test_performance_improvement` | `benchmark_tests.rs` |

---

## Cache Implementation Details

```rust
// Static singleton
static REGEX_CACHE: OnceLock<RwLock<LruCache<String, Regex>>> = OnceLock::new();

// Lazy initialization
fn get_cache() -> &'static RwLock<LruCache<String, Regex>> {
    REGEX_CACHE.get_or_init(|| {
        let capacity = env::var("HL7V2_REGEX_CACHE_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(100);
        RwLock::new(LruCache::new(capacity))
    })
}

// Get or compile
pub fn get_or_compile(pattern: &str) -> Option<Regex> {
    let cache = get_cache();
    
    // Fast path: read lock
    if let Ok(read_guard) = cache.read() {
        if let Some(regex) = read_guard.peek(pattern) {
            return Some(regex.clone());
        }
    }
    
    // Slow path: compile
    let regex = Regex::new(pattern).ok()?;
    
    // Store: write lock
    if let Ok(mut write_guard) = cache.write() {
        write_guard.put(pattern.to_string(), regex.clone());
    }
    
    Some(regex)
}
```

---

## Decision Log for Testers

| Decision | Value | Rationale |
|----------|-------|-----------|
| Cache capacity | 100 default | Covers typical use cases, minimal memory |
| Eviction policy | LRU | Standard, predictable |
| Lock type | RwLock | Read-heavy workload optimized |
| Invalid pattern handling | Cache as None | Prevents repeated compilation errors |
| Return type | Option<Regex> | Explicit error handling |
| Clone behavior | Clone on retrieval | Regex uses Arc, cheap clone |

---

## Known Gaps (Future Work)

| Gap | Impact | Planned Resolution |
|-----|--------|-------------------|
| Cache statistics | Low | Add optional metrics export |
| Cache warming | Low | Pre-populate with common patterns |
| Custom eviction | Low | Pluggable eviction policy |
| Distributed cache | Very Low | Not needed for this use case |

---

**Owner:** Spec Verifier  
**Status:** Ready for verification
