# ADR-001: Pluggable Search Backend Architecture for hl7v2-index

## Status

- **Status**: Proposed
- **Date**: 2026-04-04
- **Decision**: Create new `hl7v2-index` crate with trait-based pluggable backends
- **Deciders**: Spec Designer ([f770e387-ccf5-441f-8b09-45986023f6df](/EFF/agents/f770e387-ccf5-441f-8b09-45986023f6df))

## Context

The hl7v2-rs workspace currently has `hl7v2-query` which provides path-based field extraction (e.g., `PID.5.1` to get patient name). However, there is no facility for:

1. Full-text search across HL7 message content
2. Faceted filtering (e.g., "all ADT^A01 messages from Epic")
3. Time-range queries on message timestamps
4. Relevance-scored search results

Healthcare integration scenarios frequently require searching historical messages by:
- Patient identifiers (MRN, account numbers)
- Time ranges ("all messages from yesterday")
- Message types and sources
- Free-text content within messages

## Decision

We will create a new crate `hl7v2-index` that provides:

1. **Trait-based backend system** (`IndexBackend`) allowing multiple storage implementations
2. **Three backend implementations**:
   - `MemoryBackend`: HashMap-based, always available, for testing
   - `SqliteBackend`: SQLite with FTS5, for embedded deployments
   - `TantivyBackend`: Full-text search engine, for high-performance search
3. **Feature flags** to control which backends are compiled
4. **Unified query interface** that maps to backend-specific query languages

### Backend Selection Criteria

| Backend | Use Case | Trade-offs |
|---------|----------|------------|
| Memory | Unit tests, ephemeral data | Fast, no persistence, memory bound |
| SQLite | Embedded systems, small deployments | ACID, familiar SQL, moderate performance |
| Tantivy | High-volume search, analytics | Best search performance, more complex setup |

### Trait Design Rationale

The `IndexBackend` trait uses synchronous methods (not async) because:
1. Tantivy's writer is inherently serialized
2. Search operations on local indexes are typically sub-10ms
3. The trait can be wrapped in async at the service layer if needed

## Consequences

### Positive

- **Flexibility**: Consumers can choose appropriate backend for their deployment
- **Testability**: Memory backend enables fast, isolated unit tests
- **Performance**: Tantivy provides production-grade search without external dependencies
- **Maintainability**: Clear trait boundary prevents backend-specific code from leaking

### Negative

- **Complexity**: Three backends to maintain and test
- **Binary size**: Feature flags help, but Tantivy adds ~2MB to binaries
- **API surface**: Query abstraction may limit access to backend-specific features
- **Migration**: No automatic migration between backends (export/import required)

### Risks

| Risk | Mitigation |
|------|------------|
| Schema evolution breaks existing indexes | Version field in schema, migration guide |
| Tantivy writer contention | Document batching recommendation, async queue pattern |
| Query abstraction too limited | Escape hatch for raw backend queries |

## Alternatives Considered

### Option A: Use SQLite FTS5 exclusively

**Pros**: Single backend to maintain, ACID transactions, familiar SQL  
**Cons**: FTS5 is basic (no relevance scoring, limited boolean queries), performance ceiling  
**Rejected**: Search capabilities insufficient for analytics use case

### Option B: Use external search (Elasticsearch/OpenSearch)

**Pros**: Enterprise-grade, horizontal scaling, rich query DSL  
**Cons**: External dependency, deployment complexity, network overhead  
**Rejected**: Contradicts crate's goal of self-contained deployment

### Option C: Add indexing to hl7v2-query crate

**Pros**: Existing crate, path-based and search queries together  
**Cons**: Violates SRP, forces index dependencies on all query users  
**Rejected**: Indexing is distinct concern, should be separate crate

## Implementation

See [spec.md](/EFF/issues/EFF-824#document-spec) for detailed implementation specification.

Key files:
- `crates/hl7v2-index/src/backend.rs` - Trait definition
- `crates/hl7v2-index/src/backends/tantivy.rs` - Tantivy implementation
- `crates/hl7v2-index/src/backends/memory.rs` - Reference implementation

## References

- [EFF-450](/EFF/issues/EFF-450) - Tantivy backend implementation
- [EFF-824](/EFF/issues/EFF-824) - Spec design task
- Tantivy docs: https://docs.rs/tantivy/0.21.0/tantivy/
- SQLite FTS5: https://www.sqlite.org/fts5.html
