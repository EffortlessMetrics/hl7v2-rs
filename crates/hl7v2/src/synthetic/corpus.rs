//! HL7 v2 test corpus generation and management utilities.
//!
//! This module provides functionality for managing test corpora of HL7 v2 messages.
//! It includes:
//!
//! - Manifest handling for reproducible test data
//! - Golden hash verification for regression testing
//! - Train/validation/test split management
//! - SHA-256 hash computation utilities
//!
//! # Manifest Management
//!
//! The [`CorpusManifest`] type tracks all metadata needed for reproducible
//! corpus generation:
//!
//! - Templates and their hashes
//! - Generation seed
//! - Message metadata
//! - Train/validation/test splits
//!
//! # Example
//!
//! ```
//! use hl7v2::synthetic::corpus::{CorpusManifest, compute_sha256};
//!
//! let mut manifest = CorpusManifest::new(42);
//! manifest.add_template("test.yaml", "template content");
//! manifest.add_message("msg001.hl7", "MSH|^~\\&|...", "ADT^A01", 0);
//!
//! let json = manifest.to_json().unwrap();
//! let parsed = CorpusManifest::from_json(&json).unwrap();
//! assert_eq!(parsed.seed, 42);
//! ```

use crate::model::{Atom, Message};
use crate::writer::write;
use chrono::{DateTime, Utc};
use rand::{RngExt, SeedableRng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Configuration for corpus generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusConfig {
    /// Random seed for deterministic generation
    pub seed: u64,
    /// Number of messages to generate
    pub count: usize,
    /// Batch size for memory-efficient generation
    pub batch_size: usize,
    /// Optional output directory for generated files
    pub output_dir: Option<String>,
    /// Whether to create train/validation/test splits
    pub create_splits: bool,
    /// Split ratios (train, validation, test) - should sum to 1.0
    pub split_ratios: Option<(f64, f64, f64)>,
}

impl Default for CorpusConfig {
    fn default() -> Self {
        Self {
            seed: 42,
            count: 100,
            batch_size: 50,
            output_dir: None,
            create_splits: false,
            split_ratios: Some((0.7, 0.15, 0.15)),
        }
    }
}

/// Information about a template file in the manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateInfo {
    /// Relative path to the template file
    pub path: String,
    /// SHA-256 hash of the template file
    pub sha256: String,
}

/// Information about a profile file in the manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileInfo {
    /// Relative path to the profile file
    pub path: String,
    /// SHA-256 hash of the profile file
    pub sha256: String,
}

/// Information about a generated message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageInfo {
    /// Relative path to the message file
    pub path: String,
    /// SHA-256 hash of the message content
    pub sha256: String,
    /// Message type (e.g., "ADT^A01")
    pub message_type: String,
    /// Template index used to generate this message
    pub template_index: usize,
}

/// Train/validation/test split information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CorpusSplits {
    /// Training set message paths
    pub train: Vec<String>,
    /// Validation set message paths
    pub validation: Vec<String>,
    /// Test set message paths
    pub test: Vec<String>,
}

/// Manifest for reproducible message corpus generation
///
/// This struct tracks all metadata needed to reproduce a corpus,
/// including template hashes, generation seed, and message information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusManifest {
    /// Schema version
    pub version: String,
    /// hl7v2-rs tool version
    pub tool_version: String,
    /// Random seed used for generation
    pub seed: u64,
    /// Template files used
    pub templates: Vec<TemplateInfo>,
    /// Profile files used for validation (optional)
    #[serde(default)]
    pub profiles: Vec<ProfileInfo>,
    /// Generated message files
    pub messages: Vec<MessageInfo>,
    /// Timestamp of generation
    pub generated_at: DateTime<Utc>,
    /// Train/validation/test splits (optional)
    #[serde(default)]
    pub splits: CorpusSplits,
}

impl CorpusManifest {
    /// Create a new empty manifest
    pub fn new(seed: u64) -> Self {
        Self {
            version: "1.0.0".to_string(),
            tool_version: env!("CARGO_PKG_VERSION").to_string(),
            seed,
            templates: Vec::new(),
            profiles: Vec::new(),
            messages: Vec::new(),
            generated_at: Utc::now(),
            splits: CorpusSplits::default(),
        }
    }

    /// Add a template to the manifest
    pub fn add_template(&mut self, path: &str, content: &str) {
        let sha256 = compute_sha256(content);
        self.templates.push(TemplateInfo {
            path: path.to_string(),
            sha256,
        });
    }

    /// Add a profile to the manifest
    pub fn add_profile(&mut self, path: &str, content: &str) {
        let sha256 = compute_sha256(content);
        self.profiles.push(ProfileInfo {
            path: path.to_string(),
            sha256,
        });
    }

    /// Add a message to the manifest
    pub fn add_message(
        &mut self,
        path: &str,
        content: &str,
        message_type: &str,
        template_index: usize,
    ) {
        let sha256 = compute_sha256(content);
        self.messages.push(MessageInfo {
            path: path.to_string(),
            sha256,
            message_type: message_type.to_string(),
            template_index,
        });
    }

    /// Serialize the manifest to JSON
    ///
    /// # Errors
    ///
    /// Returns [`CorpusError::SerializationError`] if the manifest cannot be
    /// serialized.
    pub fn to_json(&self) -> Result<String, CorpusError> {
        serde_json::to_string_pretty(self)
            .map_err(|e| CorpusError::SerializationError(e.to_string()))
    }

    /// Deserialize a manifest from JSON
    ///
    /// # Errors
    ///
    /// Returns [`CorpusError::SerializationError`] if the JSON is malformed or
    /// does not match the manifest schema.
    pub fn from_json(json: &str) -> Result<Self, CorpusError> {
        serde_json::from_str(json).map_err(|e| CorpusError::SerializationError(e.to_string()))
    }

    /// Get the total number of messages
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Get message types and their counts
    pub fn message_type_counts(&self) -> HashMap<String, usize> {
        let mut counts = HashMap::new();
        for msg in &self.messages {
            let count = counts.entry(msg.message_type.clone()).or_insert(0usize);
            *count = count.saturating_add(1);
        }
        counts
    }

    /// Create train/validation/test splits
    pub fn create_splits(&mut self, ratios: (f64, f64, f64)) {
        let total = self.messages.len();
        if total == 0 {
            return;
        }

        let train_count = rounded_ratio_count(total, ratios.0);
        let remaining_after_train = total.saturating_sub(train_count);
        let val_count = rounded_ratio_count(total, ratios.1).min(remaining_after_train);
        let validation_end = train_count.saturating_add(val_count);

        // Shuffle indices based on seed for reproducibility
        let mut rng = rand::rngs::StdRng::seed_from_u64(self.seed);
        let mut indices: Vec<usize> = (0..total).collect();

        // Fisher-Yates shuffle
        for i in (1..total).rev() {
            let j = rng.random_range(0..=i);
            indices.swap(i, j);
        }

        self.splits.train = indices
            .get(..train_count)
            .unwrap_or_default()
            .iter()
            .filter_map(|&i| self.messages.get(i).map(|message| message.path.clone()))
            .collect();

        self.splits.validation = indices
            .get(train_count..validation_end)
            .unwrap_or_default()
            .iter()
            .filter_map(|&i| self.messages.get(i).map(|message| message.path.clone()))
            .collect();

        self.splits.test = indices
            .get(validation_end..)
            .unwrap_or_default()
            .iter()
            .filter_map(|&i| self.messages.get(i).map(|message| message.path.clone()))
            .collect();
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "split ratios are configured as f64 percentages by the public API"
)]
fn rounded_ratio_count(total: usize, ratio: f64) -> usize {
    if !ratio.is_finite() || ratio <= 0.0 {
        return 0;
    }

    let total_f64 = total as f64;
    let rounded = (total_f64 * ratio).round();

    if rounded <= 0.0 {
        0
    } else if rounded >= total_f64 {
        total
    } else {
        rounded as usize
    }
}

/// Compute SHA-256 hash of a string
pub fn compute_sha256(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let hash_result = hasher.finalize();
    format!("{hash_result:x}")
}

/// Compute SHA-256 hash of a message
pub fn compute_message_hash(message: &Message) -> String {
    let message_bytes = write(message);
    // Convert bytes to string for hashing (HL7 messages are ASCII-based)
    let message_string = String::from_utf8_lossy(&message_bytes);
    compute_sha256(&message_string)
}

/// Error type for corpus operations
#[derive(Debug, Clone, thiserror::Error)]
pub enum CorpusError {
    /// Error during serialization/deserialization
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Error during file I/O
    #[error("IO error: {0}")]
    IoError(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Invalid split ratios
    #[error("Invalid split ratios: must sum to 1.0")]
    InvalidSplitRatios,
}

/// Extract message type from a message's MSH.9 field
pub fn extract_message_type(message: &Message) -> String {
    // Find MSH segment
    for segment in &message.segments {
        if &segment.id == b"MSH" {
            // MSH.9 is at index 8 (0-indexed: field 9 - 1 for skipping MSH-1/MSH-2)
            if let Some(field) = segment.fields.get(7)
                && let Some(rep) = field.reps.first()
                && !rep.comps.is_empty()
            {
                // Build the message type from components
                let parts: Vec<String> = rep
                    .comps
                    .iter()
                    .filter_map(|c| match c.subs.first() {
                        Some(Atom::Text(t)) => Some(t.clone()),
                        _ => None,
                    })
                    .collect();
                return parts.join("^");
            }
        }
    }
    "UNKNOWN".to_string()
}
