//! Message archival, retention, and compliance management.

use chrono::{DateTime, Duration, Utc};
use hl7v2_core::Message;
use serde::{Deserialize, Serialize};

/// Policy for message retention
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// Duration to keep messages
    pub retention_duration: Duration,
    /// Whether to archive messages after retention period
    pub archive_after: bool,
}

/// Archive metadata for a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveMetadata {
    /// Message control ID
    pub message_id: String,
    /// When the message was received
    pub received_at: DateTime<Utc>,
    /// When the message should be deleted or archived
    pub expiry_date: DateTime<Utc>,
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
    pub fn prepare_archive(&self, message: &Message) -> ArchiveMetadata {
        let now = Utc::now();
        let message_id = message
            .segments
            .iter()
            .find(|s| &s.id == b"MSH")
            .and_then(|msh| msh.fields.get(9))
            .and_then(|f| f.first_text())
            .unwrap_or("UNKNOWN")
            .to_string();

        ArchiveMetadata {
            message_id,
            received_at: now,
            expiry_date: now + self.policy.retention_duration,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hl7v2_core::parse;

    #[test]
    fn test_prepare_archive() {
        let hl7 =
            b"MSH|^~\\&|SENDER|FACILITY|RECEIVER|FACILITY|20250101120000||ADT^A01|MSG123|P|2.5\r";
        let message = parse(hl7).unwrap();

        let policy = RetentionPolicy {
            retention_duration: Duration::days(365),
            archive_after: true,
        };

        let archive = MessageArchive::new(policy);
        let metadata = archive.prepare_archive(&message);

        assert_eq!(metadata.message_id, "MSG123");
        assert!(metadata.expiry_date > metadata.received_at);
    }
}
