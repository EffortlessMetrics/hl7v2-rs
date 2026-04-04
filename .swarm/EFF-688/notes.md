# Working Notes: EFF-688

**Status:** Spec complete  
**Date:** 2026-04-04  
**Location:** `.swarm/EFF-688/`

---

## Discovery

Upon examination of `crates/hl7v2-parser/src/lib.rs`:

1. **Four locations identified**:
   - Line 465: `current_message_lines.to_vec().join("\r")`
   - Line 480: `current_message_lines.to_vec().join("\r")`
   - Line 526: `current_batch_lines.to_vec().join("\r")`
   - Line 546: `current_batch_lines.to_vec().join("\r")`

2. **Double allocation pattern**:
   - `to_vec()` allocates Vec, copies pointers
   - `join()` allocates String, copies all content
   - 2 allocations per message

3. **Re-parsing on error** (lines 530, 551):
   - When `parse_batch` fails, code calls `parse()` on same data
   - Doubles CPU cost for error cases
   - Left for future optimization

---

## Key Design Decisions

### Decision 1: String with_capacity vs Vec<u8>

**Rationale:**
- `String` works with existing `parse(&[u8])` API via `as_bytes()`
- `String` has same allocation behavior as `Vec<u8>`
- `String::clear()` preserves capacity
- No need to change parser API

### Decision 2: Buffer sizes (4KB message, 16KB batch)

**Rationale:**
- 4KB covers ~80% of HL7 messages without growth
- 16KB for batches allows larger batch headers
- Both are reasonable defaults that auto-grow if needed

### Decision 3: Keep error path re-parse (for now)

**Rationale:**
- Fixing re-parse requires more refactoring
- Main goal is allocation reduction, not CPU optimization
- Can be addressed in follow-up issue

---

## Options Considered

### Option 1: Reusable String buffer (Selected)

**Pros:**
- Minimal code changes
- Works with existing API
- Proven pattern

**Cons:**
- Still allocates once per parse call
- Not zero-allocation

**Verdict:** Selected - best balance of improvement vs complexity

### Option 2: Parser API change to accept `&[&str]`

**Pros:**
- Zero-allocation possible
- Cleanest design

**Cons:**
- Breaking API change
- Requires parser modification
- More complex

**Verdict:** Deferred - good for v2

### Option 3: Buffer pool / object pool

**Pros:**
- Amortizes allocations across parse calls
- Good for high-throughput

**Cons:**
- Requires synchronization
- More complex
- Overkill for current use case

**Verdict:** Deferred - consider if throughput becomes critical

### Option 4: Streaming parser

**Pros:**
- Minimal memory regardless of batch size
- Yields messages as found

**Cons:**
- Major architectural change
- Complex implementation

**Verdict:** Deferred - v2 architecture consideration

---

## Risks and Unknowns

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Buffer too small causing many growths | Low | Low | Start with 4KB, grows automatically |
| Buffer retained too long (memory bloat) | Low | Low | Buffer dropped when parse returns |
| Thread safety issues | Very Low | Low | Buffer is local to parse call |
| Performance regression for small batches | Low | Low | Benchmark to verify |

---

## Recommendation

**Proceed with reusable String buffer implementation.**

The spec:
1. Adds reusable buffers (4KB message, 16KB batch)
2. Replaces `.to_vec().join()` patterns
3. Maintains backward compatibility
4. Reduces allocations by 50%+

---

## Next Owner

**Spec Verifier** - Verify spec completeness and testability.

---

## Open Questions

1. Should we measure actual allocation overhead before/after?
2. Should we expose buffer size as configurable parameter?
3. Is 4KB/16KB the right default for healthcare use cases?

**Recommendation:** Ship with defaults, measure in production, adjust if needed.
