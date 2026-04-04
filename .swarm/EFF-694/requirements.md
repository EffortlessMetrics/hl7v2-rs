# Requirements: Streaming Parser Buffer Allocation (EFF-694)

**Issue:** [EFF-694](/EFF/issues/EFF-694)  
**Status:** Requirements Defined  
**Last Updated:** 2026-04-04

---

## Problem Statement

The `hl7v2-stream` crate allocates a 1024-byte temporary buffer on every read loop iteration when parsing messages. This defeats the zero-allocation goals of a streaming parser.

### Affected Location

- **Line 350**: `let mut temp_buf = vec![0u8; 1024];`

### Allocation Impact

- **Every read iteration**: 1KB heap allocation
- **10MB message**: ~10,000 allocations (10MB / 1KB per read)
- **Buffer immediately copied** into `self.buffer`, then dropped
- **Zero reuse**: New allocation every time more data is needed

---

## Functional Requirements

### FR-1: Eliminate Heap Allocation in Read Loop
- **FR-1.1:** The read loop SHALL NOT allocate on every iteration
- **FR-1.2:** The temporary read buffer SHALL use stack allocation
- **FR-1.3:** The change SHALL NOT affect parser behavior or correctness

### FR-2: Buffer Size Compatibility
- **FR-2.1:** The stack buffer SHALL be 1024 bytes (same as current)
- **FR-2.2:** The buffer size SHALL be sufficient for typical reads
- **FR-2.3:** Buffer overflow SHALL be handled gracefully

### FR-3: Performance Improvement
- **FR-3.1:** Allocation count SHALL be reduced to near-zero for reads
- **FR-3.2:** Throughput SHALL improve for large message processing
- **FR-3.3:** Memory pressure SHALL be reduced in high-throughput scenarios

---

## Non-Functional Requirements

### NFR-1: Code Simplicity
- **NFR-1.1:** The fix SHALL be a minimal code change
- **NFR-1.2:** The change SHALL be easily reviewable
- **NFR-1.3:** Intent SHALL be documented in code comments

### NFR-2: Backward Compatibility
- **NFR-2.1:** Public API SHALL remain unchanged
- **NFR-2.2:** All existing tests SHALL pass
- **NFR-2.3:** Parser behavior SHALL be identical

### NFR-3: Safety
- **NFR-3.1:** The change SHALL NOT introduce unsafe code
- **NFR-3.2:** Stack buffer SHALL be properly bounded
- **NFR-3.3:** No stack overflow risk (1024B is safe)

---

## Verification Criteria

| Req ID | Test Method | Success Criteria |
|--------|-------------|------------------|
| FR-1.1 | Code review | No `vec![]` in read loop |
| FR-1.2 | Unit test | Stack buffer used correctly |
| FR-2.1 | Code review | `[0u8; 1024]` used |
| FR-3.1 | Allocation profiling | Zero heap allocations in read loop |
| FR-3.2 | Benchmark | Improved throughput for large messages |
| NFR-2.2 | Test suite | All existing tests pass |
| NFR-3.1 | Code review | No `unsafe` blocks introduced |

---

## Out of Scope (Future Work)

1. **Additional allocations in file** (lines 375, 487, 509) - May require API changes
2. **Buffer size configurability** - 1024B is sufficient for now
3. **read_buf API adoption** - Modern Rust API, deferred
4. **Zero-copy field references** - Requires Event API changes

---

## References

- Issue: [EFF-694](/EFF/issues/EFF-694)
- Code: `crates/hl7v2-stream/src/lib.rs` line 350
- Related: [EFF-687](/EFF/issues/EFF-687) - Regex caching
- Related: [EFF-688](/EFF/issues/EFF-688) - Batch parsing allocation
