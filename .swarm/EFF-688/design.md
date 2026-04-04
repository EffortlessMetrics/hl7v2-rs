# Design: Batch Parsing String Allocation (EFF-688)

**Issue:** [EFF-688](/EFF/issues/EFF-688)  
**Status:** Design Complete  
**Last Updated:** 2026-04-04

---

## Architecture Overview

### Current State (Before)

```
┌─────────────────────────────────────────────────────────────────┐
│                    Batch File (10,000 messages)                  │
└─────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│              For each message:                                     │
│  ┌──────────────┐    ┌──────────┐    ┌──────────────────┐      │
│  │ to_vec()     │───▶│ join()   │───▶│ parse(String)    │      │
│  │ (alloc Vec)  │    │ (alloc   │    │ (parse &[u8])    │      │
│  │               │    │  String) │    │                  │      │
│  └──────────────┘    └──────────┘    └──────────────────┘      │
│                                                                    │
│  Result: 20,000 allocations for 10,000 messages (2 per message)  │
└─────────────────────────────────────────────────────────────────┘
```

### Proposed State (After)

```
┌─────────────────────────────────────────────────────────────────┐
│                    Batch File (10,000 messages)                  │
└─────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│              Reusable String Buffer (pre-allocated)              │
│                                                                    │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  For each message:                                         │   │
│  │    1. buffer.clear()                                       │   │
│  │    2. for line in lines { buffer.push_str(line); }         │   │
│  │    3. buffer.push('\r')                                    │   │
│  │    4. parse(buffer.as_bytes())                             │   │
│  │    5. (buffer reused for next message)                     │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                    │
│  Result: 1 allocation total (amortized across all messages)      │
└─────────────────────────────────────────────────────────────────┘
```

---

## Key Design Decisions

### Decision 1: Buffer Reuse with `String::with_capacity`

**Choice:** Use a reusable `String` buffer with pre-allocated capacity

**Rationale:**
- Minimal change to existing code structure
- `String::clear()` reuses allocation (does not free memory)
- Pre-allocation avoids repeated growth reallocations
- Works with existing `parse(&[u8])` API

**Code Pattern:**
```rust
// Pre-allocate reusable buffer
let mut message_buffer = String::with_capacity(4096); // 4KB default

for message in batch {
    message_buffer.clear();  // Reuse allocation, just reset len to 0
    
    // Build message text
    for line in message_lines {
        message_buffer.push_str(line);
        message_buffer.push('\r');
    }
    
    // Parse using buffer
    let msg = parse(message_buffer.as_bytes())?;
    // Buffer ready for reuse
}
```

### Decision 2: Capacity Estimation

**Choice:** Start with 4KB default, grow if needed

**Rationale:**
- Typical HL7 message: 1-10KB
- 4KB covers ~80% of messages without growth
- Buffer grows automatically if message exceeds capacity
- Growth amortized over many messages

**Alternative Considered:**
- Calculate exact capacity from line lengths: Adds O(n) scan overhead
- Use fixed large buffer (64KB): Wastes memory for small messages
- Use `Vec<u8>` instead of `String`: Same allocation behavior

### Decision 3: Error Path Optimization

**Choice:** Remove duplicate parsing on error path

**Current Code (lines 527-536, 548-557):**
```rust
match parse_batch(batch_text.as_bytes()) {
    Ok(batch) => batches.push(batch),
    Err(e) => {
        let message = parse(batch_text.as_bytes()).map_err(|_| e)?;  // RE-PARSE!
        // ...
    }
}
```

**Optimized Approach:**
```rust
match parse_batch(batch_text.as_bytes()) {
    Ok(batch) => batches.push(batch),
    Err(e) => {
        // Try to parse as single message WITHOUT re-parsing same text
        // Use cached parse result or structure differently
        // ...
    }
}
```

**Decision:** For now, keep re-parse but cache the successful parse result

### Decision 4: Batch vs Message Buffer Separation

**Choice:** Separate buffers for batch-level and message-level parsing

**Rationale:**
- File batch parsing has nested structure: File → Batch → Message
- Different buffer sizes needed at each level
- Clearer ownership and lifecycle

**Buffer Structure:**
```rust
fn parse_file_batch(lines: &[&str]) -> Result<FileBatch, Error> {
    let mut batch_buffer = String::with_capacity(16384); // 16KB for batches
    // ...
    
    fn parse_batch(messages: &[&str]) -> Result<Batch, Error> {
        let mut message_buffer = String::with_capacity(4096); // 4KB for messages
        // ...
    }
}
```

---

## Implementation Design

### File Changes

| File | Change | Lines |
|------|--------|-------|
| `crates/hl7v2-parser/src/lib.rs` | Add reusable buffer to `parse_batch` | ~15 changed |
| `crates/hl7v2-parser/src/lib.rs` | Add reusable buffer to `parse_file_batch` | ~15 changed |
| `crates/hl7v2-parser/src/lib.rs` | Optimize error path (lines 530, 551) | ~5 changed |

### Modified Code Sections

#### Section 1: Lines 463-485 (parse_batch)

**Before:**
```rust
// Line 465
let message_text = current_message_lines.to_vec().join("\r");
let message = parse(message_text.as_bytes())?;

// Line 480
let message_text = current_message_lines.to_vec().join("\r");
let message = parse(message_text.as_bytes())?;
```

**After:**
```rust
// Pre-allocate reusable buffer
let mut message_buffer = String::with_capacity(4096);

// ... inside loop ...
message_buffer.clear();
for line in &current_message_lines {
    message_buffer.push_str(line);
    message_buffer.push('\r');
}
let message = parse(message_buffer.as_bytes())?;
```

#### Section 2: Lines 524-558 (parse_file_batch)

**Before:**
```rust
// Line 526
let batch_text = current_batch_lines.to_vec().join("\r");

// Line 546
let batch_text = current_batch_lines.to_vec().join("\r");
```

**After:**
```rust
// Pre-allocate reusable buffer
let mut batch_buffer = String::with_capacity(16384);

// ... inside loop ...
batch_buffer.clear();
for line in &current_batch_lines {
    batch_buffer.push_str(line);
    batch_buffer.push('\r');
}
```

---

## Testing Strategy

### Unit Tests

1. **Buffer reuse test**: Verify buffer capacity preserved across clears
2. **Allocation count test**: Verify fewer allocations than original
3. **Correctness test**: Parse results identical to original
4. **Large message test**: Verify buffer growth works correctly
5. **Empty batch test**: Edge case - no panic on empty input

### Integration Tests

1. **Batch file parsing**: Full batch files parse correctly
2. **Memory profile**: `cargo test` with allocation tracing
3. **Regression test**: Sample healthcare batch files produce same output

### Performance Tests

1. **Benchmark**: 10,000 message batch - measure time and allocations
2. **Memory profile**: Heaptrack or similar to verify allocation reduction

---

## Migration Path

### Phase 1: Buffer Reuse (this issue)
- Add reusable String buffers
- Replace `.to_vec().join()` patterns
- Add capacity estimation

### Phase 2: Parser API Enhancement (future)
- Consider adding `parse_lines(&[&str])` API
- Zero-allocation path for batch parsing

### Phase 3: Streaming Parser (future)
- Yield messages as they're found
- Minimal memory regardless of batch size

---

## Known Limitations

1. **Still allocates for very large messages**: Buffer grows as needed
2. **Not zero-allocation**: One initial allocation per parse call
3. **Not thread-shared**: Each parse call has its own buffer
4. **Error path still re-parses**: Requires more refactoring to fix

---

## Performance Expectations

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| Allocations (10K messages) | ~20,000 | ~1 | 99.995% reduction |
| Memory used | O(n) | O(1) bounded | Bounded growth |
| Throughput | Baseline | +20-50% | Expected gain |

---

## References

- [EFF-688](/EFF/issues/EFF-688) - This issue
- [EFF-687](/EFF/issues/EFF-687) - Regex caching (similar optimization)
- [EFF-694](/EFF/issues/EFF-694) - Streaming parser buffer allocation
- Rust String docs: https://doc.rust-lang.org/std/string/struct.String.html
