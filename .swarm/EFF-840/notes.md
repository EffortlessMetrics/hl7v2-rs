# Working Notes: EFF-840

**Status:** Spec Complete  
**Date:** 2026-04-04  
**Location:** `.swarm/EFF-840/`

---

## Discovery Findings

### Current Gap Analysis

Upon examination of the hl7v2-rs workspace, confirmed zero ML/anomaly detection exists:

**Search Results:**
- "anomaly": 0 issues
- "ml" or "machine learning": 0 issues  
- "classification": 0 issues
- "detect": 0 issues

**Existing Crates:**
| Crate | Current Capability | Gap |
|-------|-------------------|-----|
| hl7v2-validation | Field-level type validation | No behavioral patterns |
| hl7v2-audit | Event logging | No pattern analysis |
| hl7v2-analytics | Count, average statistics | No anomaly scoring |

---

## Healthcare Threat Landscape

### Security Threats

| Threat | Description | Impact |
|--------|-------------|--------|
| PHI Exfiltration | Unusual volume of ADT queries | $10M+ HIPAA fines |
| Replay Attacks | Duplicate control IDs with modifications | Data integrity loss |
| Injection | Unexpected message types | System compromise |
| Timing Attacks | Off-hours access from internal IPs | Insider threats |

### Operational Issues

| Issue | Description | Patient Safety |
|-------|-------------|----------------|
| Duplicate Orders | Same ORM^O01 within 5 minutes | Medication overdose risk |
| Misdirected Results | ORU^R01 to wrong provider | Delayed treatment |
| Volume Anomalies | 10x normal volume | System failure/DoS |
| Format Drift | Gradual field usage changes | Integration breakage |

---

## Design Decisions

### Decision 1: Statistical ML Only

**Why not neural networks?**
- Healthcare requires explainable decisions (FDA/HIPAA)
- Black box models can't pass compliance audits
- Statistical methods are sufficient for our threat model
- No training data requirements

**Selected algorithms:**
- Z-score for volume spike detection
- SHA-256 hashing for duplicate detection
- Welford's online algorithm for baseline stats

---

### Decision 2: Local-Only Processing

**Why no external ML APIs?**
- PHI must never leave the system (HIPAA)
- External APIs = latency + availability risk
- Air-gapped deployments need to work offline

---

### Decision 3: Default Alert-Only

**Why never block by default?**
- Patient safety is paramount
- False positives can't block care
- Gradual adoption: alert → review → block

---

### Decision 4: 7-Day Sliding Window

**Why 7 days?**
- Captures weekly patterns (weekday vs weekend)
- Healthcare is seasonal but slow-changing
- Sliding window adapts without retraining

---

## Options Considered

### Option A: Embedded Statistical ML (SELECTED)
- **Pros:** Deterministic, explainable, fast, offline, no PHI leakage
- **Cons:** Limited to simple patterns
- **Why:** Healthcare requires explainable decisions

### Option B: External ML Service (REJECTED)
- **Pros:** Advanced models (neural networks)
- **Cons:** PHI leakage risk, latency, cost, dependency
- **Why:** HIPAA compliance violation

### Option C: SIEM Integration Only (DEFERRED)
- **Pros:** Uses existing security infrastructure
- **Cons:** No HL7-specific patterns
- **Why:** Should be complementary, not replacement

---

## Implementation Phases

### Phase 1: Statistical Anomaly Detection
- Time-series analysis (volume spikes)
- Field value statistics (outliers)
- Baseline learning (7-day window)
- Anomaly scoring (Z-score)

### Phase 2: Pattern Classification
- Duplicate detection (SHA-256)
- Replay detection (control ID tracking)
- Sender behavior profiles
- Timing analysis (off-hours)

### Phase 3: Automated Response
- Alert (webhook/email/Slack)
- Quarantine (hold for review)
- Block (reject message)
- Throttle (rate limit)

### Phase 4: Optional ML Integration
- ONNX runtime for pre-trained models
- Only if Phases 1-3 insufficient

---

## Risks and Unknowns

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| False positives block care | Medium | Critical | Default alert-only, never block |
| Baseline poisoning | Low | High | Sliding window, manual reset option |
| Performance impact | Low | Medium | <10ms target, Welford's algorithm |
| PHI in features | Medium | High | Hash-based, no raw values in baselines |
| Alert fatigue | Medium | Medium | Severity levels, prioritization |

---

## Dependencies

| Crate | Purpose | Integration Type |
|-------|---------|------------------|
| hl7v2-audit | Audit trail | Events logged |
| hl7v2-analytics | Baseline stats | Data source |
| hl7v2-terminology | Code patterns | Optional |

---

## Open Questions

1. **Detection latency:** Real-time (per-message) or batch (windowed)?
   - *Decision:* Real-time with <10ms overhead

2. **Baseline scope:** Per-sender, per-facility, or global?
   - *Decision:* Per-sender primarily, with global fallback

3. **PHI in features:** Can we use field values without privacy violations?
   - *Decision:* Use hashed values, not raw PHI

4. **Alert fatigue:** How to prioritize anomalies?
   - *Decision:* Severity scoring (INFO/WARN/CRITICAL)

5. **SIEM integration:** Feed into Splunk/ELK or replace?
   - *Decision:* Complementary - hl7v2-guard for HL7-specific, SIEM for general

---

## Recommendation

**Proceed with Phased Implementation:**

1. **Immediate (Phase 1):** Statistical anomaly detection
   - Volume spike detection
   - Field outlier detection
   - 7-day baseline learning

2. **Short-term (Phase 2):** Pattern classification
   - Duplicate/replay detection
   - Sender behavior profiles
   - Timing analysis

3. **Medium-term (Phase 3):** Automated response
   - Alert channels
   - Quarantine queue
   - Rate limiting

4. **Future (Phase 4):** ML model integration (optional)
   - Only if statistical methods insufficient

---

## Next Owner

**Spec Verifier** - Verify ML gap exists and validate approach:
- Confirm no anomaly detection in existing crates
- Review statistical ML approach vs alternatives
- Validate 20 BDD scenarios cover requirements
- Assess performance targets (<10ms, 1000 msg/sec)
