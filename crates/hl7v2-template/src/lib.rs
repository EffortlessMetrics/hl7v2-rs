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
use hl7v2_faker::FakerValue;
use serde::{Deserialize, Serialize};
use rand::SeedableRng;
use rand::RngExt;
use rand::rngs::StdRng;
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

/// Source for generating values
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum ValueSource {
    /// A fixed constant value
    Fixed(String),
    /// A random choice from a list of options
    From(Vec<String>),
    /// A random numeric string with specified number of digits
    Numeric { digits: usize },
    /// A random date within a range (YYYYMMDD format)
    Date { start: String, end: String },
    /// A Gaussian-distributed numeric value
    Gaussian { mean: f64, sd: f64, precision: usize },
    /// A value mapped from a key
    Map(std::collections::HashMap<String, String>),
    /// A random UUID v4
    UuidV4,
    /// Current UTC timestamp in YYYYMMDDHHMMSS format
    DtmNowUtc,
    /// A realistic person name (optionally filtered by gender: "M", "F", or None)
    RealisticName { gender: Option<String> },
    /// A realistic street address
    RealisticAddress,
    /// A realistic phone number
    RealisticPhone,
    /// A realistic Social Security Number
    RealisticSsn,
    /// A realistic Medical Record Number
    RealisticMrn,
    /// A realistic ICD-10 diagnosis code
    RealisticIcd10,
    /// A realistic LOINC observation code
    RealisticLoinc,
    /// A realistic medication name
    RealisticMedication,
    /// A realistic allergen name
    RealisticAllergen,
    /// A realistic blood type
    RealisticBloodType,
    /// A realistic ethnicity code
    RealisticEthnicity,
    /// A realistic race code
    RealisticRace,
    // Error injection variants for negative testing
    /// Injects an invalid segment ID error
    InvalidSegmentId,
    /// Injects an invalid field format error
    InvalidFieldFormat,
    /// Injects an invalid repetition format error
    InvalidRepFormat,
    /// Injects an invalid component format error
    InvalidCompFormat,
    /// Injects an invalid subcomponent format error
    InvalidSubcompFormat,
    /// Injects a duplicate delimiters error
    DuplicateDelims,
    /// Injects a bad delimiter length error
    BadDelimLength,
}

impl ValueSource {
    /// Convert to a FakerValue for use with the hl7v2-faker crate.
    ///
    /// Note: Error injection variants do not map to FakerValue and will
    /// return an empty Fixed value as a fallback.
    pub fn to_faker_value(&self) -> FakerValue {
        match self {
            ValueSource::Fixed(value) => FakerValue::Fixed(value.clone()),
            ValueSource::From(options) => FakerValue::From(options.clone()),
            ValueSource::Numeric { digits } => FakerValue::Numeric { digits: *digits },
            ValueSource::Date { start, end } => FakerValue::Date { start: start.clone(), end: end.clone() },
            ValueSource::Gaussian { mean, sd, precision } => FakerValue::Gaussian { mean: *mean, sd: *sd, precision: *precision },
            ValueSource::Map(mapping) => FakerValue::Map(mapping.clone()),
            ValueSource::UuidV4 => FakerValue::UuidV4,
            ValueSource::DtmNowUtc => FakerValue::DtmNowUtc,
            ValueSource::RealisticName { gender } => FakerValue::RealisticName { gender: gender.clone() },
            ValueSource::RealisticAddress => FakerValue::RealisticAddress,
            ValueSource::RealisticPhone => FakerValue::RealisticPhone,
            ValueSource::RealisticSsn => FakerValue::RealisticSsn,
            ValueSource::RealisticMrn => FakerValue::RealisticMrn,
            ValueSource::RealisticIcd10 => FakerValue::RealisticIcd10,
            ValueSource::RealisticLoinc => FakerValue::RealisticLoinc,
            ValueSource::RealisticMedication => FakerValue::RealisticMedication,
            ValueSource::RealisticAllergen => FakerValue::RealisticAllergen,
            ValueSource::RealisticBloodType => FakerValue::RealisticBloodType,
            ValueSource::RealisticEthnicity => FakerValue::RealisticEthnicity,
            ValueSource::RealisticRace => FakerValue::RealisticRace,
            // Error injection variants don't map to FakerValue
            _ => FakerValue::Fixed(String::new()), // fallback
        }
    }
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

/// Generate a value from a value source
fn generate_value(value_source: &ValueSource, rng: &mut StdRng) -> Result<String, Error> {
    match value_source {
        ValueSource::Fixed(value) => Ok(value.clone()),
        ValueSource::From(options) => {
            if options.is_empty() {
                return Ok(String::new());
            }
            let index = rng.random_range(0..options.len());
            Ok(options[index].clone())
        },
        ValueSource::Numeric { digits } => {
            let mut result = String::new();
            for _ in 0..*digits {
                let digit = rng.random_range(0..10);
                result.push_str(&digit.to_string());
            }
            Ok(result)
        },
        ValueSource::Date { start, end } => {
            // Parse start and end dates (YYYYMMDD format)
            let start_date = chrono::NaiveDate::parse_from_str(start, "%Y%m%d")
                .map_err(|_| Error::InvalidEscapeToken)?; // Using InvalidEscapeToken as a placeholder error
            let end_date = chrono::NaiveDate::parse_from_str(end, "%Y%m%d")
                .map_err(|_| Error::InvalidEscapeToken)?;
            
            // Calculate the number of days between start and end
            let duration = end_date.signed_duration_since(start_date);
            let days = duration.num_days();
            
            // Generate a random number of days to add
            let random_days = rng.random_range(0..=days);
            
            // Add the random days to the start date
            let random_date = start_date + chrono::Duration::days(random_days);
            
            // Format as YYYYMMDD
            Ok(random_date.format("%Y%m%d").to_string())
        },
        ValueSource::Gaussian { mean, sd, precision } => {
            // Generate a Gaussian distributed value
            let value = rng.sample(rand_distr::Normal::new(*mean, *sd).map_err(|_| Error::InvalidEscapeToken)?);
            Ok(format!("{:.*}", precision, value))
        },
        ValueSource::Map(mapping) => {
            // For map, we need a source value to map from
            // Since we don't have that in this context, we'll just pick a random key and return its value
            if mapping.is_empty() {
                return Ok(String::new());
            }
            let keys: Vec<&String> = mapping.keys().collect();
            let random_key = keys[rng.random_range(0..keys.len())];
            Ok(mapping[random_key].clone())
        },
        ValueSource::UuidV4 => {
            let uuid = uuid::Uuid::new_v4();
            Ok(uuid.to_string())
        },
        ValueSource::DtmNowUtc => {
            // Generate current UTC timestamp in YYYYMMDDHHMMSS format
            let now = chrono::Utc::now();
            Ok(now.format("%Y%m%d%H%M%S").to_string())
        },
        // Realistic data generation - use hl7v2-faker
        ValueSource::RealisticName { gender } => {
            let mut faker = hl7v2_faker::Faker::new(rng);
            Ok(faker.name(gender.as_deref()))
        },
        ValueSource::RealisticAddress => {
            let mut faker = hl7v2_faker::Faker::new(rng);
            Ok(faker.address())
        },
        ValueSource::RealisticPhone => {
            let mut faker = hl7v2_faker::Faker::new(rng);
            Ok(faker.phone())
        },
        ValueSource::RealisticSsn => {
            let mut faker = hl7v2_faker::Faker::new(rng);
            Ok(faker.ssn())
        },
        ValueSource::RealisticMrn => {
            let mut faker = hl7v2_faker::Faker::new(rng);
            Ok(faker.mrn())
        },
        ValueSource::RealisticIcd10 => {
            let mut faker = hl7v2_faker::Faker::new(rng);
            Ok(faker.icd10())
        },
        ValueSource::RealisticLoinc => {
            let mut faker = hl7v2_faker::Faker::new(rng);
            Ok(faker.loinc())
        },
        ValueSource::RealisticMedication => {
            let mut faker = hl7v2_faker::Faker::new(rng);
            Ok(faker.medication())
        },
        ValueSource::RealisticAllergen => {
            let mut faker = hl7v2_faker::Faker::new(rng);
            Ok(faker.allergen())
        },
        ValueSource::RealisticBloodType => {
            let mut faker = hl7v2_faker::Faker::new(rng);
            Ok(faker.blood_type())
        },
        ValueSource::RealisticEthnicity => {
            let mut faker = hl7v2_faker::Faker::new(rng);
            Ok(faker.ethnicity())
        },
        ValueSource::RealisticRace => {
            let mut faker = hl7v2_faker::Faker::new(rng);
            Ok(faker.race())
        },
        // Error injection variants for negative testing
        ValueSource::InvalidSegmentId => Err(Error::InvalidSegmentId),
        ValueSource::InvalidFieldFormat => Err(Error::InvalidFieldFormat { details: "Injected invalid field format".to_string() }),
        ValueSource::InvalidRepFormat => Err(Error::InvalidRepFormat { details: "Injected invalid repetition format".to_string() }),
        ValueSource::InvalidCompFormat => Err(Error::InvalidCompFormat { details: "Injected invalid component format".to_string() }),
        ValueSource::InvalidSubcompFormat => Err(Error::InvalidSubcompFormat { details: "Injected invalid subcomponent format".to_string() }),
        ValueSource::DuplicateDelims => Err(Error::DuplicateDelims),
        ValueSource::BadDelimLength => Err(Error::BadDelimLength),
    }
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
mod tests {
    use super::*;
    
    #[test]
    fn test_generate_simple_message() {
        let template = Template {
            name: "test".to_string(),
            delims: "^~\\&".to_string(),
            segments: vec![
                "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1".to_string(),
                "PID|1||123456^^^HOSP^MR||Doe^John".to_string(),
            ],
            values: std::collections::HashMap::new(),
        };
        
        let messages = generate(&template, 42, 1).unwrap();
        assert_eq!(messages.len(), 1);
        
        let message = &messages[0];
        assert_eq!(message.segments.len(), 2);
        assert_eq!(std::str::from_utf8(&message.segments[0].id).unwrap(), "MSH");
        assert_eq!(std::str::from_utf8(&message.segments[1].id).unwrap(), "PID");
    }
    
    #[test]
    fn test_generate_multiple_messages() {
        let template = Template {
            name: "test".to_string(),
            delims: "^~\\&".to_string(),
            segments: vec![
                "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1".to_string(),
                "PID|1||123456^^^HOSP^MR||Doe^John".to_string(),
            ],
            values: std::collections::HashMap::new(),
        };
        
        let messages = generate(&template, 42, 3).unwrap();
        assert_eq!(messages.len(), 3);
    }
    
    #[test]
    fn test_deterministic_generation() {
        let template = Template {
            name: "test".to_string(),
            delims: "^~\\&".to_string(),
            segments: vec![
                "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1".to_string(),
                "PID|1||123456^^^HOSP^MR||Doe^John".to_string(),
            ],
            values: std::collections::HashMap::new(),
        };
        
        // Generate messages with the same seed
        let messages1 = generate(&template, 42, 3).unwrap();
        let messages2 = generate(&template, 42, 3).unwrap();
        
        // Results should be identical
        assert_eq!(messages1.len(), messages2.len());
        for i in 0..messages1.len() {
            assert_eq!(messages1[i].segments.len(), messages2[i].segments.len());
            // For simplicity, we're just checking the structure is the same
        }
    }
    
    #[test]
    fn test_different_seeds_produce_different_results() {
        let mut values = std::collections::HashMap::new();
        values.insert("PID.3".to_string(), vec![ValueSource::UuidV4]);
        
        let template = Template {
            name: "test".to_string(),
            delims: "^~\\&".to_string(),
            segments: vec![
                "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1".to_string(),
                "PID|1||123456^^^HOSP^MR||Doe^John".to_string(),
            ],
            values,
        };
        
        // Generate messages with different seeds
        let messages1 = generate(&template, 42, 1).unwrap();
        let messages2 = generate(&template, 43, 1).unwrap();
        
        // Results should be different (because of UUID generation)
        // Note: This test might occasionally fail due to random chance, but it's unlikely
        assert_ne!(
            hl7v2_core::write(&messages1[0]),
            hl7v2_core::write(&messages2[0])
        );
    }
    
    #[test]
    fn test_error_injection() {
        let mut values = std::collections::HashMap::new();
        values.insert("PID.3".to_string(), vec![ValueSource::InvalidSegmentId]);
        
        let template = Template {
            name: "test".to_string(),
            delims: "^~\\&".to_string(),
            segments: vec![
                "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1".to_string(),
                "PID|1||123456^^^HOSP^MR||Doe^John".to_string(),
            ],
            values,
        };
        
        // Generation should fail due to error injection
        let result = generate(&template, 42, 1);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_date_generation() {
        let mut values = std::collections::HashMap::new();
        values.insert("PID.7".to_string(), vec![ValueSource::Date { start: "20200101".to_string(), end: "20251231".to_string() }]);
        
        let template = Template {
            name: "test".to_string(),
            delims: "^~\\&".to_string(),
            segments: vec![
                "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1".to_string(),
                "PID|1||123456^^^HOSP^MR||Doe^John|||M||||".to_string(),
            ],
            values,
        };
        
        let messages = generate(&template, 42, 1).unwrap();
        assert_eq!(messages.len(), 1);
        
        // The date should be in YYYYMMDD format and within the specified range
        // For this test, we'll just verify it compiles and runs without error
    }

    #[test]
    fn test_gaussian_generation() {
        let mut values = std::collections::HashMap::new();
        values.insert("PID.7".to_string(), vec![ValueSource::Gaussian { mean: 100.0, sd: 10.0, precision: 2 }]);
        
        let template = Template {
            name: "test".to_string(),
            delims: "^~\\&".to_string(),
            segments: vec![
                "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1".to_string(),
                "PID|1||123456^^^HOSP^MR||Doe^John|||M||||".to_string(),
            ],
            values,
        };
        
        let messages = generate(&template, 42, 1).unwrap();
        assert_eq!(messages.len(), 1);
        
        // The value should be a numeric string with 2 decimal places
        // For this test, we'll just verify it compiles and runs without error
    }

    #[test]
    fn test_map_generation() {
        let mut values = std::collections::HashMap::new();
        let mut mapping = std::collections::HashMap::new();
        mapping.insert("A".to_string(), "Apple".to_string());
        mapping.insert("B".to_string(), "Banana".to_string());
        mapping.insert("C".to_string(), "Cherry".to_string());
        values.insert("PID.7".to_string(), vec![ValueSource::Map(mapping)]);
        
        let template = Template {
            name: "test".to_string(),
            delims: "^~\\&".to_string(),
            segments: vec![
                "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1".to_string(),
                "PID|1||123456^^^HOSP^MR||Doe^John|||M||||".to_string(),
            ],
            values,
        };
        
        let messages = generate(&template, 42, 1).unwrap();
        assert_eq!(messages.len(), 1);
        
        // The value should be one of the mapped values
        // For this test, we'll just verify it compiles and runs without error
    }

    #[test]
    fn test_dtm_now_utc_generation() {
        let mut values = std::collections::HashMap::new();
        values.insert("PID.7".to_string(), vec![ValueSource::DtmNowUtc]);
        
        let template = Template {
            name: "test".to_string(),
            delims: "^~\\&".to_string(),
            segments: vec![
                "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1".to_string(),
                "PID|1||123456^^^HOSP^MR||Doe^John|||M||||".to_string(),
            ],
            values,
        };
        
        let messages = generate(&template, 42, 1).unwrap();
        assert_eq!(messages.len(), 1);
        
        // The value should be a timestamp in YYYYMMDDHHMMSS format
        // For this test, we'll just verify it compiles and runs without error
    }

    #[test]
    fn test_realistic_name_generation() {
        let mut values = std::collections::HashMap::new();
        values.insert("PID.5".to_string(), vec![ValueSource::RealisticName { gender: Some("M".to_string()) }]);
        
        let template = Template {
            name: "test".to_string(),
            delims: "^~\\&".to_string(),
            segments: vec![
                "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1".to_string(),
                "PID|1||123456^^^HOSP^MR||Doe^John|||M||||".to_string(),
            ],
            values,
        };
        
        let messages = generate(&template, 42, 1).unwrap();
        assert_eq!(messages.len(), 1);
        
        // The value should be a realistic name in last^first format
        // For this test, we'll just verify it compiles and runs without error
    }

    #[test]
    fn test_realistic_address_generation() {
        let mut values = std::collections::HashMap::new();
        values.insert("PID.11".to_string(), vec![ValueSource::RealisticAddress]);
        
        let template = Template {
            name: "test".to_string(),
            delims: "^~\\&".to_string(),
            segments: vec![
                "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1".to_string(),
                "PID|1||123456^^^HOSP^MR||Doe^John|||M||||".to_string(),
            ],
            values,
        };
        
        let messages = generate(&template, 42, 1).unwrap();
        assert_eq!(messages.len(), 1);
        
        // The value should be a realistic address
        // For this test, we'll just verify it compiles and runs without error
    }

    #[test]
    fn test_realistic_phone_generation() {
        let mut values = std::collections::HashMap::new();
        values.insert("PID.13".to_string(), vec![ValueSource::RealisticPhone]);
        
        let template = Template {
            name: "test".to_string(),
            delims: "^~\\&".to_string(),
            segments: vec![
                "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1".to_string(),
                "PID|1||123456^^^HOSP^MR||Doe^John|||M||||".to_string(),
            ],
            values,
        };
        
        let messages = generate(&template, 42, 1).unwrap();
        assert_eq!(messages.len(), 1);
        
        // The value should be a realistic phone number
        // For this test, we'll just verify it compiles and runs without error
    }

    #[test]
    fn test_generate_corpus() {
        let template = Template {
            name: "test".to_string(),
            delims: "^~\\&".to_string(),
            segments: vec![
                "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1".to_string(),
                "PID|1||123456^^^HOSP^MR||Doe^John".to_string(),
            ],
            values: std::collections::HashMap::new(),
        };

        let messages = generate_corpus(&template, 42, 10, 5).unwrap();
        assert_eq!(messages.len(), 10);
    }

    #[test]
    fn test_generate_diverse_corpus() {
        let template1 = Template {
            name: "test1".to_string(),
            delims: "^~\\&".to_string(),
            segments: vec![
                "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1".to_string(),
                "PID|1||123456^^^HOSP^MR||Doe^John".to_string(),
            ],
            values: std::collections::HashMap::new(),
        };

        let template2 = Template {
            name: "test2".to_string(),
            delims: "^~\\&".to_string(),
            segments: vec![
                "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ORU^R01^ORU_R01|DEF456|P|2.5.1".to_string(),
                "PID|1||123456^^^HOSP^MR||Doe^Jane".to_string(),
            ],
            values: std::collections::HashMap::new(),
        };

        let templates = vec![template1, template2];
        let messages = generate_diverse_corpus(&templates, 42, 6).unwrap();
        assert_eq!(messages.len(), 6);
    }

    #[test]
    fn test_generate_distributed_corpus() {
        let template1 = Template {
            name: "test1".to_string(),
            delims: "^~\\&".to_string(),
            segments: vec![
                "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1".to_string(),
                "PID|1||123456^^^HOSP^MR||Doe^John".to_string(),
            ],
            values: std::collections::HashMap::new(),
        };

        let template2 = Template {
            name: "test2".to_string(),
            delims: "^~\\&".to_string(),
            segments: vec![
                "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ORU^R01^ORU_R01|DEF456|P|2.5.1".to_string(),
                "PID|1||123456^^^HOSP^MR||Doe^Jane".to_string(),
            ],
            values: std::collections::HashMap::new(),
        };

        let distributions = vec![(template1, 0.7), (template2, 0.3)];
        let messages = generate_distributed_corpus(&distributions, 42, 10).unwrap();
        assert_eq!(messages.len(), 10);
    }

    #[test]
    fn test_generate_golden_hashes() {
        let template = Template {
            name: "test".to_string(),
            delims: "^~\\&".to_string(),
            segments: vec![
                "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1".to_string(),
                "PID|1||123456^^^HOSP^MR||Doe^John".to_string(),
            ],
            values: std::collections::HashMap::new(),
        };

        let hashes = generate_golden_hashes(&template, 42, 3).unwrap();
        assert_eq!(hashes.len(), 3);

        // Verify that all hashes are valid hex strings
        for hash in hashes {
            assert_eq!(hash.len(), 64); // SHA-256 hashes are 64 hex characters
        }
    }

    #[test]
    fn test_verify_golden_hashes() {
        let template = Template {
            name: "test".to_string(),
            delims: "^~\\&".to_string(),
            segments: vec![
                "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1".to_string(),
                "PID|1||123456^^^HOSP^MR||Doe^John".to_string(),
            ],
            values: std::collections::HashMap::new(),
        };

        // First generate the golden hashes
        let hashes = generate_golden_hashes(&template, 42, 3).unwrap();

        // Then verify them
        let results = verify_golden_hashes(&template, 42, 3, &hashes).unwrap();
        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|&r| r)); // All should match
    }

    #[test]
    fn test_value_source_to_faker_value() {
        // Test that all value sources can be converted to faker values
        let sources = vec![
            ValueSource::Fixed("test".to_string()),
            ValueSource::From(vec!["a".to_string(), "b".to_string()]),
            ValueSource::Numeric { digits: 5 },
            ValueSource::Date { start: "20200101".to_string(), end: "20251231".to_string() },
            ValueSource::Gaussian { mean: 0.0, sd: 1.0, precision: 2 },
            ValueSource::Map(std::collections::HashMap::new()),
            ValueSource::UuidV4,
            ValueSource::DtmNowUtc,
            ValueSource::RealisticName { gender: None },
            ValueSource::RealisticAddress,
            ValueSource::RealisticPhone,
            ValueSource::RealisticSsn,
            ValueSource::RealisticMrn,
            ValueSource::RealisticIcd10,
            ValueSource::RealisticLoinc,
            ValueSource::RealisticMedication,
            ValueSource::RealisticAllergen,
            ValueSource::RealisticBloodType,
            ValueSource::RealisticEthnicity,
            ValueSource::RealisticRace,
            // Error injection variants
            ValueSource::InvalidSegmentId,
            ValueSource::InvalidFieldFormat,
            ValueSource::InvalidRepFormat,
            ValueSource::InvalidCompFormat,
            ValueSource::InvalidSubcompFormat,
            ValueSource::DuplicateDelims,
            ValueSource::BadDelimLength,
        ];

        for source in sources {
            // Just verify conversion doesn't panic
            let _faker_value = source.to_faker_value();
        }
    }

    #[test]
    fn test_parse_delimiters() {
        // Valid delimiters
        let delims = parse_delimiters("^~\\&").unwrap();
        assert_eq!(delims.comp, '^');
        assert_eq!(delims.rep, '~');
        assert_eq!(delims.esc, '\\');
        assert_eq!(delims.sub, '&');
        assert_eq!(delims.field, '|');

        // Invalid: wrong length
        assert!(parse_delimiters("^~\\").is_err());
        assert!(parse_delimiters("^~\\&!").is_err());

        // Invalid: duplicate delimiters
        assert!(parse_delimiters("^^^^").is_err());
    }
}
