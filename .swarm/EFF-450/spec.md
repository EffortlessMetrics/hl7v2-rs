# EFF-450: Tantivy Backend Implementation Spec

> **Issue**: [EFF-450](/EFF/issues/EFF-450)  
> **Spec Task**: [EFF-824](/EFF/issues/EFF-824)  
> **Branch**: `feature/EFF-450-tantivy-backend`  
> **Status**: Spec Design Complete  
> **Next Owner**: Spec Verifier

---

## 1. Executive Summary

This specification defines the implementation of a Tantivy-based search backend for the new `hl7v2-index` crate. The crate provides pluggable indexing capabilities for HL7 v2 messages, enabling fast full-text search, faceted filtering, and relevance scoring.

### Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| **New crate: `hl7v2-index`** | Search is a distinct concern from query (path-based extraction). Keeping it separate maintains SRP and allows optional dependency. |
| **Trait-based backend system** | Enables swapping between Memory (testing), SQLite (embedded), and Tantivy (high-performance search) without changing consumer code. |
| **Tantivy 0.21** | Current stable release with good async support and schema evolution capabilities. |
| **Feature-flagged backends** | Consumers only compile the backends they need, reducing binary size and compile times. |

---

## 2. Crate Structure

```
crates/hl7v2-index/
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs              # Public API exports
    ├── backend.rs          # IndexBackend trait definition
    ├── error.rs            # IndexError types
    ├── config.rs           # BackendConfig, BackendType
    ├── query.rs            # HL7 query types
    ├── results.rs          # SearchResult, SearchHit
    ├── backends/
    │   ├── mod.rs          # Backend module exports
    │   ├── memory.rs       # In-memory backend (always available)
    │   ├── sqlite.rs       # SQLite FTS backend (feature: sqlite-backend)
    │   └── tantivy.rs      # Tantivy backend (feature: tantivy-backend)
    └── tests/
        ├── bdd_tests.rs    # Cucumber BDD scenarios
        └── integration_tests.rs
```

---

## 3. Interface Design

### 3.1 IndexBackend Trait

```rust
/// Trait for pluggable HL7 message indexing backends.
/// 
/// Implementations must be Send + Sync for concurrent access.
/// All operations return `IndexError` for consistent error handling.
pub trait IndexBackend: Send + Sync {
    /// Index a new HL7 message
    /// 
    /// # Arguments
    /// * `entry` - The message entry to index
    /// 
    /// # Returns
    /// * `Ok(())` - Message successfully indexed
    /// * `Err(IndexError)` - Indexing failed (duplicate ID, disk full, etc.)
    fn add_message(&mut self, entry: &MessageEntry) -> Result<(), IndexError>;

    /// Execute a search query
    /// 
    /// # Arguments
    /// * `query` - The search query to execute
    /// 
    /// # Returns
    /// * `Ok(SearchResult)` - Search completed with hits
    /// * `Err(IndexError)` - Query parsing or execution failed
    fn search(&self, query: &SearchQuery) -> Result<SearchResult, IndexError>;

    /// Retrieve a message by its unique ID
    /// 
    /// # Arguments
    /// * `id` - The message ID (typically MSH-10 Message Control ID)
    /// 
    /// # Returns
    /// * `Ok(Some(entry))` - Message found
    /// * `Ok(None)` - Message not found
    /// * `Err(IndexError)` - Storage access failed
    fn get(&self, id: &str) -> Result<Option<MessageEntry>, IndexError>;

    /// Remove a message from the index
    /// 
    /// # Arguments
    /// * `id` - The message ID to remove
    /// 
    /// # Returns
    /// * `Ok(true)` - Message was removed
    /// * `Ok(false)` - Message was not found
    /// * `Err(IndexError)` - Removal failed
    fn remove(&mut self, id: &str) -> Result<bool, IndexError>;

    /// Persist pending changes to storage
    /// 
    /// # Returns
    /// * `Ok(())` - All pending changes persisted
    /// * `Err(IndexError)` - Flush failed (disk full, I/O error)
    fn flush(&mut self) -> Result<(), IndexError>;

    /// Get index statistics
    /// 
    /// # Returns
    /// * `Ok(IndexStats)` - Current statistics
    /// * `Err(IndexError)` - Stats collection failed
    fn stats(&self) -> Result<IndexStats, IndexError>;

    /// Update an existing message (atomic remove + add)
    /// 
    /// Default implementation calls remove then add.
    /// Backends may override for atomic update operations.
    fn update(&mut self, entry: &MessageEntry) -> Result<(), IndexError> {
        self.remove(&entry.id)?;
        self.add_message(entry)?;
        Ok(())
    }
}
```

### 3.2 Core Types

```rust
/// Unique identifier for indexed messages
pub type MessageId = String;

/// Entry stored in the index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEntry {
    /// Unique message identifier (typically MSH-10)
    pub id: MessageId,
    
    /// Message type (MSH-9.1^MSH-9.2, e.g., "ADT^A01")
    pub message_type: String,
    
    /// Timestamp from MSH-7 (Date/Time of Message)
    pub timestamp: DateTime<Utc>,
    
    /// Source sending application (MSH-3)
    pub source: String,
    
    /// Source sending facility (MSH-4)
    pub facility: String,
    
    /// Full message content for retrieval
    pub content: String,
    
    /// Pre-extracted searchable fields
    pub searchable_fields: SearchableFields,
}

/// Fields extracted for full-text search
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchableFields {
    /// Patient identifiers (PID-3)
    pub patient_ids: Vec<String>,
    
    /// Patient name components (PID-5)
    pub patient_names: Vec<String>,
    
    /// Account numbers (PID-18)
    pub account_numbers: Vec<String>,
    
    /// Any additional extracted fields
    pub custom: HashMap<String, Vec<String>>,
}

/// Search query types
#[derive(Debug, Clone)]
pub enum SearchQuery {
    /// Full-text search across all indexed content
    FullText(String),
    
    /// Field-specific search
    Field {
        field: SearchField,
        value: String,
    },
    
    /// Boolean combination of queries
    Boolean {
        operator: BoolOperator,
        queries: Vec<SearchQuery>,
    },
    
    /// Time-range filter
    TimeRange {
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    },
    
    /// Faceted search with filters
    Faceted {
        query: Box<SearchQuery>,
        facets: Vec<FacetFilter>,
    },
}

/// Searchable field names
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchField {
    MessageType,
    Source,
    Facility,
    PatientId,
    PatientName,
    AccountNumber,
    Content,
}

/// Boolean operators for query composition
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoolOperator {
    And,
    Or,
    Not,
}

/// Facet filter for refined searches
#[derive(Debug, Clone)]
pub struct FacetFilter {
    pub field: SearchField,
    pub value: String,
}

/// Search results
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Total matching documents (regardless of limit)
    pub total: usize,
    
    /// Returned hits (paginated)
    pub hits: Vec<SearchHit>,
    
    /// Facet counts if requested
    pub facets: Option<HashMap<SearchField, Vec<FacetCount>>>,
    
    /// Time taken to execute query
    pub elapsed_ms: u64,
}

/// Individual search hit
#[derive(Debug, Clone)]
pub struct SearchHit {
    /// Message entry (with content if stored)
    pub entry: MessageEntry,
    
    /// Relevance score (higher = more relevant)
    pub score: f32,
    
    /// Highlighted snippets if applicable
    pub highlights: Vec<String>,
}

/// Facet value count
#[derive(Debug, Clone)]
pub struct FacetCount {
    pub value: String,
    pub count: usize,
}

/// Index statistics
#[derive(Debug, Clone)]
pub struct IndexStats {
    /// Total indexed documents
    pub document_count: usize,
    
    /// Index size on disk (if applicable)
    pub size_bytes: Option<usize>,
    
    /// Last commit timestamp
    pub last_commit: Option<DateTime<Utc>>,
    
    /// Backend-specific metrics
    pub backend_metrics: HashMap<String, String>,
}
```

### 3.3 Configuration

```rust
/// Backend type selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendType {
    /// In-memory hashmap (testing, volatile)
    #[default]
    Memory,
    
    /// SQLite with FTS5
    #[cfg(feature = "sqlite-backend")]
    Sqlite,
    
    /// Tantivy full-text search engine
    #[cfg(feature = "tantivy-backend")]
    Tantivy,
}

/// Backend configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfig {
    /// Backend type to use
    pub backend_type: BackendType,
    
    /// Storage path (for file-based backends)
    pub data_dir: Option<PathBuf>,
    
    /// Tantivy-specific configuration
    #[serde(default)]
    pub tantivy: TantivyConfig,
    
    /// SQLite-specific configuration
    #[serde(default)]
    pub sqlite: SqliteConfig,
}

impl Default for BackendConfig {
    fn default() -> Self {
        Self {
            backend_type: BackendType::Memory,
            data_dir: None,
            tantivy: TantivyConfig::default(),
            sqlite: SqliteConfig::default(),
        }
    }
}

/// Tantivy-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TantivyConfig {
    /// Maximum memory usage for indexing (MB)
    pub memory_budget_mb: usize,
    
    /// Number of indexing threads
    pub indexing_threads: usize,
    
    /// Commit interval (seconds, 0 = manual only)
    pub auto_commit_secs: u64,
    
    /// Enable highlighting support
    pub enable_highlighting: bool,
    
    /// Index writer heap size per thread (MB)
    pub writer_heap_size_mb: usize,
}

impl Default for TantivyConfig {
    fn default() -> Self {
        Self {
            memory_budget_mb: 100,
            indexing_threads: 1,
            auto_commit_secs: 0,  // Manual commit by default
            enable_highlighting: true,
            writer_heap_size_mb: 50,
        }
    }
}
```

---

## 4. Tantivy Schema Design

### 4.1 Field Definitions

```rust
use tantivy::schema::*;

/// Build the Tantivy schema for HL7 message indexing
pub fn build_schema() -> Schema {
    let mut builder = Schema::builder();
    
    // Primary key: Message ID (MSH-10) - exact match, stored
    builder.add_text_field(
        "id",
        STRING 
            | STORED 
            | FAST 
            | INDEXED
    );
    
    // Message type (MSH-9.1^MSH-9.2) - facet/filter capable
    builder.add_text_field(
        "message_type",
        STRING 
            | STORED 
            | FAST 
            | INDEXED
    );
    
    // Timestamp (MSH-7) - range queries, sorted
    builder.add_date_field(
        "timestamp",
        FAST | INDEXED | STORED
    );
    
    // Source application (MSH-3) - filter/facet
    builder.add_text_field(
        "source",
        STRING 
            | STORED 
            | FAST 
            | INDEXED
    );
    
    // Source facility (MSH-4) - filter/facet
    builder.add_text_field(
        "facility",
        STRING 
            | STORED 
            | FAST 
            | INDEXED
    );
    
    // Patient ID (PID-3) - searchable, exact match preferred
    builder.add_text_field(
        "patient_id",
        TEXT 
            | STORED 
            | FAST 
            | INDEXED
    );
    
    // Patient name (PID-5) - full-text searchable
    builder.add_text_field(
    "patient_name",
        TEXT 
            | STORED 
            | FAST 
            | INDEXED
    );
    
    // Account number (PID-18) - searchable
    builder.add_text_field(
        "account_number",
        TEXT 
            | STORED 
            | FAST 
            | INDEXED
    );
    
    // Full message content - full-text indexed, not stored (separate retrieval)
    builder.add_text_field(
        "content",
        TEXT | STORED | INDEXED
    );
    
    // Searchable fields as JSON blob for flexible retrieval
    builder.add_text_field(
        "searchable_fields_json",
        STORED
    );
    
    builder.build()
}
```

### 4.2 Field Attributes Explained

| Field | Tantivy Options | Purpose |
|-------|-----------------|---------|
| `id` | `STRING\|STORED\|FAST\|INDEXED` | Exact match retrieval, unique identifier |
| `message_type` | `STRING\|STORED\|FAST\|INDEXED` | Faceting, filtering (e.g., "ADT^A01") |
| `timestamp` | `FAST\|INDEXED\|STORED` | Range queries, sorting by time |
| `source` | `STRING\|STORED\|FAST\|INDEXED` | Filter by sending application |
| `facility` | `STRING\|STORED\|FAST\|INDEXED` | Filter by sending facility |
| `patient_id` | `TEXT\|STORED\|FAST\|INDEXED` | Full-text patient identifier search |
| `patient_name` | `TEXT\|STORED\|FAST\|INDEXED` | Full-text name search |
| `account_number` | `TEXT\|STORED\|FAST\|INDEXED` | Account number search |
| `content` | `TEXT\|STORED\|INDEXED` | Full HL7 message content search |
| `searchable_fields_json` | `STORED` | Serialized SearchableFields struct |

### 4.3 Tokenizer Configuration

```rust
use tantivy::tokenizer::*;

/// Configure custom tokenizers for HL7 fields
pub fn configure_tokenizers() -> TokenizerManager {
    let manager = TokenizerManager::default();
    
    // Standard tokenizer for general text
    manager.register(
        "default",
        TextAnalyzer::builder(SimpleTokenizer::default())
            .filter(RemoveLongFilter::limit(40))
            .filter(LowerCaser)
            .filter(Stemmer::new(Language::English))
            .build(),
    );
    
    // ID tokenizer: preserve case, split on separators
    manager.register(
        "id_tokenizer",
        TextAnalyzer::builder(SimpleTokenizer::default())
            .filter(RemoveLongFilter::limit(100))
            .build(),
    );
    
    // Raw tokenizer: no processing (for exact match fields)
    manager.register(
        "raw",
        TextAnalyzer::builder(RawTokenizer::default()).build(),
    );
    
    manager
}
```

---

## 5. TantivyBackend Implementation

### 5.1 Struct Definition

```rust
use tantivy::{Index, IndexReader, IndexWriter, TantivyDocument};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// Tantivy-based search backend
pub struct TantivyBackend {
    /// Tantivy index handle
    index: Index,
    
    /// Index reader for searches
    reader: IndexReader,
    
    /// Index writer for modifications (wrapped for thread safety)
    writer: Arc<RwLock<IndexWriter>>,
    
    /// Schema reference
    schema: Schema,
    
    /// Field handles (avoid repeated lookups)
    fields: FieldHandles,
    
    /// Configuration
    config: TantivyConfig,
    
    /// Data directory path
    data_dir: PathBuf,
}

/// Cached field handles for performance
struct FieldHandles {
    id: Field,
    message_type: Field,
    timestamp: Field,
    source: Field,
    facility: Field,
    patient_id: Field,
    patient_name: Field,
    account_number: Field,
    content: Field,
    searchable_fields_json: Field,
}

impl FieldHandles {
    fn from_schema(schema: &Schema) -> Self {
        Self {
            id: schema.get_field("id").expect("id field"),
            message_type: schema.get_field("message_type").expect("message_type field"),
            timestamp: schema.get_field("timestamp").expect("timestamp field"),
            source: schema.get_field("source").expect("source field"),
            facility: schema.get_field("facility").expect("facility field"),
            patient_id: schema.get_field("patient_id").expect("patient_id field"),
            patient_name: schema.get_field("patient_name").expect("patient_name field"),
            account_number: schema.get_field("account_number").expect("account_number field"),
            content: schema.get_field("content").expect("content field"),
            searchable_fields_json: schema.get_field("searchable_fields_json")
                .expect("searchable_fields_json field"),
        }
    }
}
```

### 5.2 Constructor

```rust
impl TantivyBackend {
    /// Open or create a Tantivy index at the specified path
    pub fn open_or_create(
        data_dir: impl AsRef<std::path::Path>,
        config: TantivyConfig,
    ) -> Result<Self, IndexError> {
        let data_dir = data_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&data_dir)?;
        
        let schema = build_schema();
        
        let index = if Index::exists(&data_dir)? {
            Index::open_in_dir(&data_dir)?
        } else {
            Index::create_in_dir(&data_dir, schema.clone())?
        };
        
        // Configure tokenizer manager
        index.set_tokenizers(configure_tokenizers());
        
        let writer = index.writer_with_num_threads(
            config.indexing_threads,
            config.memory_budget_mb * 1024 * 1024,
        )?;
        
        let reader = index.reader()?;
        let fields = FieldHandles::from_schema(&schema);
        
        Ok(Self {
            index,
            reader,
            writer: Arc::new(RwLock::new(writer)),
            schema,
            fields,
            config,
            data_dir,
        })
    }
}
```

### 5.3 IndexBackend Implementation

```rust
impl IndexBackend for TantivyBackend {
    fn add_message(&mut self, entry: &MessageEntry) -> Result<(), IndexError> {
        use tantivy::schema::document::Value;
        
        let mut doc = TantivyDocument::default();
        
        // Add all fields
        doc.add_field_value(self.fields.id, &entry.id);
        doc.add_field_value(self.fields.message_type, &entry.message_type);
        doc.add_field_value(
            self.fields.timestamp,
            &tantivy::DateTime::from_utc(entry.timestamp)
        );
        doc.add_field_value(self.fields.source, &entry.source);
        doc.add_field_value(self.fields.facility, &entry.facility);
        
        // Add patient identifiers
        for pid in &entry.searchable_fields.patient_ids {
            doc.add_field_value(self.fields.patient_id, pid);
        }
        
        // Add patient names
        for name in &entry.searchable_fields.patient_names {
            doc.add_field_value(self.fields.patient_name, name);
        }
        
        // Add account numbers
        for acct in &entry.searchable_fields.account_numbers {
            doc.add_field_value(self.fields.account_number, acct);
        }
        
        // Store full content
        doc.add_field_value(self.fields.content, &entry.content);
        
        // Store searchable fields as JSON
        let fields_json = serde_json::to_string(&entry.searchable_fields)?;
        doc.add_field_value(self.fields.searchable_fields_json, &fields_json);
        
        // Add to index
        let mut writer = self.writer.write()
            .map_err(|_| IndexError::LockError("Writer lock poisoned".into()))?;
        writer.add_document(doc)?;
        
        Ok(())
    }

    fn search(&self, query: &SearchQuery) -> Result<SearchResult, IndexError> {
        let searcher = self.reader.searcher();
        let query_parser = QueryParser::for_index(&self.index, vec![
            self.fields.content,
            self.fields.patient_name,
            self.fields.patient_id,
        ]);
        
        let tantivy_query = self.convert_query(query)?;
        
        let start = std::time::Instant::now();
        
        let top_docs = searcher.search(
            &tantivy_query,
            &TopDocs::with_limit(100),
        )?;
        
        let elapsed_ms = start.elapsed().as_millis() as u64;
        
        // Collect results
        let mut hits = Vec::new();
        for (score, doc_address) in top_docs {
            let doc = searcher.doc::<TantivyDocument>(doc_address)?;
            let entry = self.doc_to_entry(&doc)?;
            hits.push(SearchHit {
                entry,
                score,
                highlights: vec![],  // TODO: highlighting
            });
        }
        
        // Get total count
        let total = searcher.num_docs() as usize;  // Approximation, refine with query
        
        Ok(SearchResult {
            total,
            hits,
            facets: None,  // TODO: facet collection
            elapsed_ms,
        })
    }

    fn get(&self, id: &str) -> Result<Option<MessageEntry>, IndexError> {
        let searcher = self.reader.searcher();
        
        // Build term query for exact ID match
        let term = Term::from_field_text(self.fields.id, id);
        let query = TermQuery::new(term, IndexRecordOption::Basic);
        
        let top_docs = searcher.search(&query, &TopDocs::with_limit(1))?;
        
        if let Some((_score, doc_address)) = top_docs.into_iter().next() {
            let doc = searcher.doc::<TantivyDocument>(doc_address)?;
            let entry = self.doc_to_entry(&doc)?;
            Ok(Some(entry))
        } else {
            Ok(None)
        }
    }

    fn remove(&mut self, id: &str) -> Result<bool, IndexError> {
        let term = Term::from_field_text(self.fields.id, id);
        
        let mut writer = self.writer.write()
            .map_err(|_| IndexError::LockError("Writer lock poisoned".into()))?;
        
        // Tantivy doesn't return count, so we check existence first
        let existed = self.get(id)?.is_some();
        writer.delete_term(term);
        
        Ok(existed)
    }

    fn flush(&mut self) -> Result<(), IndexError> {
        let mut writer = self.writer.write()
            .map_err(|_| IndexError::LockError("Writer lock poisoned".into()))?;
        writer.commit()?;
        
        // Reload reader to pick up changes
        self.reader.reload()?;
        
        Ok(())
    }

    fn stats(&self) -> Result<IndexStats, IndexError> {
        let searcher = self.reader.searcher();
        let doc_count = searcher.num_docs() as usize;
        
        // Calculate directory size
        let size_bytes = if self.data_dir.exists() {
            Some(self.calculate_dir_size(&self.data_dir)?)
        } else {
            None
        };
        
        let mut backend_metrics = HashMap::new();
        backend_metrics.insert("segments".to_string(), 
            searcher.segment_readers().len().to_string());
        
        Ok(IndexStats {
            document_count: doc_count,
            size_bytes,
            last_commit: None,  // TODO: track last commit
            backend_metrics,
        })
    }
}
```

### 5.4 Query Conversion

```rust
impl TantivyBackend {
    /// Convert HL7 SearchQuery to Tantivy Query
    fn convert_query(&self, query: &SearchQuery) -> Result<Box<dyn Query>, IndexError> {
        use SearchQuery::*;
        
        match query {
            FullText(text) => {
                let query_parser = QueryParser::for_index(&self.index, vec![
                    self.fields.content,
                    self.fields.patient_name,
                ]);
                Ok(query_parser.parse_query(text)?)
            }
            
            Field { field, value } => {
                let field = self.search_field_to_tantivy(*field);
                let term = Term::from_field_text(field, value);
                Ok(Box::new(TermQuery::new(term, IndexRecordOption::WithFreqsAndPositions)))
            }
            
            Boolean { operator, queries } => {
                let subqueries: Vec<Box<dyn Query>> = queries
                    .iter()
                    .map(|q| self.convert_query(q))
                    .collect::<Result<Vec<_>, _>>()?;
                
                match operator {
                    BoolOperator::And => Ok(Box::new(BooleanQuery::new_multiterms_query(
                        subqueries.into_iter().map(|q| q as _).collect()
                    ))),
                    BoolOperator::Or => Ok(Box::new(BooleanQuery::new_multiterms_query(
                        subqueries.into_iter().map(|q| q as _).collect()
                    ))),
                    BoolOperator::Not => {
                        if let Some(first) = subqueries.into_iter().next() {
                            Ok(Box::new(BooleanQuery::new_multiterms_query(vec![first])))
                        } else {
                            Err(IndexError::InvalidQuery("NOT requires subquery".into()))
                        }
                    }
                }
            }
            
            TimeRange { start, end } => {
                let start = tantivy::DateTime::from_utc(*start);
                let end = tantivy::DateTime::from_utc(*end);
                let range_query = RangeQuery::new(
                    self.fields.timestamp,
                    start..end,
                );
                Ok(Box::new(range_query))
            }
            
            Faceted { query, facets } => {
                // Convert base query and add facet filters
                let mut converted = self.convert_query(query)?;
                
                for facet in facets {
                    let field = self.search_field_to_tantivy(facet.field);
                    let term = Term::from_field_text(field, &facet.value);
                    let filter = TermQuery::new(term, IndexRecordOption::Basic);
                    // Combine with AND
                    converted = Box::new(BooleanQuery::new_multiterms_query(vec![
                        converted, Box::new(filter)
                    ]));
                }
                
                Ok(converted)
            }
        }
    }
    
    /// Map SearchField to Tantivy Field
    fn search_field_to_tantivy(&self, field: SearchField) -> Field {
        match field {
            SearchField::MessageType => self.fields.message_type,
            SearchField::Source => self.fields.source,
            SearchField::Facility => self.fields.facility,
            SearchField::PatientId => self.fields.patient_id,
            SearchField::PatientName => self.fields.patient_name,
            SearchField::AccountNumber => self.fields.account_number,
            SearchField::Content => self.fields.content,
        }
    }
    
    /// Convert Tantivy Document to MessageEntry
    fn doc_to_entry(&self, doc: &TantivyDocument) -> Result<MessageEntry, IndexError> {
        // Extract fields using schema
        let get_text = |field: Field| -> Option<String> {
            doc.get_first(field)
                .and_then(|v| v.as_text())
                .map(|s| s.to_string())
        };
        
        let id = get_text(self.fields.id)
            .ok_or_else(|| IndexError::CorruptData("Missing id field".into()))?;
        
        let fields_json = get_text(self.fields.searchable_fields_json)
            .ok_or_else(|| IndexError::CorruptData("Missing searchable_fields_json".into()))?;
        let searchable_fields: SearchableFields = serde_json::from_str(&fields_json)?;
        
        Ok(MessageEntry {
            id,
            message_type: get_text(self.fields.message_type).unwrap_or_default(),
            timestamp: doc.get_first(self.fields.timestamp)
                .and_then(|v| v.as_datetime())
                .map(|dt| dt.into_utc())
                .unwrap_or_else(|| Utc::now()),
            source: get_text(self.fields.source).unwrap_or_default(),
            facility: get_text(self.fields.facility).unwrap_or_default(),
            content: get_text(self.fields.content).unwrap_or_default(),
            searchable_fields,
        })
    }
}
```

---

## 6. Cargo.toml Configuration

```toml
[package]
name = "hl7v2-index"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
description = "HL7 v2 message indexing and search"
license.workspace = true
repository.workspace = true
homepage.workspace = true
keywords = ["hl7", "healthcare", "search", "index", "tantivy"]
categories = ["database-implementations", "text-processing"]

[features]
default = ["memory-backend"]

# Backends
memory-backend = []  # Always available, in-memory hashmap
sqlite-backend = ["dep:rusqlite", "dep:regex"]  # SQLite FTS5
tantivy-backend = ["dep:tantivy", "dep:chrono"]  # Tantivy search engine

# Convenience feature for all backends
all-backends = ["sqlite-backend", "tantivy-backend"]

[dependencies]
# Core dependencies
hl7v2-model = { version = "1.2.0", path = "../hl7v2-model" }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }

# Optional: Tantivy backend
tantivy = { version = "0.21", optional = true }

# Optional: SQLite backend
rusqlite = { version = "0.29", features = ["bundled", "fts5"], optional = true }
regex = { version = "1.10", optional = true }

# Optional: Time handling for Tantivy
chrono = { workspace = true, optional = true }

[dev-dependencies]
tempfile = "3.10"
criterion = "0.5"
proptest = "1.6"
cucumber = "0.22"
tokio = { workspace = true, features = ["macros", "rt-multi-thread"] }

[[bench]]
name = "backend_bench"
harness = false

[[test]]
name = "bdd_tests"
harness = false
```

---

## 7. BDD Scenarios

### Feature: Message Indexing

```gherkin
Feature: Message Indexing
  As a healthcare integration developer
  I want to index HL7 messages for fast retrieval
  So that I can search and retrieve messages efficiently

  Background:
    Given a Tantivy backend is initialized with default configuration
    And the index is empty

  Scenario: Index a single ADT message
    Given a valid ADT^A01 message:
      """
      MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20231119120000||ADT^A01|MSG001|P|2.5
      PID|1||MRN12345||Doe^John||19800101|M
      """
    When I index the message with ID "MSG001"
    Then the index should contain 1 document
    And I should be able to retrieve "MSG001" by ID

  Scenario: Index multiple messages with batching
    Given 1000 unique HL7 messages of type ORU^R01
    When I index all messages in a batch
    Then the index should contain 1000 documents
    And the flush operation should complete successfully

  Scenario: Prevent duplicate message IDs
    Given a message with ID "MSG001" is already indexed
    When I attempt to index another message with ID "MSG001"
    Then the operation should succeed
    And the index should contain 2 documents with ID "MSG001"
    # Note: Tantivy allows duplicates; deduplication is consumer responsibility
```

### Feature: Full-Text Search

```gherkin
Feature: Full-Text Search
  As a healthcare data analyst
  I want to search HL7 messages by content
  So that I can find relevant clinical data

  Background:
    Given the index contains these messages:
      | id      | message_type | patient_name | content_snippet          |
      | MSG001  | ADT^A01      | John Doe     | Patient admitted         |
      | MSG002  | ORU^R01      | Jane Smith   | Glucose level 120        |
      | MSG003  | ADT^A08      | John Doe     | Patient info updated     |
      | MSG004  | ORU^R01      | Bob Johnson  | Cholesterol 180          |

  Scenario: Search by patient name
    When I search for "John Doe"
    Then I should get 2 results
    And the results should contain IDs "MSG001" and "MSG003"

  Scenario: Search by message content
    When I search for "Glucose"
    Then I should get 1 result
    And the first result should have ID "MSG002"

  Scenario: Search with no results
    When I search for "xyznonexistent"
    Then I should get 0 results
    And the total count should be 0

  Scenario: Boolean AND search
    When I search for "John" AND "admitted"
    Then I should get 1 result
    And the first result should have ID "MSG001"

  Scenario: Faceted search by message type
    Given I search for all messages
    When I filter by message type "ORU^R01"
    Then I should get 2 results
    And all results should have message type "ORU^R01"
```

### Feature: Time-Range Queries

```gherkin
Feature: Time-Range Queries
  As a system administrator
  I want to query messages by time range
  So that I can audit recent activity

  Background:
    Given the index contains messages with timestamps:
      | id      | timestamp           |
      | MSG001  | 2023-11-01T10:00:00Z |
      | MSG002  | 2023-11-15T14:30:00Z |
      | MSG003  | 2023-11-30T08:00:00Z |

  Scenario: Search within date range
    When I query messages from "2023-11-10T00:00:00Z" to "2023-11-20T23:59:59Z"
    Then I should get 1 result
    And the result should have ID "MSG002"

  Scenario: Search with no time range matches
    When I query messages from "2023-12-01T00:00:00Z" to "2023-12-31T23:59:59Z"
    Then I should get 0 results
```

### Feature: Backend Performance

```gherkin
Feature: Backend Performance
  As a performance engineer
  I want to compare Tantivy and SQLite backends
  So that I can select the appropriate backend for my workload

  Background:
    Given a corpus of 10000 HL7 messages
    And both Tantivy and SQLite backends are configured

  Scenario: Index throughput comparison
    When I index the corpus with Tantivy backend
    And I index the corpus with SQLite backend
    Then Tantivy should index at least 5000 messages per second
    And SQLite should index at least 1000 messages per second

  Scenario: Search latency comparison
    Given the corpus is indexed in both backends
    When I execute 100 full-text searches with Tantivy
    And I execute 100 full-text searches with SQLite
    Then Tantivy average latency should be under 10ms
    And SQLite average latency should be under 50ms

  Scenario: Scalability test
    Given I start with 1000 messages indexed
    When I add 1000 messages 10 times
    Then each addition should complete in under 5 seconds
    And search latency should remain under 20ms
```

---

## 8. Error Types

```rust
/// Errors that can occur during indexing operations
#[derive(Debug, thiserror::Error)]
pub enum IndexError {
    #[error("Backend not available: {0}")]
    BackendNotAvailable(String),
    
    #[error("Backend error: {0}")]
    BackendError(String),
    
    #[error("Invalid query: {0}")]
    InvalidQuery(String),
    
    #[error("Document not found: {0}")]
    NotFound(String),
    
    #[error("Corrupt data: {0}")]
    CorruptData(String),
    
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Lock poisoned: {0}")]
    LockError(String),
    
    /// Tantivy-specific errors
    #[cfg(feature = "tantivy-backend")]
    #[error("Tantivy error: {0}")]
    Tantivy(#[from] tantivy::TantivyError),
    
    /// SQLite-specific errors
    #[cfg(feature = "sqlite-backend")]
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}
```

---

## 9. Dependencies

| Crate | Version | Purpose | Feature Gate |
|-------|---------|---------|--------------|
| `tantivy` | 0.21 | Full-text search engine | `tantivy-backend` |
| `rusqlite` | 0.29 | SQLite FTS5 backend | `sqlite-backend` |
| `serde` | 1.0 | Serialization | Always |
| `serde_json` | 1.0 | JSON handling | Always |
| `thiserror` | 2.0 | Error definitions | Always |
| `chrono` | 0.4 | Date/time handling | `tantivy-backend` |
| `regex` | 1.10 | Pattern matching | `sqlite-backend` |

---

## 10. Implementation Phases

### Phase 1: Foundation (Week 1)
- [ ] Create `hl7v2-index` crate structure
- [ ] Define `IndexBackend` trait
- [ ] Implement `MemoryBackend` (reference)
- [ ] Unit tests for trait interface

### Phase 2: Tantivy Core (Week 2)
- [ ] Implement `TantivyBackend::open_or_create`
- [ ] Schema definition and field mapping
- [ ] `add_message` implementation
- [ ] `get` by ID implementation

### Phase 3: Search & Query (Week 3)
- [ ] Query conversion layer
- [ ] `search` implementation
- [ ] Time-range queries
- [ ] Faceted search support

### Phase 4: Polish & Performance (Week 4)
- [ ] `remove` and `flush` operations
- [ ] Highlighting support
- [ ] Performance benchmarks
- [ ] BDD scenario implementation

---

## 11. Performance Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| Index throughput | ≥5,000 msg/sec | 10k ADT messages, NVMe SSD |
| Search latency (p50) | <10ms | Single term query, warm cache |
| Search latency (p99) | <50ms | Complex boolean query |
| Index size | <2x raw data | 10k messages, with stored content |
| Memory usage | <200MB | Indexing 10k messages |

---

## 12. Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Tantivy schema changes break existing indexes | Medium | High | Version schema, migration path |
| Concurrent write contention | Medium | Medium | Writer mutex, batching recommendation |
| Large index directory size | Low | Medium | Compression, segment merging |
| Query parsing errors for complex HL7 content | Medium | Low | Escape special chars, fallback to raw search |
| Dependency version conflicts | Low | High | Pin versions, test with workspace deps |

---

## 13. Open Questions

1. **Segment merging strategy**: Should we expose Tantivy's merge policy configuration?
2. **Index sharding**: Is horizontal scaling needed for single indexes >10GB?
3. **Real-time requirements**: Do we need near-real-time (<1s) commit guarantees?
4. **PHI handling**: Should search indexes support field-level encryption?

---

## 14. References

- Parent Issue: [EFF-450](/EFF/issues/EFF-450)
- Tantivy Documentation: https://docs.rs/tantivy/0.21.0/tantivy/
- HL7 v2 Standard: http://www.hl7.org/implement/standards/product_brief.cfm?product_id=185
- SQLite FTS5: https://www.sqlite.org/fts5.html

---

*Spec created by Spec Designer ([f770e387-ccf5-441f-8b09-45986023f6df](/EFF/agents/f770e387-ccf5-441f-8b09-45986023f6df))*  
*Next Owner: Spec Verifier*
