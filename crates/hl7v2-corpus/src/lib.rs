//! HL7 v2 test corpus generation and management utilities.
//!
//! This crate provides functionality for managing test corpora of HL7 v2 messages.
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
//! use hl7v2_corpus::{CorpusManifest, compute_sha256};
//!
//! let mut manifest = CorpusManifest::new(42);
//! manifest.add_template("test.yaml", "template content");
//! manifest.add_message("msg001.hl7", "MSH|^~\\&|...", "ADT^A01", 0);
//!
//! let json = manifest.to_json().unwrap();
//! let parsed = CorpusManifest::from_json(&json).unwrap();
//! assert_eq!(parsed.seed, 42);
//! ```

use hl7v2_core::Message;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use rand::{SeedableRng, Rng};

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
    pub fn add_message(&mut self, path: &str, content: &str, message_type: &str, template_index: usize) {
        let sha256 = compute_sha256(content);
        self.messages.push(MessageInfo {
            path: path.to_string(),
            sha256,
            message_type: message_type.to_string(),
            template_index,
        });
    }

    /// Serialize the manifest to JSON
    pub fn to_json(&self) -> Result<String, CorpusError> {
        serde_json::to_string_pretty(self)
            .map_err(|e| CorpusError::SerializationError(e.to_string()))
    }

    /// Deserialize a manifest from JSON
    pub fn from_json(json: &str) -> Result<Self, CorpusError> {
        serde_json::from_str(json)
            .map_err(|e| CorpusError::SerializationError(e.to_string()))
    }

    /// Get the total number of messages
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Get message types and their counts
    pub fn message_type_counts(&self) -> HashMap<String, usize> {
        let mut counts = HashMap::new();
        for msg in &self.messages {
            *counts.entry(msg.message_type.clone()).or_insert(0) += 1;
        }
        counts
    }

    /// Create train/validation/test splits
    pub fn create_splits(&mut self, ratios: (f64, f64, f64)) {
        let total = self.messages.len();
        if total == 0 {
            return;
        }

        let train_count = (total as f64 * ratios.0).round() as usize;
        let val_count = (total as f64 * ratios.1).round() as usize;
        
        // Shuffle indices based on seed for reproducibility
        let mut rng = rand::rngs::StdRng::seed_from_u64(self.seed);
        let mut indices: Vec<usize> = (0..total).collect();
        
        // Fisher-Yates shuffle
        for i in (1..total).rev() {
            let j = rng.gen_range(0..=i);
            indices.swap(i, j);
        }

        self.splits.train = indices[..train_count]
            .iter()
            .map(|&i| self.messages[i].path.clone())
            .collect();
        
        self.splits.validation = indices[train_count..train_count + val_count]
            .iter()
            .map(|&i| self.messages[i].path.clone())
            .collect();
        
        self.splits.test = indices[train_count + val_count..]
            .iter()
            .map(|&i| self.messages[i].path.clone())
            .collect();
    }
}

/// Compute SHA-256 hash of a string
pub fn compute_sha256(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let hash_result = hasher.finalize();
    format!("{:x}", hash_result)
}

/// Compute SHA-256 hash of a message
pub fn compute_message_hash(message: &Message) -> String {
    let message_bytes = hl7v2_core::write(message);
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
            if segment.fields.len() > 7 {
                let field = &segment.fields[7]; // MSH.9
                if !field.reps.is_empty() {
                    let rep = &field.reps[0];
                    if !rep.comps.is_empty() {
                        // Build the message type from components
                        let parts: Vec<String> = rep.comps.iter()
                            .filter_map(|c| {
                                if c.subs.is_empty() {
                                    None
                                } else if let hl7v2_core::Atom::Text(t) = &c.subs[0] {
                                    Some(t.clone())
                                } else {
                                    None
                                }
                            })
                            .collect();
                        return parts.join("^");
                    }
                }
            }
        }
    }
    "UNKNOWN".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_corpus_config_default() {
        let config = CorpusConfig::default();
        assert_eq!(config.seed, 42);
        assert_eq!(config.count, 100);
        assert_eq!(config.batch_size, 50);
        assert!(!config.create_splits);
    }
    
    #[test]
    fn test_corpus_manifest_new() {
        let manifest = CorpusManifest::new(42);
        assert_eq!(manifest.seed, 42);
        assert_eq!(manifest.version, "1.0.0");
        assert!(manifest.templates.is_empty());
        assert!(manifest.messages.is_empty());
    }
    
    #[test]
    fn test_corpus_manifest_add_template() {
        let mut manifest = CorpusManifest::new(42);
        manifest.add_template("test.yaml", "content");
        
        assert_eq!(manifest.templates.len(), 1);
        assert_eq!(manifest.templates[0].path, "test.yaml");
        assert_eq!(manifest.templates[0].sha256.len(), 64);
    }
    
    #[test]
    fn test_corpus_manifest_add_message() {
        let mut manifest = CorpusManifest::new(42);
        manifest.add_message("msg001.hl7", "MSH|...", "ADT^A01", 0);
        
        assert_eq!(manifest.messages.len(), 1);
        assert_eq!(manifest.messages[0].path, "msg001.hl7");
        assert_eq!(manifest.messages[0].message_type, "ADT^A01");
    }
    
    #[test]
    fn test_corpus_manifest_message_type_counts() {
        let mut manifest = CorpusManifest::new(42);
        manifest.add_message("msg001.hl7", "content1", "ADT^A01", 0);
        manifest.add_message("msg002.hl7", "content2", "ADT^A01", 0);
        manifest.add_message("msg003.hl7", "content3", "ORU^R01", 0);
        
        let counts = manifest.message_type_counts();
        assert_eq!(*counts.get("ADT^A01").unwrap(), 2);
        assert_eq!(*counts.get("ORU^R01").unwrap(), 1);
    }
    
    #[test]
    fn test_corpus_manifest_json_roundtrip() {
        let mut manifest = CorpusManifest::new(42);
        manifest.add_template("test.yaml", "content");
        manifest.add_message("msg001.hl7", "MSH|...", "ADT^A01", 0);
        
        let json = manifest.to_json().unwrap();
        let parsed = CorpusManifest::from_json(&json).unwrap();
        
        assert_eq!(parsed.seed, manifest.seed);
        assert_eq!(parsed.templates.len(), manifest.templates.len());
        assert_eq!(parsed.messages.len(), manifest.messages.len());
    }
    
    #[test]
    fn test_compute_sha256() {
        let hash = compute_sha256("test content");
        assert_eq!(hash.len(), 64);
        // SHA-256 is deterministic
        assert_eq!(hash, compute_sha256("test content"));
    }
    
    #[test]
    fn test_corpus_manifest_create_splits() {
        let mut manifest = CorpusManifest::new(42);
        
        // Add some messages
        for i in 0..20 {
            manifest.add_message(
                &format!("msg{:03}.hl7", i),
                &format!("content{}", i),
                "ADT^A01",
                0
            );
        }
        
        manifest.create_splits((0.7, 0.15, 0.15));
        
        // Check that splits were created
        assert!(!manifest.splits.train.is_empty());
        assert!(!manifest.splits.validation.is_empty());
        assert!(!manifest.splits.test.is_empty());
        
        // Total should equal original count
        let total = manifest.splits.train.len() 
            + manifest.splits.validation.len() 
            + manifest.splits.test.len();
        assert_eq!(total, 20);
    }
    
    #[test]
    fn test_corpus_manifest_empty_splits() {
        let mut manifest = CorpusManifest::new(42);
        manifest.create_splits((0.7, 0.15, 0.15));
        
        // Should not panic on empty manifest
        assert!(manifest.splits.train.is_empty());
        assert!(manifest.splits.validation.is_empty());
        assert!(manifest.splits.test.is_empty());
    }
    
    #[test]
    fn test_template_info_serialization() {
        let info = TemplateInfo {
            path: "test.yaml".to_string(),
            sha256: "abc123".to_string(),
        };
        
        let json = serde_json::to_string(&info).unwrap();
        let parsed: TemplateInfo = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed.path, info.path);
        assert_eq!(parsed.sha256, info.sha256);
    }
    
    #[test]
    fn test_message_info_serialization() {
        let info = MessageInfo {
            path: "msg001.hl7".to_string(),
            sha256: "def456".to_string(),
            message_type: "ADT^A01".to_string(),
            template_index: 0,
        };
        
        let json = serde_json::to_string(&info).unwrap();
        let parsed: MessageInfo = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed.path, info.path);
        assert_eq!(parsed.message_type, info.message_type);
        assert_eq!(parsed.template_index, info.template_index);
    }
    
    #[test]
    fn test_corpus_splits_default() {
        let splits = CorpusSplits::default();
        assert!(splits.train.is_empty());
        assert!(splits.validation.is_empty());
        assert!(splits.test.is_empty());
    }
    
    #[test]
    fn test_corpus_error_display() {
        let err = CorpusError::SerializationError("test error".to_string());
        assert!(err.to_string().contains("test error"));
        
        let err = CorpusError::IoError("file not found".to_string());
        assert!(err.to_string().contains("file not found"));
        
        let err = CorpusError::InvalidConfig("bad config".to_string());
        assert!(err.to_string().contains("bad config"));
    }
}
