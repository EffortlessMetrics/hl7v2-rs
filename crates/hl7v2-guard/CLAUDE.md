# hl7v2-guard

ML-based anomaly detection and automated threat response for HL7 messages.

## Build
```bash
cargo build -p hl7v2-guard
```

## Test
```bash
cargo test -p hl7v2-guard
```

## Lint
```bash
cargo clippy -p hl7v2-guard -- -D warnings
```

## Features

- **Statistical Anomaly Detection**: Z-score based volume spike and field outlier detection
- **Pattern Classification**: Duplicate detection, replay attack detection, sender behavior profiling
- **Automated Response**: Configurable actions (alert, quarantine, block, throttle)
- **Local Processing**: No external ML APIs - PHI never leaves the system

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Incoming HL7 Message                      │
└───────────────────────────────┬─────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────┐
│                     hl7v2-guard Pipeline                      │
│  ┌─────────────────┐  ┌─────────────────┐  ┌──────────────┐   │
│  │  Message Hash   │  │  Feature        │  │  Anomaly     │   │
│  │  (SHA-256)      │→ │  Extraction     │→ │  Scoring     │   │
│  └─────────────────┘  └─────────────────┘  └──────────────┘   │
│         │                      │                      │      │
│         ▼                      ▼                      ▼      │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │                    Pattern Detection Engine               │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │ │
│  │  │ Duplicate    │  │ Replay       │  │ Timing       │   │ │
│  │  │ Detection    │  │ Detection    │  │ Analysis     │   │ │
│  │  └──────────────┘  └──────────────┘  └──────────────┘   │ │
│  └─────────────────────────────────────────────────────────┘ │
│                               │                              │
│                               ▼                              │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │                    Decision Engine                        │ │
│  │  Score → Severity → Action (alert/quarantine/block)       │ │
│  └─────────────────────────────────────────────────────────┘ │
└───────────────────────────────────────────────────────────────┘
```
