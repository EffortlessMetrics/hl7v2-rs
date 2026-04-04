# BDD Scenarios: hl7v2-guard Anomaly Detection

**Issue:** [EFF-840](/EFF/issues/EFF-840)  
**Purpose:** ML-based anomaly detection scenarios for test implementation

---

## Scenario 1: Volume spike detection

```gherkin
Given a sender "hospital-a" has a baseline of 10 messages per minute
When the sender sends 50 messages in one minute (5x baseline)
Then the anomaly score should exceed the threshold (Z-score > 3.0)
And the severity should be CRITICAL
And an alert should be sent via webhook
```

## Scenario 2: Normal volume within baseline

```gherkin
Given a sender "hospital-a" has a baseline of 10 messages per minute
When the sender sends 12 messages in one minute (within normal variance)
Then no anomaly should be detected
And the message should be processed normally
```

## Scenario 3: Duplicate medication order detection

```gherkin
Given an ORM^O01 order message was received 2 minutes ago
When the exact same ORM^O01 order is received again
Then the duplicate should be detected within the 5-minute window
And the severity should be WARNING
And an alert should indicate "Duplicate medication order detected"
```

## Scenario 4: Near-duplicate detection

```gherkin
Given an ORM^O01 order for patient "12345" was received
When a similar ORM^O01 order for patient "12346" with same medication is received
Then a near-duplicate should be detected
And the severity should be INFO
And the alert should show field-level differences
```

## Scenario 5: Replay attack detection

```gherkin
Given a message with control ID "ABC123" was received yesterday
When a new message with control ID "ABC123" but different content is received
Then a replay attack should be detected
And the severity should be CRITICAL
And the action should quarantine the message
```

## Scenario 6: Legitimate duplicate control ID

```gherkin
Given a message with control ID "ABC123" was received 10 minutes ago
When the exact same message with control ID "ABC123" is received (retry)
Then it should be flagged as a duplicate, not a replay
And the severity should be INFO
And the message should be acknowledged but not reprocessed
```

## Scenario 7: Off-hours access detection

```gherkin
Given business hours are configured as 08:00-18:00
When a message is received at 02:00 from sender "internal-ip"
Then an off-hours anomaly should be detected
And the severity should be WARNING
And the alert should indicate "Off-hours access detected"
```

## Scenario 8: Expected off-hours access

```gherkin
Given sender "batch-processor" regularly sends messages at 03:00
When a message is received at 03:00 from "batch-processor"
Then no anomaly should be detected (pattern learned)
And the message should be processed normally
```

## Scenario 9: Unexpected message type from sender

```gherkin
Given sender "lab-system" typically sends ORU^R01 results
When sender "lab-system" sends ADT^A01 admission messages
Then unexpected message type should be detected
And the severity should be WARNING
And the alert should indicate "Unexpected message type from sender"
```

## Scenario 10: Message type distribution drift

```gherkin
Given normal distribution is 60% ADT, 30% ORU, 10% ORM
When distribution changes to 30% ADT, 50% ORU, 20% ORM over 1 hour
Then distribution drift should be detected
And the severity should be INFO
And the alert should indicate "Message type distribution changed"
```

## Scenario 11: Field value outlier detection

```gherkin
Given PV1-19 (Visit Number) typically has values between 1000-9999
When a message with PV1-19 = "999999" is received
Then a field value outlier should be detected
And the severity should be WARNING
And the alert should indicate "Unusual value in PV1-19"
```

## Scenario 12: Quarantine action

```gherkin
Given the guard is configured with action = quarantine
And a CRITICAL anomaly is detected
Then the message should be placed in quarantine queue
And a 202 Accepted response should be returned
And the alert should notify operators
```

## Scenario 13: Block action

```gherkin
Given the guard is configured with action = block
And a CRITICAL replay attack is detected
Then the message should be rejected
And a 403 Forbidden response should be returned
And the alert should indicate "Message blocked - replay detected"
```

## Scenario 14: Throttle action

```gherkin
Given the guard is configured with action = throttle
And a sender exceeds anomaly threshold 3 times in 10 minutes
Then the sender should be rate-limited
And subsequent messages should receive 429 Too Many Requests
And the throttle should auto-expire after cooldown period
```

## Scenario 15: Alert-only default action

```gherkin
Given the guard is configured with default action = alert
When any anomaly is detected
Then the message should be processed normally
And an alert should be sent via configured webhook
And no blocking should occur
```

## Scenario 16: Baseline learning over 7 days

```gherkin
Given a new sender "hospital-b" starts sending messages
When 7 days of message patterns are collected
Then a baseline should be established for the sender
And anomaly detection should use the learned baseline
And older data (>7 days) should be evicted from baseline
```

## Scenario 17: Severity levels

```gherkin
Given Z-score thresholds: INFO (1.0-2.0), WARN (2.0-3.0), CRITICAL (>3.0)
When anomalies with various Z-scores are detected
Then severity should be assigned correctly
And INFO anomalies should be logged only
And WARNING anomalies should trigger alerts
And CRITICAL anomalies should trigger immediate response
```

## Scenario 18: Webhook alert delivery

```gherkin
Given HL7V2_GUARD_WEBHOOK_URL is configured
When a WARNING or CRITICAL anomaly is detected
Then a POST request should be sent to the webhook URL
And the payload should include: message_id, severity, reasons, timestamp
And the webhook timeout should be 5 seconds
```

## Scenario 19: Guard disabled

```gherkin
Given HL7V2_GUARD_ENABLED is set to false
When messages are received
Then no anomaly detection should occur
And all messages should be processed normally
And no alerts should be sent
```

## Scenario 20: Multi-factor anomaly scoring

```gherkin
Given a sender has volume spike (Z-score 2.5)
And the message is sent off-hours (Z-score 2.0)
And the sender has unexpected message type (Z-score 1.5)
When the combined anomaly score is calculated
Then the score should weight volume highest (0.4)
And the combined score should be weighted average
And the severity should reflect the combined score
```
