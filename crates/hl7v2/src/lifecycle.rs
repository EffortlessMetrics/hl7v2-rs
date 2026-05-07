//! Message lifecycle and archive metadata helpers.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::Message;

/// Policy for message retention.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// Duration to keep messages in active storage.
    pub active_duration: Duration,
    /// Duration to keep messages in archive before purging.
    pub archive_duration: Duration,
    /// Whether to archive messages after active period.
    pub archive_after: bool,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            active_duration: Duration::days(90),
            archive_duration: Duration::days(2_555),
            archive_after: true,
        }
    }
}

/// Message state in the lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageState {
    /// Message is in active storage and accessible.
    Active,
    /// Message has been moved to long-term storage.
    Archived,
    /// Message has been permanently deleted.
    Purged,
}

/// Legal hold status for a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegalHold {
    /// Whether the hold is active.
    pub is_active: bool,
    /// Reason for the hold.
    pub reason: String,
    /// Who placed the hold.
    pub placed_by: String,
    /// When the hold was placed.
    pub placed_at: DateTime<Utc>,
}

/// Archive metadata for a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveMetadata {
    /// Message control ID.
    pub message_id: String,
    /// Current state of the message.
    pub state: MessageState,
    /// When the message was received.
    pub received_at: DateTime<Utc>,
    /// When the message was last moved or updated.
    pub updated_at: DateTime<Utc>,
    /// When the message should be archived, if active, or purged, if archived.
    pub next_action_date: DateTime<Utc>,
    /// Legal hold information.
    pub legal_hold: Option<LegalHold>,
    /// SHA-256 hash of the message content for integrity verification.
    pub message_hash: String,
}

impl ArchiveMetadata {
    /// Check if the message can be moved to the next state.
    pub fn can_transition(&self, now: DateTime<Utc>) -> bool {
        if let Some(ref hold) = self.legal_hold
            && hold.is_active
        {
            return false;
        }

        now >= self.next_action_date
    }
}

/// Simple in-memory archive.
///
/// This is a placeholder for database-backed lifecycle integrations.
pub struct MessageArchive {
    policy: RetentionPolicy,
}

impl MessageArchive {
    /// Create a new message archive with the given policy.
    pub fn new(policy: RetentionPolicy) -> Self {
        Self { policy }
    }

    /// Prepare a message for archival by generating metadata.
    pub fn prepare_metadata(&self, message: &Message, raw_hl7: &[u8]) -> ArchiveMetadata {
        use sha2::{Digest, Sha256};

        let now = Utc::now();
        let message_id = message
            .segments
            .iter()
            .find(|segment| segment.id == *b"MSH")
            .and_then(|msh| msh.fields.get(8))
            .and_then(|field| field.first_text())
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
            next_action_date: add_duration(now, self.policy.active_duration),
            legal_hold: None,
            message_hash: hash,
        }
    }

    /// Calculate the next state and action date for a message.
    pub fn next_lifecycle_step(&self, metadata: &ArchiveMetadata) -> (MessageState, DateTime<Utc>) {
        match metadata.state {
            MessageState::Active => {
                if self.policy.archive_after {
                    (
                        MessageState::Archived,
                        add_duration(metadata.next_action_date, self.policy.archive_duration),
                    )
                } else {
                    (MessageState::Purged, metadata.next_action_date)
                }
            }
            MessageState::Archived | MessageState::Purged => {
                (MessageState::Purged, metadata.next_action_date)
            }
        }
    }
}

fn add_duration(timestamp: DateTime<Utc>, duration: Duration) -> DateTime<Utc> {
    timestamp.checked_add_signed(duration).unwrap_or(timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Field, Message, Segment};

    #[test]
    fn prepares_archive_metadata() {
        let hl7 =
            b"MSH|^~\\&|SENDER|FACILITY|RECEIVER|FACILITY|20250101120000||ADT^A01|MSG123|P|2.5\r";
        let message = message_with_control_id("MSG123");

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
            add_duration(metadata.received_at, Duration::days(30))
        );
    }

    #[test]
    fn legal_hold_blocks_transition() {
        let now = Utc::now();
        let metadata = ArchiveMetadata {
            message_id: "TEST".to_string(),
            state: MessageState::Active,
            received_at: now,
            updated_at: now,
            next_action_date: now,
            legal_hold: Some(LegalHold {
                is_active: true,
                reason: "Audit".to_string(),
                placed_by: "Compliance".to_string(),
                placed_at: now,
            }),
            message_hash: "hash".to_string(),
        };

        assert!(!metadata.can_transition(now));
    }

    #[test]
    fn lifecycle_transitions() {
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
            next_action_date: add_duration(now, Duration::days(30)),
            legal_hold: None,
            message_hash: "hash".to_string(),
        };

        let (next_state, next_date) = archive.next_lifecycle_step(&metadata);
        assert_eq!(next_state, MessageState::Archived);
        assert_eq!(
            next_date,
            add_duration(metadata.next_action_date, Duration::days(365))
        );

        metadata.state = next_state;
        metadata.next_action_date = next_date;

        let (final_state, _) = archive.next_lifecycle_step(&metadata);
        assert_eq!(final_state, MessageState::Purged);
    }

    #[test]
    fn serde_roundtrip_preserves_archive_metadata() {
        let now = Utc::now();
        let metadata = ArchiveMetadata {
            message_id: "TEST".to_string(),
            state: MessageState::Active,
            received_at: now,
            updated_at: now,
            next_action_date: add_duration(now, Duration::days(30)),
            legal_hold: None,
            message_hash: "hash".to_string(),
        };

        let json = serialize_metadata(&metadata);
        let deserialized = deserialize_metadata(&json);

        assert_eq!(metadata.message_id, deserialized.message_id);
        assert_eq!(metadata.state, deserialized.state);
        assert_eq!(metadata.message_hash, deserialized.message_hash);
    }

    fn message_with_control_id(control_id: &str) -> Message {
        let mut msh = Segment::new(b"MSH");
        msh.fields = vec![Field::new(); 8];
        msh.add_field(Field::from_text(control_id));

        Message::with_segments(vec![msh])
    }

    fn serialize_metadata(metadata: &ArchiveMetadata) -> String {
        match serde_json::to_string(metadata) {
            Ok(json) => json,
            Err(error) => {
                assert!(
                    error.to_string().is_empty(),
                    "archive metadata should serialize: {error}"
                );
                String::new()
            }
        }
    }

    fn deserialize_metadata(json: &str) -> ArchiveMetadata {
        match serde_json::from_str(json) {
            Ok(metadata) => metadata,
            Err(error) => {
                assert!(
                    error.to_string().is_empty(),
                    "archive metadata should deserialize: {error}"
                );
                ArchiveMetadata {
                    message_id: String::new(),
                    state: MessageState::Purged,
                    received_at: Utc::now(),
                    updated_at: Utc::now(),
                    next_action_date: Utc::now(),
                    legal_hold: None,
                    message_hash: String::new(),
                }
            }
        }
    }
}
