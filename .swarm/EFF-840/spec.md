# EFF-840: hl7v2-guard Execution Specification

## Overview

**Issue**: [EFF-840](/EFF/issues/EFF-840)  
**Crate**: `hl7v2-guard`  
**Purpose**: ML-based anomaly detection and message classification for HL7v2 messages  
**Branch**: `EFF-840`  
**Status**: Spec Design Phase

---

## 1. Requirements

### 1.1 Functional Requirements

| ID | Requirement | Priority | Phase |
|----|-------------|----------|-------|
| FR-1 | Detect message volume anomalies (spikes/drops) | P0 | 1 |
| FR-2 | Detect duplicate messages within configurable time windows | P0 | 1 |
| FR-3 | Identify unusual field values via statistical outlier detection | P0 | 1 |
| FR-4 | Detect replay attacks via control ID + timestamp analysis | P0 | 2 |
| FR-5 | Profile sender behavior and detect deviations | P1 | 2 |
| FR-6 | Analyze timing patterns (business hours vs off-hours) | P1 | 2 |
| FR-7 | Support configurable anomaly scoring thresholds | P0 | 1 |
| FR-8 | Provide explainable anomaly reasons (compliance requirement) | P0 | 1-2 |
| FR-9 | Support configurable response actions (alert, quarantine, block) | P1 | 3 |
| FR-10 | Integrate with hl7v2-audit for anomaly event trails | P0 | 1 |

### 1.2 Non-Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| NFR-1 | Deterministic, explainable detection (no black-box ML) | P0 |
| NFR-2 | Sub-millisecond anomaly scoring latency (per-message) | P0 |
| NFR-3 | No external ML service dependencies (on-premise only) | P0 |
| NFR-4 | HIPAA-compliant (no PHI leakage in features/models) | P0 |
| NFR-5 | Thread-safe for concurrent message processing | P0 |
| NFR-6 | Configurable baselines (per-sender, global, or hybrid) | P1 |
| NFR-7 | Memory-bounded state management for sliding windows | P1 |

---

## 2. Architecture

### 2.1 Crate Structure

```
crates/hl7v2-guard/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Public API exports
│   ├── config.rs           # Configuration types
│   ├── detector/           # Anomaly detection engines
│   │   ├── mod.rs
│   │   ├── volume.rs       # Time-series volume detection
│   │   ├── duplicate.rs    # Duplicate message detection
│   │   ├── outlier.rs      # Statistical outlier detection
│   │   ├── replay.rs       # Replay attack detection
│   │   └── timing.rs       # Timing pattern analysis
│   ├── profiler/           # Sender behavior profiling
│   │   ├── mod.rs
│   │   └── sender.rs
│   ├── scorer.rs           # Anomaly scoring engine
│   ├── response/           # Response actions
│   │   ├── mod.rs
│   │   ├── alert.rs
│   │   ├── quarantine.rs
│   │   └── throttle.rs
│   ├── baseline/           # Baseline learning
│   │   ├── mod.rs
│   │   ├── window.rs       # Sliding window statistics
│   │   └── learner.rs      # Baseline learning logic
│   └── types.rs            # Core types (Anomaly, Score, etc.)
└── tests/
    ├── bdd/                # BDD scenario tests
    └── integration/        # Integration tests
```

### 2.2 Core Types

```rust
/// Unique identifier for an anomaly detection rule
pub struct RuleId(pub String);

/// Severity level for anomalies
pub enum Severity {
    Low,      // Log only
    Medium,   // Alert + log
    High,     // Quarantine + alert
    Critical, // Block + alert
}

/// Anomaly detection result
pub struct Anomaly {
    pub rule_id: RuleId,
    pub severity: Severity,
    pub score: f64,           // 0.0 - 1.0
    pub reason: String,       // Human-readable explanation
    pub confidence: f64,      // 0.0 - 1.0
    pub timestamp: DateTime<Utc>,
    pub message_hash: String, // For correlation
}

/// Configurable response to anomalies
pub enum ResponseAction {
    Log,        // Record in audit trail only
    Alert(AlertConfig),      // Send notification
    Quarantine(Duration),    // Hold for review
    Block,      // Reject message
    Throttle(RateLimit),     // Rate limit sender
}

/// Detection configuration
pub struct GuardConfig {
    pub volume_config: VolumeConfig,
    pub duplicate_window: Duration,
    pub outlier_threshold: f64,  // Z-score threshold
    pub baseline_window: Duration,
    pub response_rules: Vec<ResponseRule>,
}
```

### 2.3 Public API

```rust
/// Main guard engine - thread-safe, clonable
#[derive(Clone)]
pub struct GuardEngine {
    // ... internal state
}

impl GuardEngine {
    /// Create new guard engine with configuration
    pub fn new(config: GuardConfig) -> Result<Self, GuardError>;
    
    /// Analyze a message for anomalies
    pub async fn analyze(&self, message: &Hl7v2Message) -> Vec<Anomaly>;
    
    /// Get current baseline statistics
    pub fn baseline_stats(&self) -> BaselineStats;
    
    /// Update configuration at runtime
    pub fn reconfigure(&mut self, config: GuardConfig) -> Result<(), GuardError>;
}

/// Convenience function for one-off analysis
pub async fn detect_anomalies(
    message: &Hl7v2Message,
    config: &GuardConfig,
) -> Result<Vec<Anomaly>, GuardError>;
```

---

## 3. Design Decisions

### 3.1 Statistical vs ML Approach

**Decision**: Use statistical anomaly detection (Z-score, IQR, moving averages) rather than neural networks or external ML services.

**Rationale**:
- Healthcare requires explainable decisions for compliance audits
- Deterministic behavior is safety-critical (no unpredictable false positives)
- No PHI leakage risk to external APIs
- Simpler to validate and test exhaustively

**Trade-offs**:
- Limited to known statistical patterns
- Cannot detect complex multi-dimensional anomalies as effectively as deep learning
- Mitigation: Phase 4 (optional) can add ONNX support for pre-trained models

### 3.2 Real-time vs Batch Detection

**Decision**: Real-time per-message detection with optional batch correlation.

**Rationale**:
- Immediate response to threats (replay attacks, duplicates)
- Lower latency for message processing pipeline
- Can still correlate across messages via sliding window state

**Implementation**:
- Each message analyzed immediately upon receipt
- Stateful detectors maintain sliding window statistics
- Async batch correlation for cross-sender patterns

### 3.3 Baseline Scope

**Decision**: Support per-sender, global, and hybrid baselines via configuration.

**Rationale**:
- Different senders have different normal patterns (lab vs ADT vs pharmacy)
- Global baseline catches system-wide issues
- Hybrid approach: per-sender for volume/timing, global for duplicates

**Configuration**:
```rust
pub enum BaselineScope {
    Global,              // One baseline for all messages
    PerSender,           // Baseline per sending application/facility
    Hybrid(ScopeRules),  // Configurable per-detector
}
```

### 3.4 State Management

**Decision**: In-memory sliding windows with optional persistence for baselines.

**Rationale**:
- Fast access for real-time scoring
- Bounded memory via configurable window sizes
- Baseline persistence allows restart without re-learning

**Memory bounds**:
- Volume windows: Fixed-size circular buffers (configurable, default 10k entries)
- Duplicate detection: Bloom filter + LRU cache (configurable, default 100k messages)
- Sender profiles: LRU cache with TTL (configurable, default 1000 active senders)

---

## 4. BDD Scenarios

### 4.1 Volume Anomaly Detection

```gherkin
Feature: Message Volume Anomaly Detection
  As a security operator
  I want to detect unusual message volume patterns
  So that I can identify potential DDoS attacks or system failures

  Background:
    Given the guard engine is configured with default settings
    And the baseline window is 7 days

  Scenario: Normal volume within expected range
    Given the baseline average is 1000 messages per hour
    When 950 messages are received in one hour
    Then no volume anomaly should be detected
    And the anomaly score should be below 0.3

  Scenario: Volume spike detected
    Given the baseline average is 1000 messages per hour
    When 5000 messages are received in one hour
    Then a volume anomaly should be detected
    And the severity should be "High"
    And the anomaly score should be above 0.8
    And the reason should mention "5x above baseline"

  Scenario: Volume drop detected
    Given the baseline average is 1000 messages per hour
    When 50 messages are received in one hour
    Then a volume anomaly should be detected
    And the severity should be "Medium"
    And the reason should mention "95% below baseline"

  Scenario: Gradual volume increase (baseline poisoning prevention)
    Given the baseline average is 1000 messages per hour
    When volume increases by 10% per hour for 12 hours
    Then no anomaly should be detected after 12 hours
    And the baseline should have adapted to the new normal
```

### 4.2 Duplicate Detection

```gherkin
Feature: Duplicate Message Detection
  As a patient safety officer
  I want to detect duplicate medication orders
  So that I can prevent duplicate dosing

  Background:
    Given the duplicate detection window is 5 minutes
    And the guard engine is configured with default settings

  Scenario: No duplicate - different control IDs
    Given a message with control ID "MSG001" is processed
    When a message with control ID "MSG002" is received within 5 minutes
    Then no duplicate anomaly should be detected

  Scenario: Exact duplicate detected
    Given a message with control ID "MSG001" and hash "abc123" is processed
    When a message with control ID "MSG001" and hash "abc123" is received within 5 minutes
    Then a duplicate anomaly should be detected
    And the severity should be "Critical"
    And the reason should mention "exact duplicate"

  Scenario: Near-duplicate (replay attack) detected
    Given a message with control ID "MSG001" and timestamp "10:00:00" is processed
    When a message with control ID "MSG001" and timestamp "10:00:01" is received
    Then a replay anomaly should be detected
    And the severity should be "High"
    And the reason should mention "control ID replay"

  Scenario: Duplicate outside time window
    Given a message with control ID "MSG001" was processed 6 minutes ago
    When a message with control ID "MSG001" is received now
    Then no duplicate anomaly should be detected
    And the reason should note "outside detection window"
```

### 4.3 Statistical Outlier Detection

```gherkin
Feature: Statistical Outlier Detection
  As a data quality analyst
  I want to detect unusual field values
  So that I can identify data corruption or injection attempts

  Background:
    Given the outlier threshold is Z-score > 3.0
    And the baseline has 1000 observations of field "PV1.2"

  Scenario: Normal value within distribution
    Given the field "PV1.2" has mean 5.0 and stddev 1.0
    When a message with "PV1.2" = 5.5 is received
    Then no outlier anomaly should be detected

  Scenario: Extreme outlier detected
    Given the field "PV1.2" has mean 5.0 and stddev 1.0
    When a message with "PV1.2" = 15.0 is received
    Then an outlier anomaly should be detected
    And the Z-score should be approximately 10.0
    And the severity should be "High"

  Scenario: Insufficient data for outlier detection
    Given the field "PV1.3" has only 10 observations
    When a message with "PV1.3" = 999.0 is received
    Then the confidence should be low (< 0.5)
    And the severity should be reduced to "Low"
```

### 4.4 Sender Behavior Profiling

```gherkin
Feature: Sender Behavior Profiling
  As a security analyst
  I want to detect when a sender behaves unusually
  So that I can identify compromised accounts

  Background:
    Given sender "LAB001" has an established profile
    And the profile tracks message types and volume

  Scenario: Normal behavior from known sender
    Given sender "LAB001" typically sends "ORU^R01" messages
    When "LAB001" sends an "ORU^R01" message
    Then no behavior anomaly should be detected

  Scenario: Unexpected message type from sender
    Given sender "LAB001" typically sends "ORU^R01" messages
    And "LAB001" has never sent "ADT^A01" messages
    When "LAB001" sends an "ADT^A01" message
    Then a behavior anomaly should be detected
    And the reason should mention "unexpected message type"
    And the severity should be "Medium"

  Scenario: Off-hours access detected
    Given sender "LAB001" typically sends messages 08:00-18:00
    When "LAB001" sends a message at 02:00
    Then a timing anomaly should be detected
    And the reason should mention "off-hours access"
    And the severity should be "Low" (may be legitimate on-call)
```

### 4.5 Response Actions

```gherkin
Feature: Anomaly Response Actions
  As a security operator
  I want configurable responses to anomalies
  So that I can balance security with operational continuity

  Background:
    Given the guard engine is configured with response rules

  Scenario: Low severity - log only
    Given a "Low" severity anomaly is detected
    And the response rule maps "Low" to "Log"
    When the anomaly is processed
    Then the anomaly should be recorded in audit trail
    And no alert should be sent
    And the message should proceed normally

  Scenario: High severity - quarantine
    Given a "High" severity anomaly is detected
    And the response rule maps "High" to "Quarantine(30min)"
    When the anomaly is processed
    Then the message should be held in quarantine
    And an alert should be sent
    And the audit trail should record the quarantine action

  Scenario: Critical severity - block
    Given a "Critical" severity anomaly is detected
    And the response rule maps "Critical" to "Block"
    When the anomaly is processed
    Then the message should be rejected
    And an alert should be sent
    And the sender should receive an error response
```

---

## 5. Implementation Phases

### Phase 1: Statistical Anomaly Detection (MVP)

**Goal**: Core anomaly detection capabilities

**Deliverables**:
- [ ] Volume spike/drop detection (time-series)
- [ ] Duplicate message detection (hash + control ID)
- [ ] Statistical outlier detection (Z-score)
- [ ] Basic explainability (reason strings)
- [ ] Integration with hl7v2-audit

**Test coverage**: BDD scenarios for FR-1, FR-2, FR-3, FR-7, FR-8, FR-10

### Phase 2: Pattern Classification

**Goal**: Advanced pattern detection

**Deliverables**:
- [ ] Replay attack detection (control ID + timestamp correlation)
- [ ] Sender behavior profiling (message type tracking)
- [ ] Timing pattern analysis (business hours detection)
- [ ] Per-sender baselines
- [ ] Confidence scoring

**Test coverage**: BDD scenarios for FR-4, FR-5, FR-6

### Phase 3: Automated Response

**Goal**: Configurable automated actions

**Deliverables**:
- [ ] Response rule engine
- [ ] Alert notification system (webhook, email)
- [ ] Quarantine queue implementation
- [ ] Rate limiting for throttling
- [ ] Response action audit trail

**Test coverage**: BDD scenarios for FR-9

### Phase 4: ML Model Integration (Optional)

**Goal**: Advanced ML capabilities (if Phases 1-3 insufficient)

**Deliverables**:
- [ ] ONNX runtime integration
- [ ] Feature extraction pipeline
- [ ] Model versioning
- [ ] A/B testing framework

**Decision gate**: Proceed only if customer requirements exceed statistical capabilities

---

## 6. Dependencies

### 6.1 Internal Dependencies

| Crate | Purpose | Integration Point |
|-------|---------|-------------------|
| hl7v2-audit | Anomaly event trails | `audit::log_event()` |
| hl7v2-analytics | Baseline statistics | `analytics::MessageStats` |
| hl7v2-parser | Message parsing | `parser::Hl7v2Message` |
| hl7v2-types | Core HL7 types | `types::Message` |

### 6.2 External Dependencies

| Crate | Purpose | Version |
|-------|---------|---------|
| chrono | Date/time handling | ^0.4 |
| statrs | Statistical functions | ^0.16 |
| bloomfilter | Duplicate detection | ^1.0 |
| lru | LRU caches | ^0.12 |
| serde | Configuration serialization | ^1.0 |
| thiserror | Error handling | ^1.0 |
| tracing | Observability | ^0.1 |

### 6.3 Optional Dependencies (Phase 4)

| Crate | Purpose | Version |
|-------|---------|---------|
| ort | ONNX runtime | ^2.0 |

---

## 7. Risks and Mitigations

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| False positives block legitimate messages | Critical (patient safety) | Medium | Configurable thresholds, confidence scoring, quarantine before block |
| Baseline poisoning by gradual attack | High | Low | Minimum sample sizes, anomaly on baseline shift, manual baseline review |
| Performance degradation | Medium | Low | Bounded memory, async processing, benchmark tests |
| PHI in features/models | Critical (compliance) | Low | Hash-based features only, no raw values in models |
| Alert fatigue | Medium | Medium | Severity levels, alert aggregation, configurable rules |

---

## 8. Open Questions

1. **Baseline learning period**: Is 7-day default appropriate for all deployment scenarios?
2. **PHI handling**: Should we use field hashes or can we use normalized values?
3. **Integration with SIEM**: Should alerts be SIEM-compatible (CEF, LEEF formats)?
4. **Multi-tenancy**: Do we need separate baselines per tenant in multi-tenant deployments?
5. **Model updates**: How frequently should baselines be persisted to storage?

---

## 9. Acceptance Criteria

- [ ] All Phase 1 BDD scenarios pass
- [ ] Anomaly detection latency < 1ms per message (p99)
- [ ] Memory usage bounded (< 100MB for default config)
- [ ] 100% test coverage for detector logic
- [ ] Audit trail integration verified
- [ ] Documentation complete (API docs, user guide)
- [ ] No external ML service dependencies
- [ ] HIPAA compliance checklist complete

---

## 10. References

- Issue: [EFF-840](/EFF/issues/EFF-840)
- Dependencies: [EFF-116](/EFF/issues/EFF-116), [EFF-17](/EFF/issues/EFF-17)
- Scout Recommendation: See issue description Section 7
- Related: [EFF-832](/EFF/issues/EFF-832) (terminology)

---

*Spec Version: 1.0*  
*Last Updated: 2026-04-04*  
*Spec Designer: [@spec-designer](/EFF/agents/spec-designer)*
