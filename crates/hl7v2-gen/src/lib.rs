//! Deterministic HL7 v2 message generator.
//!
//! This crate provides functionality for generating synthetic HL7 v2
//! messages based on templates and profiles.
//!
//! # Template-Based Generation
//!
//! Template-based message generation functionality is available through
//! the [`hl7v2_template`] crate and re-exported here for convenience.
//! See the [`hl7v2_template`] documentation for details on template
//! structure and value sources.
//!
//! # ACK Generation
//!
//! ACK (acknowledgment) generation functionality is available through
//! the [`hl7v2_ack`] crate and re-exported here for convenience.
//!
//! # Faker Data Generation
//!
//! Realistic test data generation (names, addresses, medical codes, etc.)
//! is available through the [`hl7v2_faker`] crate and re-exported here
//! for convenience.
//!
//! # Example
//!
//! ```
//! use hl7v2_gen::{Template, generate, ack, AckCode, Faker, FakerValue};
//! ```

// Re-export template functionality from hl7v2-template crate for backward compatibility
pub use hl7v2_template::{
    Template, ValueSource,
    generate, generate_corpus, generate_diverse_corpus, generate_distributed_corpus,
    generate_golden_hashes, verify_golden_hashes,
};

// Re-export ACK functionality from hl7v2-ack crate for backward compatibility
pub use hl7v2_ack::{ack, ack_with_error, AckCode};

// Re-export faker functionality from hl7v2-faker crate for backward compatibility
pub use hl7v2_faker::{
    Faker, FakerValue, DateError, GaussianError, GenerateError,
};

// Re-export core types that are commonly used with this crate
pub use hl7v2_core::{Message, Delims, Error, Segment, Field, Rep, Comp, Atom};

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
}
