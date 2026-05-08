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

use crate::model::{Atom, Field, Message};
use crate::parser::{parse, parse_mllp};
use crate::transport::mllp::is_mllp_framed;
use crate::writer::write;
use chrono::{DateTime, Utc};
use rand::{RngExt, SeedableRng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

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

/// Count for a corpus-level string dimension.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorpusCount {
    /// Dimension value, such as a message type or segment ID.
    pub value: String,
    /// Number of times the value was observed.
    pub count: usize,
}

/// Field-presence count across a corpus.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorpusFieldPresence {
    /// HL7 path, such as `PID.3` or `OBX.5`.
    pub path: String,
    /// Number of parsed messages where this field path was present.
    pub message_count: usize,
    /// Number of field occurrences across all parsed messages.
    pub occurrence_count: usize,
}

/// Parse failure captured while summarizing a corpus.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorpusParseFailure {
    /// File path relative to the summarized root when possible.
    pub path: String,
    /// Parser error string for this file.
    pub error: String,
}

/// Summary of a directory or file corpus of HL7 v2 messages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorpusSummary {
    /// Root path that was summarized.
    pub root: String,
    /// Number of regular files scanned.
    pub file_count: usize,
    /// Number of messages parsed successfully.
    pub message_count: usize,
    /// Number of files that could not be parsed as HL7 v2.
    pub parse_error_count: usize,
    /// Total input bytes across scanned files.
    pub total_bytes: usize,
    /// Message type counts from parsed MSH-9 values.
    pub message_types: Vec<CorpusCount>,
    /// Segment ID counts across parsed messages.
    pub segments: Vec<CorpusCount>,
    /// Field presence counts across parsed messages.
    pub field_presence: Vec<CorpusFieldPresence>,
    /// Per-file parse failures.
    pub parse_errors: Vec<CorpusParseFailure>,
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

/// Summarize a file or directory of HL7 v2 messages.
///
/// Directories are scanned recursively. Each regular file is read and parsed as
/// plain HL7 unless it is MLLP framed. Files that fail to parse are recorded in
/// the returned summary rather than failing the whole operation.
///
/// # Errors
///
/// Returns [`CorpusError::InvalidConfig`] if the path is neither a regular file
/// nor a directory. Returns [`CorpusError::IoError`] if directory traversal or
/// file reading fails.
pub fn summarize_corpus_path(path: impl AsRef<Path>) -> Result<CorpusSummary, CorpusError> {
    let root = path.as_ref();
    let mut files = Vec::new();
    collect_corpus_files(root, &mut files)?;
    files.sort();

    let mut message_type_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut segment_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut field_message_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut field_occurrence_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut parse_errors = Vec::new();
    let mut total_bytes = 0usize;
    let mut message_count = 0usize;

    for file in &files {
        let relative_path = relative_corpus_path(root, file);
        let bytes =
            fs::read(file).map_err(|e| CorpusError::IoError(format!("{relative_path}: {e}")))?;
        total_bytes = total_bytes.saturating_add(bytes.len());

        let parsed = if is_mllp_framed(&bytes) {
            parse_mllp(&bytes)
        } else {
            parse(&bytes)
        };

        match parsed {
            Ok(message) => {
                message_count = message_count.saturating_add(1);
                increment_count(&mut message_type_counts, extract_message_type(&message));
                record_message_shape(
                    &message,
                    &mut segment_counts,
                    &mut field_message_counts,
                    &mut field_occurrence_counts,
                );
            }
            Err(error) => parse_errors.push(CorpusParseFailure {
                path: relative_path,
                error: error.to_string(),
            }),
        }
    }

    let mut field_presence: Vec<CorpusFieldPresence> = field_occurrence_counts
        .into_iter()
        .map(|(path, occurrence_count)| CorpusFieldPresence {
            message_count: field_message_counts.get(&path).copied().unwrap_or_default(),
            path,
            occurrence_count,
        })
        .collect();
    field_presence.sort_by(|left, right| compare_field_paths(&left.path, &right.path));

    Ok(CorpusSummary {
        root: root.to_string_lossy().to_string(),
        file_count: files.len(),
        message_count,
        parse_error_count: parse_errors.len(),
        total_bytes,
        message_types: counts_to_vec(message_type_counts),
        segments: counts_to_vec(segment_counts),
        field_presence,
        parse_errors,
    })
}

fn collect_corpus_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), CorpusError> {
    if path.is_file() {
        files.push(path.to_path_buf());
        return Ok(());
    }

    if !path.is_dir() {
        return Err(CorpusError::InvalidConfig(format!(
            "{} is not a file or directory",
            path.display()
        )));
    }

    for entry in fs::read_dir(path).map_err(|e| CorpusError::IoError(e.to_string()))? {
        let entry = entry.map_err(|e| CorpusError::IoError(e.to_string()))?;
        let child = entry.path();
        if child.is_dir() {
            collect_corpus_files(&child, files)?;
        } else if child.is_file() {
            files.push(child);
        }
    }

    Ok(())
}

fn relative_corpus_path(root: &Path, file: &Path) -> String {
    let relative = if root.is_dir() {
        file.strip_prefix(root).unwrap_or(file)
    } else {
        file.file_name().map(Path::new).unwrap_or(file)
    };
    relative.to_string_lossy().replace('\\', "/")
}

fn record_message_shape(
    message: &Message,
    segment_counts: &mut BTreeMap<String, usize>,
    field_message_counts: &mut BTreeMap<String, usize>,
    field_occurrence_counts: &mut BTreeMap<String, usize>,
) {
    let mut message_field_paths = BTreeSet::new();

    for segment in &message.segments {
        let segment_id = segment.id_str().to_string();
        increment_count(segment_counts, segment_id.clone());

        for (field_index, field) in segment.fields.iter().enumerate() {
            if !field_is_present(field) {
                continue;
            }

            let display_index = if segment_id == "MSH" {
                field_index.saturating_add(2)
            } else {
                field_index.saturating_add(1)
            };
            let path = format!("{segment_id}.{display_index}");
            increment_count(field_occurrence_counts, path.clone());
            message_field_paths.insert(path);
        }
    }

    for path in message_field_paths {
        increment_count(field_message_counts, path);
    }
}

fn field_is_present(field: &Field) -> bool {
    field.reps.iter().any(|rep| {
        rep.comps.iter().any(|comp| {
            comp.subs.iter().any(|atom| match atom {
                Atom::Text(text) => !text.is_empty(),
                Atom::Null => true,
            })
        })
    })
}

fn compare_field_paths(left: &str, right: &str) -> Ordering {
    let (left_segment, left_index) = split_field_path(left);
    let (right_segment, right_index) = split_field_path(right);

    left_segment
        .cmp(right_segment)
        .then(left_index.cmp(&right_index))
        .then(left.cmp(right))
}

fn split_field_path(path: &str) -> (&str, usize) {
    let Some((segment, field)) = path.split_once('.') else {
        return (path, usize::MAX);
    };
    let index = field.parse::<usize>().unwrap_or(usize::MAX);
    (segment, index)
}

fn increment_count(counts: &mut BTreeMap<String, usize>, value: String) {
    let count = counts.entry(value).or_insert(0);
    *count = count.saturating_add(1);
}

fn counts_to_vec(counts: BTreeMap<String, usize>) -> Vec<CorpusCount> {
    counts
        .into_iter()
        .map(|(value, count)| CorpusCount { value, count })
        .collect()
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

#[cfg(test)]
mod summary_tests {
    #![expect(
        clippy::panic,
        reason = "Corpus summary tests fail explicitly on test setup errors."
    )]

    use super::*;

    const ADT_A01: &str = "MSH|^~\\&|SENDAPP|SENDFAC|RECVAPP|RECVFAC|202605080101||ADT^A01|CTRL123|P|2.5\rPID|1||123456^^^HOSP^MR||Doe^John||19700101|M";
    const ORU_R01: &str = "MSH|^~\\&|LAB|LAB|EHR|HOSP|202605080101||ORU^R01|CTRL456|P|2.5\rPID|1||123456^^^HOSP^MR||Doe^John||19700101|M\rOBR|1|ORD1|FILL1|CBC^Complete Blood Count\rOBX|1|NM|718-7^Hemoglobin||13.2|g/dL";

    fn write_message(path: &Path, contents: &str) {
        let result = fs::write(path, contents);
        assert!(result.is_ok(), "test message should be written: {result:?}");
    }

    #[test]
    fn summarize_corpus_path_counts_messages_segments_and_fields() {
        let Ok(dir) = tempfile::tempdir() else {
            panic!("test temp dir should be created");
        };
        write_message(&dir.path().join("adt.hl7"), ADT_A01);
        write_message(&dir.path().join("oru.hl7"), ORU_R01);

        let Ok(summary) = summarize_corpus_path(dir.path()) else {
            panic!("corpus should summarize");
        };

        assert_eq!(summary.file_count, 2);
        assert_eq!(summary.message_count, 2);
        assert_eq!(summary.parse_error_count, 0);
        assert!(
            summary
                .message_types
                .iter()
                .any(|count| count.value == "ADT^A01" && count.count == 1)
        );
        assert!(
            summary
                .message_types
                .iter()
                .any(|count| count.value == "ORU^R01" && count.count == 1)
        );
        assert!(
            summary
                .segments
                .iter()
                .any(|count| count.value == "PID" && count.count == 2)
        );
        assert!(
            summary
                .field_presence
                .iter()
                .any(|field| field.path == "PID.3" && field.message_count == 2)
        );
    }

    #[test]
    fn summarize_corpus_path_records_parse_failures() {
        let Ok(dir) = tempfile::tempdir() else {
            panic!("test temp dir should be created");
        };
        write_message(&dir.path().join("valid.hl7"), ADT_A01);
        write_message(&dir.path().join("invalid.hl7"), "not an hl7 message");

        let Ok(summary) = summarize_corpus_path(dir.path()) else {
            panic!("corpus should summarize");
        };

        assert_eq!(summary.file_count, 2);
        assert_eq!(summary.message_count, 1);
        assert_eq!(summary.parse_error_count, 1);
        assert_eq!(
            summary
                .parse_errors
                .first()
                .map(|failure| failure.path.as_str()),
            Some("invalid.hl7")
        );
    }
}
