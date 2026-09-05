//! HL7 v2 template-based message generation.
//!
//! This module provides functionality for generating synthetic HL7 v2
//! messages based on templates with variable substitution.
//!
//! # Template Structure
//!
//! A [`Template`] defines the structure of HL7 messages to generate:
//! - `name`: A descriptive name for the template
//! - `delims`: The delimiter characters (e.g., "^~\\&")
//! - `segments`: A list of segment templates
//! - `values`: A map of field paths to value sources
//!
//! # Value Sources
//!
//! The [`ValueSource`] enum defines how values are generated:
//! - `Fixed`: A constant value
//! - `From`: A random choice from a list
//! - `Numeric`: A random numeric string
//! - `Date`: A random date within a range
//! - `Gaussian`: A Gaussian-distributed numeric value
//! - `Map`: A value mapped from a key
//! - `UuidV4`: A random UUID v4
//! - `DtmNowUtc`: Current UTC timestamp
//! - Realistic data generators (names, addresses, etc.)
//! - Error injection variants for negative testing
//!
//! # Corpus Generation
//!
//! For corpus generation functionality (batch generation, manifest handling,
//! golden hash verification), see the [`crate::synthetic::corpus`] module which is
//! re-exported here for convenience.
//!
//! # Example
//!
//! ```
//! use hl7v2::synthetic::template::{Template, ValueSource, generate};
//! use std::collections::HashMap;
//!
//! let template = Template {
//!     name: "ADT_A01".to_string(),
//!     delims: "^~\\&".to_string(),
//!     segments: vec![
//!         "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1".to_string(),
//!         "PID|1||123456^^^HOSP^MR||Doe^John".to_string(),
//!     ],
//!     values: HashMap::new(),
//! };
//!
//! let messages = generate(&template, 42, 1).unwrap();
//! assert_eq!(messages.len(), 1);
//! ```

use crate::model::{Atom, Comp, Delims, Error, Field, Message, Rep, Segment};
use crate::synthetic::values::generate_value;
use crate::writer::write;
use rand::{RngExt, SeedableRng, rngs::StdRng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

// Re-export corpus types for backward compatibility
pub use crate::synthetic::corpus::{
    CorpusConfig, CorpusError, CorpusManifest, CorpusSplits, MessageInfo, ProfileInfo,
    TemplateInfo, compute_message_hash, compute_sha256, extract_message_type,
};
pub use crate::synthetic::values::ValueSource;

/// Message template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    /// Name of the template
    pub name: String,
    /// Delimiter characters (component, repetition, escape, subcomponent)
    pub delims: String,
    /// Segment templates
    pub segments: Vec<String>,
    /// Value sources mapped to field paths (e.g., "PID.3" -> [ValueSource::UuidV4])
    #[serde(default)]
    pub values: std::collections::HashMap<String, Vec<ValueSource>>,
}

/// Generate messages from a template
///
/// # Arguments
///
/// * `template` - The template to use for generating messages
/// * `seed` - The random seed for deterministic generation
/// * `count` - The number of messages to generate
///
/// # Returns
///
/// A vector of generated messages
///
/// # Errors
///
/// Returns [`Error`] if delimiters, segment IDs, or value generation fail.
pub fn generate(template: &Template, seed: u64, count: usize) -> Result<Vec<Message>, Error> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut messages = Vec::with_capacity(count);

    for i in 0..count {
        let message = generate_single_message(template, &mut rng, i)?;
        messages.push(message);
    }

    Ok(messages)
}

/// Generate a single message from a template
fn generate_single_message(
    template: &Template,
    rng: &mut StdRng,
    _index: usize,
) -> Result<Message, Error> {
    // Parse delimiters
    let delims = parse_delimiters(&template.delims)?;

    // Generate segments
    let mut segments = Vec::new();

    for segment_template in &template.segments {
        let segment = generate_segment(segment_template, &template.values, &delims, rng)?;
        segments.push(segment);
    }

    Ok(Message {
        delims,
        segments,
        charsets: vec![],
    })
}

/// Parse delimiters from a string
fn parse_delimiters(delims_str: &str) -> Result<Delims, Error> {
    if delims_str.len() != 4 {
        return Err(Error::BadDelimLength);
    }

    let mut chars = delims_str.chars();
    let (Some(comp), Some(rep), Some(esc), Some(sub), None) = (
        chars.next(),
        chars.next(),
        chars.next(),
        chars.next(),
        chars.next(),
    ) else {
        return Err(Error::BadDelimLength);
    };

    // Check that all delimiters are distinct
    if comp == rep || comp == esc || comp == sub || rep == esc || rep == sub || esc == sub {
        return Err(Error::DuplicateDelims);
    }

    Ok(Delims {
        field: '|', // Field separator is always |
        comp,
        rep,
        esc,
        sub,
    })
}

/// Generate a segment from a template
fn generate_segment(
    segment_template: &str,
    values: &HashMap<String, Vec<ValueSource>>,
    delims: &Delims,
    rng: &mut StdRng,
) -> Result<Segment, Error> {
    // Split the segment into ID and fields
    let mut parts = segment_template.split('|');

    // Parse segment ID
    let id_str = parts.next().ok_or(Error::InvalidSegmentId)?;
    if id_str.len() != 3 {
        return Err(Error::InvalidSegmentId);
    }

    let id: [u8; 3] = id_str
        .as_bytes()
        .try_into()
        .map_err(|_err| Error::InvalidSegmentId)?;

    // Ensure segment ID is all uppercase ASCII letters or digits
    for &byte in &id {
        if !(byte.is_ascii_uppercase() || byte.is_ascii_digit()) {
            return Err(Error::InvalidSegmentId);
        }
    }

    // Generate fields
    let mut fields = Vec::new();
    let field_templates: Vec<&str> = parts.collect();

    // For MSH segment, we need special handling
    if id_str == "MSH" {
        // MSH segment has special format: MSH|^~\&|...
        // The second field (MSH-2) is the encoding characters
        if let Some(encoding_template) = field_templates.first() {
            // Add the encoding characters field
            let encoding_field = generate_field(encoding_template, values, "MSH.2", delims, rng)?;
            fields.push(encoding_field);
        }

        // Process remaining fields starting from MSH-3
        for (offset, field_template) in field_templates.iter().enumerate().skip(1) {
            let field_number = offset
                .checked_add(2)
                .ok_or_else(|| Error::InvalidFieldFormat {
                    details: "field number overflow".to_string(),
                })?;
            let field_path = format!("MSH.{field_number}");
            let field = generate_field(field_template, values, &field_path, delims, rng)?;
            fields.push(field);
        }
    } else {
        // For other segments, process all fields. `field_templates` holds the
        // fields after the segment ID, so offset 0 is field 1 (for example,
        // PID-1). MSH is different because MSH-1 is the field separator and
        // MSH-2 is emitted separately above.
        for (offset, field_template) in field_templates.iter().enumerate() {
            let field_number = offset
                .checked_add(1)
                .ok_or_else(|| Error::InvalidFieldFormat {
                    details: "field number overflow".to_string(),
                })?;
            let field_path = format!("{id_str}.{field_number}");
            let field = generate_field(field_template, values, &field_path, delims, rng)?;
            fields.push(field);
        }
    }

    Ok(Segment { id, fields })
}

/// Generate a field from a template
fn generate_field(
    field_template: &str,
    values: &HashMap<String, Vec<ValueSource>>,
    field_path: &str,
    delims: &Delims,
    rng: &mut StdRng,
) -> Result<Field, Error> {
    // Split repetitions
    let rep_templates: Vec<&str> = field_template.split(delims.rep).collect();
    let mut reps = Vec::new();

    for rep_template in rep_templates {
        let rep = generate_rep(rep_template, values, field_path, delims, rng)?;
        reps.push(rep);
    }

    Ok(Field { reps })
}

/// Generate a repetition from a template
fn generate_rep(
    rep_template: &str,
    values: &HashMap<String, Vec<ValueSource>>,
    field_path: &str,
    delims: &Delims,
    rng: &mut StdRng,
) -> Result<Rep, Error> {
    // Split components
    let comp_templates: Vec<&str> = rep_template.split(delims.comp).collect();
    let mut comps = Vec::new();

    for comp_template in comp_templates {
        let comp = generate_comp(comp_template, values, field_path, delims, rng)?;
        comps.push(comp);
    }

    Ok(Rep { comps })
}

/// Generate a component from a template
fn generate_comp(
    comp_template: &str,
    values: &HashMap<String, Vec<ValueSource>>,
    field_path: &str,
    delims: &Delims,
    rng: &mut StdRng,
) -> Result<Comp, Error> {
    // Split subcomponents
    let sub_templates: Vec<&str> = comp_template.split(delims.sub).collect();
    let mut subs = Vec::new();

    for sub_template in sub_templates {
        let atom = generate_atom(sub_template, values, field_path, rng)?;
        subs.push(atom);
    }

    Ok(Comp { subs })
}

/// Generate an atom from a template
fn generate_atom(
    atom_template: &str,
    values: &HashMap<String, Vec<ValueSource>>,
    field_path: &str,
    rng: &mut StdRng,
) -> Result<Atom, Error> {
    // Check if this field has a value source defined in the template
    if let Some(value_sources) = values.get(field_path)
        && !value_sources.is_empty()
    {
        // Use the first value source for now (in a real implementation, we might cycle through them)
        if let Some(value_source) = value_sources.first() {
            let value = generate_value(value_source, rng)?;
            return Ok(Atom::Text(value));
        }
    }

    // If no value source is defined, use the template text as-is
    Ok(Atom::Text(atom_template.to_string()))
}

/// Generate a corpus of messages
///
/// This function generates a large set of HL7 messages with varying characteristics
/// for testing and benchmarking purposes.
///
/// # Arguments
///
/// * `template` - The template to use for generating messages
/// * `seed` - The random seed for deterministic generation
/// * `count` - The number of messages to generate
/// * `batch_size` - The number of messages to generate in each batch
///
/// # Returns
///
/// A vector of generated messages
///
/// # Errors
///
/// Returns [`Error`] if any generated message fails validation while being
/// rendered from the template.
pub fn generate_corpus(
    template: &Template,
    seed: u64,
    count: usize,
    batch_size: usize,
) -> Result<Vec<Message>, Error> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut messages = Vec::with_capacity(count);

    let effective_batch_size = batch_size.max(1);
    let mut remaining = count;
    while remaining > 0 {
        let batch_count = std::cmp::min(effective_batch_size, remaining);
        for _ in 0..batch_count {
            let message = generate_single_message(template, &mut rng, messages.len())?;
            messages.push(message);
        }
        remaining = remaining.saturating_sub(batch_count);
    }

    Ok(messages)
}

/// Generate a diverse corpus with different message types
///
/// This function generates a corpus with different types of HL7 messages
/// (ADT, ORU, etc.) to provide comprehensive testing data.
///
/// # Arguments
///
/// * `templates` - A vector of templates to use for generating messages
/// * `seed` - The random seed for deterministic generation
/// * `count` - The number of messages to generate
///
/// # Returns
///
/// A vector of generated messages with different types
///
/// # Errors
///
/// Returns [`Error`] if no templates are provided or if any selected template
/// cannot generate a message.
pub fn generate_diverse_corpus(
    templates: &[Template],
    seed: u64,
    count: usize,
) -> Result<Vec<Message>, Error> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut messages = Vec::with_capacity(count);

    if templates.is_empty() {
        return Err(Error::InvalidFieldFormat {
            details: "at least one template is required".to_string(),
        });
    }

    for i in 0..count {
        // Select a random template
        let template_index = rng.random_range(0..templates.len());
        let Some(template) = templates.get(template_index) else {
            return Err(Error::InvalidFieldFormat {
                details: "template index out of range".to_string(),
            });
        };

        let message = generate_single_message(template, &mut rng, i)?;
        messages.push(message);
    }

    Ok(messages)
}

/// Generate a corpus with specific distributions
///
/// This function generates a corpus with specific distributions of message characteristics
/// (e.g., specific percentages of different message types, error rates, etc.)
///
/// # Arguments
///
/// * `template_distributions` - A vector of (template, percentage) pairs
/// * `seed` - The random seed for deterministic generation
/// * `count` - The number of messages to generate
///
/// # Returns
///
/// A vector of generated messages following the specified distributions
///
/// # Errors
///
/// Returns [`Error`] if no distributions are provided, the distribution total is
/// invalid, or generation from a selected template fails.
pub fn generate_distributed_corpus(
    template_distributions: &[(Template, f64)],
    seed: u64,
    count: usize,
) -> Result<Vec<Message>, Error> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut messages = Vec::with_capacity(count);

    if template_distributions.is_empty() {
        return Err(Error::InvalidFieldFormat {
            details: "at least one template distribution is required".to_string(),
        });
    }

    // Normalize percentages to ensure they sum to 1.0
    let total_percentage: f64 = template_distributions.iter().map(|(_, p)| *p).sum();
    if !total_percentage.is_finite() || total_percentage <= 0.0 {
        return Err(Error::InvalidFieldFormat {
            details: "template distribution total must be positive".to_string(),
        });
    }
    let normalized_distributions: Vec<(Template, f64)> = template_distributions
        .iter()
        .map(|(t, p)| (t.clone(), p / total_percentage))
        .collect();

    // Create cumulative distribution
    let mut cumulative_distribution = Vec::new();
    let mut cumulative = 0.0;
    for (template, percentage) in &normalized_distributions {
        cumulative += percentage;
        cumulative_distribution.push((template.clone(), cumulative));
    }

    for i in 0..count {
        // Select template based on distribution
        let random_value = rng.random_range(0.0..1.0);
        let template = cumulative_distribution
            .iter()
            .find(|(_, cumulative)| random_value <= *cumulative)
            .map(|(t, _)| t)
            .or_else(|| {
                normalized_distributions
                    .last()
                    .map(|(template, _)| template)
            })
            .ok_or_else(|| Error::InvalidFieldFormat {
                details: "no template distribution selected".to_string(),
            })?;

        let message = generate_single_message(template, &mut rng, i)?;
        messages.push(message);
    }

    Ok(messages)
}

/// Generate golden hash values for a template
///
/// This function generates messages and returns their SHA-256 hash values
/// for use as golden hashes in future verification.
///
/// # Arguments
///
/// * `template` - The template to use for generating messages
/// * `seed` - The random seed for deterministic generation
/// * `count` - The number of messages to generate
///
/// # Returns
///
/// A vector of SHA-256 hash values for the generated messages
///
/// # Errors
///
/// Returns [`Error`] if template generation fails.
pub fn generate_golden_hashes(
    template: &Template,
    seed: u64,
    count: usize,
) -> Result<Vec<String>, Error> {
    // Generate messages
    let messages = generate(template, seed, count)?;

    // Calculate hash for each message
    let mut hashes = Vec::with_capacity(count);
    for message in &messages {
        // Convert message to string
        let message_string = write(message);

        // Calculate SHA-256 hash
        let mut hasher = Sha256::new();
        hasher.update(&message_string);
        let hash_result = hasher.finalize();
        let hash_hex = format!("{hash_result:x}");

        hashes.push(hash_hex);
    }

    Ok(hashes)
}

/// Verify that generated messages match expected hash values
///
/// This function generates messages and verifies that their SHA-256 hashes
/// match the expected golden hash values. This is useful for testing and
/// validation purposes.
///
/// # Arguments
///
/// * `template` - The template to use for generating messages
/// * `seed` - The random seed for deterministic generation
/// * `count` - The number of messages to generate
/// * `expected_hashes` - A vector of expected SHA-256 hash values
///
/// # Returns
///
/// A vector of booleans indicating whether each message's hash matches the expected hash
///
/// # Errors
///
/// Returns [`Error`] if template generation fails.
pub fn verify_golden_hashes(
    template: &Template,
    seed: u64,
    count: usize,
    expected_hashes: &[String],
) -> Result<Vec<bool>, Error> {
    // Generate messages
    let messages = generate(template, seed, count)?;

    // Verify each message against its expected hash
    let mut results = Vec::with_capacity(count);
    for (i, message) in messages.iter().enumerate() {
        // Convert message to string
        let message_string = write(message);

        // Calculate SHA-256 hash
        let mut hasher = Sha256::new();
        hasher.update(&message_string);
        let hash_result = hasher.finalize();
        let hash_hex = format!("{hash_result:x}");

        // Compare with expected hash
        results.push(
            expected_hashes
                .get(i)
                .is_some_and(|expected| hash_hex == *expected),
        );
    }

    Ok(results)
}

/// Create a corpus manifest from generated messages
///
/// This function creates a manifest tracking all generated messages,
/// their hashes, and the templates used.
///
/// # Arguments
///
/// * `seed` - The random seed used for generation
/// * `templates` - The templates used for generation with their paths
/// * `messages` - The generated messages
/// * `base_path` - The base path for message files
///
/// # Returns
///
/// A `CorpusManifest` tracking all corpus metadata
pub fn create_manifest(
    seed: u64,
    templates: &[(String, Template)],
    messages: &[Message],
    base_path: &str,
) -> CorpusManifest {
    let mut manifest = CorpusManifest::new(seed);

    // Add templates
    for (path, template) in templates {
        let template_json = serde_json::to_string(template).unwrap_or_default();
        manifest.add_template(path, &template_json);
    }

    // Add messages
    for (i, message) in messages.iter().enumerate() {
        let content = write(message);
        let content_str = String::from_utf8_lossy(&content);
        let message_number = i.saturating_add(1);
        let path = format!("{base_path}/message_{message_number:06}.hl7");

        // Extract message type from MSH.9 if available
        let message_type = extract_message_type(message);

        manifest.add_message(&path, &content_str, &message_type, 0);
    }

    manifest
}

#[cfg(test)]
mod tests {
    use super::{Template, ValueSource, generate};
    use std::collections::HashMap;

    #[test]
    fn value_source_maps_to_named_non_msh_field() -> Result<(), Box<dyn std::error::Error>> {
        let mut values: HashMap<String, Vec<ValueSource>> = HashMap::new();
        values.insert(
            "PID.3".to_string(),
            vec![ValueSource::Fixed("MRNVAL".to_string())],
        );
        let template = Template {
            name: "regression".to_string(),
            delims: "^~\\&".to_string(),
            segments: vec![
                "MSH|^~\\&|APP".to_string(),
                "PID|1||ORIGMRN||Doe^John".to_string(),
            ],
            values,
        };

        let messages = generate(&template, 42, 1)?;
        let msg = messages
            .first()
            .ok_or_else(|| std::io::Error::other("expected one generated message"))?;

        assert_eq!(crate::get(msg, "PID.3"), Some("MRNVAL"));
        assert_ne!(crate::get(msg, "PID.2"), Some("MRNVAL"));
        Ok(())
    }
}
