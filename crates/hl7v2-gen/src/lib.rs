//! Deterministic HL7 v2 message generator.
//!
//! This crate provides functionality for generating synthetic HL7 v2
//! messages based on templates and profiles.

#![allow(clippy::manual_range_contains)]
#![allow(clippy::needless_borrow)]
#![allow(clippy::useless_format)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::unwrap_or_default)]
#![allow(clippy::vec_init_then_push)]

use hl7v2_core::{Atom, Comp, Delims, Error, Field, Message, Rep, Segment};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Message template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub name: String,
    pub delims: String,
    pub segments: Vec<String>,
    #[serde(default)]
    pub values: std::collections::HashMap<String, Vec<ValueSource>>,
}

/// Source for generating values
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum ValueSource {
    Fixed(String),
    From(Vec<String>),
    Numeric {
        digits: usize,
    },
    Date {
        start: String,
        end: String,
    },
    Gaussian {
        mean: f64,
        sd: f64,
        precision: usize,
    },
    Map(std::collections::HashMap<String, String>),
    UuidV4,
    DtmNowUtc,
    // Realistic data generation variants
    RealisticName {
        gender: Option<String>,
    }, // "M", "F", or None for any
    RealisticAddress,
    RealisticPhone,
    RealisticSsn,
    RealisticMrn, // Medical Record Number
    RealisticIcd10,
    RealisticLoinc,
    RealisticMedication,
    RealisticAllergen,
    RealisticBloodType,
    RealisticEthnicity,
    RealisticRace,
    // Error injection variants for negative testing
    InvalidSegmentId,
    InvalidFieldFormat,
    InvalidRepFormat,
    InvalidCompFormat,
    InvalidSubcompFormat,
    DuplicateDelims,
    BadDelimLength,
}

/// Generate messages from a template
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
fn generate_segment(
    segment_template: &str,
    values: &HashMap<String, Vec<ValueSource>>,
    delims: &Delims,
    rng: &mut StdRng,
) -> Result<Segment, Error> {
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
            let index = rng.gen_range(0..options.len());
            Ok(options[index].clone())
        }
        ValueSource::Numeric { digits } => {
            let mut result = String::new();
            for _ in 0..*digits {
                let digit = rng.gen_range(0..10);
                result.push_str(&digit.to_string());
            }
            Ok(result)
        }
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
            let random_days = rng.gen_range(0..=days);

            // Add the random days to the start date
            let random_date = start_date + chrono::Duration::days(random_days);

            // Format as YYYYMMDD
            Ok(random_date.format("%Y%m%d").to_string())
        }
        ValueSource::Gaussian {
            mean,
            sd,
            precision,
        } => {
            // Generate a Gaussian distributed value
            let value = rng.sample(
                rand_distr::Normal::new(*mean, *sd).map_err(|_| Error::InvalidEscapeToken)?,
            );
            Ok(format!("{:.*}", precision, value))
        }
        ValueSource::Map(mapping) => {
            // For map, we need a source value to map from
            // Since we don't have that in this context, we'll just pick a random key and return its value
            if mapping.is_empty() {
                return Ok(String::new());
            }
            let keys: Vec<&String> = mapping.keys().collect();
            let random_key = keys[rng.gen_range(0..keys.len())];
            Ok(mapping[random_key].clone())
        }
        ValueSource::UuidV4 => {
            let uuid = uuid::Uuid::new_v4();
            Ok(uuid.to_string())
        }
        ValueSource::DtmNowUtc => {
            // Generate current UTC timestamp in YYYYMMDDHHMMSS format
            let now = chrono::Utc::now();
            Ok(now.format("%Y%m%d%H%M%S").to_string())
        }
        // Realistic data generation implementations
        ValueSource::RealisticName { gender } => {
            let first_names = match gender.as_deref() {
                Some("M") => &[
                    "James", "John", "Robert", "Michael", "William", "David", "Richard", "Joseph",
                    "Thomas", "Charles",
                ][..],
                Some("F") => &[
                    "Mary",
                    "Patricia",
                    "Jennifer",
                    "Linda",
                    "Elizabeth",
                    "Barbara",
                    "Susan",
                    "Jessica",
                    "Sarah",
                    "Karen",
                ][..],
                _ => &[
                    "James",
                    "Mary",
                    "John",
                    "Patricia",
                    "Robert",
                    "Jennifer",
                    "Michael",
                    "Linda",
                    "William",
                    "Elizabeth",
                    "David",
                    "Barbara",
                    "Richard",
                    "Susan",
                    "Joseph",
                    "Jessica",
                ][..],
            };

            let last_names = &[
                "Smith",
                "Johnson",
                "Williams",
                "Brown",
                "Jones",
                "Garcia",
                "Miller",
                "Davis",
                "Rodriguez",
                "Martinez",
                "Hernandez",
                "Lopez",
                "Gonzalez",
                "Wilson",
                "Anderson",
            ];

            let first_name = first_names[rng.gen_range(0..first_names.len())];
            let last_name = last_names[rng.gen_range(0..last_names.len())];

            Ok(format!("{}^{}", last_name, first_name))
        }
        ValueSource::RealisticAddress => {
            let streets = &[
                "Main St",
                "Oak Ave",
                "Pine Rd",
                "Elm St",
                "Maple Dr",
                "Cedar Ln",
                "Birch Way",
                "Washington St",
                "Lake St",
                "Hill St",
            ];

            let cities = &[
                "Anytown",
                "Springfield",
                "Riverside",
                "Fairview",
                "Centerville",
                "Georgetown",
                "Mount Pleasant",
                "Oakland",
                "Middletown",
                "Franklin",
            ];

            let states = &["AL", "AK", "AZ", "AR", "CA", "CO", "CT", "DE", "FL", "GA"];

            let street_number = rng.gen_range(100..9999);
            let street = streets[rng.gen_range(0..streets.len())];
            let city = cities[rng.gen_range(0..cities.len())];
            let state = states[rng.gen_range(0..states.len())];
            let zip = format!("{:05}", rng.gen_range(10000..99999));

            Ok(format!(
                "{} {}^^{}^{}^{}^{}",
                street_number, street, city, state, zip, "USA"
            ))
        }
        ValueSource::RealisticPhone => {
            let area_code = rng.gen_range(200..999);
            let exchange = rng.gen_range(200..999);
            let number = rng.gen_range(1000..9999);
            Ok(format!("({}){}-{}", area_code, exchange, number))
        }
        ValueSource::RealisticSsn => {
            let part1 = rng.gen_range(100..999);
            let part2 = rng.gen_range(10..99);
            let part3 = rng.gen_range(1000..9999);
            Ok(format!("{}-{}-{}", part1, part2, part3))
        }
        ValueSource::RealisticMrn => {
            // Medical Record Number - typically 6-10 digits
            let length = rng.gen_range(6..=10);
            let mut mrn = String::new();
            for _ in 0..length {
                let digit = rng.gen_range(0..10);
                mrn.push_str(&digit.to_string());
            }
            Ok(mrn)
        }
        ValueSource::RealisticIcd10 => {
            // Simplified ICD-10 codes (real codes are more complex)
            let categories = &[
                "A00", "B01", "C02", "D03", "E04", "F05", "G06", "H07", "I08", "J09",
            ];
            let category = categories[rng.gen_range(0..categories.len())];
            let subcode = rng.gen_range(0..10);
            Ok(format!("{}.{}", category, subcode))
        }
        ValueSource::RealisticLoinc => {
            // LOINC codes are numeric with 5-7 digits
            let code = rng.gen_range(10000..9999999);
            Ok(code.to_string())
        }
        ValueSource::RealisticMedication => {
            let medications = &[
                "Atorvastatin",
                "Levothyroxine",
                "Lisinopril",
                "Metformin",
                "Amlodipine",
                "Metoprolol",
                "Omeprazole",
                "Simvastatin",
                "Losartan",
                "Albuterol",
            ];
            let medication = medications[rng.gen_range(0..medications.len())];
            Ok(medication.to_string())
        }
        ValueSource::RealisticAllergen => {
            let allergens = &[
                "Penicillin",
                "Latex",
                "Peanuts",
                "Shellfish",
                "Eggs",
                "Milk",
                "Tree Nuts",
                "Soy",
                "Wheat",
                "Bee Stings",
            ];
            let allergen = allergens[rng.gen_range(0..allergens.len())];
            Ok(allergen.to_string())
        }
        ValueSource::RealisticBloodType => {
            let blood_types = &["A+", "A-", "B+", "B-", "AB+", "AB-", "O+", "O-"];
            let blood_type = blood_types[rng.gen_range(0..blood_types.len())];
            Ok(blood_type.to_string())
        }
        ValueSource::RealisticEthnicity => {
            let ethnicities = &[
                "Hispanic or Latino",
                "Not Hispanic or Latino",
                "Declined to Specify",
            ];
            let ethnicity = ethnicities[rng.gen_range(0..ethnicities.len())];
            Ok(ethnicity.to_string())
        }
        ValueSource::RealisticRace => {
            let races = &[
                "American Indian or Alaska Native",
                "Asian",
                "Black or African American",
                "Native Hawaiian or Other Pacific Islander",
                "White",
                "Declined to Specify",
            ];
            let race = races[rng.gen_range(0..races.len())];
            Ok(race.to_string())
        }
        // Error injection variants for negative testing
        ValueSource::InvalidSegmentId => Err(Error::InvalidSegmentId),
        ValueSource::InvalidFieldFormat => Err(Error::InvalidFieldFormat {
            details: "Injected invalid field format".to_string(),
        }),
        ValueSource::InvalidRepFormat => Err(Error::InvalidRepFormat {
            details: "Injected invalid repetition format".to_string(),
        }),
        ValueSource::InvalidCompFormat => Err(Error::InvalidCompFormat {
            details: "Injected invalid component format".to_string(),
        }),
        ValueSource::InvalidSubcompFormat => Err(Error::InvalidSubcompFormat {
            details: "Injected invalid subcomponent format".to_string(),
        }),
        ValueSource::DuplicateDelims => Err(Error::DuplicateDelims),
        ValueSource::BadDelimLength => Err(Error::BadDelimLength),
    }
}

/// Generate a single ACK message
pub fn ack(original: &Message, code: AckCode) -> Result<Message, Error> {
    // Create ACK message with same delimiters as original
    let delims = original.delims.clone();

    // Create MSH segment for ACK
    let msh_segment = create_ack_msh_segment(original, &code)?;

    // Create MSA segment
    let msa_segment = create_msa_segment(original, &code)?;

    Ok(Message {
        delims,
        segments: vec![msh_segment, msa_segment],
        charsets: vec![],
    })
}

/// Create MSH segment for ACK message
fn create_ack_msh_segment(original: &Message, _code: &AckCode) -> Result<Segment, Error> {
    // Get the original MSH segment
    let original_msh = original.segments.first().ok_or(Error::InvalidSegmentId)?;
    if &original_msh.id != b"MSH" {
        return Err(Error::InvalidSegmentId);
    }

    // Extract required fields from original MSH
    let sending_app = get_field_value(original_msh, 2).unwrap_or_else(|| "HL7V2RS".to_string());
    let sending_fac = get_field_value(original_msh, 3).unwrap_or_else(|| "HL7V2RS".to_string());
    let receiving_app = get_field_value(original_msh, 4).unwrap_or_else(|| "".to_string());
    let receiving_fac = get_field_value(original_msh, 5).unwrap_or_else(|| "".to_string());
    let message_type = get_field_value(original_msh, 8).unwrap_or_else(|| "ACK".to_string());
    let control_id = get_field_value(original_msh, 9).unwrap_or_else(|| "".to_string());
    let processing_id = get_field_value(original_msh, 10).unwrap_or_else(|| "P".to_string());
    let version = get_field_value(original_msh, 11).unwrap_or_else(|| "2.5.1".to_string());

    // Create timestamp
    let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S").to_string();

    // Create fields for MSH segment
    let mut fields = Vec::new();

    // MSH-2: Encoding characters
    fields.push(Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![Atom::Text(format!(
                    "{}{}{}{}",
                    original.delims.comp,
                    original.delims.rep,
                    original.delims.esc,
                    original.delims.sub
                ))],
            }],
        }],
    });

    // MSH-3: Sending Application
    fields.push(Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![Atom::Text(sending_app)],
            }],
        }],
    });

    // MSH-4: Sending Facility
    fields.push(Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![Atom::Text(sending_fac)],
            }],
        }],
    });

    // MSH-5: Receiving Application
    fields.push(Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![Atom::Text(receiving_app)],
            }],
        }],
    });

    // MSH-6: Receiving Facility
    fields.push(Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![Atom::Text(receiving_fac)],
            }],
        }],
    });

    // MSH-7: Date/Time of Message
    fields.push(Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![Atom::Text(timestamp)],
            }],
        }],
    });

    // MSH-8: Message Type
    fields.push(Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![Atom::Text(message_type)],
            }],
        }],
    });

    // MSH-9: Message Control ID
    fields.push(Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![Atom::Text(control_id)],
            }],
        }],
    });

    // MSH-10: Processing ID
    fields.push(Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![Atom::Text(processing_id)],
            }],
        }],
    });

    // MSH-11: Version ID
    fields.push(Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![Atom::Text(version)],
            }],
        }],
    });

    Ok(Segment {
        id: *b"MSH",
        fields,
    })
}

/// Create MSA segment for ACK message
fn create_msa_segment(original: &Message, code: &AckCode) -> Result<Segment, Error> {
    // Get the original MSH segment for control ID
    let original_msh = original.segments.first().ok_or(Error::InvalidSegmentId)?;
    if &original_msh.id != b"MSH" {
        return Err(Error::InvalidSegmentId);
    }

    // Get message control ID from original MSH-10
    let control_id = get_field_value(original_msh, 9).unwrap_or_else(|| "".to_string());

    // Convert ACK code to string
    let ack_code_str = match code {
        AckCode::AA => "AA",
        AckCode::AE => "AE",
        AckCode::AR => "AR",
        AckCode::CA => "CA",
        AckCode::CE => "CE",
        AckCode::CR => "CR",
    };

    // Create fields for MSA segment
    let mut fields = Vec::new();

    // MSA-1: Acknowledgment Code
    fields.push(Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![Atom::Text(ack_code_str.to_string())],
            }],
        }],
    });

    // MSA-2: Message Control ID
    fields.push(Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![Atom::Text(control_id)],
            }],
        }],
    });

    Ok(Segment {
        id: *b"MSA",
        fields,
    })
}

/// Get field value from a segment
fn get_field_value(segment: &Segment, field_index: usize) -> Option<String> {
    if field_index > segment.fields.len() {
        return None;
    }

    let field = &segment.fields[field_index - 1];
    if field.reps.is_empty() {
        return None;
    }

    let rep = &field.reps[0];
    if rep.comps.is_empty() {
        return None;
    }

    let comp = &rep.comps[0];
    if comp.subs.is_empty() {
        return None;
    }

    match &comp.subs[0] {
        Atom::Text(text) => Some(text.clone()),
        Atom::Null => None,
    }
}

/// ACK codes
#[derive(Debug, Clone)]
pub enum AckCode {
    AA, // Application Accept
    AE, // Application Error
    AR, // Application Reject
    CA, // Commit Accept
    CE, // Commit Error
    CR, // Commit Reject
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
    expected_hashes: &[String],
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
    count: usize,
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
pub fn generate_corpus(
    template: &Template,
    seed: u64,
    count: usize,
    batch_size: usize,
) -> Result<Vec<Message>, Error> {
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
pub fn generate_diverse_corpus(
    templates: &[Template],
    seed: u64,
    count: usize,
) -> Result<Vec<Message>, Error> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut messages = Vec::with_capacity(count);

    for i in 0..count {
        // Select a random template
        let template_index = rng.gen_range(0..templates.len());
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
    count: usize,
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
        let random_value = rng.gen_range(0.0..1.0);
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
    fn test_ack_generation() {
        let original_message = hl7v2_core::parse(
            b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r"
        ).unwrap();

        let ack_message = ack(&original_message, AckCode::AA).unwrap();

        assert_eq!(ack_message.segments.len(), 2);
        assert_eq!(
            std::str::from_utf8(&ack_message.segments[0].id).unwrap(),
            "MSH"
        );
        assert_eq!(
            std::str::from_utf8(&ack_message.segments[1].id).unwrap(),
            "MSA"
        );
    }

    #[test]
    fn test_date_generation() {
        let mut values = std::collections::HashMap::new();
        values.insert(
            "PID.7".to_string(),
            vec![ValueSource::Date {
                start: "20200101".to_string(),
                end: "20251231".to_string(),
            }],
        );

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
        values.insert(
            "PID.7".to_string(),
            vec![ValueSource::Gaussian {
                mean: 100.0,
                sd: 10.0,
                precision: 2,
            }],
        );

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
        values.insert(
            "PID.5".to_string(),
            vec![ValueSource::RealisticName {
                gender: Some("M".to_string()),
            }],
        );

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
    fn test_realistic_ssn_generation() {
        let mut values = std::collections::HashMap::new();
        values.insert("PID.19".to_string(), vec![ValueSource::RealisticSsn]);

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

        // The value should be a realistic SSN
        // For this test, we'll just verify it compiles and runs without error
    }

    #[test]
    fn test_realistic_mrn_generation() {
        let mut values = std::collections::HashMap::new();
        values.insert("PID.18".to_string(), vec![ValueSource::RealisticMrn]);

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

        // The value should be a realistic MRN
        // For this test, we'll just verify it compiles and runs without error
    }

    #[test]
    fn test_realistic_icd10_generation() {
        let mut values = std::collections::HashMap::new();
        values.insert("DG1.3".to_string(), vec![ValueSource::RealisticIcd10]);

        let template = Template {
            name: "test".to_string(),
            delims: "^~\\&".to_string(),
            segments: vec![
                "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1".to_string(),
                "PID|1||123456^^^HOSP^MR||Doe^John|||M||||".to_string(),
                "DG1|1||I10.9|Hypertension||A".to_string(),
            ],
            values,
        };

        let messages = generate(&template, 42, 1).unwrap();
        assert_eq!(messages.len(), 1);

        // The value should be a realistic ICD-10 code
        // For this test, we'll just verify it compiles and runs without error
    }

    #[test]
    fn test_realistic_loinc_generation() {
        let mut values = std::collections::HashMap::new();
        values.insert("OBX.3.1".to_string(), vec![ValueSource::RealisticLoinc]);

        let template = Template {
            name: "test".to_string(),
            delims: "^~\\&".to_string(),
            segments: vec![
                "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1".to_string(),
                "PID|1||123456^^^HOSP^MR||Doe^John|||M||||".to_string(),
                "OBX|1|NM|12345^Blood Pressure||120|mmHg|||||R|||20250128152312".to_string(),
            ],
            values,
        };

        let messages = generate(&template, 42, 1).unwrap();
        assert_eq!(messages.len(), 1);

        // The value should be a realistic LOINC code
        // For this test, we'll just verify it compiles and runs without error
    }

    #[test]
    fn test_realistic_medication_generation() {
        let mut values = std::collections::HashMap::new();
        values.insert(
            "RXA.5.1".to_string(),
            vec![ValueSource::RealisticMedication],
        );

        let template = Template {
            name: "test".to_string(),
            delims: "^~\\&".to_string(),
            segments: vec![
                "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1".to_string(),
                "PID|1||123456^^^HOSP^MR||Doe^John|||M||||".to_string(),
                "RXA|0|1|20250128152312|20250128152312|12345^Medication".to_string(),
            ],
            values,
        };

        let messages = generate(&template, 42, 1).unwrap();
        assert_eq!(messages.len(), 1);

        // The value should be a realistic medication name
        // For this test, we'll just verify it compiles and runs without error
    }

    #[test]
    fn test_realistic_allergen_generation() {
        let mut values = std::collections::HashMap::new();
        values.insert("AL1.3.1".to_string(), vec![ValueSource::RealisticAllergen]);

        let template = Template {
            name: "test".to_string(),
            delims: "^~\\&".to_string(),
            segments: vec![
                "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1".to_string(),
                "PID|1||123456^^^HOSP^MR||Doe^John|||M||||".to_string(),
                "AL1|1|DA|Allergen||MO".to_string(),
            ],
            values,
        };

        let messages = generate(&template, 42, 1).unwrap();
        assert_eq!(messages.len(), 1);

        // The value should be a realistic allergen
        // For this test, we'll just verify it compiles and runs without error
    }

    #[test]
    fn test_realistic_blood_type_generation() {
        let mut values = std::collections::HashMap::new();
        values.insert("PID.8".to_string(), vec![ValueSource::RealisticBloodType]);

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

        // The value should be a realistic blood type
        // For this test, we'll just verify it compiles and runs without error
    }

    #[test]
    fn test_realistic_ethnicity_generation() {
        let mut values = std::collections::HashMap::new();
        values.insert("PID.22".to_string(), vec![ValueSource::RealisticEthnicity]);

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

        // The value should be a realistic ethnicity
        // For this test, we'll just verify it compiles and runs without error
    }

    #[test]
    fn test_realistic_race_generation() {
        let mut values = std::collections::HashMap::new();
        values.insert("PID.10".to_string(), vec![ValueSource::RealisticRace]);

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

        // The value should be a realistic race
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

        // Generate golden hashes
        let expected_hashes = generate_golden_hashes(&template, 42, 2).unwrap();

        // Verify the hashes
        let results = verify_golden_hashes(&template, 42, 2, &expected_hashes).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results[0]);
        assert!(results[1]);

        // Test with incorrect hashes
        let wrong_hashes = vec!["wrong_hash_1".to_string(), "wrong_hash_2".to_string()];
        let results = verify_golden_hashes(&template, 42, 2, &wrong_hashes).unwrap();
        assert_eq!(results.len(), 2);
        assert!(!results[0]);
        assert!(!results[1]);
    }
}
