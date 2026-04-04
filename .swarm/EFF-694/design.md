# Design: Streaming Parser Buffer Allocation (EFF-694)

**Issue:** [EFF-694](/EFF/issues/EFF-694)  
**Status:** Design Complete  
**Last Updated:** 2026-04-04

---

## Architecture Overview

### Current State (Before)

```rust
loop {
    if cr_pos.is_none() {
        let mut temp_buf = vec![0u8; 1024];  // <-- HEAP ALLOCATION
        match self.reader.read(&mut temp_buf) {
            Ok(n) => {
                self.buffer.extend_from_slice(&temp_buf[..n]);
            }
        }
    }
}
```

**Flow per iteration:**
1. Allocate 1KB on heap
2. Read data into heap buffer
3. Copy data to `self.buffer`
4. Drop heap buffer
5. Repeat for next iteration

### Proposed State (After)

```rust
loop {
    if cr_pos.is_none() {
        let mut temp_buf = [0u8; 1024];  // <-- STACK ALLOCATION
        match self.reader.read(&mut temp_buf) {
            Ok(n) => {
                self.buffer.extend_from_slice(&temp_buf[..n]);
            }
        }
    }
}
```

**Flow per iteration:**
1. Allocate 1KB on stack (no heap)
2. Read data into stack buffer
3. Copy data to `self.buffer`
4. Stack buffer auto-freed on scope exit
5. Repeat for next iteration

---

## Key Design Decisions

### Decision 1: Stack vs Heap Allocation

**Choice:** Use stack-allocated array `[0u8; 1024]`

**Rationale:**
- 1024 bytes fits comfortably on stack (typical stack is 1-8MB)
- Zero heap allocation = zero allocator pressure
- Same read API works (`&mut [u8]`)
- No unsafe code required

**Code Pattern:**
```rust
// Before (heap)
let mut temp_buf = vec![0u8; 1024];

// After (stack)
let mut temp_buf = [0u8; 1024];
```

### Decision 2: Buffer Size

**Choice:** Keep 1024 bytes

**Rationale:**
- Same as current implementation
- Typical socket read size is 1-4KB
- 1024B provides good balance of stack usage vs read efficiency
- Can be increased later if profiling shows benefit

**Considerations:**
- Larger buffer (4KB): Fewer reads, more stack usage
- Smaller buffer (512B): More reads, less stack usage
- Current 1KB is reasonable default

### Decision 3: Other Allocations (Out of Scope)

**Deferred for future issues:**
- Line 375: `segment_data.to_vec()` - May need API change
- Line 487: `field.to_vec()` - Event API uses owned data
- Line 509: `field.to_vec()` - Same as above

**Rationale:**
- Line 350 fix is immediate win (10x+ allocation reduction)
- Other changes may require Event API refactoring
- Separate concerns for cleaner commits

---

## Implementation Design

### File Changes

| File | Change | Lines |
|------|--------|-------|
| `crates/hl7v2-stream/src/lib.rs` | Replace vec! with array | 1 changed |

### Code Change

**Line 350 Before:**
```rust
let mut temp_buf = vec![0u8; 1024];
```

**Line 350 After:**
```rust
let mut temp_buf = [0u8; 1024]; // Stack allocation, no heap
```

### Why This Works

Both `Vec<u8>` and `[u8; 1024]` implement `AsMut<[u8]>` which `read()` requires:

```rust
// Signature: fn read(&mut self, buf: &mut [u8]) -> Result<usize>
// Works with: &mut Vec<u8>  (via DerefMut)
// Works with: &mut [u8; N] (via array reference)
```

---

## Testing Strategy

### Unit Tests

1. **Parsing still works**: Basic message parsing succeeds
2. **Large message**: 10MB message parses without OOM
3. **Empty read**: EOF handling works correctly
4. **Partial reads**: Multiple iterations handle partial data

### Integration Tests

1. **Streaming test**: Full message streaming works
2. **Memory profile**: Allocation count measured

### Performance Tests

1. **Large message benchmark**: 10MB message throughput
2. **Allocation tracking**: Verify zero heap allocations in loop

---

## Expected Results

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| Allocations (10MB msg) | ~10,000 | 0 | 100% reduction |
| Heap pressure | High | Low | Significant |
| Throughput | Baseline | +10-20% expected | Measurable |
| Stack usage | Minimal | +1KB per call | Negligible |

---

## Risks and Mitigations

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Stack overflow | Very Low | 1KB is tiny compared to stack size |
| Read API incompatibility | Very Low | Arrays implement same traits |
| Performance regression | Low | Benchmark to verify |
| Behavior change | Very Low | Same buffer semantics |

---

## Migration Path

### Phase 1: Stack Buffer (this issue)
- Replace `vec![0u8; 1024]` with `[0u8; 1024]`
- Verify tests pass
- Benchmark improvement

### Phase 2: Other Allocations (future)
- Investigate `segment_data.to_vec()` at line 375
- Consider `bytes::Bytes` for zero-copy
- Evaluate Event API changes

### Phase 3: Advanced Optimizations (future)
- `read_buf` API for reduced copies
- Vectorized reads
- Configurable buffer sizes

---

## References

- [EFF-694](/EFF/issues/EFF-694) - This issue
- [EFF-687](/EFF/issues/EFF-687) - Regex caching
- [EFF-688](/EFF/issues/EFF-688) - Batch parsing allocation
- Rust arrays: https://doc.rust-lang.org/std/primitive.array.html
