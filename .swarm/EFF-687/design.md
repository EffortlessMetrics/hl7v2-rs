# Design: Regex Caching in hl7v2-validation (EFF-687)

**Issue:** [EFF-687](/EFF/issues/EFF-687)  
**Status:** Design Complete  
**Last Updated:** 2026-04-04

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    Validation Request                            │
│                    (matches_regex call)                          │
└─────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│              RegexCache (thread-safe singleton)                    │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  HashMap<String, Regex> + RwLock                         │   │
│  │                                                          │   │
│  │  Key: pattern string                                     │   │
│  │  Value: compiled Regex                                   │   │
│  │                                                          │   │
│  │  Capacity: 100 (configurable)                            │   │
│  │  Eviction: LRU (Least Recently Used)                     │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                            │
              ┌─────────────┴─────────────┐
              ▼                           ▼
        ┌──────────┐               ┌──────────┐
        │ Cache Hit │               │ Cache Miss│
        │ O(1)     │               │ Compile   │
        │ Return   │               │ Store     │
        │ Regex    │               │ Return    │
        └──────────┘               └──────────┘
```

---

## Key Design Decisions

### Decision 1: Cache Implementation Strategy

**Choice:** Use `std::sync::RwLock<HashMap<String, Regex>>` with LRU eviction

**Rationale:**
- Simple and well-understood pattern
- No additional crate dependencies (using `std` only)
- `RwLock` allows concurrent reads, exclusive writes
- LRU prevents unbounded memory growth

**Trade-offs:**
- HashMap lookup is O(1) but not cache-line optimized
- RwLock may have some contention under extreme load
- Simpler than dashmap or other concurrent hash maps

### Decision 2: Singleton Pattern

**Choice:** Module-level static cache initialized with `std::sync::OnceLock`

**Rationale:**
- Cache should be global across all validation calls
- `OnceLock` (Rust 1.70+) is the modern standard for lazy static initialization
- No need for explicit initialization - first use triggers creation

**Code Pattern:**
```rust
use std::sync::{RwLock, OnceLock};
use std::collections::HashMap;

static REGEX_CACHE: OnceLock<RwLock<HashMap<String, Regex>>> = OnceLock::new();

fn get_regex_cache() -> &'static RwLock<HashMap<String, Regex>> {
    REGEX_CACHE.get_or_init(|| RwLock::new(HashMap::with_capacity(100)))
}
```

### Decision 3: LRU Eviction Implementation

**Choice:** Use `lru` crate already present in workspace (via hl7v2-prof)

**Rationale:**
- `lru` crate already used in `hl7v2-prof/src/loader.rs`
- Provides O(1) get/put with automatic LRU eviction
- Bounded capacity prevents memory leaks

**Alternative Considered:**
- Manual LRU with HashMap + VecDeque: More code, error-prone
- Fixed-size ring buffer: Would need collision handling
- Unbounded HashMap: Memory leak risk

### Decision 4: Error Handling for Invalid Patterns

**Choice:** Cache the error result as `None`, return false on use

**Rationale:**
- Avoid re-compiling invalid patterns repeatedly
- Invalid pattern should consistently return false
- Don't panic on user-provided pattern

**Code Pattern:**
```rust
enum RegexCacheEntry {
    Valid(Regex),
    Invalid,  // Pattern failed to compile
}

fn get_or_compile_regex(pattern: &str) -> Option<&Regex> {
    // Check cache first
    // If miss: try compile, store result (Valid or Invalid)
    // Return Some(regex) for Valid, None for Invalid
}
```

### Decision 5: Cache Capacity

**Choice:** Default capacity of 100 patterns

**Rationale:**
- 100 patterns covers typical validation profile use cases
- Each pattern is small (string + compiled regex)
- Configurable via environment variable for advanced users

**Memory Estimate:**
- Pattern string: ~50 bytes average
- Compiled Regex: ~200-500 bytes (depends on complexity)
- Per entry: ~300-600 bytes
- 100 entries: ~30-60KB total

---

## Implementation Design

### File Changes

| File | Change | Lines |
|------|--------|-------|
| `crates/hl7v2-validation/src/lib.rs` | Add cache module | ~50 new |
| `crates/hl7v2-validation/src/lib.rs` | Modify matches_regex | ~5 changed |
| `crates/hl7v2-validation/src/lib.rs` | Modify matches_complex_pattern | ~10 changed |

### New Module: regex_cache

```rust
// At top of lib.rs or in new module
use std::sync::{RwLock, OnceLock};
use regex::Regex;
use lru::LruCache;

const DEFAULT_CACHE_SIZE: usize = 100;

/// Global regex cache for validation operations
static REGEX_CACHE: OnceLock<RwLock<LruCache<String, Regex>>> = OnceLock::new();

fn get_regex_cache() -> &'static RwLock<LruCache<String, Regex>> {
    REGEX_CACHE.get_or_init(|| {
        let cache_size = std::env::var("HL7V2_REGEX_CACHE_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_CACHE_SIZE);
        RwLock::new(LruCache::new(cache_size))
    })
}

/// Get or compile a regex pattern, using the cache
pub fn get_or_compile_regex(pattern: &str) -> Option<Regex> {
    let cache = get_regex_cache();
    
    // Try read lock first for lookup
    {
        let cache_read = cache.read().ok()?;
        if let Some(regex) = cache_read.get(pattern) {
            return Some(regex.clone());
        }
    }
    
    // Not in cache - compile and store
    let regex = Regex::new(pattern).ok()?;
    
    if let Ok(mut cache_write) = cache.write() {
        cache_write.put(pattern.to_string(), regex.clone());
    }
    
    Some(regex)
}
```

### Modified matches_regex (line ~831)

```rust
"matches_regex" => {
    if let (Some(l), Some(pat)) = (lhs, rhs_first) {
        // Use cached regex for performance
        get_or_compile_regex(pat)
            .map(|re| re.is_match(l))
            .unwrap_or(false)
    } else {
        false
    }
}
```

### Modified matches_complex_pattern (line ~382)

```rust
pub fn matches_complex_pattern(value: &str, patterns: &[&str]) -> bool {
    // All patterns must match
    patterns.iter().all(|pattern| {
        get_or_compile_regex(pattern)
            .map(|re| re.is_match(value))
            .unwrap_or(false)
    })
}
```

---

## Thread Safety Analysis

### Read-Heavy Workload (typical)

```
Thread 1: Read lock ──► lookup ──► unlock ──► use regex
Thread 2: Read lock ──► lookup ──► unlock ──► use regex  [concurrent reads OK]
```

### Write Scenario (cache miss)

```
Thread 1: Read lock ──► miss ──► unlock ──► compile ──► Write lock ──► store
Thread 2: Read lock ──► wait ──► (Thread 1 writes) ──► lookup ──► hit
```

### Risk Mitigation

1. **Compile outside write lock**: Regex compilation happens BEFORE acquiring write lock
2. **Minimal critical section**: Write lock only for HashMap insert
3. **Graceful degradation**: If lock fails, still compile and use (just don't cache)

---

## Testing Strategy

### Unit Tests

1. **Cache hit test**: Same pattern twice should use cache
2. **Cache miss test**: Different patterns should both compile
3. **Thread safety test**: 100 threads, same pattern, no panic
4. **Invalid pattern test**: Invalid regex should return false consistently
5. **Capacity test**: Fill cache to 100, verify LRU eviction

### Integration Tests

1. **Validation rule test**: matches_regex operator uses cache
2. **Batch validation test**: 10,000 validations with same pattern

### Performance Tests

1. **Before/after benchmark**: Measure regex validation throughput
2. **Cache hit ratio**: Verify >90% hit rate in typical workloads

---

## Migration Path

### Phase 1: Add caching to hl7v2-validation (this issue)
- Add regex_cache module
- Modify matches_regex and matches_complex_pattern
- Add unit tests

### Phase 2: Consider hl7v2-prof caching (future)
- Evaluate if hl7v2-prof regex compilation is hot path
- Apply same pattern if needed

### Phase 3: Advanced optimizations (future)
- Consider RegexSet for matches_complex_pattern with many patterns
- Add metrics/monitoring for cache performance

---

## Known Limitations

1. **Process-local only**: Cache is not shared between processes
2. **No persistence**: Cache is lost on process restart
3. **Fixed capacity**: 100 patterns default (may need tuning)
4. **No cache warming**: Cold start has no cached patterns

---

## References

- [EFF-687](/EFF/issues/EFF-687) - This issue
- [EFF-67](/EFF/issues/EFF-67) - Similar caching issue in profile parsing
- `lru` crate docs: https://docs.rs/lru/
- Rust OnceLock docs: https://doc.rust-lang.org/std/sync/struct.OnceLock.html
