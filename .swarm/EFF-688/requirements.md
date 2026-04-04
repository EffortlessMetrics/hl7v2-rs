# Requirements: Batch Parsing String Allocation (EFF-688)

**Issue:** [EFF-688](/EFF/issues/EFF-688)  
**Status:** Requirements Defined  
**Last Updated:** 2026-04-04

---

## Problem Statement

The batch parsing functions in `hl7v2-parser` allocate a new `String` for every message when processing batch files. This creates unnecessary memory pressure when processing large batch files with thousands of messages.

### Affected Locations

1. **Line 465**: `let message_text = current_message_lines.to_vec().join("\r");`
2. **Line 480**: `let message_text = current_message_lines.to_vec().join("\r");`
3. **Line 526**: `let batch_text = current_batch_lines.to_vec().join("\r");`
4. **Line 546**: `let batch_text = current_batch_lines.to_vec().join("\r");`

### Allocation Breakdown

The `.to_vec().join("\r")` pattern causes:
1. **Vec allocation**: Copy all string pointers to new Vec
2. **String allocation**: Allocate new String with joined content
3. **UTF-8 validation**: Unnecessary since parser accepts `&[u8]`

---

## Functional Requirements

### FR-1: Reduce Per-Message Allocations
- **FR-1.1:** The system SHALL minimize allocations when processing batch messages
- **FR-1.2:** The `.to_vec().join()` pattern SHALL be replaced with more efficient approach
- **FR-1.3:** Message parsing SHALL work with borrowed data where possible

### FR-2: Buffer Reuse
- **FR-2.1:** The system MAY use a reusable String buffer instead of allocating per-message
- **FR-2.2:** Buffer capacity SHALL be pre-allocated based on estimated message size
- **FR-2.3:** Buffer SHALL be cleared and reused between messages (not reallocated)

### FR-3: Parser API Compatibility
- **FR-3.1:** The solution SHALL work with existing `parse(&[u8])` API
- **FR-3.2:** OR the solution SHALL modify parser to accept `&[&str]` slices
- **FR-3.3:** All existing batch parsing functionality SHALL be preserved

### FR-4: Eliminate Re-parsing
- **FR-4.1:** Lines 530 and 551 re-parse the same data on error - this SHALL be optimized
- **FR-4.2:** Error handling path SHALL NOT parse the same text twice

---

## Non-Functional Requirements

### NFR-1: Performance
- **NFR-1.1:** Memory allocations SHALL be reduced by at least 50% for batch processing
- **NFR-1.2:** Throughput SHALL improve for batch files with 1000+ messages
- **NFR-1.3:** Single-message processing SHALL NOT regress in performance

### NFR-2: Memory Efficiency
- **NFR-2.1:** Peak memory usage SHALL NOT increase significantly
- **NFR-2.2:** Buffer reuse SHALL prevent O(n) allocations for n messages

### NFR-3: Backward Compatibility
- **NFR-3.1:** Public API SHALL remain backward compatible
- **NFR-3.2:** Existing batch files SHALL parse identically
- **NFR-3.3:** Error messages SHALL remain helpful and descriptive

### NFR-4: Code Clarity
- **NFR-4.1:** The solution SHALL be understandable and maintainable
- **NFR-4.2:** Performance optimization intent SHALL be documented in comments

---

## Verification Criteria

| Req ID | Test Method | Success Criteria |
|--------|-------------|------------------|
| FR-1.1 | Allocation profiling | Fewer allocations in batch processing |
| FR-1.2 | Code review | No `.to_vec().join()` patterns in batch parsing |
| FR-2.1 | Unit test | Buffer reused across multiple messages |
| FR-4.1 | Code review | No duplicate `parse()` calls on error path |
| NFR-1.1 | Benchmark | 50%+ reduction in allocations |
| NFR-3.1 | Integration test | All existing batch tests pass |
| NFR-3.2 | Regression test | Sample batch files parse identically |

---

## Out of Scope (Future Work)

The following are NOT requirements for this implementation:

1. **Streaming batch parser** - Major architectural change (deferred)
2. **Buffer pooling across threads** - Cross-thread optimization (deferred)
3. **Zero-allocation parsing** - May require parser API changes (deferred)
4. **Parallel batch processing** - Multi-threaded parsing (deferred)

---

## References

- Issue: [EFF-688](/EFF/issues/EFF-688)
- Code: `crates/hl7v2-parser/src/lib.rs` lines 465, 480, 526, 546
- Related: [EFF-687](/EFF/issues/EFF-687) - Regex caching
- Related: [EFF-694](/EFF/issues/EFF-694) - Streaming parser buffer allocation
