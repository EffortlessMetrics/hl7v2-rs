# Spec: hl7v2-guard ML Anomaly Detection (EFF-840)

**Issue:** [EFF-840](/EFF/issues/EFF-840)  
**Status:** Spec Complete - Ready for Verification  
**Branch:** `EFF-840`  
**PR:** Not yet created (spec exists on branch only)

---

## Problem Statement

Healthcare messaging systems face security threats (PHI exfiltration, replay attacks, injection) and operational issues (duplicate orders, misdirected results). The hl7v2-rs workspace currently has:
- **hl7v2-validation**: Structure-only field validation
- **hl7v2-audit**: Event logging without pattern detection  
- **hl7v2-analytics**: Basic statistics without anomaly scoring

**Gap:** Zero intelligent ML-based anomaly detection exists in the entire workspace (confirmed: 0 issues for "anomaly", "ml", "classification").

---

## Solution: hl7v2-guard Crate

A new crate providing statistical ML-based anomaly detection and automated threat response for HL7 messages.

### Guiding Principles

1. **Deterministic & Explainable:** Statistical methods only (no neural networks) for healthcare compliance
2. **Local Processing:** No external ML APIs (PHI never leaves the system)
3. **Safety First:** Default alert-only, never block by default (patient safety)
4. **Bounded Resources:** Memory and CPU limits for production safety

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Incoming HL7 Message                      │
└───────────────────────────────┬─────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────┐
│                     Feature Extraction                         │
│  • Message hash (SHA-256)                                     │
│  • Sender ID, timestamp, message type                       │
│  • Field value extraction                                     │
└───────────────────────────────┬─────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────┐
│                    Anomaly Detection                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │ Volume       │  │ Field        │  │ Timing       │      │
│  │ Analysis     │  │ Outliers     │  │ Analysis     │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │ Duplicate    │  │ Replay       │  │ Sender       │      │
│  │ Detection    │  │ Detection    │  │ Behavior     │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└───────────────────────────────┬─────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────┐
│                    Decision & Action                         │
│  Score → Severity → Action (alert/quarantine/block/throttle)│
└─────────────────────────────────────────────────────────────┘
```

---

## Phased Implementation

### Phase 1: Statistical Anomaly Detection

**Capabilities:**
- **Volume spike detection:** Z-score based message volume analysis per sender
- **Field outlier detection:** Statistical outliers in numeric and coded fields
- **Baseline learning:** 7-day sliding window for normal pattern establishment
- **Anomaly scoring:** Z-score with configurable threshold (default: 3.0)

**Algorithms:**
```rust
// Volume spike detection
pub fn detect_volume_spike(
    current_count: f64,
    baseline_mean: f64,
    baseline_stddev: f64,
    threshold: f64,
) -> bool {
    let z_score = (current_count - baseline_mean) / baseline_stddev;
    z_score.abs() > threshold
}

// Welford's online algorithm for baseline stats
pub struct StreamingStats {
    count: usize,
    mean: f64,
    m2: f64, // sum of squared differences
}
```

---

### Phase 2: Pattern Classification

**Capabilities:**
- **Duplicate detection:** SHA-256 hash comparison within 5-minute window
- **Replay detection:** Control ID (MSH-10) tracking with content hash
- **Sender behavior profiles:** Expected message types per sender
- **Timing analysis:** Off-hours access detection
- **Distribution drift:** Message type mix changes

**Algorithms:**
```rust
// Duplicate detection
pub fn is_duplicate(
    message_hash: &str,
    recent_hashes: &HashMap<String, DateTime<Utc>>,
    window: Duration,
) -> bool {
    if let Some(timestamp) = recent_hashes.get(message_hash) {
        Utc::now() - timestamp < window
    } else {
        false
    }
}

// Replay detection
pub fn is_replay(
    control_id: &str,
    content_hash: &str,
    known_ids: &HashMap<String, String>, // control_id -> content_hash
) -> bool {
    if let Some(known_hash) = known_ids.get(control_id) {
        known_hash != content_hash // Same ID, different content = replay
    } else {
        false
    }
}
```

---

### Phase 3: Automated Response

**Actions:**
| Action | Behavior | Use Case |
|--------|----------|----------|
| `alert` | Send notification, continue processing | Default safe option |
| `quarantine` | Hold for review, return 202 Accepted | Suspicious messages |
| `block` | Reject with 403 Forbidden | Confirmed threats |
| `throttle` | Rate limit sender, return 429 | Anomalous senders |

**Alert Channels:**
- Webhook: POST to configured URL
- Email: Send to configured recipients
- Slack: Post to configured channel
- Audit: Log to hl7v2-audit

---

## API Surface

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
    pub max_memory_mb: usize,
}

/// Anomaly detection result
pub struct AnomalyResult {
    pub message_id: String,
    pub timestamp: DateTime<Utc>,
    pub score: f64,           // 0.0-1.0 combined score
    pub severity: Severity,   // Info/Warning/Critical
    pub reasons: Vec<String>, // Explainable reasons
    pub action_taken: GuardAction,
}

pub enum Severity {
    Info,     // Z-score 1.0-2.0
    Warning,  // Z-score 2.0-3.0  
    Critical, // Z-score > 3.0
}

pub enum GuardAction {
    Alert,
    Quarantine,
    Block,
    Throttle,
}
```

### Main Interface

```rust
impl Guard {
    pub fn new(config: GuardConfig) -> Self;
    
    /// Analyze a message for anomalies
    pub fn analyze(&self, message: &Hl7Message) -> AnomalyResult;
    
    /// Update baseline with new observation
    pub fn update_baseline(&self, observation: Observation);
    
    /// Get current baseline stats for sender
    pub fn get_sender_stats(&self, sender_id: &str) -> Option<SenderStats>;
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
| `HL7V2_GUARD_DUPLICATE_WINDOW_MIN` | `5` | Duplicate detection window |
| `HL7V2_GUARD_BUSINESS_HOURS_START` | `08:00` | Business hours start |
| `HL7V2_GUARD_BUSINESS_HOURS_END` | `18:00` | Business hours end |
| `HL7V2_GUARD_WEBHOOK_URL` | - | Alert webhook URL |
| `HL7V2_GUARD_ACTION` | `alert` | Default action |
| `HL7V2_GUARD_MAX_MEMORY_MB` | `100` | Max baseline memory |

### Code Example

```rust
use hl7v2_guard::{Guard, GuardConfig, GuardAction};

let config = GuardConfig::builder()
    .enabled(true)
    .baseline_days(7)
    .z_threshold(3.0)
    .duplicate_window(Duration::minutes(5))
    .action(GuardAction::Alert)
    .webhook_url("https://alerts.example.com/webhook")
    .build();

let guard = Guard::new(config);

// Analyze a message
let result = guard.analyze(&message);

if result.severity == Severity::Critical {
    println!("Critical anomaly: {:?}", result.reasons);
}
```

---

## Integration Points

### With hl7v2-server (Middleware)

```rust
async fn guard_middleware(
    message: Hl7Message,
    guard: Arc<Guard>,
) -> Result<Response, GuardError> {
    let result = guard.analyze(&message);
    
    match result.action_taken {
        GuardAction::Alert => {
            send_alert(&result).await;
            Ok(Response::Continue)
        }
        GuardAction::Quarantine => {
            quarantine_message(&message).await;
            Ok(Response::Accepted) // 202
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

### With hl7v2-audit

```rust
// All anomalies logged for compliance
audit::log_event(AuditEvent::AnomalyDetected {
    message_id: result.message_id,
    severity: result.severity,
    reasons: result.reasons,
    action: result.action_taken,
});
```

---

## Performance Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| Anomaly scoring latency | <10ms p99 | Per-message |
| Throughput | 1000 msg/sec | Sustained |
| Memory usage | <100MB | Baseline storage |
| CPU overhead | <5% | Relative to no guard |

---

## Test Coverage

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

### BDD Scenarios (20 scenarios)
See `.swarm/EFF-840/scenarios.md` for complete list covering:
1. Volume spike detection
2. Duplicate medication orders
3. Replay attack detection
4. Off-hours access
5. Sender behavior changes
6. Action enforcement (alert/quarantine/block/throttle)
7. Severity levels
8. Webhook delivery
9. Guard enable/disable
10. Multi-factor scoring

---

## Design Decisions

### 1. Statistical ML Only (No Neural Networks)
**Why:** Healthcare requires explainable decisions for FDA/HIPAA compliance. Black box models can't pass audits.

### 2. Local-Only Processing (No External APIs)
**Why:** PHI must never leave the system. External APIs create HIPAA compliance risk.

### 3. Default Alert-Only (Never Block by Default)
**Why:** Patient safety is paramount. False positives cannot block care. Gradual adoption: alert → review → block.

### 4. 7-Day Sliding Window Baseline
**Why:** Captures weekly patterns (weekday vs weekend). Healthcare changes slowly. No retraining required.

---

## Options Considered

| Option | Decision | Rationale |
|--------|----------|-----------|
| Embedded statistical ML | **SELECTED** | Explainable, deterministic, compliant |
| External ML API (AWS/Azure) | REJECTED | PHI leakage risk, latency, cost |
| SIEM integration only | DEFERRED | Complementary, not replacement |
| Neural networks | REJECTED | Black box, not explainable |

---

## Dependencies

| Crate | Purpose | Integration |
|-------|---------|-------------|
| hl7v2-audit | Audit trail | Anomaly events logged |
| hl7v2-analytics | Baseline stats | Data source |
| hl7v2-terminology | Code patterns | Optional |

---

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| False positives block care | Medium | Critical | Default alert-only, never block |
| Baseline poisoning | Low | High | Sliding window, manual reset |
| Performance overhead | Low | Medium | <10ms target, Welford's algorithm |
| PHI in ML features | Medium | High | Hash-based, no raw values |

---

## Out of Scope

1. **Deep learning models** (transformers, neural networks) - Not explainable
2. **External ML APIs** - PHI leakage risk
3. **Automatic model retraining** - Requires labeled data
4. **Cross-tenant learning** - Privacy violation
5. **Real-time model serving** - Overkill for statistical methods

---

## Future Enhancements

1. **ONNX Runtime Integration** - For pre-trained models (Phase 4)
2. **Distributed Baselines** - Shared learning across instances
3. **Custom Rule Engine** - User-defined detection rules
4. **Automatic Threshold Tuning** - Self-adjusting Z-scores

---

## Verification Checklist

- [ ] New crate `hl7v2-guard` created
- [ ] Volume spike detection (Z-score > 3.0)
- [ ] Field outlier detection
- [ ] 7-day baseline learning
- [ ] Duplicate detection (5-min window)
- [ ] Replay detection (control ID tracking)
- [ ] Off-hours access detection
- [ ] Sender behavior profiles
- [ ] Alert action (webhook delivery)
- [ ] Quarantine action (queue storage)
- [ ] Block action (403 Forbidden)
- [ ] Throttle action (429 Too Many Requests)
- [ ] Default alert-only (never block by default)
- [ ] Performance <10ms p99
- [ ] All 20 BDD scenarios pass
- [ ] Integration with hl7v2-audit

---

## References

- Requirements: `.swarm/EFF-840/requirements.md`
- Design: `.swarm/EFF-840/design.md`
- BDD Scenarios: `.swarm/EFF-840/scenarios.md`
- Working Notes: `.swarm/EFF-840/notes.md`
