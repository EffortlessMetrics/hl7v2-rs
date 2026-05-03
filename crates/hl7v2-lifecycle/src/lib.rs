//! Message archival, retention, and compliance management.

use chrono::{DateTime, Duration, Utc};
use hl7v2_core::Message;
use serde::{Deserialize, Serialize};

/// Policy for message retention
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// Duration to keep messages in active storage
    pub active_duration: Duration,
    /// Duration to keep messages in archive before purging
    pub archive_duration: Duration,
    /// Whether to archive messages after active period
    pub archive_after: bool,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            active_duration: Duration::days(90),
            archive_duration: Duration::days(365 * 7), // 7 years standard
            archive_after: true,
        }
    }
}

/// Message state in the lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageState {
    /// Message is in active storage and accessible
    Active,
    /// Message has been moved to long-term storage
    Archived,
    /// Message has been permanently deleted
    Purged,
}

/// Legal hold status for a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegalHold {
    /// Whether the hold is active
    pub is_active: bool,
    /// Reason for the hold
    pub reason: String,
    /// Who placed the hold
    pub placed_by: String,
    /// When the hold was placed
    pub placed_at: DateTime<Utc>,
}

/// Archive metadata for a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveMetadata {
    /// Message control ID
    pub message_id: String,
    /// Current state of the message
    pub state: MessageState,
    /// When the message was received
    pub received_at: DateTime<Utc>,
    /// When the message was last moved or updated
    pub updated_at: DateTime<Utc>,
    /// When the message should be archived (if Active) or purged (if Archived)
    pub next_action_date: DateTime<Utc>,
    /// Legal hold information
    pub legal_hold: Option<LegalHold>,
    /// SHA-256 hash of the message content for integrity verification
    pub message_hash: String,
}

impl ArchiveMetadata {
    /// Check if the message can be moved to the next state
    pub fn can_transition(&self, now: DateTime<Utc>) -> bool {
        // Legal hold overrides any retention policy
        if let Some(ref hold) = self.legal_hold
            && hold.is_active
        {
            return false;
        }

        now >= self.next_action_date
    }
}

/// Simple in-memory archive (placeholder for database integration)
pub struct MessageArchive {
    policy: RetentionPolicy,
}

impl MessageArchive {
    /// Create a new message archive with the given policy
    pub fn new(policy: RetentionPolicy) -> Self {
        Self { policy }
    }

    /// Prepare a message for archival by generating metadata
    pub fn prepare_metadata(&self, message: &Message, raw_hl7: &[u8]) -> ArchiveMetadata {
        use sha2::{Digest, Sha256};

        let now = Utc::now();
        let message_id = message
            .segments
            .iter()
            .find(|s| &s.id == b"MSH")
            .and_then(|msh| msh.fields.get(8))
            .and_then(|f| f.first_text())
            .unwrap_or("UNKNOWN")
            .to_string();

        let mut hasher = Sha256::new();
        hasher.update(raw_hl7);
        let hash = format!("{:x}", hasher.finalize());

        ArchiveMetadata {
            message_id,
            state: MessageState::Active,
            received_at: now,
            updated_at: now,
            next_action_date: now + self.policy.active_duration,
            legal_hold: None,
            message_hash: hash,
        }
    }

    /// Calculate the next state and action date for a message
    pub fn next_lifecycle_step(&self, metadata: &ArchiveMetadata) -> (MessageState, DateTime<Utc>) {
        match metadata.state {
            MessageState::Active => {
                if self.policy.archive_after {
                    (
                        MessageState::Archived,
                        metadata.next_action_date + self.policy.archive_duration,
                    )
                } else {
                    (MessageState::Purged, metadata.next_action_date)
                }
            }
            MessageState::Archived => (MessageState::Purged, metadata.next_action_date),
            MessageState::Purged => (MessageState::Purged, metadata.next_action_date),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hl7v2_core::parse;

    #[test]
    fn test_prepare_metadata() {
        let hl7 =
            b"MSH|^~\\&|SENDER|FACILITY|RECEIVER|FACILITY|20250101120000||ADT^A01|MSG123|P|2.5\r";
        let message = parse(hl7).unwrap();

        let policy = RetentionPolicy {
            active_duration: Duration::days(30),
            archive_duration: Duration::days(365),
            archive_after: true,
        };

        let archive = MessageArchive::new(policy);
        let metadata = archive.prepare_metadata(&message, hl7);

        assert_eq!(metadata.message_id, "MSG123");
        assert_eq!(metadata.state, MessageState::Active);
        assert!(!metadata.message_hash.is_empty());
        assert_eq!(
            metadata.next_action_date,
            metadata.received_at + Duration::days(30)
        );
    }

    #[test]
    fn test_legal_hold_blocks_transition() {
        let now = Utc::now();
        let metadata = ArchiveMetadata {
            message_id: "TEST".to_string(),
            state: MessageState::Active,
            received_at: now - Duration::days(100),
            updated_at: now - Duration::days(100),
            next_action_date: now - Duration::days(10),
            legal_hold: Some(LegalHold {
                is_active: true,
                reason: "Audit".to_string(),
                placed_by: "Compliance".to_string(),
                placed_at: now - Duration::days(5),
            }),
            message_hash: "hash".to_string(),
        };

        // Even though next_action_date is in the past, legal hold blocks transition
        assert!(!metadata.can_transition(now));
    }

    #[test]
    fn test_lifecycle_transitions() {
        let policy = RetentionPolicy {
            active_duration: Duration::days(30),
            archive_duration: Duration::days(365),
            archive_after: true,
        };
        let archive = MessageArchive::new(policy);

        let now = Utc::now();
        let mut metadata = ArchiveMetadata {
            message_id: "TEST".to_string(),
            state: MessageState::Active,
            received_at: now,
            updated_at: now,
            next_action_date: now + Duration::days(30),
            legal_hold: None,
            message_hash: "hash".to_string(),
        };

        let (next_state, next_date) = archive.next_lifecycle_step(&metadata);
        assert_eq!(next_state, MessageState::Archived);
        assert_eq!(next_date, metadata.next_action_date + Duration::days(365));

        // Update state to archived
        metadata.state = next_state;
        metadata.next_action_date = next_date;

        let (final_state, _) = archive.next_lifecycle_step(&metadata);
        assert_eq!(final_state, MessageState::Purged);
    }

    #[test]
    fn test_serde_stability() {
        let now = Utc::now();
        let metadata = ArchiveMetadata {
            message_id: "TEST".to_string(),
            state: MessageState::Active,
            received_at: now,
            updated_at: now,
            next_action_date: now + Duration::days(30),
            legal_hold: None,
            message_hash: "hash".to_string(),
        };

        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: ArchiveMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(metadata.message_id, deserialized.message_id);
        assert_eq!(metadata.state, deserialized.state);
        assert_eq!(metadata.message_hash, deserialized.message_hash);
    }
}
