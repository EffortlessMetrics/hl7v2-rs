# Implementation Notes: hl7v2-guard

## Phase 1 Implementation Checklist

### Core Types
- [ ] `RuleId` - Newtype for rule identifiers
- [ ] `Severity` - Enum: Low, Medium, High, Critical
- [ ] `Anomaly` - Struct with all required fields
- [ ] `ResponseAction` - Enum for configurable responses
- [ ] `GuardConfig` - Top-level configuration struct
- [ ] `GuardEngine` - Main public API struct

### Detectors (Phase 1)
- [ ] `VolumeDetector` - Time-series volume spike/drop detection
  - Circular buffer for sliding window
  - Z-score calculation
  - Configurable threshold
- [ ] `DuplicateDetector` - Exact duplicate detection
  - Message hashing (SHA-256 of canonical form)
  - Bloom filter for probabilistic tracking
  - LRU cache for exact storage
  - Time window expiration
- [ ] `OutlierDetector` - Statistical outlier detection
  - Per-field statistics (mean, stddev)
  - Z-score calculation
  - Minimum sample handling
- [ ] Trait `AnomalyDetector` - Common interface

### Baseline Management
- [ ] `SlidingWindow` - Generic sliding window with stats
- [ ] `RunningStats` - Online mean/stddev calculation (Welford's algorithm)
- [ ] `Baseline` - Aggregates window and stats
- [ ] Baseline persistence (JSON/JSONL format)

### Response System (Phase 1 - Log only)
- [ ] `ResponseEngine` - Routes anomalies to actions
- [ ] Audit integration - Send events to hl7v2-audit

### Integration
- [ ] `MessageContext` - Pre-computed message features
- [ ] Feature extraction from Hl7v2Message
- [ ] hl7v2-audit integration crate dependency

### Testing
- [ ] Unit tests for each detector
- [ ] Integration tests for GuardEngine
- [ ] BDD scenario implementations
- [ ] Performance benchmarks

---

## Code Structure

### File Organization

```
src/
├── lib.rs
├── types.rs           # Core types (Anomaly, Severity, etc.)
├── config.rs          # Configuration structs
├── context.rs         # MessageContext
├── engine.rs          # GuardEngine
├── error.rs           # GuardError
├── detector/
│   ├── mod.rs         # AnomalyDetector trait
│   ├── volume.rs
│   ├── duplicate.rs
│   └── outlier.rs
├── baseline/
│   ├── mod.rs
│   ├── window.rs      # SlidingWindow
│   └── stats.rs       # RunningStats
├── response/
│   └── mod.rs         # Phase 1: audit only
└── util/
    └── hash.rs        # Message hashing utilities
```

### Key Implementation Details

#### Welford's Algorithm for Running Statistics

```rust
/// Online mean and variance calculation
pub struct RunningStats {
    count: u64,
    mean: f64,
    m2: f64,  // Sum of squares of differences
}

impl RunningStats {
    pub fn add(&mut self, value: f64) {
        self.count += 1;
        let delta = value - self.mean;
        self.mean += delta / self.count as f64;
        let delta2 = value - self.mean;
        self.m2 += delta * delta2;
    }
    
    pub fn variance(&self) -> f64 {
        if self.count < 2 {
            0.0
        } else {
            self.m2 / (self.count - 1) as f64
        }
    }
    
    pub fn stddev(&self) -> f64 {
        self.variance().sqrt()
    }
    
    pub fn z_score(&self, value: f64) -> Option<f64> {
        let std = self.stddev();
        if std == 0.0 || self.count < 30 {
            None
        } else {
            Some((value - self.mean) / std)
        }
    }
}
```

#### Message Canonicalization for Hashing

```rust
/// Create a canonical form for consistent hashing
pub fn canonicalize_for_hash(msg: &Hl7v2Message) -> String {
    // Include: message type, control ID, key patient fields
    // Exclude: timestamp (varies), processing ID (varies)
    format!(
        "{}|{}|{}|{}|{}",
        msg.message_type(),
        msg.control_id(),
        msg.patient_id(),
        msg.order_number().unwrap_or_default(),
        msg.observation_count()
    )
}

pub fn hash_message(msg: &Hl7v2Message) -> String {
    let canonical = canonicalize_for_hash(msg);
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    format!("{:x}", hasher.finalize())
}
```

#### Confidence Calculation

```rust
/// Calculate confidence based on sample size
pub fn confidence_from_sample_size(n: usize, min_samples: usize) -> f64 {
    if n >= min_samples * 2 {
        1.0
    } else if n >= min_samples {
        0.8
    } else if n >= min_samples / 2 {
        0.5
    } else {
        0.2
    }
}

/// Adjust severity based on confidence
pub fn adjust_severity_for_confidence(
    base_severity: Severity,
    confidence: f64,
) -> Severity {
    if confidence < 0.3 {
        // Reduce severity for low confidence
        match base_severity {
            Severity::Critical => Severity::High,
            Severity::High => Severity::Medium,
            Severity::Medium => Severity::Low,
            Severity::Low => Severity::Low,
        }
    } else {
        base_severity
    }
}
```

---

## Dependencies to Add

### Cargo.toml

```toml
[package]
name = "hl7v2-guard"
version = "0.1.0"
edition = "2021"

[dependencies]
# Core HL7 dependencies
hl7v2-types = { path = "../hl7v2-types" }
hl7v2-parser = { path = "../hl7v2-parser" }
hl7v2-audit = { path = "../hl7v2-audit" }

# Time handling
chrono = { version = "0.4", features = ["serde"] }

# Statistical functions
statrs = "0.16"

# Data structures
bloomfilter = "1.0"
lru = "0.12"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Error handling
thiserror = "1.0"

# Observability
tracing = "0.1"

# Async (for future phases)
tokio = { version = "1", features = ["sync"], optional = true }

# Hashing
sha2 = "0.10"

[dev-dependencies]
tokio-test = "0.4"
criterion = "0.5"
proptest = "1.0"

[[bench]]
name = "detector_bench"
harness = false
```

---

## Testing Strategy

### Unit Test Examples

```rust
#[cfg(test)]
mod volume_tests {
    use super::*;
    
    #[test]
    fn test_volume_spike_detection() {
        let mut detector = VolumeDetector::new(VolumeConfig {
            window_size: Duration::from_secs(3600),
            z_threshold: 3.0,
            min_samples: 30,
        });
        
        // Seed baseline: 1000 messages/hour
        for _ in 0..100 {
            detector.learn(&MessageContext::with_volume(1000));
        }
        
        // Detect spike: 5000 messages
        let ctx = MessageContext::with_volume(5000);
        let anomalies = detector.detect(&ctx);
        
        assert_eq!(anomalies.len(), 1);
        assert_eq!(anomalies[0].severity, Severity::Critical);
        assert!(anomalies[0].score > 0.8);
    }
}

#[cfg(test)]
mod duplicate_tests {
    use super::*;
    
    #[test]
    fn test_exact_duplicate_detection() {
        let mut detector = DuplicateDetector::new(DuplicateConfig {
            window: Duration::from_secs(300),
            bloom_capacity: 1000,
            lru_capacity: 100,
        });
        
        let msg1 = MessageContext::with_hash("abc123");
        let msg2 = MessageContext::with_hash("abc123"); // Same hash
        
        // First message - not a duplicate
        let result1 = detector.detect(&msg1);
        assert!(result1.is_empty());
        
        // Second message - duplicate
        let result2 = detector.detect(&msg2);
        assert_eq!(result2.len(), 1);
        assert_eq!(result2[0].rule_id, RuleId::new("duplicate"));
    }
}
```

### Property-Based Tests

```rust
proptest! {
    #[test]
    fn test_score_in_valid_range(score in 0.0f64..1.0f64) {
        let anomaly = Anomaly {
            score,
            ..Default::default()
        };
        prop_assert!(anomaly.score >= 0.0 && anomaly.score <= 1.0);
    }
    
    #[test]
    fn test_z_score_monotonicity(values in prop::collection::vec(1.0f64..100.0, 10..1000)) {
        let mut stats = RunningStats::default();
        for v in &values {
            stats.add(*v);
        }
        
        // Z-score should be higher for values further from mean
        let z1 = stats.z_score(values[0]).unwrap();
        let z2 = stats.z_score(values[0] + stats.stddev() * 5.0).unwrap();
        prop_assert!(z2 > z1);
    }
}
```

### Benchmark Tests

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_volume_detector(c: &mut Criterion) {
    let detector = create_populated_detector();
    let msg = create_test_message();
    
    c.bench_function("volume_detect", |b| {
        b.iter(|| detector.detect(black_box(&msg)))
    });
}

fn benchmark_duplicate_detector(c: &mut Criterion) {
    let mut detector = create_duplicate_detector();
    let msg = create_test_message();
    
    c.bench_function("duplicate_check", |b| {
        b.iter(|| detector.check(black_box(&msg)))
    });
}

criterion_group!(benches, benchmark_volume_detector, benchmark_duplicate_detector);
criterion_main!(benches);
```

---

## Audit Integration

```rust
use hl7v2_audit::{AuditEvent, AuditLogger};

pub struct AuditResponder {
    logger: Arc<dyn AuditLogger>,
}

impl AuditResponder {
    pub fn log_anomaly(&self, anomaly: &Anomaly, msg: &MessageContext) {
        let event = AuditEvent::builder()
            .event_type("ANOMALY_DETECTED")
            .anomaly_id(&anomaly.rule_id.0)
            .severity(format!("{:?}", anomaly.severity))
            .score(anomaly.score)
            .message_hash(&msg.message_hash)
            .sender(&msg.sender)
            .build();
        
        self.logger.log(event);
    }
}
```

---

## Open Questions for Implementation

1. **Message hashing**: Should we use full message content or just key fields? (Privacy vs accuracy trade-off)
2. **Baseline storage**: JSON file, SQLite, or external store? (Simplicity vs scalability)
3. **Time source**: System time vs message timestamp for detection windows? (Clock skew handling)
4. **Threading**: Tokio async vs rayon parallel? (Depends on integration point)
5. **Configuration**: File-based (YAML/TOML) or code-based? (Deployment flexibility)

---

*Document Version: 1.0*  
*For: [EFF-840](/EFF/issues/EFF-840)*
