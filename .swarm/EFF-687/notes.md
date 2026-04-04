# Working Notes: EFF-687

**Status:** Spec complete  
**Date:** 2026-04-04  
**Location:** `.swarm/EFF-687/`

---

## Discovery

Upon examination of the codebase:

1. **Issue is acknowledged in code**: Line 830 comment explicitly states "compile per-call for simplicity; optimize later with a cache if needed"
2. **Two affected locations identified**:
   - `crates/hl7v2-validation/src/lib.rs` line 831 (`matches_regex` operator)
   - `crates/hl7v2-validation/src/lib.rs` line 382 (`matches_complex_pattern` function)
3. **hl7v2-prof has similar issues**: 15+ locations (out of scope for this issue)
4. **`lru` crate already available**: Used in `hl7v2-prof/src/loader.rs`

---

## Key Design Decisions

### Decision 1: Use std::sync::OnceLock (not lazy_static)

**Rationale:**
- `OnceLock` is in std (Rust 1.70+), no external dependency needed
- Standard pattern for Rust 1.70+ applications
- `lazy_static` would require another dependency

### Decision 2: Use lru::LruCache (not HashMap)

**Rationale:**
- `lru` crate already in workspace (via hl7v2-prof)
- Built-in capacity limit and eviction
- O(1) get/put operations
- Prevents unbounded memory growth

### Decision 3: Cache stores Regex (not &Regex)

**Rationale:**
- `Regex` implements `Clone`
- Cloning regex is cheap (Arc internally)
- Safer than lifetime management with cache
- Allows returning owned Regex to callers

### Decision 4: Invalid patterns cached as None

**Rationale:**
- Avoid re-compiling invalid patterns repeatedly
- Invalid pattern is deterministic - won't suddenly become valid
- Consistent behavior: always return false for invalid

---

## Options Considered

### Option 1: std-only with RwLock<HashMap> (Selected)

Use standard library only with HashMap and manual size limit.

**Pros:**
- No additional dependencies
- Simple implementation

**Cons:**
- Manual eviction logic needed
- More code to write and maintain

**Verdict:** Rejected - prefer lru crate for cleaner code

### Option 2: lru::LruCache with RwLock (Selected)

Use existing lru crate for bounded cache.

**Pros:**
- Clean, proven implementation
- Automatic LRU eviction
- Already in workspace

**Cons:**
- One more explicit dependency for hl7v2-validation

**Verdict:** Selected - best balance of simplicity and functionality

### Option 3: dashmap (Concurrent HashMap)

Use dashmap for lock-free concurrent access.

**Pros:**
- Better concurrent performance
- No RwLock contention

**Cons:**
- New dependency to add
- Overkill for expected workload (read-heavy)

**Verdict:** Deferred - can be optimization later if needed

### Option 4: Thread-local cache

Use thread_local! with per-thread HashMap.

**Pros:**
- No locking needed
- Perfect thread isolation

**Cons:**
- Memory overhead per thread
- Duplicate compilations across threads
- Cache not shared

**Verdict:** Rejected - prefer shared cache for better hit rate

---

## Risks and Unknowns

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| RwLock contention under high load | Low | Medium | Monitor metrics; can switch to dashmap later |
| Memory overhead from large patterns | Low | Low | Bounded cache prevents unbounded growth |
| Invalid pattern DoS (fill cache) | Low | Low | Cache invalid patterns as None (minimal size) |
| Clone overhead of Regex | Low | Low | Regex uses Arc internally, clone is cheap |
| lru crate compatibility | Very Low | Low | Already used in hl7v2-prof |

---

## Recommendation

**Proceed with lru::LruCache + RwLock implementation.**

The implementation should:
1. Add regex cache module with `OnceLock<RwLock<LruCache<String, Regex>>>`
2. Provide `get_or_compile_regex(pattern: &str) -> Option<Regex>` function
3. Modify line 831 and lines 382-387 to use cached regex
4. Add comprehensive unit tests
5. Verify thread safety with concurrent tests

---

## Next Owner

**Spec Verifier** - Verify the spec is complete and testable.

Verification checklist:
- [ ] All requirements are clear and testable
- [ ] BDD scenarios cover edge cases
- [ ] Design is implementable within constraints
- [ ] No major gaps or ambiguities
- [ ] Thread safety approach is sound

---

## Open Questions

1. Should cache capacity be configurable at runtime or only via env var?
2. Should we expose cache statistics for debugging?
3. Should we pre-populate cache with common patterns?

**Recommendation:** Keep it simple for now. Env var configuration is sufficient. Statistics and warming can be future enhancements.
