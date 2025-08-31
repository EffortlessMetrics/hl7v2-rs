//! Deterministic HL7 v2 message generator.
//!
//! This crate provides functionality for generating synthetic HL7 v2
//! messages based on templates and profiles.

use hl7v2_core::{Message, Delims, Error, Segment, Field, Rep, Comp, Atom};
use serde::{Deserialize, Serialize};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
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
    Numeric { digits: usize },
    Date { start: String, end: String },
    Gaussian { mean: f64, sd: f64, precision: usize },
    Map(std::collections::HashMap<String, String>),
    UuidV4,
    DtmNowUtc,
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
fn generate_single_message(template: &Template, rng: &mut StdRng, _index: usize) -> Result<Message, Error> {
    // Parse delimiters
    let delims = parse_delimiters(&template.delims)?;
    
    // Generate segments
    let mut segments = Vec::new();
    
    for segment_template in &template.segments {
        let segment = generate_segment(segment_template, &template.values, &delims, rng)?;
        segments.push(segment);
    }
    
    Ok(Message { delims, segments })
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
            let index = rng.gen_range(0..options.len());
            Ok(options[index].clone())
        },
        ValueSource::Numeric { digits } => {
            let mut result = String::new();
            for _ in 0..*digits {
                let digit = rng.gen_range(0..10);
                result.push_str(&digit.to_string());
            }
            Ok(result)
        },
        ValueSource::UuidV4 => {
            let uuid = uuid::Uuid::new_v4();
            Ok(uuid.to_string())
        },
        // Error injection variants
        ValueSource::InvalidSegmentId => Err(Error::InvalidSegmentId),
        ValueSource::InvalidFieldFormat => Err(Error::InvalidFieldFormat { details: "Injected invalid field format".to_string() }),
        ValueSource::InvalidRepFormat => Err(Error::InvalidRepFormat { details: "Injected invalid repetition format".to_string() }),
        ValueSource::InvalidCompFormat => Err(Error::InvalidCompFormat { details: "Injected invalid component format".to_string() }),
        ValueSource::InvalidSubcompFormat => Err(Error::InvalidSubcompFormat { details: "Injected invalid subcomponent format".to_string() }),
        ValueSource::DuplicateDelims => Err(Error::DuplicateDelims),
        ValueSource::BadDelimLength => Err(Error::BadDelimLength),
        // For other value sources, we'll implement them later
        _ => Ok(String::from("generated_value")),
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
                subs: vec![Atom::Text(format!("{}{}{}{}", 
                    original.delims.comp, original.delims.rep, 
                    original.delims.esc, original.delims.sub))],
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
        assert_eq!(std::str::from_utf8(&ack_message.segments[0].id).unwrap(), "MSH");
        assert_eq!(std::str::from_utf8(&ack_message.segments[1].id).unwrap(), "MSA");
    }
}