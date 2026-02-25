//! Integration tests for hl7v2-gen

use hl7v2_gen::{Template, ValueSource, generate, ack, ack_with_error, AckCode, Faker, FakerValue};
use std::collections::HashMap;

// =============================================================================
// Template-Based Generation Integration Tests
// =============================================================================

#[test]
fn test_full_adt_a01_generation() {
    let template = Template {
        name: "adt_a01".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1"#.to_string(),
            r#"EVN|A01|20250128152312||"#.to_string(),
            r#"PID|1||123456^^^HOSP^MR||Doe^John^Robert||19700101|M|||123 Main St^^Anytown^CA^12345^USA||(555)123-4567|||M|S|123456789||"#.to_string(),
            r#"PV1|1|I|ICU^01^01||||123456^Smith^John^J^^MD||||||||ADM|A0||||||||||||||||||||||||||"#.to_string(),
        ],
        values: HashMap::new(),
    };
    
    let messages = generate(&template, 42, 1).unwrap();
    assert_eq!(messages.len(), 1);
    
    let message = &messages[0];
    assert_eq!(message.segments.len(), 4);
    
    // Verify segment IDs
    assert_eq!(std::str::from_utf8(&message.segments[0].id).unwrap(), "MSH");
    assert_eq!(std::str::from_utf8(&message.segments[1].id).unwrap(), "EVN");
    assert_eq!(std::str::from_utf8(&message.segments[2].id).unwrap(), "PID");
    assert_eq!(std::str::from_utf8(&message.segments[3].id).unwrap(), "PV1");
}

#[test]
fn test_generation_with_value_substitution() {
    let mut values = HashMap::new();
    values.insert("PID.3".to_string(), vec![ValueSource::Numeric { digits: 6 }]);
    values.insert("PID.5".to_string(), vec![ValueSource::RealisticName { gender: Some("M".to_string()) }]);
    values.insert("PID.7".to_string(), vec![ValueSource::Date { 
        start: "19500101".to_string(), 
        end: "20001231".to_string() 
    }]);
    
    let template = Template {
        name: "test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1"#.to_string(),
            r#"PID|1||123456^^^HOSP^MR||Doe^John||19700101|M"#.to_string(),
        ],
        values,
    };
    
    let messages = generate(&template, 42, 5).unwrap();
    assert_eq!(messages.len(), 5);
    
    // All messages should be valid
    for message in &messages {
        assert_eq!(message.segments.len(), 2);
    }
}

#[test]
fn test_generation_with_uuid() {
    let mut values = HashMap::new();
    values.insert("MSH.10".to_string(), vec![ValueSource::UuidV4]);
    
    let template = Template {
        name: "test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1"#.to_string(),
        ],
        values,
    };
    
    let messages = generate(&template, 42, 10).unwrap();
    
    // Each message should have a unique control ID
    let mut control_ids: Vec<Vec<u8>> = messages.iter()
        .map(|m| hl7v2_core::write(m))
        .collect();
    
    // Check uniqueness
    control_ids.sort();
    control_ids.dedup();
    assert_eq!(control_ids.len(), 10);
}

// =============================================================================
// ACK Generation Integration Tests
// =============================================================================

#[test]
fn test_ack_commit_accept() {
    let original = hl7v2_core::parse(
        b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r"
    ).unwrap();
    
    let ack_msg = ack(&original, AckCode::AA).unwrap();
    
    assert_eq!(ack_msg.segments.len(), 2);
    
    // Verify MSA segment has AA
    let ack_bytes = hl7v2_core::write(&ack_msg);
    let ack_str = String::from_utf8(ack_bytes).unwrap();
    assert!(ack_str.contains("MSA|AA"));
}

#[test]
fn test_ack_error() {
    let original = hl7v2_core::parse(
        b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r"
    ).unwrap();
    
    let ack_msg = ack_with_error(&original, AckCode::AE, Some("Segment sequence error")).unwrap();
    
    assert_eq!(ack_msg.segments.len(), 3); // MSH + MSA + ERR
    
    let ack_bytes = hl7v2_core::write(&ack_msg);
    let ack_str = String::from_utf8(ack_bytes).unwrap();
    assert!(ack_str.contains("MSA|AE"));
    assert!(ack_str.contains("ERR"));
}

#[test]
fn test_ack_reject() {
    let original = hl7v2_core::parse(
        b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\r"
    ).unwrap();
    
    let ack_msg = ack(&original, AckCode::AR).unwrap();
    
    let ack_bytes = hl7v2_core::write(&ack_msg);
    let ack_str = String::from_utf8(ack_bytes).unwrap();
    assert!(ack_str.contains("MSA|AR"));
}

// =============================================================================
// Faker Integration Tests
// =============================================================================

#[test]
fn test_faker_integration() {
    use rand::SeedableRng;
    use rand::rngs::StdRng;
    
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    
    // Generate various realistic values
    let name = faker.name(None);
    assert!(name.contains('^'));
    
    let address = faker.address();
    assert!(address.contains("USA"));
    
    let phone = faker.phone();
    assert!(phone.starts_with('('));
    
    let mrn = faker.mrn();
    assert!((6..=10).contains(&mrn.len()));
    
    let icd10 = faker.icd10();
    assert!(icd10.contains('.'));
}

#[test]
fn test_faker_value_integration() {
    use rand::SeedableRng;
    use rand::rngs::StdRng;
    
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    
    // Test various FakerValue types
    let fixed = FakerValue::Fixed("test".to_string());
    assert_eq!(fixed.generate(&mut faker).unwrap(), "test");
    
    let from = FakerValue::From(vec!["a".to_string(), "b".to_string()]);
    let result = from.generate(&mut faker).unwrap();
    assert!(result == "a" || result == "b");
    
    let numeric = FakerValue::Numeric { digits: 5 };
    let result = numeric.generate(&mut faker).unwrap();
    assert_eq!(result.len(), 5);
}

// =============================================================================
// Cross-Crate Integration Tests
// =============================================================================

#[test]
fn test_generate_parse_query() {
    let template = Template {
        name: "test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1"#.to_string(),
            r#"PID|1||123456^^^HOSP^MR||Doe^John"#.to_string(),
        ],
        values: HashMap::new(),
    };
    
    let messages = generate(&template, 42, 1).unwrap();
    let message = &messages[0];
    
    // Write to bytes
    let bytes = hl7v2_core::write(message);
    
    // Parse again
    let reparsed = hl7v2_core::parse(&bytes).unwrap();
    assert_eq!(reparsed.segments.len(), 2);
    
    // Query using hl7v2-query
    let name = hl7v2_query::get(&reparsed, "PID.5.1").unwrap();
    assert_eq!(name, "Doe");
}

#[test]
fn test_generate_normalize() {
    let template = Template {
        name: "test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1"#.to_string(),
            r#"PID|1||123456^^^HOSP^MR||Doe^John"#.to_string(),
        ],
        values: HashMap::new(),
    };
    
    let messages = generate(&template, 42, 1).unwrap();
    let message = &messages[0];
    
    // Write to bytes
    let bytes = hl7v2_core::write(message);
    
    // Normalize
    let normalized = hl7v2_normalize::normalize(&bytes, true).unwrap();
    
    // Should still be parseable
    let reparsed = hl7v2_parser::parse(&normalized).unwrap();
    assert_eq!(reparsed.segments.len(), 2);
}

// =============================================================================
// Large Scale Generation Tests
// =============================================================================

#[test]
fn test_large_corpus_generation() {
    let template = Template {
        name: "test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1"#.to_string(),
            r#"PID|1||123456^^^HOSP^MR||Doe^John"#.to_string(),
        ],
        values: HashMap::new(),
    };
    
    let messages = generate(&template, 42, 1000).unwrap();
    assert_eq!(messages.len(), 1000);
    
    // All messages should have the same structure
    for message in &messages {
        assert_eq!(message.segments.len(), 2);
    }
}

// =============================================================================
// Different Message Types Tests
// =============================================================================

#[test]
fn test_oru_r01_generation() {
    let template = Template {
        name: "oru_r01".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|LabSystem|Lab|HIS|Hospital|20250128152312||ORU^R01|LAB00001|P|2.5.1"#.to_string(),
            r#"PID|1||PATID123||Smith^Jane"#.to_string(),
            r#"OBR|1||ORD001|CBC^Complete Blood Count^L"#.to_string(),
            r#"OBX|1|NM|HB^Hemoglobin^L||13.2|g/dL|11.5-17.5||||F"#.to_string(),
            r#"OBX|2|NM|WBC^White Blood Count^L||7.5|10^9/L|4.0-11.0||||F"#.to_string(),
        ],
        values: HashMap::new(),
    };
    
    let messages = generate(&template, 42, 1).unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].segments.len(), 5);
}

#[test]
fn test_adt_a04_generation() {
    let template = Template {
        name: "adt_a04".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|ADT1|MCM|LABADT|MCM|198808181126||ADT^A04|MSG00001|P|2.5.1"#.to_string(),
            r#"PID|1||PATID1234^5^M11^ADT1^MR^MCM~~~123456789||JONES^WILLIAM^A^III||19610615|M"#.to_string(),
        ],
        values: HashMap::new(),
    };
    
    let messages = generate(&template, 42, 1).unwrap();
    assert_eq!(messages.len(), 1);
    
    let bytes = hl7v2_core::write(&messages[0]);
    let str = String::from_utf8(bytes).unwrap();
    assert!(str.contains("ADT^A04"));
}

// =============================================================================
// Error Handling Integration Tests
// =============================================================================

#[test]
fn test_invalid_template_handling() {
    let template = Template {
        name: "test".to_string(),
        delims: "^^".to_string(), // Invalid - too short
        segments: vec![
            r#"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1"#.to_string(),
        ],
        values: HashMap::new(),
    };
    
    let result = generate(&template, 42, 1);
    assert!(result.is_err());
}

// =============================================================================
// Determinism Tests
// =============================================================================

#[test]
fn test_same_seed_same_output() {
    let template = Template {
        name: "test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1"#.to_string(),
            r#"PID|1||123456^^^HOSP^MR||Doe^John"#.to_string(),
        ],
        values: HashMap::new(),
    };
    
    let messages1 = generate(&template, 12345, 10).unwrap();
    let messages2 = generate(&template, 12345, 10).unwrap();
    
    for i in 0..10 {
        let bytes1 = hl7v2_core::write(&messages1[i]);
        let bytes2 = hl7v2_core::write(&messages2[i]);
        assert_eq!(bytes1, bytes2);
    }
}

#[test]
fn test_different_seed_different_output() {
    let mut values = HashMap::new();
    values.insert("PID.3".to_string(), vec![ValueSource::Numeric { digits: 6 }]);
    
    let template = Template {
        name: "test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1"#.to_string(),
            r#"PID|1||123456^^^HOSP^MR||Doe^John"#.to_string(),
        ],
        values,
    };
    
    let messages1 = generate(&template, 111, 1).unwrap();
    let messages2 = generate(&template, 222, 1).unwrap();
    
    let bytes1 = hl7v2_core::write(&messages1[0]);
    let bytes2 = hl7v2_core::write(&messages2[0]);
    
    // Different seeds should produce different output (with high probability)
    assert_ne!(bytes1, bytes2);
}
