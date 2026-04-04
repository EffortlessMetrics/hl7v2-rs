# Behavior Grid: Streaming Parser Buffer Allocation (EFF-694)

**Purpose:** Reference matrix for implementers and testers

---

## Allocation Behavior Matrix

| Message Size | Iterations | Before (Allocations) | After (Allocations) | Reduction |
|--------------|------------|---------------------|---------------------|-----------|
| 1KB | 1 | 1 | 0 | 100% |
| 10KB | 10 | 10 | 0 | 100% |
| 100KB | 100 | 100 | 0 | 100% |
| 1MB | 1,000 | 1,000 | 0 | 100% |
| 10MB | 10,000 | 10,000 | 0 | 100% |

---

## Code Change Matrix

| Line | Before | After | Allocation Type |
|------|--------|-------|-----------------|
| 350 | `vec![0u8; 1024]` | `[0u8; 1024]` | Heap → Stack |

---

## Buffer State Matrix

| Phase | Before (Heap) | After (Stack) |
|-------|---------------|---------------|
| Declaration | Allocates 1KB on heap | Reserves 1KB on stack |
| Read | Fills heap buffer | Fills stack buffer |
| Copy to self.buffer | Copies from heap | Copies from stack |
| Drop | Frees heap memory | Auto-freed on scope exit |

---

## Input/Output Examples

### Example 1: 3KB Message

**Input:** 3KB HL7 message read in 1KB chunks

**Before:**
```
Iteration 1: vec![0u8; 1024] allocated → read 1024B → copy → drop
Iteration 2: vec![0u8; 1024] allocated → read 1024B → copy → drop
Iteration 3: vec![0u8; 1024] allocated → read 1024B → copy → drop
Total: 3 allocations
```

**After:**
```
Iteration 1: [0u8; 1024] on stack → read 1024B → copy → auto-drop
Iteration 2: [0u8; 1024] on stack → read 1024B → copy → auto-drop
Iteration 3: [0u8; 1024] on stack → read 1024B → copy → auto-drop
Total: 0 allocations
```

---

## Test File Mappings

| Scenario | Test Function | File |
|----------|--------------|------|
| Zero allocation | `test_zero_allocations` | `streaming_tests.rs` |
| Large message | `test_large_message` | `streaming_tests.rs` |
| Correctness | `test_parse_correctness` | `integration_tests.rs` |
| Edge cases | `test_edge_cases` | `streaming_tests.rs` |

---

## Performance Expectations

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| Allocations (10MB) | 10,000 | 0 | 100% |
| Heap pressure | High | Low | Significant |
| Throughput | Baseline | +10-20% | Measurable |
| Stack usage | 0 | 1KB | Negligible |

---

**Owner:** Spec Verifier  
**Status:** Ready for verification
