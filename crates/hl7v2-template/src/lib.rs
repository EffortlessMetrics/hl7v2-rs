//! HL7 v2 template-based message generation.
//!
//! This crate provides functionality for generating synthetic HL7 v2
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
//! golden hash verification), see the [`hl7v2_corpus`] crate which is
//! re-exported here for convenience.
//!
//! # Example
//!
//! ```
//! use hl7v2_template::{Template, ValueSource, generate};
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

use hl7v2_core::{Message, Delims, Error, Segment, Field, Rep, Comp, Atom};
use serde::{Deserialize, Serialize};
use rand::{rngs::StdRng, RngExt, SeedableRng};
pub use hl7v2_template_values::ValueSource;
use hl7v2_template_values::generate_value;
use std::collections::HashMap;
use sha2::{Sha256, Digest};

// Re-export corpus types for backward compatibility
pub use hl7v2_corpus::{
    CorpusConfig, CorpusManifest, CorpusSplits, CorpusError,
    TemplateInfo, ProfileInfo, MessageInfo,
    compute_sha256, compute_message_hash, extract_message_type,
};

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
fn generate_single_message(template: &Template, rng: &mut StdRng, _index: usize) -> Result<Message, Error> {
    // Parse delimiters
    let delims = parse_delimiters(&template.delims)?;
    
    // Generate segments
    let mut segments = Vec::new();
    
    for segment_template in &template.segments {
        let segment = generate_segment(segment_template, &template.values, &delims, rng)?;
        segments.push(segment);
    }
    
    Ok(Message { delims, segments, charsets: vec![] })
}

/// Parse delimiters from a string
fn parse_delimiters(delims_str: &str) -> Result<Delims, Error> {
    if delims_str.len() != 4 {
        return Err(Error::BadDelimLength);
    }
    
    let chars: Vec<char> = delims_str.chars().collect();
    
    // Check that all delimiters are distinct
    let delimiters = [chars[0], chars[1], chars[2], chars[3]];
    for i in 0..delimiters.len() {
        for j in (i + 1)..delimiters.len() {
            if delimiters[i] == delimiters[j] {
                return Err(Error::DuplicateDelims);
            }
        }
    }
    
    Ok(Delims {
        field: '|', // Field separator is always |
        comp: chars[0],
        rep: chars[1],
        esc: chars[2],
        sub: chars[3],
    })
}

/// Generate a segment from a template
fn generate_segment(segment_template: &str, values: &HashMap<String, Vec<ValueSource>>, delims: &Delims, rng: &mut StdRng) -> Result<Segment, Error> {
    // Split the segment into ID and fields
    let parts: Vec<&str> = segment_template.split('|').collect();
    if parts.is_empty() {
        return Err(Error::InvalidSegmentId);
    }
    
    // Parse segment ID
    let id_str = parts[0];
    if id_str.len() != 3 {
        return Err(Error::InvalidSegmentId);
    }
    
    let id_bytes = id_str.as_bytes();
    let mut id = [0u8; 3];
    id.copy_from_slice(&id_bytes[0..3]);
    
    // Ensure segment ID is all uppercase ASCII letters or digits
    for &byte in &id {
        if !((byte >= b'A' && byte <= b'Z') || (byte >= b'0' && byte <= b'9')) {
            return Err(Error::InvalidSegmentId);
        }
    }
    
    // Generate fields
    let mut fields = Vec::new();
    
    // For MSH segment, we need special handling
    if id_str == "MSH" {
        // MSH segment has special format: MSH|^~\&|...
        // The second field (MSH-2) is the encoding characters
        if parts.len() > 1 {
            // Add the encoding characters field
            let encoding_field = generate_field(&parts[1], values, &format!("MSH.2"), delims, rng)?;
            fields.push(encoding_field);
        }
        
        // Process remaining fields starting from MSH-3
        for (i, field_template) in parts.iter().enumerate().skip(2) {
            let field_path = format!("MSH.{}", i + 1);
            let field = generate_field(field_template, values, &field_path, delims, rng)?;
            fields.push(field);
        }
    } else {
        // For other segments, process all fields
        for (i, field_template) in parts.iter().enumerate().skip(1) {
            let field_path = format!("{}.{}", id_str, i + 1);
            let field = generate_field(field_template, values, &field_path, delims, rng)?;
            fields.push(field);
        }
    }
    
    Ok(Segment { id, fields })
}

/// Generate a field from a template
fn generate_field(field_template: &str, values: &HashMap<String, Vec<ValueSource>>, field_path: &str, delims: &Delims, rng: &mut StdRng) -> Result<Field, Error> {
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
fn generate_rep(rep_template: &str, values: &HashMap<String, Vec<ValueSource>>, field_path: &str, delims: &Delims, rng: &mut StdRng) -> Result<Rep, Error> {
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
fn generate_comp(comp_template: &str, values: &HashMap<String, Vec<ValueSource>>, field_path: &str, delims: &Delims, rng: &mut StdRng) -> Result<Comp, Error> {
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
fn generate_atom(atom_template: &str, values: &HashMap<String, Vec<ValueSource>>, field_path: &str, rng: &mut StdRng) -> Result<Atom, Error> {
    // Check if this field has a value source defined in the template
    if let Some(value_sources) = values.get(field_path) {
        if !value_sources.is_empty() {
            // Use the first value source for now (in a real implementation, we might cycle through them)
            let value_source = &value_sources[0];
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
pub fn generate_corpus(template: &Template, seed: u64, count: usize, batch_size: usize) -> Result<Vec<Message>, Error> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut messages = Vec::with_capacity(count);
    
    // Generate messages in batches to manage memory usage
    let mut generated = 0;
    while generated < count {
        let batch_count = std::cmp::min(batch_size, count - generated);
        for _ in 0..batch_count {
            let message = generate_single_message(template, &mut rng, generated)?;
            messages.push(message);
            generated += 1;
        }
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
pub fn generate_diverse_corpus(templates: &[Template], seed: u64, count: usize) -> Result<Vec<Message>, Error> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut messages = Vec::with_capacity(count);
    
    for i in 0..count {
        // Select a random template
        let template_index = rng.random_range(0..templates.len());
        let template = &templates[template_index];
        
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
pub fn generate_distributed_corpus(
    template_distributions: &[(Template, f64)],
    seed: u64,
    count: usize
) -> Result<Vec<Message>, Error> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut messages = Vec::with_capacity(count);
    
    // Normalize percentages to ensure they sum to 1.0
    let total_percentage: f64 = template_distributions.iter().map(|(_, p)| *p).sum();
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
            .unwrap_or(&normalized_distributions.last().unwrap().0);
        
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
pub fn generate_golden_hashes(
    template: &Template,
    seed: u64,
    count: usize
) -> Result<Vec<String>, Error> {
    // Generate messages
    let messages = generate(template, seed, count)?;
    
    // Calculate hash for each message
    let mut hashes = Vec::with_capacity(count);
    for message in messages.iter() {
        // Convert message to string
        let message_string = hl7v2_core::write(message);
        
        // Calculate SHA-256 hash
        let mut hasher = Sha256::new();
        hasher.update(&message_string);
        let hash_result = hasher.finalize();
        let hash_hex = format!("{:x}", hash_result);
        
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
pub fn verify_golden_hashes(
    template: &Template,
    seed: u64,
    count: usize,
    expected_hashes: &[String]
) -> Result<Vec<bool>, Error> {
    // Generate messages
    let messages = generate(template, seed, count)?;
    
    // Verify each message against its expected hash
    let mut results = Vec::with_capacity(count);
    for (i, message) in messages.iter().enumerate() {
        if i < expected_hashes.len() {
            // Convert message to string
            let message_string = hl7v2_core::write(message);
            
            // Calculate SHA-256 hash
            let mut hasher = Sha256::new();
            hasher.update(&message_string);
            let hash_result = hasher.finalize();
            let hash_hex = format!("{:x}", hash_result);
            
            // Compare with expected hash
            results.push(hash_hex == expected_hashes[i]);
        } else {
            // No expected hash provided for this message
            results.push(false);
        }
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
) -> hl7v2_corpus::CorpusManifest {
    let mut manifest = hl7v2_corpus::CorpusManifest::new(seed);
    
    // Add templates
    for (path, template) in templates {
        let template_json = serde_json::to_string(template).unwrap_or_default();
        manifest.add_template(path, &template_json);
    }
    
    // Add messages
    for (i, message) in messages.iter().enumerate() {
        let content = hl7v2_core::write(message);
        let content_str = String::from_utf8_lossy(&content);
        let path = format!("{}/message_{:06}.hl7", base_path, i + 1);
        
        // Extract message type from MSH.9 if available
        let message_type = extract_message_type(message);
        
        manifest.add_message(&path, &content_str, &message_type, 0);
    }
    
    manifest
}

#[cfg(test)]
mod tests;
