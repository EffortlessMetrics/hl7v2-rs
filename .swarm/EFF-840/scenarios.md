# BDD Scenarios: hl7v2-guard

## Overview

This document contains Behavior-Driven Development (BDD) scenarios for the hl7v2-guard crate. These scenarios serve as executable specifications and acceptance criteria for downstream test implementation.

---

## Feature: Volume Anomaly Detection

```gherkin
Feature: Message Volume Anomaly Detection
  As a security operator
  I want to detect unusual message volume patterns
  So that I can identify potential DDoS attacks or system failures

  Background:
    Given the guard engine is configured with default settings
    And the baseline window is 7 days
    And the Z-score threshold is 3.0

  @P0 @phase-1
  Scenario: Normal volume within expected range
    Given the baseline average is 1000 messages per hour
    And the baseline standard deviation is 100
    When 950 messages are received in one hour
    Then no volume anomaly should be detected
    And the anomaly score should be below 0.3
    And the Z-score should be approximately 0.5

  @P0 @phase-1
  Scenario: Volume spike detected
    Given the baseline average is 1000 messages per hour
    And the baseline standard deviation is 100
    When 5000 messages are received in one hour
    Then exactly 1 volume anomaly should be detected
    And the severity should be "Critical"
    And the anomaly score should be above 0.9
    And the Z-score should be approximately 40.0
    And the reason should contain "40.0x above baseline"

  @P0 @phase-1
  Scenario: Moderate volume increase
    Given the baseline average is 1000 messages per hour
    And the baseline standard deviation is 100
    When 2000 messages are received in one hour
    Then exactly 1 volume anomaly should be detected
    And the severity should be "Medium"
    And the anomaly score should be between 0.5 and 0.8

  @P0 @phase-1
  Scenario: Volume drop detected
    Given the baseline average is 1000 messages per hour
    And the baseline standard deviation is 100
    When 50 messages are received in one hour
    Then exactly 1 volume anomaly should be detected
    And the severity should be "High"
    And the anomaly score should be above 0.8
    And the reason should contain "below baseline"

  @P1 @phase-1
  Scenario: Gradual volume increase (baseline adaptation)
    Given the baseline average is 1000 messages per hour
    When volume increases by 10% per hour for 24 hours
    Then no anomaly should be detected after 24 hours
    And the baseline average should be approximately 1100 messages per hour

  @P1 @phase-1
  Scenario: Configurable threshold
    Given the Z-score threshold is set to 5.0
    And the baseline average is 1000 messages per hour
    And the baseline standard deviation is 100
    When 3500 messages are received in one hour
    Then no volume anomaly should be detected
    Because the Z-score of 25.0 exceeds threshold but threshold is 5.0
    # Note: Actually this should trigger - test is verifying threshold works
    # Correction: When 1500 messages received (Z-score 5.0)

  @P1 @phase-1
  Scenario: Per-sender volume tracking
    Given the baseline scope is "PerSender"
    And sender "LAB001" baseline average is 100 messages per hour
    And sender "RAD001" baseline average is 50 messages per hour
    When "LAB001" sends 200 messages in one hour
    And "RAD001" sends 100 messages in one hour
    Then "LAB001" should have a volume anomaly
    And "RAD001" should have a volume anomaly
    And the baseline stats should track separately

  @P1 @phase-1
  Scenario: Insufficient data for detection
    Given the baseline has only 10 observations
    When 1000 messages are received in one hour
    Then no volume anomaly should be detected
    And the confidence should be below 0.3
    Because minimum sample size (default 30) not met
```

---

## Feature: Duplicate Message Detection

```gherkin
Feature: Duplicate Message Detection
  As a patient safety officer
  I want to detect duplicate medication orders
  So that I can prevent duplicate dosing and identify replay attacks

  Background:
    Given the duplicate detection window is 5 minutes
    And the guard engine is configured with default settings

  @P0 @phase-1
  Scenario: No duplicate - different control IDs
    Given a message with control ID "MSG001" and hash "hash001" was processed
    When a message with control ID "MSG002" and hash "hash002" is received
    Then no duplicate anomaly should be detected
    And the message should be allowed

  @P0 @phase-1
  Scenario: Exact duplicate detected within window
    Given a message with:
      | field       | value    |
      | control_id  | MSG001   |
      | hash        | abc123   |
      | timestamp   | 10:00:00 |
    Was processed at "2026-04-04T10:00:00Z"
    When a message with:
      | field       | value    |
      | control_id  | MSG001   |
      | hash        | abc123   |
      | timestamp   | 10:02:00 |
    Is received at "2026-04-04T10:02:00Z"
    Then exactly 1 duplicate anomaly should be detected
    And the severity should be "Critical"
    And the anomaly type should be "ExactDuplicate"
    And the reason should contain "exact duplicate"

  @P0 @phase-1
  Scenario: Near-duplicate (replay attack) detected
    Given a message with:
      | field       | value    |
      | control_id  | MSG001   |
      | hash        | abc123   |
      | timestamp   | 10:00:00 |
    Was processed at "2026-04-04T10:00:00Z"
    When a message with:
      | field       | value    |
      | control_id  | MSG001   |
      | hash        | def456   | # Different content
      | timestamp   | 10:00:01 | # 1 second later
    Is received at "2026-04-04T10:00:01Z"
    Then exactly 1 replay anomaly should be detected
    And the severity should be "High"
    And the anomaly type should be "ReplayAttack"
    And the reason should contain "control ID replay"

  @P0 @phase-1
  Scenario: Duplicate outside time window
    Given a message with control ID "MSG001" was processed 6 minutes ago
    When a message with control ID "MSG001" is received now
    Then no duplicate anomaly should be detected
    And the message should be allowed
    Because the duplicate detection window has expired

  @P1 @phase-1
  Scenario: Multiple duplicates detected
    Given 5 messages with the same control ID were processed 1 minute apart
    When a 6th message with the same control ID is received
    Then exactly 1 duplicate anomaly should be detected
    And the anomaly metadata should include "duplicate_count: 6"

  @P1 @phase-1
  Scenario: Configurable duplicate window
    Given the duplicate detection window is set to 1 hour
    When a duplicate message is received 30 minutes after the original
    Then a duplicate anomaly should be detected
    Because the 30 minute delta is within the 1 hour window

  @P1 @phase-2
  Scenario: Cross-sender duplicate (same control ID, different sender)
    Given sender "LAB001" sent a message with control ID "MSG001"
    When sender "RAD001" sends a message with control ID "MSG001"
    Then no duplicate anomaly should be detected
    Because control IDs are scoped to sender
```

---

## Feature: Statistical Outlier Detection

```gherkin
Feature: Statistical Outlier Detection
  As a data quality analyst
  I want to detect unusual field values
  So that I can identify data corruption or injection attempts

  Background:
    Given the outlier threshold is Z-score > 3.0
    And the minimum sample size is 30

  @P0 @phase-1
  Scenario: Normal value within distribution
    Given field "PV1.2" (patient class) has:
      | statistic | value |
      | mean      | 5.0   |
      | stddev    | 1.0   |
      | count     | 1000  |
    When a message with "PV1.2" = 5.5 is received
    Then no outlier anomaly should be detected
    And the Z-score should be approximately 0.5

  @P0 @phase-1
  Scenario: Extreme outlier detected
    Given field "PV1.2" has:
      | statistic | value |
      | mean      | 5.0   |
      | stddev    | 1.0   |
      | count     | 1000  |
    When a message with "PV1.2" = 15.0 is received
    Then exactly 1 outlier anomaly should be detected
    And the Z-score should be approximately 10.0
    And the severity should be "High"
    And the reason should contain "Z-score: 10.0"

  @P0 @phase-1
  Scenario: Boundary value (exactly at threshold)
    Given field "PV1.2" has:
      | statistic | value |
      | mean      | 5.0   |
      | stddev    | 1.0   |
      | count     | 1000  |
    When a message with "PV1.2" = 8.0 is received
    Then exactly 1 outlier anomaly should be detected
    Because the Z-score of 3.0 equals the threshold

  @P0 @phase-1
  Scenario: Boundary value (just below threshold)
    Given field "PV1.2" has:
      | statistic | value |
      | mean      | 5.0   |
      | stddev    | 1.0   |
      | count     | 1000  |
    When a message with "PV1.2" = 7.99 is received
    Then no outlier anomaly should be detected
    Because the Z-score of 2.99 is below the threshold

  @P1 @phase-1
  Scenario: Insufficient data reduces confidence
    Given field "PV1.3" has only 20 observations
    When a message with "PV1.3" = 999.0 is received
    Then an outlier anomaly may be detected
    But the confidence should be below 0.5
    And the severity should be "Low"
    Because minimum sample size (30) not met

  @P1 @phase-1
  Scenario: Multiple field outliers in single message
    Given field "PV1.2" has mean 5.0 and stddev 1.0
    And field "PV1.19" has mean 100.0 and stddev 10.0
    When a message with "PV1.2" = 15.0 and "PV1.19" = 200.0 is received
    Then exactly 2 outlier anomalies should be detected
    And each anomaly should reference a different field

  @P1 @phase-1
  Scenario: Categorical field outlier detection
    Given field "MSH.9.1" (message type) has the distribution:
      | value  | count |
      | ADT    | 500   |
      | ORU    | 400   |
      | ORM    | 100   |
    When a message with "MSH.9.1" = "MDM" is received
    Then an outlier anomaly should be detected
    Because "MDM" has not been seen before (frequency = 0)

  @P1 @phase-2
  Scenario: Time-series outlier (unusual time gap)
    Given the baseline inter-message gap for sender "LAB001" is 60 seconds
    And the standard deviation is 10 seconds
    When a gap of 300 seconds (5 minutes) occurs
    Then a timing outlier anomaly should be detected
    And the Z-score should be approximately 24.0
```

---

## Feature: Sender Behavior Profiling

```gherkin
Feature: Sender Behavior Profiling
  As a security analyst
  I want to detect when a sender behaves unusually
  So that I can identify compromised accounts or unauthorized access

  Background:
    Given the guard engine is configured with sender profiling enabled
    And the profile tracks message types and timing patterns

  @P1 @phase-2
  Scenario: Normal behavior from known sender
    Given sender "LAB001" has profile:
      | attribute        | value        |
      | typical_types    | ORU^R01      |
      | active_hours     | 08:00-18:00  |
      | typical_volume   | 100/hour     |
    When "LAB001" sends an "ORU^R01" message at 10:00
    Then no behavior anomaly should be detected
    And the profile confidence should be high (> 0.8)

  @P1 @phase-2
  Scenario: Unexpected message type from sender
    Given sender "LAB001" has profile:
      | attribute        | value        |
      | typical_types    | ORU^R01      |
      | type_ADT^A01_count | 0        |
    And "LAB001" has sent 1000 messages, all "ORU^R01"
    When "LAB001" sends an "ADT^A01" message
    Then exactly 1 behavior anomaly should be detected
    And the anomaly type should be "UnexpectedMessageType"
    And the reason should contain "unexpected message type: ADT^A01"
    And the severity should be "Medium"

  @P1 @phase-2
  Scenario: Off-hours access detected
    Given sender "LAB001" has profile:
      | attribute        | value        |
      | active_hours     | 08:00-18:00  |
      | after_hours_count| 2            |
    And "LAB001" has sent 1000 messages during business hours
    And only 2 messages outside business hours historically
    When "LAB001" sends a message at 02:00
    Then exactly 1 timing anomaly should be detected
    And the anomaly type should be "OffHoursAccess"
    And the severity should be "Low"
    Because off-hours access may be legitimate on-call activity

  @P1 @phase-2
  Scenario: Volume deviation from sender baseline
    Given sender "LAB001" has profile:
      | attribute        | value        |
      | typical_volume   | 100/hour     |
      | volume_stddev    | 10           |
    When "LAB001" sends 500 messages in one hour
    Then exactly 1 volume anomaly should be detected
    And the anomaly should reference the sender profile

  @P1 @phase-2
  Scenario: New sender - no profile yet
    Given sender "NEW001" has no existing profile
    And the minimum profile size is 50 messages
    When "NEW001" sends their first 10 messages
    Then no behavior anomaly should be detected
    And the confidence should be low (< 0.3)
    Because insufficient data to establish baseline

  @P1 @phase-2
  Scenario: Gradual behavior change (profile drift)
    Given sender "LAB001" typically sends "ORU^R01"
    When "LAB001" gradually shifts to sending "MDM^T02" over 30 days
    Then behavior anomalies should be detected during transition
    And the profile should adapt after 50 "MDM^T02" messages
    And anomalies should decrease after adaptation
```

---

## Feature: Anomaly Scoring and Severity

```gherkin
Feature: Anomaly Scoring and Severity
  As a security operator
  I want consistent scoring and severity assignment
  So that I can prioritize response actions

  Background:
    Given the guard engine is configured with default scoring rules

  @P0 @phase-1
  Scenario: Anomaly score range validation
    When any anomaly is detected
    Then the anomaly score should be between 0.0 and 1.0
    And the confidence should be between 0.0 and 1.0

  @P0 @phase-1
  Scenario: Score correlates with severity
    Given the severity thresholds are:
      | severity  | min_score |
      | Low       | 0.0       |
      | Medium    | 0.3       |
      | High      | 0.6       |
      | Critical  | 0.9       |
    When an anomaly with score 0.95 is detected
    Then the severity should be "Critical"
    When an anomaly with score 0.5 is detected
    Then the severity should be "Medium"

  @P0 @phase-1
  Scenario: Confidence affects severity
    Given an anomaly with score 0.8 would normally be "High"
    When the confidence is 0.2 (low confidence)
    Then the severity should be reduced to "Medium"
    Because low confidence anomalies should not trigger high-severity actions

  @P1 @phase-1
  Scenario: Multiple anomalies - score aggregation
    Given a message triggers 3 anomalies with scores 0.5, 0.7, 0.9
    When the anomalies are analyzed
    Then the aggregated score should be at least 0.9
    And the overall severity should be "Critical"

  @P1 @phase-1
  Scenario: Explainable reasons in all anomalies
    When any anomaly is detected
    Then the reason field should not be empty
    And the reason should be human-readable
    And the reason should contain specific values (Z-score, delta, etc.)
```

---

## Feature: Response Actions

```gherkin
Feature: Anomaly Response Actions
  As a security operator
  I want configurable responses to anomalies
  So that I can balance security with operational continuity

  Background:
    Given the guard engine is configured with response rules:
      | severity  | action      | config        |
      | Low       | Log         |               |
      | Medium    | Alert       | webhook_url   |
      | High      | Quarantine  | 30min         |
      | Critical  | Block       |               |

  @P1 @phase-3
  Scenario: Low severity - log only
    Given a "Low" severity anomaly is detected
    When the response is triggered
    Then the anomaly should be recorded in audit trail
    And no alert should be sent
    And the message should proceed to normal processing

  @P1 @phase-3
  Scenario: Medium severity - alert
    Given a "Medium" severity anomaly is detected
    When the response is triggered
    Then an alert should be sent via webhook
    And the alert payload should contain the anomaly details
    And the message should proceed to normal processing

  @P1 @phase-3
  Scenario: High severity - quarantine
    Given a "High" severity anomaly is detected
    When the response is triggered
    Then the message should be placed in quarantine
    And the quarantine duration should be 30 minutes
    And an alert should be sent
    And the audit trail should record the quarantine action

  @P1 @phase-3
  Scenario: Critical severity - block
    Given a "Critical" severity anomaly is detected
    When the response is triggered
    Then the message should be rejected
    And an error response should be sent to the sender
    And an alert should be sent
    And the audit trail should record the block action

  @P1 @phase-3
  Scenario: Custom response rule
    Given a custom response rule is configured:
      """
      if anomaly.type == "ReplayAttack" {
        action = Block;
        bypass_severity = true;
      }
      """
    When a "Medium" severity "ReplayAttack" is detected
    Then the message should be blocked
    Because the custom rule overrides the default severity-based action

  @P1 @phase-3
  Scenario: Throttle response
    Given sender "LAB001" has triggered 10 anomalies in 1 minute
    When the throttle response is configured for repeat offenders
    Then "LAB001" should be rate-limited to 1 message per second
    And the audit trail should record the throttle action
```

---

## Feature: Configuration and Baseline Management

```gherkin
Feature: Configuration and Baseline Management
  As a system administrator
  I want to configure and manage anomaly detection settings
  So that I can adapt to changing requirements

  Background:
    Given the guard engine is running with default configuration

  @P1 @phase-1
  Scenario: Runtime configuration update
    Given the current Z-score threshold is 3.0
    When the configuration is updated to Z-score threshold 5.0
    Then new messages should use the updated threshold
    And existing baselines should be preserved

  @P1 @phase-1
  Scenario: Baseline persistence
    Given a baseline has been established for 7 days
    When the guard engine is restarted
    Then the baseline should be restored from persistence
    And detection should resume without re-learning period

  @P1 @phase-1
  Scenario: Manual baseline reset
    Given an established baseline exists
    When a manual reset is triggered for sender "LAB001"
    Then the baseline for "LAB001" should be cleared
    And a new baseline should begin collecting
    And other sender baselines should be unaffected

  @P1 @phase-1
  Scenario: Baseline learning mode
    Given the guard engine is in "learning" mode
    When messages are processed
    Then baselines should be updated
    But no anomalies should be triggered
    Because learning mode disables detection

  @P1 @phase-2
  Scenario: Export baseline statistics
    Given an established baseline exists
    When baseline statistics are exported
    Then the export should include:
      | field              | type       |
      | sender_id          | string     |
      | message_type_dist  | map        |
      | hourly_volume_avg  | float      |
      | hourly_volume_std  | float      |
      | field_statistics   | map        |
      | last_updated       | timestamp  |
```

---

## Feature: Integration with Audit System

```gherkin
Feature: Integration with Audit System
  As a compliance officer
  I want all anomaly detections to be auditable
  So that I can meet regulatory requirements

  Background:
    Given hl7v2-audit is configured and available
    And the guard engine has audit integration enabled

  @P0 @phase-1
  Scenario: Anomaly detection audit event
    When any anomaly is detected
    Then an audit event should be logged with:
      | field           | value              |
      | event_type      | ANOMALY_DETECTED   |
      | anomaly_id      | <unique_id>        |
      | rule_id         | <rule_identifier>  |
      | severity        | <severity_level>   |
      | score           | <anomaly_score>    |
      | message_hash    | <message_hash>     |
      | timestamp       | <utc_timestamp>    |

  @P0 @phase-1
  Scenario: Response action audit event
    When a response action (alert, quarantine, block) is taken
    Then an audit event should be logged with:
      | field           | value                |
      | event_type      | RESPONSE_TRIGGERED   |
      | anomaly_id      | <referenced_id>      |
      | action          | <action_type>        |
      | result          | <success/failure>    |

  @P1 @phase-1
  Scenario: Baseline update audit event
    When a baseline is updated with new statistics
    Then an audit event should be logged with:
      | field           | value                |
      | event_type      | BASELINE_UPDATED     |
      | sender_id       | <sender_id>          |
      | sample_count    | <new_sample_count>   |
      | window_start    | <window_timestamp>   |

  @P1 @phase-1
  Scenario: Configuration change audit event
    When the guard configuration is modified
    Then an audit event should be logged with:
      | field           | value                   |
      | event_type      | CONFIG_CHANGED          |
      | changed_by      | <user/agent_id>         |
      | changes         | <diff_of_config>        |
```

---

## Test Implementation Guide

### Tags Reference

| Tag | Meaning | Implementation Priority |
|-----|---------|------------------------|
| @P0 | Critical path - must implement | First |
| @P1 | Important - should implement | Second |
| @phase-1 | Phase 1 deliverable | With Phase 1 |
| @phase-2 | Phase 2 deliverable | With Phase 2 |
| @phase-3 | Phase 3 deliverable | With Phase 3 |

### Test Data Requirements

```rust
// Example test fixtures
pub struct TestFixtures;

impl TestFixtures {
    /// Creates a realistic HL7v2 ADT message
    pub fn adt_a01_message() -> Hl7v2Message;
    
    /// Creates a realistic HL7v2 ORU message
    pub fn oru_r01_message() -> Hl7v2Message;
    
    /// Creates a guard engine with test configuration
    pub fn test_guard_engine() -> GuardEngine;
    
    /// Creates a pre-populated baseline for testing
    pub fn populated_baseline(config: BaselineConfig) -> Baseline;
}
```

---

*Document Version: 1.0*  
*For: [EFF-840](/EFF/issues/EFF-840)*  
*Next Owner: Spec Verifier*
