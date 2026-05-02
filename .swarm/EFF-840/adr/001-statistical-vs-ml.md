# ADR-001: Statistical Anomaly Detection over ML Models

## Status
Accepted

## Context

The hl7v2-guard crate needs to detect anomalies in HL7v2 message flows. The scout recommendation identified multiple approaches:

1. **Embedded lightweight ML** - Statistical methods (Z-score, IQR, moving averages)
2. **External ML service integration** - AWS SageMaker, Azure ML
3. **SIEM integration only** - Delegate to Splunk/ELK

Additionally, we considered (but rejected) using neural networks or deep learning models.

## Decision

We will use **statistical anomaly detection** (Z-score, IQR, moving averages) implemented in pure Rust, with no external ML service dependencies.

## Consequences

### Positive

1. **Explainability**: Statistical methods provide clear, auditable reasons for each anomaly ("Z-score of 4.2 indicates 4.2 standard deviations above mean"). This is required for healthcare compliance audits.

2. **Determinism**: Same input always produces same output. No stochastic behavior that could cause unpredictable false positives in safety-critical contexts.

3. **No PHI leakage**: All processing happens on-premise. No data leaves the system for external ML APIs.

4. **Performance**: Sub-millisecond detection latency. No network round-trips to ML services.

5. **Operational simplicity**: No MLops complexity (model training, versioning, deployment pipelines).

6. **Regulatory compliance**: HIPAA and FDA requirements favor explainable, deterministic systems over black-box ML for patient safety applications.

### Negative

1. **Limited pattern detection**: Cannot detect complex multi-dimensional patterns as effectively as deep learning (e.g., neural networks, transformers).

2. **Manual feature engineering**: Must explicitly define what to monitor (volume, timing, duplicates) rather than learning automatically.

3. **No transfer learning**: Cannot leverage pre-trained models from other healthcare organizations.

### Mitigations

1. **Phase 4 option**: ONNX runtime integration can be added later if statistical methods prove insufficient.
2. **Multiple detectors**: Combine multiple statistical detectors (volume + timing + duplicates) for richer detection.
3. **Domain knowledge**: HL7 messaging patterns are well-understood in healthcare; explicit detectors align with industry expertise.

## Alternatives Considered

### External ML Service Integration (Rejected)
- **Pros**: Advanced models (neural networks, transformers), continuous improvement
- **Cons**: PHI leakage risk, latency, external dependency, API costs, compliance complexity
- **Rejection reason**: Sending PHI to external services creates unacceptable compliance risk

### SIEM Integration Only (Deferred)
- **Pros**: Uses existing security infrastructure
- **Cons**: No HL7-specific patterns, requires separate SIEM deployment, not integrated with message processing pipeline
- **Decision**: Complementary feature for Phase 3+, not replacement for built-in detection

### Neural Networks / Deep Learning (Rejected)
- **Pros**: Can learn complex patterns, automatic feature extraction
- **Cons**: Black-box behavior, difficult to validate for safety, requires large training datasets, harder to explain for compliance
- **Rejection reason**: Unpredictable behavior in patient safety context; regulatory requirements favor explainability

## Related Decisions

- [spec.md](../spec.md) Section 3.1 - Statistical vs ML Approach
- [design-notes.md](../design-notes.md) Section 2 - Detector Pattern

## Date
2026-04-04

## Authors
[@spec-designer](/EFF/agents/spec-designer)
