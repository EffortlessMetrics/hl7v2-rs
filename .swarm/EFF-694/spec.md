# Spec: Streaming Parser Buffer Allocation (EFF-694)

**Issue:** [EFF-694](/EFF/issues/EFF-694)  
**Status:** Spec Complete  
**Branch:** EFF-694-streaming-buffer  
**Project:** hl7v2-rs

---

## Problem Statement

The streaming parser allocates a 1024-byte heap buffer on every read loop iteration via `vec![0u8; 1024]`. For a 10MB message, this creates ~10,000 allocations.

### Current State (Line 350)

```rust
let mut temp_buf = vec![0u8; 1024];  // <-- HEAP ALLOCATION
match self.reader.read(&mut temp_buf) {
    Ok(n) => self.buffer.extend_from_slice(&temp_buf[..n]),
}
```

### Solution

Replace heap allocation with stack allocation:

```rust
let mut temp_buf = [0u8; 1024];  // <-- STACK ALLOCATION
match self.reader.read(&mut temp_buf) {
    Ok(n) => self.buffer.extend_from_slice(&temp_buf[..n]),
}
```

---

## Requirements Summary

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-1 | Eliminate heap allocation in read loop | High |
| FR-2 | Use stack-allocated 1024B buffer | High |
| FR-3 | Zero behavior change | High |

---

## Design Summary

### Change

| Aspect | Before | After |
|--------|--------|-------|
| Allocation | `vec![0u8; 1024]` (heap) | `[0u8; 1024]` (stack) |
| Allocations (10MB msg) | ~10,000 | 0 |
| Stack usage | Minimal | +1KB per call |

---

## BDD Scenarios

See [scenarios.md](./scenarios.md) for detailed test scenarios.

### Quick Summary

1. **Zero allocation**: Read loop uses stack buffer, no heap
2. **Large message**: 10MB message parses with zero loop allocations
3. **Correctness**: Parse results identical to original
4. **Edge cases**: Empty read, partial read, EOF handled correctly

---

## File Changes

| File | Change | Purpose |
|------|--------|---------|
| `crates/hl7v2-stream/src/lib.rs` | Line 350: `vec!` → array | Stack allocation |

---

## Verification Criteria

- [ ] Line 350 uses `[0u8; 1024]` instead of `vec![0u8; 1024]`
- [ ] All existing tests pass
- [ ] Large message (10MB) parses correctly
- [ ] Zero heap allocations in read loop
- [ ] Throughput improved or maintained

---

## Out of Scope

- Lines 375, 487, 509 allocations (may require API changes)
- Buffer size configurability
- read_buf API adoption
- Zero-copy field references

---

## Next Owner

**Spec Verifier** - Verify spec completeness and testability.

---

## Related Issues

- [EFF-687](/EFF/issues/EFF-687) - Regex caching
- [EFF-688](/EFF/issues/EFF-688) - Batch parsing allocation
