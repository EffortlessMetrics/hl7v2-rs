# hl7v2-guard

ML-based anomaly detection and automated threat response for HL7 messages.

## Features

- **Statistical Anomaly Detection**: Z-score based volume spike and field outlier detection
- **Pattern Classification**: Duplicate detection, replay attack detection, sender behavior profiling
- **Automated Response**: Configurable actions (alert, quarantine, block, throttle)
- **Local Processing**: No external ML APIs - PHI never leaves the system
- **Explainable Decisions**: Statistical methods only for healthcare compliance

## Usage

```rust
use hl7v2_guard::{Guard, GuardConfig, GuardAction};
use std::time::Duration;

let config = GuardConfig::builder()
    .enabled(true)
    .baseline_days(7)
    .z_threshold(3.0)
    .duplicate_window(Duration::from_secs(300))
    .action(GuardAction::Alert)
    .build();

let guard = Guard::new(config);

// Analyze a message
let result = guard.analyze(&message);

if result.severity == Severity::Critical {
    println!("Critical anomaly: {:?}", result.reasons);
}
```

## Safety First

Default action is `alert` - never blocks messages by default to ensure patient safety.
