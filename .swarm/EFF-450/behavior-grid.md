# EFF-450 Behavior Grid: Tantivy Backend

> Behavior grid for downstream test implementation  
> Spec: [spec.md](/EFF/issues/EFF-824#document-spec)  
> Parent: [EFF-450](/EFF/issues/EFF-450)

## Behavior Matrix

| ID | Feature | Scenario | Given | When | Then | Priority | Test Type |
|----|---------|----------|-------|------|------|----------|-----------|
| IDX-001 | Indexing | Single ADT message | Empty Tantivy backend | Index ADT^A01 with ID MSG001 | Index has 1 doc, retrievable by ID | P0 | Unit |
| IDX-002 | Indexing | ORU with observations | Empty backend | Index ORU^R01 with OBX segments | Stored message has correct type | P0 | Unit |
| IDX-003 | Indexing | Batch 1000 messages | 1000 ORU messages | Index in batch | All 1000 indexed, flush OK | P0 | Integration |
| IDX-004 | Indexing | Mixed message types | 500 ADT + 500 ORU | Index all | Retrievable by each type | P1 | Integration |
| IDX-005 | Indexing | Duplicate IDs allowed | MSG001 exists | Index another MSG001 | 2 docs with same ID | P1 | Unit |
| IDX-006 | Indexing | Update via remove/add | MSG001 "Original" indexed | Remove MSG001, add "Updated" | Retrieved content is "Updated" | P1 | Unit |
| IDX-007 | Indexing | Reopen existing index | 100 messages at /tmp/hl7_index | Reopen same path | All 100 retrievable | P1 | Integration |
| IDX-008 | Indexing | Survives restart | 50 messages indexed | Close and reopen backend | All 50 retrievable | P1 | Integration |
| SRCH-001 | Search | By patient name | 5 messages indexed | Search "John Doe" | 2 results: MSG001, MSG003 | P0 | Unit |
| SRCH-002 | Search | Partial name match | 5 messages indexed | Search "John" | ≥2 results with "John Doe" | P1 | Unit |
| SRCH-003 | Search | By content | 5 messages indexed | Search "Glucose" | 1 result: MSG002 | P0 | Unit |
| SRCH-004 | Search | No results | 5 messages indexed | Search "xyznonexistent" | 0 results, total=0 | P0 | Unit |
| SRCH-005 | Search | Boolean AND | 5 messages indexed | Search "John AND admitted" | 1 result: MSG001 | P1 | Unit |
| SRCH-006 | Search | Boolean OR | 5 messages indexed | Search "Glucose OR Cholesterol" | 2 results: MSG002, MSG004 | P1 | Unit |
| SRCH-007 | Search | Boolean NOT | 5 messages indexed | Search all, exclude "ORU^R01" | 3 results, no ORU^R01 | P1 | Unit |
| SRCH-008 | Search | Field: message_type | 5 messages indexed | Search "message_type:ADT^A01" | 1 result: MSG001 | P0 | Unit |
| SRCH-009 | Search | Field: source | 5 messages indexed | Search "source:Epic" | 2 results, all Epic | P1 | Unit |
| SRCH-010 | Search | Field: patient_id | 5 messages indexed | Search "patient_id:MRN12345" | 1 result | P1 | Unit |
| SRCH-011 | Search | Combined field+text | 5 messages indexed | Search "message_type:ADT^A01 AND John" | 1 result: MSG001 | P1 | Unit |
| SRCH-012 | Search | Relevance ordered | Messages with "Glucose" | Search "Glucose" | Ordered by score desc | P2 | Unit |
| SRCH-013 | Search | Score range | 5 messages indexed | Search "Patient" | All scores in [0.0, 1.0] | P2 | Unit |
| TIME-001 | TimeRange | Single day | 5 messages with timestamps | Query 2023-11-15 | 1 result: MSG002 | P0 | Unit |
| TIME-002 | TimeRange | Month range | 5 messages with timestamps | Query 2023-11-01 to 2023-11-30 | 3 results: MSG001-003 | P0 | Unit |
| TIME-003 | TimeRange | No matches | 5 messages with timestamps | Query 2024-01-01 to 2024-01-31 | 0 results | P0 | Unit |
| TIME-004 | TimeRange | Combined with text | 5 messages indexed | Search "ORU^R01" + time filter Nov | 1 result: MSG002 | P1 | Unit |
| TIME-005 | TimeRange | Recent with content | 5 messages indexed | Search "ADT" + Dec filter | 1 result: MSG005 | P1 | Unit |
| TIME-006 | TimeRange | Equal start/end | 5 messages with timestamps | Query 2023-11-15T14:30:00Z to same | 1 result with that timestamp | P1 | Unit |
| TIME-007 | TimeRange | Inclusive boundaries | 5 messages with timestamps | Query 2023-11-01T10:00:00Z to 2023-11-30T08:00:00Z | Includes MSG001 and MSG003 | P1 | Unit |
| TIME-008 | TimeRange | Invalid range | 5 messages indexed | Query end before start | Error: "Invalid time range" | P1 | Unit |
| PERF-001 | Performance | Tantivy throughput | 10000 message corpus | Index with Tantivy | ≥5000 msg/sec, <2s total | P0 | Benchmark |
| PERF-002 | Performance | SQLite throughput | 10000 message corpus | Index with SQLite | ≥1000 msg/sec, <10s total | P0 | Benchmark |
| PERF-003 | Performance | Batch improvement | Individual 1ms/msg | Batch 1000 in 100s | <0.5ms avg per msg | P1 | Benchmark |
| PERF-004 | Performance | Tantivy search latency | 10000 messages indexed | 100 searches | Avg <10ms, p99 <50ms, max <100ms | P0 | Benchmark |
| PERF-005 | Performance | SQLite search latency | 10000 messages indexed | 100 searches | Avg <50ms, p99 <200ms | P0 | Benchmark |
| PERF-006 | Performance | Cold vs warm cache | 10000 messages indexed | First search, then repeat | First may be >10ms, repeat <5ms | P2 | Benchmark |
| PERF-007 | Performance | Linear scaling | 1000 messages | Add 1000 x10 times | Each <5s, search <20ms, 11000 final | P1 | Benchmark |
| PERF-008 | Performance | Large messages | 1000 x 10KB messages | Index all | Index <20MB, search OK | P1 | Benchmark |
| PERF-009 | Performance | Memory during indexing | Memory budget 200MB | Index 10000 | Peak <200MB | P0 | Benchmark |
| PERF-010 | Performance | Memory after flush | 10000 messages indexed | Flush writer | Memory <50MB | P1 | Benchmark |
| PERF-011 | Performance | Concurrent readers | Indexing in progress | 5 concurrent searches | All complete, no corruption | P1 | Integration |
| PERF-012 | Performance | Writer thread safety | 10 threads, 100 msg each | All threads index | 1000 final, no panic/deadlock | P1 | Integration |

## Coverage Summary

| Category | Total | P0 | P1 | P2 |
|----------|-------|-----|-----|-----|
| Indexing | 8 | 3 | 5 | 0 |
| Search | 13 | 4 | 6 | 3 |
| TimeRange | 8 | 3 | 5 | 0 |
| Performance | 12 | 5 | 6 | 1 |
| **Total** | **41** | **15** | **22** | **4** |

## Test Stage Mapping

| Test Type | Behaviors | Implementation Stage |
|-----------|-----------|----------------------|
| Unit | 19 | TDD Refactor |
| Integration | 8 | Integration Test |
| Benchmark | 12 | Performance Test |

## Dependencies

- `tantivy` 0.21
- `rusqlite` 0.29 (for comparison tests)
- `criterion` 0.5 (benchmarks)
- `cucumber` 0.22 (BDD tests)
- `tempfile` 3.10 (isolated test directories)
