# Design: hl7v2-guard ML Anomaly Detection (EFF-840)

**Issue:** [EFF-840](/EFF/issues/EFF-840)  
**Status:** Design Complete  
**Last Updated:** 2026-04-04

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                      Incoming HL7 Message                             │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     hl7v2-guard Pipeline                            │
│                                                                     │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────┐   │
│  │  Message Hash   │  │  Feature        │  │  Anomaly Scoring    │   │
│  │  (SHA-256)      │→ │  Extraction     │→ │  (Z-score, stats)   │   │
│  └─────────────────┘  └─────────────────┘  └─────────────────────┘   │
│         │                      │                      │             │
│         ▼                      ▼                      ▼             │
│  ┌─────────────────────────────────────────────────────────────────┐ │
│  │                    Pattern Detection Engine                       │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │ │
│  │  │ Duplicate    │  │ Replay       │  │ Timing       │          │ │
│  │  │ Detection    │  │ Detection    │  │ Analysis     │          │ │
│  │  └──────────────┘  └──────────────┘  └──────────────┘          │ │
│  │  ┌──────────────┐  ┌──────────────┐                           │ │
│  │  │ Sender       │  │ Distribution │                           │ │
│  │  │ Behavior     │  │ Drift        │                           │ │
│  │  └──────────────┘  └──────────────┘                           │ │
│  └─────────────────────────────────────────────────────────────────┘ │
│                               │                                       │
│                               ▼                                       │
│  ┌─────────────────────────────────────────────────────────────────┐ │
│  │                    Decision Engine                                │ │
│  │  Score → Severity → Action (alert/quarantine/block/throttle)      │ │
│  └─────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    Response Actions                                 │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐           │
│  │ Alert    │  │Quarantine│  │  Block   │  │ Throttle │           │
│  │(webhook) │  │ (queue)  │  │ (reject) │  │ (rate)   │           │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘           │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Key Design Decisions

### Decision 1: Statistical ML Only (No Neural Networks)

**Choice:** Use statistical anomaly detection (Z-score, IQR, moving averages) rather than deep learning.

**Rationale:**
- Healthcare requires deterministic, explainable decisions (FDA/ HIPAA)
- Neural networks are black boxes - can't explain why message flagged
- Statistical methods are sufficient for the patterns we need to detect
- No training data requirements
- Fast inference (<10ms)

**Implementation:**
```rust
pub fn calculate_z_score(value: f64, mean: f64, stddev: f64) -> f64 {
    (value - mean) / stddev
}

pub fn is_anomaly(z_score: f64, threshold: f64) -> bool {
    z_score.abs() > threshold
}
```

---

### Decision 2: Local-Only Processing (No External ML APIs)

**Choice:** All anomaly detection runs in-process, no external API calls.

**Rationale:**
- PHI must not leave the system (HIPAA compliance)
- External APIs introduce latency and availability risk
- Air-gapped deployments must work without internet

**Implementation:**
- Pure Rust implementation
- No HTTP clients for ML inference
- All models embedded or statistical

---

### Decision 3: Default Alert-Only (Never Block by Default)

**Choice:** Default action is `alert`, never `block` or `quarantine`.

**Rationale:**
- Patient safety is paramount - false positives can't block care
- Operators must explicitly opt-in to blocking actions
- Gradual adoption path: alert → review → configure block

**Implementation:**
```rust
impl Default for GuardConfig {
    fn default() -> Self {
        Self {
            action: GuardAction::Alert, // Never Block by default
            ...
        }
    }
}
```

---

### Decision 4: Sliding Window Baselines

**Choice:** Use 7-day sliding window for baseline learning, not fixed training period.

**Rationale:**
- Healthcare patterns change slowly (seasonal, weekly cycles)
- Sliding window adapts to gradual changes without retraining
- 7 days captures weekly patterns (weekday vs weekend)

**Implementation:**
```rust
pub struct Baseline {
    window: Duration, // 7 days
    data: VecDeque<TimestampedValue>,
}

impl Baseline {
    pub fn update(&mut self, value: f64) {
        self.data.push_back(TimestampedValue::now(value));
        // Remove values older than window
        self.data.retain(|v| v.age() < self.window);
    }
    
    pub fn mean(&self) -> f64 { /* calculate from data */ }
    pub fn stddev(&self) -> f64 { /* calculate from data */ }
}
```

---

### Decision 5: Multi-Factor Scoring

**Choice:** Anomaly score combines multiple factors, not single metric.

**Rationale:**
- Volume spike alone may be normal (batch job)
- Volume + off-hours + unusual sender = high confidence anomaly
- Multiple weak signals combine to strong detection

**Implementation:**
```rust
pub struct AnomalyScore {
    pub volume_score: f64,       // 0.0-1.0
    pub timing_score: f64,       // 0.0-1.0
    pub sender_score: f64,       // 0.0-1.0
    pub pattern_score: f64,      // 0.0-1.0
}

impl AnomalyScore {
    pub fn combined(&self) -> f64 {
        // Weighted average
        0.4 * self.volume_score +
        0.2 * self.timing_score +
        0.2 * self.sender_score +
        0.2 * self.pattern_score
    }
}
```

---

## Data Structures

### Core Types

```rust
/// Guard configuration
pub struct GuardConfig {
    pub enabled: bool,
    pub baseline_days: u32,
    pub z_threshold: f64,
    pub duplicate_window: Duration,
    pub action: GuardAction,
    pub webhook_url: Option<String>,
}

/// Anomaly detection result
pub struct AnomalyResult {
    pub message_id: String,
    pub timestamp: DateTime<Utc>,
    pub score: AnomalyScore,
    pub severity: Severity,
    pub reasons: Vec<String>,
    pub action_taken: GuardAction,
}

/// Severity levels
pub enum Severity {
    Info,     // Z-score 1.0-2.0
    Warning,  // Z-score 2.0-3.0
    Critical, // Z-score > 3.0
}

/// Available actions
pub enum GuardAction {
    Alert,      // Send notification
    Quarantine, // Hold for review
    Block,      // Reject message
    Throttle,   // Rate limit sender
}
```

### Baseline Storage

```rust
/// Per-sender baseline statistics
pub struct SenderBaseline {
    pub sender_id: String,
    pub volume_stats: TimeSeriesStats,  // messages per window
    pub field_stats: HashMap<String, FieldStats>,
    pub message_type_distribution: HashMap<String, f64>,
    pub last_updated: DateTime<Utc>,
}

/// Time-series statistics
pub struct TimeSeriesStats {
    pub values: VecDeque<TimestampedValue>,
    pub window: Duration,
}

/// Field-level statistics
pub struct FieldStats {
    pub field_path: String,
    pub numeric_stats: Option<NumericStats>,
    pub cardinality: usize,  // unique values count
    pub entropy: f64,          // distribution entropy
}

pub struct NumericStats {
    pub count: usize,
    pub mean: f64,
    pub m2: f64,  // for Welford's online algorithm
    pub min: f64,
    pub max: f64,
}
```

### Pattern Detection

```rust
/// Duplicate detection state
pub struct DuplicateDetector {
    pub window: Duration,
    pub hashes: HashMap<String, DateTime<Utc>>, // hash -> first seen
}

/// Replay detection state
pub struct ReplayDetector {
    pub control_ids: HashMap<String, ControlIdRecord>,
}

pub struct ControlIdRecord {
    pub first_seen: DateTime<Utc>,
    pub hash: String,  // message content hash
}

/// Sender behavior profile
pub struct SenderProfile {
    pub sender_id: String,
    pub expected_message_types: HashSet<String>,
    pub business_hours: BusinessHours,
    pub reputation_score: f64,  // 0.0-1.0
}
```

---

## Algorithms

### Volume Spike Detection

```
1. Track message count per sender per time window
2. Maintain 7-day baseline (mean, stddev)
3. Calculate Z-score: (current - mean) / stddev
4. If Z-score > threshold (default 3.0), flag anomaly
5. Update baseline with new value (sliding window)
```

### Duplicate Detection

```
1. Calculate SHA-256 hash of normalized message
2. Check if hash exists in recent window (default 5 min)
3. If found: calculate field-level diff for near-duplicate detection
4. Flag exact duplicate or near-duplicate based on threshold
5. Store hash with timestamp
6. Clean up expired hashes (older than window)
```

### Replay Detection

```
1. Extract MSH-10 (Message Control ID)
2. Check if control ID seen before from same sender
3. If seen: compare content hash
4. If hash differs: flag as replay attack
5. If hash same: flag as duplicate
6. Store control ID with content hash
```

### Sender Behavior Analysis

```
1. Learn expected message types per sender over 7 days
2. Flag messages of unexpected type
3. Track sender reputation (anomaly history)
4. Adjust thresholds based on reputation (trusted senders get higher thresholds)
```

---

## Integration Points

### With hl7v2-audit

```rust
// Log anomaly detection events
audit::log_event(AuditEvent::AnomalyDetected {
    message_id: result.message_id,
    severity: result.severity,
    reasons: result.reasons,
    action: result.action_taken,
});
```

### With hl7v2-analytics

```rust
// Use analytics for baseline data
let baseline_stats = analytics::get_sender_stats(sender_id, window);
guard.update_baseline(baseline_stats);
```

### With hl7v2-server

```rust
// Middleware integration
async fn guard_middleware(
    message: Hl7Message,
    guard: Arc<Guard>,
) -> Result<Response, GuardAction> {
    let result = guard.analyze(&message);
    
    match result.action_taken {
        GuardAction::Alert => {
            send_alert(&result).await;
            Ok(Response::Continue)
        }
        GuardAction::Quarantine => {
            quarantine_message(&message).await;
            Ok(Response::Accepted) // 202 Accepted
        }
        GuardAction::Block => {
            Err(GuardError::AnomalyBlocked(result.reasons))
        }
        GuardAction::Throttle => {
            throttle_sender(&message.sender()).await;
            Ok(Response::TooManyRequests) // 429
        }
    }
}
```

---

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `HL7V2_GUARD_ENABLED` | `true` | Master enable/disable |
| `HL7V2_GUARD_BASELINE_DAYS` | `7` | Baseline learning window |
| `HL7V2_GUARD_Z_THRESHOLD` | `3.0` | Anomaly Z-score threshold |
| `HL7V2_GUARD_DUPLICATE_WINDOW_MIN` | `5` | Duplicate detection window (minutes) |
| `HL7V2_GUARD_BUSINESS_HOURS_START` | `08:00` | Business hours start |
| `HL7V2_GUARD_BUSINESS_HOURS_END` | `18:00` | Business hours end |
| `HL7V2_GUARD_WEBHOOK_URL` | - | Alert webhook URL |
| `HL7V2_GUARD_ACTION` | `alert` | Default action |
| `HL7V2_GUARD_MAX_MEMORY_MB` | `100` | Max baseline memory |

### Code Configuration

```rust
let config = GuardConfig::builder()
    .enabled(true)
    .baseline_days(7)
    .z_threshold(3.0)
    .duplicate_window(Duration::minutes(5))
    .action(GuardAction::Alert)
    .webhook_url("https://alerts.example.com/webhook")
    .build();

let guard = Guard::new(config);
```

---

## Testing Strategy

### Unit Tests
- Z-score calculation accuracy
- Baseline mean/stddev calculation
- Hash collision resistance
- Sliding window eviction

### Integration Tests
- End-to-end message analysis
- Webhook alert delivery
- Quarantine queue operations
- Rate limiting enforcement

### BDD Scenarios
See `.swarm/EFF-840/scenarios.md` for 12 detailed scenarios covering:
- Volume spike detection
- Duplicate order detection
- Replay attack detection
- Off-hours access detection
- Sender behavior changes
- Action enforcement

---

## Performance Considerations

### Memory Bounds
- Baseline data: configurable max (default 100MB)
- Hash stores: LRU cache with TTL
- Automatic eviction when memory pressure detected

### CPU Efficiency
- Statistical calculations use Welford's online algorithm
- Hash computation uses hardware-accelerated SHA-256
- No blocking operations in hot path

### Throughput
- Target: 1000 messages/second
- Anomaly scoring: <10ms p99
- No external API calls in critical path

---

## Future Enhancements

1. **ONNX Runtime Integration** - For pre-trained models (Phase 4)
2. **Distributed Baselines** - Shared learning across instances
3. **Custom Rule Engine** - User-defined detection rules
4. **ML Model A/B Testing** - Compare detection algorithms
5. **Automatic Threshold Tuning** - Self-adjusting Z-scores
