# Design Notes: hl7v2-guard

## Architecture Decisions

### 1. Detector Pattern

Each anomaly detector implements a common trait:

```rust
pub trait AnomalyDetector: Send + Sync {
    /// Unique identifier for this detector
    fn rule_id(&self) -> RuleId;
    
    /// Analyze a message and return any detected anomalies
    fn detect(&self, message: &MessageContext) -> Vec<Anomaly>;
    
    /// Update internal state/baseline with this message
    fn learn(&mut self, message: &MessageContext);
    
    /// Get current baseline statistics (for monitoring/debugging)
    fn baseline_stats(&self) -> DetectorStats;
}
```

This allows:
- Composable detectors (combine multiple strategies)
- Runtime configuration (enable/disable detectors)
- Testability (mock detectors for isolated testing)

### 2. Message Context

Instead of passing raw HL7 messages, we use a `MessageContext` that pre-computes commonly used features:

```rust
pub struct MessageContext {
    pub raw_message: Hl7v2Message,
    pub message_hash: String,          // SHA-256 of canonical form
    pub control_id: String,              // MSH.10
    pub sender: String,                // MSH.3 (sending application)
    pub message_type: MessageType,     // MSH.9 (ADT^A01, ORU^R01, etc.)
    pub timestamp: DateTime<Utc>,      // MSH.7 (message timestamp)
    pub field_values: HashMap<String, String>, // Cached field accessors
}
```

Pre-computing these values avoids redundant parsing in multiple detectors.

### 3. Sliding Window Implementation

For time-series detectors, we use a circular buffer with fixed capacity:

```rust
pub struct SlidingWindow<T> {
    buffer: VecDeque<T>,
    capacity: usize,
    window_duration: Duration,
}

impl<T: Timestamped> SlidingWindow<T> {
    pub fn add(&mut self, item: T) {
        // Remove items outside time window
        let cutoff = Utc::now() - self.window_duration;
        while self.buffer.front().map(|i| i.timestamp() < cutoff).unwrap_or(false) {
            self.buffer.pop_front();
        }
        
        // Add new item
        if self.buffer.len() >= self.capacity {
            self.buffer.pop_front();
        }
        self.buffer.push_back(item);
    }
    
    pub fn stats(&self) -> WindowStats {
        // Compute mean, stddev, min, max
    }
}
```

This provides:
- O(1) amortized insertion
- O(n) statistics computation (where n = window size, bounded)
- Automatic expiration of old data

### 4. Bloom Filter for Duplicates

For duplicate detection at scale, we combine a Bloom filter with LRU cache:

```rust
pub struct DuplicateDetector {
    bloom: Bloom<[u8]>,          // Fast negative check (definitely not seen)
    cache: LruCache<String, MessageMeta>, // Exact check (might be seen)
    window: Duration,
}

impl DuplicateDetector {
    pub fn check(&mut self, msg: &MessageContext) -> Option<DuplicateType> {
        let hash = msg.message_hash.clone();
        
        // Bloom filter: if not in bloom, definitely not duplicate
        if !self.bloom.check(hash.as_bytes()) {
            self.bloom.set(hash.as_bytes());
            self.cache.put(hash, msg.meta());
            return None;
        }
        
        // Bloom says maybe - check cache for exact match
        if let Some(meta) = self.cache.get(&hash) {
            if meta.control_id == msg.control_id {
                return Some(DuplicateType::Exact);
            } else if meta.control_id == msg.control_id {
                return Some(DuplicateType::Replay);
            }
        }
        
        None
    }
}
```

Bloom filter provides:
- Memory-efficient probabilistic tracking (no false negatives)
- ~1% false positive rate with proper sizing
- 10x less memory than exact set

### 5. Sender Profile Storage

Sender profiles use an LRU cache with time-to-live:

```rust
pub struct SenderProfileCache {
    profiles: LruCache<String, SenderProfile>,
    ttl: Duration,
}

pub struct SenderProfile {
    sender_id: String,
    message_type_counts: HashMap<String, u64>,
    hourly_distribution: [u64; 24],  // Messages per hour
    volume_stats: RunningStats,     // Online mean/stddev calculation
    last_seen: DateTime<Utc>,
}

impl SenderProfile {
    pub fn update(&mut self, msg: &MessageContext) {
        // Update message type counts
        *self.message_type_counts.entry(msg.message_type.to_string()).or_insert(0) += 1;
        
        // Update hourly distribution
        let hour = msg.timestamp.hour() as usize;
        self.hourly_distribution[hour] += 1;
        
        // Update running statistics
        self.volume_stats.add(1.0); // One message
        
        self.last_seen = msg.timestamp;
    }
    
    pub fn is_business_hours(&self) -> bool {
        // Check if typical activity is during 08:00-18:00
        let business_hours: u64 = self.hourly_distribution[8..18].iter().sum();
        let total: u64 = self.hourly_distribution.iter().sum();
        (business_hours as f64) / (total as f64) > 0.7
    }
}
```

### 6. Async Processing Strategy

The GuardEngine uses a channel-based architecture for thread safety:

```rust
pub struct GuardEngine {
    config: Arc<RwLock<GuardConfig>>,
    detectors: Arc<Vec<Box<dyn AnomalyDetector>>>,
    response_tx: mpsc::Sender<ResponseCommand>,
}

enum ResponseCommand {
    Alert(Anomaly),
    Quarantine(MessageContext, Duration),
    Block(MessageContext),
}

impl GuardEngine {
    pub async fn analyze(&self, message: &MessageContext) -> Vec<Anomaly> {
        let mut anomalies = Vec::new();
        
        // Run all detectors (parallel via rayon or sequential based on config)
        for detector in self.detectors.iter() {
            if let Some(detected) = detector.detect(message) {
                anomalies.extend(detected);
            }
        }
        
        // Trigger responses async
        for anomaly in &anomalies {
            if let Some(action) = self.config.read().await.response_for(&anomaly.severity) {
                let _ = self.response_tx.send(ResponseCommand::from((action, anomaly.clone()))).await;
            }
        }
        
        anomalies
    }
}
```

### 7. Configuration Schema

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardConfig {
    // Volume detection
    pub volume: VolumeConfig,
    
    // Duplicate detection
    pub duplicate: DuplicateConfig,
    
    // Outlier detection
    pub outlier: OutlierConfig,
    
    // Sender profiling
    pub sender_profile: SenderProfileConfig,
    
    // Response rules
    pub responses: Vec<ResponseRule>,
    
    // General settings
    pub baseline_scope: BaselineScope,
    pub learning_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeConfig {
    pub enabled: bool,
    pub window_size: Duration,      // Default: 1 hour
    pub z_threshold: f64,             // Default: 3.0
    pub min_samples: usize,           // Default: 30
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateConfig {
    pub enabled: bool,
    pub window: Duration,             // Default: 5 minutes
    pub bloom_capacity: usize,          // Default: 100_000
    pub lru_capacity: usize,          // Default: 10_000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlierConfig {
    pub enabled: bool,
    pub z_threshold: f64,             // Default: 3.0
    pub min_samples: usize,           // Default: 30
    pub fields: Vec<String>,          // Fields to monitor
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseRule {
    pub condition: ResponseCondition,
    pub action: ResponseAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ResponseCondition {
    Severity { min: Severity },
    AnomalyType { types: Vec<String> },
    ScoreThreshold { min: f64 },
    Custom { expression: String }, // For future extensibility
}
```

### 8. Error Handling Strategy

```rust
#[derive(Error, Debug)]
pub enum GuardError {
    #[error("configuration error: {0}")]
    Config(String),
    
    #[error("detection error in {detector}: {message}")]
    Detection { detector: RuleId, message: String },
    
    #[error("baseline persistence failed: {0}")]
    Persistence(String),
    
    #[error("audit integration failed: {0}")]
    Audit(String),
}

// For recoverable errors, log and continue
type GuardResult<T> = Result<T, GuardError>;

impl GuardEngine {
    pub async fn analyze_safe(&self, message: &MessageContext) -> (Vec<Anomaly>, Vec<GuardError>) {
        let mut anomalies = Vec::new();
        let mut errors = Vec::new();
        
        for detector in self.detectors.iter() {
            match detector.detect(message) {
                Ok(detected) => anomalies.extend(detected),
                Err(e) => {
                    tracing::error!("Detector {} failed: {}", detector.rule_id(), e);
                    errors.push(e);
                    // Continue with other detectors
                }
            }
        }
        
        (anomalies, errors)
    }
}
```

### 9. Testing Strategy

**Unit Tests**:
- Each detector in isolation with mock data
- Statistical calculations (Z-score, etc.)
- Configuration parsing

**Integration Tests**:
- Full message flow through GuardEngine
- Audit integration verification
- Baseline persistence round-trip

**Property-Based Tests**:
- Anomaly scores always in [0, 1]
- Confidence decreases with fewer samples
- Z-score calculation correctness

**BDD Tests**:
- Cucumber or similar for scenario execution
- Gherkin files from bdd-scenarios.md

### 10. Performance Considerations

| Operation | Target Latency | Strategy |
|-----------|----------------|----------|
| Single detector | < 0.1ms | Pre-computed features, no I/O |
| Full analysis | < 1ms | Parallel detector execution |
| Baseline update | < 0.5ms | In-memory only, async persistence |
| Duplicate check | < 0.05ms | Bloom filter + LRU |

**Memory Budget**:
- Default configuration: < 100MB
- Per-sender profile: ~1KB
- Sliding window (1k entries): ~16KB
- Bloom filter (100k entries): ~120KB

---

*Document Version: 1.0*  
*For: [EFF-840](/EFF/issues/EFF-840)*
