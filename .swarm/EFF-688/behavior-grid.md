# Behavior Grid: Batch Parsing String Allocation (EFF-688)

**Purpose:** Reference matrix for implementers and testers

---

## Allocation Behavior Matrix

| Scenario | Messages | Original Allocations | Optimized Allocations | Reduction |
|----------|----------|-------------------|----------------------|-----------|
| Small batch | 10 | 20 | 1 | 95% |
| Medium batch | 1,000 | 2,000 | 1 | 99.95% |
| Large batch | 10,000 | 20,000 | 1 | 99.995% |
| File batch (10 batches) | 10,000 | ~20,020 | 2 | 99.99% |
| Single message | 1 | 2 | 1 | 50% |

---

## Buffer State Matrix

| Operation | Before | After | Buffer State |
|-----------|--------|-------|--------------|
| First message | N/A | `with_capacity(4096)` | capacity=4096, len=0 |
| Build message | `to_vec().join()` | `clear()` + `push_str()` | len grows |
| Parse message | `parse(text.as_bytes())` | `parse(buffer.as_bytes())` | unchanged |
| Message complete | buffer dropped | (buffer retained) | len=0, capacity=4096 |
| Next message | new allocation | reuse buffer | len grows again |
| Large message (>4KB) | N/A | buffer auto-grows | capacity increased |

---

## Code Location Matrix

| Function | Line | Current Code | Optimized Code | Buffer Type |
|----------|------|--------------|----------------|-------------|
| parse_batch | 465 | `to_vec().join("\r")` | `message_buffer` with push | Message (4KB) |
| parse_batch | 480 | `to_vec().join("\r")` | `message_buffer` with push | Message (4KB) |
| parse_file_batch | 526 | `to_vec().join("\r")` | `batch_buffer` with push | Batch (16KB) |
| parse_file_batch | 546 | `to_vec().join("\r")` | `batch_buffer` with push | Batch (16KB) |

---

## Input/Output Examples

### Example 1: 3-Message Batch

**Input:**
```
BHS|^~\&|...\r
MSH|^~\&|...\r
PID|...\r
OBR|...\r
MSH|^~\&|...\r
PID|...\r
MSH|^~\&|...\r
PID|...\r
BTS|...\r
```

**Original Execution:**
```
Message 1:
  to_vec() -> Vec[&str; 4] (alloc 1)
  join() -> String "MSH...\rPID..." (alloc 2)
  parse() -> Message
  (Vec and String dropped)

Message 2:
  to_vec() -> Vec[&str; 2] (alloc 3)
  join() -> String "MSH...\rPID..." (alloc 4)
  parse() -> Message
  (Vec and String dropped)

Message 3: (alloc 5, 6)
Total: 6 allocations
```

**Optimized Execution:**
```
Pre-allocate: message_buffer = String::with_capacity(4096) (alloc 1)

Message 1:
  clear() -> buffer empty (no alloc)
  push_str() + push('\r') -> buffer grows (no alloc)
  parse() -> Message

Message 2:
  clear() -> buffer empty (no alloc)
  push_str() + push('\r') -> buffer grows (no alloc)

Message 3:
  clear() -> buffer empty (no alloc)
  push_str() + push('\r') -> buffer grows (no alloc)

Total: 1 allocation (95% reduction)
```

---

## Edge Case Handling

| Edge Case | Input | Expected Behavior |
|-----------|-------|-------------------|
| Empty batch | No messages | Returns empty Batch, no buffer ops |
| Single message | 1 MSH segment | Buffer used once, single alloc |
| Empty message | MSH with no following segments | Buffer has just MSH line |
| Very large message | 100KB message | Buffer grows to 100KB, reused |
| Unicode content | UTF-8 in segments | String handles correctly |
| Binary data | Non-UTF8 bytes | May fail at parse (expected) |

---

## Test File Mappings

| Behavior Grid Scenario | Test Function | File |
|------------------------|--------------|------|
| Allocation reduction | `test_allocation_reduction` | `allocation_tests.rs` |
| Buffer reuse | `test_buffer_reuse` | `batch_parse_tests.rs` |
| Large message | `test_large_message` | `batch_parse_tests.rs` |
| Empty batch | `test_empty_batch` | `batch_parse_tests.rs` |
| Correctness | `test_parse_correctness` | `integration_tests.rs` |

---

## Performance Expectations

| Metric | Before | After | Measurement |
|--------|--------|-------|-------------|
| Allocations (10K msgs) | 20,000 | 1 | `cargo test --features=alloc-tracing` |
| Peak memory | ~20MB | ~4MB | Heap profiler |
| Throughput | Baseline | +30% expected | Benchmark |
| Latency p99 | Baseline | Similar | Benchmark |

---

**Owner:** Spec Verifier  
**Status:** Ready for verification
