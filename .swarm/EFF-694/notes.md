# Working Notes: EFF-694

**Status:** Spec complete  
**Date:** 2026-04-04  
**Location:** `.swarm/EFF-694/`

---

## Discovery

Found at `crates/hl7v2-stream/src/lib.rs` line 350:
```rust
let mut temp_buf = vec![0u8; 1024];  // Allocates every iteration
```

This is inside a `loop` that reads data until `\r` (segment terminator) is found.

---

## Key Design Decisions

### Decision 1: Stack Buffer

Change to `let mut temp_buf = [0u8; 1024];`

**Why:**
- 1024B fits on stack (no heap allocation)
- Same behavior as Vec for read() API
- No unsafe code
- One-line change

### Decision 2: Keep 1024B Size

**Why:**
- Same as current implementation
- Good balance of reads vs stack usage
- Can be tuned later based on profiling

---

## Options Considered

| Option | Verdict |
|--------|---------|
| Stack buffer [0u8; 1024] | ✅ Selected |
| Keep heap buffer | ❌ Rejected - defeats streaming benefits |
| Increase buffer (4KB) | ⚠️ Deferred - measure first |
| Reusable buffer field | ⚠️ Deferred - more complex |

---

## Risks

| Risk | Status |
|------|--------|
| Stack overflow | Very low - 1KB is tiny |
| Read API incompatibility | Very low - array implements same traits |
| Behavior change | Very low - identical semantics |

---

## Recommendation

**Proceed with single-line change.**

This is a straightforward fix with high impact (zero loop allocations) and minimal risk.

---

## Next Owner

**Spec Verifier** - Verify spec completeness.

---

## Open Questions

1. Should we measure actual allocation impact before/after?
2. Should buffer size be configurable?

**Recommendation:** Ship the fix, measure in production, tune if needed.
