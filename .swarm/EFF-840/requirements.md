# Requirements: hl7v2-guard ML Anomaly Detection (EFF-840)

**Issue:** [EFF-840](/EFF/issues/EFF-840)  
**Status:** Spec In Progress  
**Last Updated:** 2026-04-04

---

## Background

Healthcare messaging systems face security threats (PHI exfiltration, replay attacks, injection) and operational issues (duplicate orders, misdirected results, malformed messages). The hl7v2-rs workspace currently has:
- **hl7v2-validation**: Structure-only validation (field types)
- **hl7v2-audit**: Event logging without pattern detection
- **hl7v2-analytics**: Basic statistics without anomaly scoring

This crate fills the gap for intelligent ML-based anomaly detection and automated threat response.

---

## Functional Requirements

### FR-1: Statistical Anomaly Detection (Phase 1)

**FR-1.1:** Time-Series Analysis
- Track message volume per sender per time window (configurable: 1min, 5min, 15min, 1hr)
- Calculate moving average and standard deviation over 7-day baseline
- Detect volume spikes exceeding Z-score threshold (default: 3.0)

**FR-1.2:** Field Value Statistics
- Numeric fields: Track mean, stddev, min, max per field per sender
- Coded fields: Track cardinality (unique values) and distribution entropy
- Detect outlier values exceeding statistical thresholds

**FR-1.3:** Baseline Learning
- Establish normal patterns over 7-day rolling window
- Automatically update baselines (sliding window)
- Support manual baseline reset/configuration

**FR-1.4:** Anomaly Scoring
- Calculate Z-score for numeric anomalies
- Assign severity: INFO (1.0-2.0), WARN (2.0-3.0), CRITICAL (>3.0)
- Provide confidence score (0.0-1.0) based on data quality

---

### FR-2: Pattern Classification (Phase 2)

**FR-2.1:** Duplicate Detection
- Hash messages (SHA-256) within configurable time window (default: 5 minutes)
- Detect exact duplicates and near-duplicates (field-level diff)
- Alert on duplicate medication orders (patient safety critical)

**FR-2.2:** Replay Detection
- Track message control IDs (MSH-10) per sender
- Detect duplicate control IDs with different timestamps
- Flag replay attacks (same ID, different content)

**FR-2.3:** Sender Behavior Profiles
- Learn expected message types per sender IP/facility
- Detect unexpected message types (e.g., lab results from admission system)
- Track sender reputation score based on historical behavior

**FR-2.4:** Timing Pattern Analysis
- Detect off-hours access (configurable business hours)
- Flag messages outside normal operational windows
- Track weekend/holiday access patterns

**FR-2.5:** Message Type Distribution
- Monitor distribution of message types (ADT, ORM, ORU, etc.)
- Detect distribution drift indicating upstream system changes
- Alert on sudden changes in message mix

---

### FR-3: Automated Response (Phase 3)

**FR-3.1:** Configurable Actions
- `alert`: Send notification via webhook/email/Slack
- `quarantine`: Hold suspicious messages for review
- `block`: Reject anomalous messages
- `throttle`: Rate-limit anomalous senders

**FR-3.2:** Alert Channels
- Webhook: POST to configured URL with anomaly details
- Email: Send to configured recipients
- Slack: Post to configured channel
- Audit log: Write to hl7v2-audit integration

**FR-3.3:** Quarantine Queue
- Store suspicious messages with metadata
- Support manual review and release
- Auto-expire quarantined messages (configurable TTL)

**FR-3.4:** Rate Limiting
- Throttle senders exceeding anomaly thresholds
- Configurable limits: messages/minute, anomalies/hour
- Automatic cooldown period

---

### FR-4: Configuration and Integration

**FR-4.1:** Environment Variables
- `HL7V2_GUARD_ENABLED`: Enable/disable guard (default: true)
- `HL7V2_GUARD_BASELINE_DAYS`: Baseline learning window (default: 7)
- `HL7V2_GUARD_Z_THRESHOLD`: Anomaly Z-score threshold (default: 3.0)
- `HL7V2_GUARD_DUPLICATE_WINDOW_MIN`: Duplicate detection window (default: 5)
- `HL7V2_GUARD_WEBHOOK_URL`: Alert webhook endpoint
- `HL7V2_GUARD_ACTION`: Default action (alert/quarantine/block/throttle)

**FR-4.2:** Programmatic API
```rust
pub struct GuardConfig {
    pub enabled: bool,
    pub baseline_days: u32,
    pub z_threshold: f64,
    pub duplicate_window: Duration,
    pub action: GuardAction,
}

pub enum GuardAction {
    Alert,
    Quarantine,
    Block,
    Throttle,
}
```

**FR-4.3:** Integration Points
- Integrate with `hl7v2-audit` for anomaly audit trail
- Use `hl7v2-analytics` for baseline statistics
- Optional: Use `hl7v2-terminology` for code distribution patterns

---

## Non-Functional Requirements

### NFR-1: Performance
- **NFR-1.1:** Anomaly scoring SHALL complete in <10ms per message
- **NFR-1.2:** Memory usage SHALL be bounded (configurable max 100MB for baselines)
- **NFR-1.3:** Support minimum 1000 messages/second throughput

### NFR-2: Determinism and Explainability
- **NFR-2.1:** All anomaly decisions SHALL be explainable (log reasoning)
- **NFR-2.2:** Statistical calculations SHALL be deterministic (same input = same output)
- **NFR-2.3:** No black-box neural networks (regulatory requirement)

### NFR-3: Safety
- **NFR-3.1:** Default action SHALL be `alert` (never block by default)
- **NFR-3.2:** False positive rate SHALL be <1% for CRITICAL anomalies
- **NFR-3.3:** Patient safety critical messages (orders) SHALL have lower thresholds

### NFR-4: Compliance
- **NFR-4.1:** PHI SHALL NOT be sent to external ML services
- **NFR-4.2:** All anomaly detection SHALL be local/in-process
- **NFR-4.3:** Audit trail SHALL be maintained for all decisions

---

## Verification Criteria

| Req ID | Test Method | Success Criteria |
|--------|-------------|------------------|
| FR-1.1 | Unit test | Volume spike detected when 10x baseline |
| FR-1.2 | Unit test | Outlier detected when Z-score > 3.0 |
| FR-1.3 | Integration test | Baseline updates after 7 days |
| FR-1.4 | Unit test | Severity correctly assigned by Z-score |
| FR-2.1 | Integration test | Duplicate detected within 5min window |
| FR-2.2 | Unit test | Replay detected with same control ID |
| FR-2.3 | Unit test | Unexpected message type flagged |
| FR-2.4 | Unit test | Off-hours access detected |
| FR-2.5 | Unit test | Distribution drift detected |
| FR-3.1 | Integration test | Alert sent to webhook |
| FR-3.2 | Integration test | Quarantine stores message |
| NFR-1.1 | Performance test | Scoring <10ms p99 |
| NFR-3.1 | Integration test | Default action is alert |

---

## Out of Scope

1. **Deep learning models** (neural networks, transformers) - Determinism/compliance risk
2. **External ML APIs** (AWS SageMaker, Azure ML) - PHI leakage risk
3. **Automatic model retraining** - Requires labeled data we don't have
4. **Cross-tenant learning** - Privacy violation risk
5. **Real-time model serving** - Overkill for statistical methods

---

## Dependencies

| Crate | Purpose | Integration |
|-------|---------|-------------|
| hl7v2-audit | Audit trail | Anomaly events logged |
| hl7v2-analytics | Baseline stats | Use existing aggregations |
| hl7v2-terminology | Code patterns | Optional distribution tracking |

---

## References

- BDD scenarios: `.swarm/EFF-840/scenarios.md`
- Design notes: `.swarm/EFF-840/design.md`
- Working notes: `.swarm/EFF-840/notes.md`
