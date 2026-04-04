# Spec: Batch Parsing String Allocation (EFF-688)

**Issue:** [EFF-688](/EFF/issues/EFF-688)  
**Status:** Spec Complete  
**Branch:** EFF-688-batch-allocation  
**Project:** hl7v2-rs

---

## Problem Statement

The batch parsing functions allocate a new `String` for every message via `.to_vec().join("\r")`. For batch files with thousands of messages, this creates O(n) allocations and significant memory pressure.

### Current State (Lines 465, 480, 526, 546)

```rust
// Current pattern - allocates for EVERY message
let message_text = current_message_lines.to_vec().join("\r");
let message = parse(message_text.as_bytes())?;
```

**Allocation breakdown:**
1. `to_vec()` - allocates new Vec, copies string pointers
2. `join()` - allocates new String, copies all content
3. Total: 2 allocations per message

### Impact

- **10,000 messages = 20,000 allocations**
- Healthcare batch files often contain 10,000+ messages
- No buffer reuse - allocations dropped after each parse

---

## Solution Overview

Replace `.to_vec().join()` with a reusable `String` buffer:

```rust
// Pre-allocate reusable buffer
let mut message_buffer = String::with_capacity(4096);

// For each message
message_buffer.clear();  // Reuse allocation
for line in &current_message_lines {
    message_buffer.push_str(line);
    message_buffer.push('\r');
}
let message = parse(message_buffer.as_bytes())?;
// Buffer ready for next message
```

**Benefits:**
- Single allocation amortized across all messages
- `String::clear()` preserves capacity, just resets length
- Works with existing `parse(&[u8])` API

---

## Requirements Summary

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-1 | Reduce per-message allocations | High |
| FR-2 | Use reusable String buffer | High |
| FR-3 | Parser API compatibility | High |
| FR-4 | Optimize error path re-parsing | Medium |
| NFR-1 | 50%+ allocation reduction | High |
| NFR-2 | Backward compatible | High |

---

## Design Summary

### Buffer Strategy

| Level | Buffer Size | Purpose |
|-------|-------------|---------|
| Message | 4KB | Individual HL7 messages |
| Batch | 16KB | Batch segments (BHS/BTS) |

### Target Locations

1. **Line 465**: `parse_batch()` - message buffer
2. **Line 480**: `parse_batch()` - final message buffer
3. **Line 526**: `parse_file_batch()` - batch buffer
4. **Line 546**: `parse_file_batch()` - final batch buffer

---

## BDD Scenarios

See [scenarios.md](./scenarios.md) for detailed test scenarios.

### Quick Summary

1. **Buffer reuse**: Buffer cleared and reused between messages
2. **Allocation reduction**: Significantly fewer allocations than original
3. **Large messages**: Buffer grows correctly for oversized messages
4. **Empty batch**: Handles empty input gracefully
5. **Parse correctness**: Results identical to original implementation

---

## File Changes

| File | Change | Purpose |
|------|--------|---------|
| `crates/hl7v2-parser/src/lib.rs` | Add message buffer | Reusable buffer for messages |
| `crates/hl7v2-parser/src/lib.rs` | Add batch buffer | Reusable buffer for batches |
| `crates/hl7v2-parser/src/lib.rs` | Replace `.to_vec().join()` | Use buffer instead |

---

## Verification Criteria

- [ ] Reusable buffer implemented for messages
- [ ] Reusable buffer implemented for batches
- [ ] `.to_vec().join()` patterns eliminated
- [ ] Allocation count reduced by 50%+
- [ ] Parse results identical to original
- [ ] All existing tests pass
- [ ] Large message handling works
- [ ] Empty batch handling works

---

## Out of Scope

- Streaming batch parser (major refactor)
- Cross-thread buffer pooling
- Zero-allocation parsing
- Parser API changes (unless needed)

---

## Next Owner

**Spec Verifier** - Verify spec completeness and testability.

---

## Related Issues

- [EFF-687](/EFF/issues/EFF-687) - Regex caching in validation
- [EFF-694](/EFF/issues/EFF-694) - Streaming parser buffer allocation
