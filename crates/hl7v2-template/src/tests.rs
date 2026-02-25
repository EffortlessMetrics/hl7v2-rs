//! Unit tests for hl7v2-template

use super::*;
use std::collections::HashMap;

// =============================================================================
// Template Structure Tests
// =============================================================================

#[test]
fn test_template_creation() {
    let template = Template {
        name: "test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|App|Fac|App|Fac|20250128152312||ADT^A01|123|P|2.5.1"#.to_string(),
        ],
        values: HashMap::new(),
    };
    
    assert_eq!(template.name, "test");
    assert_eq!(template.delims, "^~\\&");
    assert_eq!(template.segments.len(), 1);
}

#[test]
fn test_template_clone() {
    let template = Template {
        name: "test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec!["MSH|...".to_string()],
        values: HashMap::new(),
    };
    
    let cloned = template.clone();
    assert_eq!(template.name, cloned.name);
    assert_eq!(template.delims, cloned.delims);
}

#[test]
fn test_template_debug() {
    let template = Template {
        name: "test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec!["MSH|...".to_string()],
        values: HashMap::new(),
    };
    
    let debug_str = format!("{:?}", template);
    assert!(debug_str.contains("Template"));
    assert!(debug_str.contains("test"));
}

// =============================================================================
// Basic Generation Tests
// =============================================================================

#[test]
fn test_generate_single_message() {
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
    assert_eq!(messages.len(), 1);
    
    let message = &messages[0];
    assert_eq!(message.segments.len(), 2);
}

#[test]
fn test_generate_multiple_messages() {
    let template = Template {
        name: "test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1"#.to_string(),
        ],
        values: HashMap::new(),
    };
    
    let messages = generate(&template, 42, 10).unwrap();
    assert_eq!(messages.len(), 10);
}

#[test]
fn test_generate_zero_messages() {
    let template = Template {
        name: "test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1"#.to_string(),
        ],
        values: HashMap::new(),
    };
    
    let messages = generate(&template, 42, 0).unwrap();
    assert_eq!(messages.len(), 0);
}

// =============================================================================
// Determinism Tests
// =============================================================================

#[test]
fn test_generate_deterministic() {
    let template = Template {
        name: "test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1"#.to_string(),
        ],
        values: HashMap::new(),
    };
    
    let messages1 = generate(&template, 42, 5).unwrap();
    let messages2 = generate(&template, 42, 5).unwrap();
    
    for (m1, m2) in messages1.iter().zip(messages2.iter()) {
        let bytes1 = hl7v2_core::write(m1);
        let bytes2 = hl7v2_core::write(m2);
        assert_eq!(bytes1, bytes2);
    }
}

#[test]
fn test_generate_different_seeds() {
    let mut values = HashMap::new();
    values.insert("PID.3".to_string(), vec![ValueSource::UuidV4]);
    
    let template = Template {
        name: "test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1"#.to_string(),
            r#"PID|1||123456^^^HOSP^MR||Doe^John"#.to_string(),
        ],
        values,
    };
    
    let messages1 = generate(&template, 42, 1).unwrap();
    let messages2 = generate(&template, 43, 1).unwrap();
    
    let bytes1 = hl7v2_core::write(&messages1[0]);
    let bytes2 = hl7v2_core::write(&messages2[0]);
    
    assert_ne!(bytes1, bytes2);
}

// =============================================================================
// Delimiter Parsing Tests
// =============================================================================

#[test]
fn test_parse_delimiters_valid() {
    let delims = parse_delimiters("^~\\&").unwrap();
    assert_eq!(delims.comp, '^');
    assert_eq!(delims.rep, '~');
    assert_eq!(delims.esc, '\\');
    assert_eq!(delims.sub, '&');
}

#[test]
fn test_parse_delimiters_custom() {
    let delims = parse_delimiters("%$!@").unwrap();
    assert_eq!(delims.comp, '%');
    assert_eq!(delims.rep, '$');
    assert_eq!(delims.esc, '!');
    assert_eq!(delims.sub, '@');
}

#[test]
fn test_parse_delimiters_too_short() {
    let result = parse_delimiters("^^");
    assert!(result.is_err());
}

#[test]
fn test_parse_delimiters_too_long() {
    let result = parse_delimiters("^^^^^");
    assert!(result.is_err());
}

#[test]
fn test_parse_delimiters_duplicate() {
    let result = parse_delimiters("^^^^");
    assert!(result.is_err());
}

// =============================================================================
// Segment Generation Tests
// =============================================================================

#[test]
fn test_generate_segment_msh() {
    let delims = Delims::default();
    let mut rng = rand::rngs::StdRng::seed_from_u64(42);
    let values = HashMap::new();
    
    let segment = generate_segment(
        r#"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1"#,
        &values,
        &delims,
        &mut rng
    ).unwrap();
    
    assert_eq!(&segment.id, b"MSH");
}

#[test]
fn test_generate_segment_pid() {
    let delims = Delims::default();
    let mut rng = rand::rngs::StdRng::seed_from_u64(42);
    let values = HashMap::new();
    
    let segment = generate_segment(
        r#"PID|1||123456^^^HOSP^MR||Doe^John"#,
        &values,
        &delims,
        &mut rng
    ).unwrap();
    
    assert_eq!(&segment.id, b"PID");
}

#[test]
fn test_generate_segment_invalid_id() {
    let delims = Delims::default();
    let mut rng = rand::rngs::StdRng::seed_from_u64(42);
    let values = HashMap::new();
    
    let result = generate_segment(
        "INVALID|1|2",
        &values,
        &delims,
        &mut rng
    );
    
    assert!(result.is_err());
}

#[test]
fn test_generate_segment_lowercase_id() {
    let delims = Delims::default();
    let mut rng = rand::rngs::StdRng::seed_from_u64(42);
    let values = HashMap::new();
    
    let result = generate_segment(
        "pid|1|2",
        &values,
        &delims,
        &mut rng
    );
    
    assert!(result.is_err());
}

// =============================================================================
// Value Source Tests
// =============================================================================

#[test]
fn test_value_source_fixed() {
    let mut values = HashMap::new();
    values.insert("PID.3".to_string(), vec![ValueSource::Fixed("ABC123".to_string())]);
    
    let template = Template {
        name: "test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|App|Fac|App|Fac|20250128152312||ADT^A01|123|P|2.5.1"#.to_string(),
            r#"PID|1||123456^^^HOSP^MR||Doe^John"#.to_string(),
        ],
        values,
    };
    
    let messages = generate(&template, 42, 1).unwrap();
    assert_eq!(messages.len(), 1);
}

#[test]
fn test_value_source_numeric() {
    let mut values = HashMap::new();
    values.insert("PID.3".to_string(), vec![ValueSource::Numeric { digits: 6 }]);
    
    let template = Template {
        name: "test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|App|Fac|App|Fac|20250128152312||ADT^A01|123|P|2.5.1"#.to_string(),
            r#"PID|1||123456^^^HOSP^MR||Doe^John"#.to_string(),
        ],
        values,
    };
    
    let messages = generate(&template, 42, 1).unwrap();
    assert_eq!(messages.len(), 1);
}

#[test]
fn test_value_source_from() {
    let mut values = HashMap::new();
    values.insert("PID.5".to_string(), vec![ValueSource::From(vec![
        r#"Smith^John"#.to_string(),
        r#"Doe^Jane"#.to_string(),
    ])]);
    
    let template = Template {
        name: "test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|App|Fac|App|Fac|20250128152312||ADT^A01|123|P|2.5.1"#.to_string(),
            r#"PID|1||123456||Doe^John"#.to_string(),
        ],
        values,
    };
    
    let messages = generate(&template, 42, 10).unwrap();
    assert_eq!(messages.len(), 10);
}

#[test]
fn test_value_source_uuid_v4() {
    let mut values = HashMap::new();
    values.insert("MSH.10".to_string(), vec![ValueSource::UuidV4]);
    
    let template = Template {
        name: "test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|App|Fac|App|Fac|20250128152312||ADT^A01|123|P|2.5.1"#.to_string(),
        ],
        values,
    };
    
    let messages = generate(&template, 42, 10).unwrap();
    
    // All UUIDs should be unique
    let mut uuids = Vec::new();
    for msg in &messages {
        let bytes = hl7v2_core::write(msg);
        let s = String::from_utf8_lossy(&bytes);
        uuids.push(s.to_string());
    }
    
    // Check uniqueness
    let unique: std::collections::HashSet<_> = uuids.iter().collect();
    assert_eq!(unique.len(), 10);
}

#[test]
fn test_value_source_date() {
    let mut values = HashMap::new();
    values.insert("PID.7".to_string(), vec![ValueSource::Date {
        start: "19800101".to_string(),
        end: "20001231".to_string(),
    }]);
    
    let template = Template {
        name: "test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|App|Fac|App|Fac|20250128152312||ADT^A01|123|P|2.5.1"#.to_string(),
            r#"PID|1||123456||Doe^John||19800101"#.to_string(),
        ],
        values,
    };
    
    let messages = generate(&template, 42, 10).unwrap();
    assert_eq!(messages.len(), 10);
}

#[test]
fn test_value_source_gaussian() {
    let mut values = HashMap::new();
    values.insert("OBX.5".to_string(), vec![ValueSource::Gaussian {
        mean: 100.0,
        sd: 10.0,
        precision: 2,
    }]);
    
    let template = Template {
        name: "test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|App|Fac|App|Fac|20250128152312||ORU^R01|123|P|2.5.1"#.to_string(),
            r#"OBX|1|NM|TEST||100.0|units"#.to_string(),
        ],
        values,
    };
    
    let messages = generate(&template, 42, 10).unwrap();
    assert_eq!(messages.len(), 10);
}

// =============================================================================
// Corpus Generation Tests
// =============================================================================

#[test]
fn test_generate_corpus() {
    let template = Template {
        name: "test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|App|Fac|App|Fac|20250128152312||ADT^A01|123|P|2.5.1"#.to_string(),
        ],
        values: HashMap::new(),
    };
    
    let messages = generate_corpus(&template, 42, 100, 10).unwrap();
    assert_eq!(messages.len(), 100);
}

#[test]
fn test_generate_diverse_corpus() {
    let template1 = Template {
        name: "adt_a01".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|App|Fac|App|Fac|20250128152312||ADT^A01|123|P|2.5.1"#.to_string(),
        ],
        values: HashMap::new(),
    };
    
    let template2 = Template {
        name: "oru_r01".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|App|Fac|App|Fac|20250128152312||ORU^R01|123|P|2.5.1"#.to_string(),
        ],
        values: HashMap::new(),
    };
    
    let templates = vec![template1, template2];
    let messages = generate_diverse_corpus(&templates, 42, 50).unwrap();
    assert_eq!(messages.len(), 50);
}

#[test]
fn test_generate_distributed_corpus() {
    let template1 = Template {
        name: "adt".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|App|Fac|App|Fac|20250128152312||ADT^A01|123|P|2.5.1"#.to_string(),
        ],
        values: HashMap::new(),
    };
    
    let template2 = Template {
        name: "oru".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|App|Fac|App|Fac|20250128152312||ORU^R01|123|P|2.5.1"#.to_string(),
        ],
        values: HashMap::new(),
    };
    
    let distributions = vec![
        (template1, 0.7),
        (template2, 0.3),
    ];
    
    let messages = generate_distributed_corpus(&distributions, 42, 100).unwrap();
    assert_eq!(messages.len(), 100);
}

// =============================================================================
// Golden Hash Tests
// =============================================================================

#[test]
fn test_generate_golden_hashes() {
    let template = Template {
        name: "test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|App|Fac|App|Fac|20250128152312||ADT^A01|123|P|2.5.1"#.to_string(),
        ],
        values: HashMap::new(),
    };
    
    let hashes = generate_golden_hashes(&template, 42, 10).unwrap();
    assert_eq!(hashes.len(), 10);
    
    // All hashes should be 64 characters (SHA-256 hex)
    for hash in &hashes {
        assert_eq!(hash.len(), 64);
    }
}

#[test]
fn test_verify_golden_hashes() {
    let template = Template {
        name: "test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|App|Fac|App|Fac|20250128152312||ADT^A01|123|P|2.5.1"#.to_string(),
        ],
        values: HashMap::new(),
    };
    
    let hashes = generate_golden_hashes(&template, 42, 5).unwrap();
    let results = verify_golden_hashes(&template, 42, 5, &hashes).unwrap();
    
    assert_eq!(results.len(), 5);
    assert!(results.iter().all(|&r| r));
}

#[test]
fn test_verify_golden_hashes_mismatch() {
    let template = Template {
        name: "test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|App|Fac|App|Fac|20250128152312||ADT^A01|123|P|2.5.1"#.to_string(),
        ],
        values: HashMap::new(),
    };
    
    // Use wrong hashes
    let wrong_hashes = vec![
        "0000000000000000000000000000000000000000000000000000000000000000".to_string();
        5
    ];
    
    let results = verify_golden_hashes(&template, 42, 5, &wrong_hashes).unwrap();
    
    // All should fail
    assert!(results.iter().all(|&r| !r));
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[test]
fn test_generate_invalid_segment_id() {
    let template = Template {
        name: "test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            "INVALID|1|2|3".to_string(),
        ],
        values: HashMap::new(),
    };
    
    let result = generate(&template, 42, 1);
    assert!(result.is_err());
}

#[test]
fn test_generate_invalid_delims() {
    let template = Template {
        name: "test".to_string(),
        delims: "^^".to_string(), // Too short
        segments: vec![
            r#"MSH|^~\&|App|Fac|App|Fac|20250128152312||ADT^A01|123|P|2.5.1"#.to_string(),
        ],
        values: HashMap::new(),
    };
    
    let result = generate(&template, 42, 1);
    assert!(result.is_err());
}

// =============================================================================
// MSH Special Handling Tests
// =============================================================================

#[test]
fn test_msh_segment_field_handling() {
    let template = Template {
        name: "test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1"#.to_string(),
        ],
        values: HashMap::new(),
    };
    
    let messages = generate(&template, 42, 1).unwrap();
    let message = &messages[0];
    
    // MSH should have correct number of fields
    assert!(!message.segments[0].fields.is_empty());
}

// =============================================================================
// Realistic Value Source Tests
// =============================================================================

#[test]
fn test_realistic_name() {
    let mut values = HashMap::new();
    values.insert("PID.5".to_string(), vec![ValueSource::RealisticName { gender: Some("M".to_string()) }]);
    
    let template = Template {
        name: "test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|App|Fac|App|Fac|20250128152312||ADT^A01|123|P|2.5.1"#.to_string(),
            r#"PID|1||123456||Doe^John"#.to_string(),
        ],
        values,
    };
    
    let messages = generate(&template, 42, 10).unwrap();
    assert_eq!(messages.len(), 10);
}

#[test]
fn test_realistic_address() {
    let mut values = HashMap::new();
    values.insert("PID.11".to_string(), vec![ValueSource::RealisticAddress]);
    
    let template = Template {
        name: "test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|App|Fac|App|Fac|20250128152312||ADT^A01|123|P|2.5.1"#.to_string(),
            r#"PID|1||123456||Doe^John|||M|||123 Main St"#.to_string(),
        ],
        values,
    };
    
    let messages = generate(&template, 42, 10).unwrap();
    assert_eq!(messages.len(), 10);
}

#[test]
fn test_realistic_phone() {
    let mut values = HashMap::new();
    values.insert("PID.13".to_string(), vec![ValueSource::RealisticPhone]);
    
    let template = Template {
        name: "test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|App|Fac|App|Fac|20250128152312||ADT^A01|123|P|2.5.1"#.to_string(),
            r#"PID|1||123456||Doe^John||(555)123-4567"#.to_string(),
        ],
        values,
    };
    
    let messages = generate(&template, 42, 10).unwrap();
    assert_eq!(messages.len(), 10);
}
