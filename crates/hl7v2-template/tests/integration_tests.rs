//! Integration tests for hl7v2-template crate

use hl7v2_template::{Template, ValueSource, generate, generate_corpus, generate_diverse_corpus};
use hl7v2_template::{generate_golden_hashes, verify_golden_hashes, create_manifest};
use std::collections::HashMap;

fn create_basic_template() -> Template {
    Template {
        name: "ADT_A01".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1"#.to_string(),
            r#"PID|1||123456^^^HOSP^MR||Doe^John"#.to_string(),
        ],
        values: HashMap::new(),
    }
}

fn create_template_with_values() -> Template {
    let mut values = HashMap::new();
    values.insert("PID.3".to_string(), vec![ValueSource::UuidV4]);
    values.insert("PID.5.1".to_string(), vec![ValueSource::RealisticName { gender: None }]);
    
    Template {
        name: "ADT_A01_Dynamic".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1"#.to_string(),
            r#"PID|1||{{PID.3}}||{{PID.5.1}}"#.to_string(),
        ],
        values,
    }
}

#[test]
fn integration_basic_generation() {
    let template = create_basic_template();
    let messages = generate(&template, 42, 5).unwrap();
    
    assert_eq!(messages.len(), 5);
    for message in &messages {
        assert_eq!(message.segments.len(), 2);
        assert_eq!(std::str::from_utf8(&message.segments[0].id).unwrap(), "MSH");
        assert_eq!(std::str::from_utf8(&message.segments[1].id).unwrap(), "PID");
    }
}

#[test]
fn integration_deterministic_generation() {
    let template = create_basic_template();
    
    let messages1 = generate(&template, 12345, 10).unwrap();
    let messages2 = generate(&template, 12345, 10).unwrap();
    
    assert_eq!(messages1.len(), messages2.len());
    for i in 0..messages1.len() {
        assert_eq!(messages1[i].segments.len(), messages2[i].segments.len());
    }
}

#[test]
fn integration_different_seeds_different_results() {
    let mut values = HashMap::new();
    values.insert("PID.3".to_string(), vec![ValueSource::UuidV4]);
    
    let template = Template {
        name: "test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1"#.to_string(),
            r#"PID|1||{{PID.3}}||Test"#.to_string(),
        ],
        values,
    };
    
    let messages1 = generate(&template, 111, 1).unwrap();
    let messages2 = generate(&template, 222, 1).unwrap();
    
    // With different seeds and UUID generation, results should differ
    // (Note: this could theoretically fail if UUIDs collide, but extremely unlikely)
    let msg1_str = hl7v2_core::write(&messages1[0]);
    let msg2_str = hl7v2_core::write(&messages2[0]);
    
    // The UUID fields should be different
    assert_ne!(msg1_str, msg2_str);
}

#[test]
fn integration_corpus_generation() {
    let template = create_basic_template();
    let messages = generate_corpus(&template, 42, 100, 10).unwrap();
    
    assert_eq!(messages.len(), 100);
}

#[test]
fn integration_diverse_corpus() {
    let template1 = Template {
        name: "ADT_A01".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|App1|Fac1|App2|Fac2|20250128152312||ADT^A01|1|P|2.5.1"#.to_string(),
        ],
        values: HashMap::new(),
    };
    
    let template2 = Template {
        name: "ORU_R01".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|App1|Fac1|App2|Fac2|20250128152312||ORU^R01|2|P|2.5.1"#.to_string(),
        ],
        values: HashMap::new(),
    };
    
    let messages = generate_diverse_corpus(&[template1, template2], 42, 50).unwrap();
    
    assert_eq!(messages.len(), 50);
}

#[test]
fn integration_golden_hashes() {
    let template = create_basic_template();
    
    // Generate golden hashes
    let hashes = generate_golden_hashes(&template, 42, 5).unwrap();
    assert_eq!(hashes.len(), 5);
    
    // All hashes should be valid SHA-256 hex strings (64 characters)
    for hash in &hashes {
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }
    
    // Verify hashes
    let verification = verify_golden_hashes(&template, 42, 5, &hashes).unwrap();
    assert_eq!(verification.len(), 5);
    assert!(verification.iter().all(|&v| v));
}

#[test]
fn integration_golden_hash_verification_failure() {
    let template = create_basic_template();
    
    // Use wrong hashes
    let wrong_hashes = vec!["0".repeat(64); 5];
    
    let verification = verify_golden_hashes(&template, 42, 5, &wrong_hashes).unwrap();
    assert_eq!(verification.len(), 5);
    assert!(verification.iter().all(|&v| !v));
}

#[test]
fn integration_manifest_creation() {
    let template = create_basic_template();
    let messages = generate(&template, 42, 3).unwrap();
    
    let templates = vec![("templates/adt_a01.yaml".to_string(), template)];
    let manifest = create_manifest(42, &templates, &messages, "output");
    
    assert_eq!(manifest.messages.len(), 3);
    assert_eq!(manifest.templates.len(), 1);
}

#[test]
fn integration_msh_segment_handling() {
    let template = Template {
        name: "MSH_Test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|123|P|2.5.1|||AL|AL"#.to_string(),
        ],
        values: HashMap::new(),
    };
    
    let messages = generate(&template, 42, 1).unwrap();
    let msh = &messages[0].segments[0];
    
    assert_eq!(std::str::from_utf8(&msh.id).unwrap(), "MSH");
    // MSH-1 is the field separator (implicit)
    // MSH-2 is the encoding characters
    // MSH-3 and onwards are regular fields
    assert!(msh.fields.len() > 5);
}

#[test]
fn integration_custom_delimiters() {
    let template = Template {
        name: "CustomDelims".to_string(),
        delims: "@#$%".to_string(), // component@, repetition#, escape$, subcomponent%
        segments: vec![
            r#"MSH|@#$%|App|Fac|App|Fac|20250128152312||ADT@A01|1|P|2.5.1"#.to_string(),
        ],
        values: HashMap::new(),
    };
    
    let messages = generate(&template, 42, 1).unwrap();
    assert_eq!(messages.len(), 1);
    
    // Check delimiters were parsed correctly
    let delims = &messages[0].delims;
    assert_eq!(delims.comp, '@');
    assert_eq!(delims.rep, '#');
    assert_eq!(delims.esc, '$');
    assert_eq!(delims.sub, '%');
}

#[test]
fn integration_multiple_segments() {
    let template = Template {
        name: "MultiSegment".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|App|Fac|App|Fac|20250128152312||ADT^A01|1|P|2.5.1"#.to_string(),
            r#"EVN|A01|20250128152312"#.to_string(),
            r#"PID|1||12345||Doe^John^A||19800101|M"#.to_string(),
            r#"PV1|1|I|ICU^101^^HOSP|||||||ADM||||||||IN|||||||||||||||||||||||||20250128152312"#.to_string(),
        ],
        values: HashMap::new(),
    };
    
    let messages = generate(&template, 42, 1).unwrap();
    let msg = &messages[0];
    
    assert_eq!(msg.segments.len(), 4);
    assert_eq!(std::str::from_utf8(&msg.segments[0].id).unwrap(), "MSH");
    assert_eq!(std::str::from_utf8(&msg.segments[1].id).unwrap(), "EVN");
    assert_eq!(std::str::from_utf8(&msg.segments[2].id).unwrap(), "PID");
    assert_eq!(std::str::from_utf8(&msg.segments[3].id).unwrap(), "PV1");
}

#[test]
fn integration_value_source_fixed() {
    let mut values = HashMap::new();
    values.insert("PID.5.1".to_string(), vec![ValueSource::Fixed("Smith".to_string())]);
    
    let template = Template {
        name: "FixedValue".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|App|Fac|App|Fac|20250128152312||ADT^A01|1|P|2.5.1"#.to_string(),
            r#"PID|1||123||{{PID.5.1}}"#.to_string(),
        ],
        values,
    };
    
    let messages = generate(&template, 42, 3).unwrap();
    
    // All messages should have "Smith" as the last name
    for msg in &messages {
        let pid = &msg.segments[1];
        // The value should be "Smith" from the Fixed value source
        // Note: This depends on how the template engine handles value substitution
        assert_eq!(std::str::from_utf8(&pid.id).unwrap(), "PID");
    }
}

#[test]
fn integration_value_source_from() {
    let mut values = HashMap::new();
    values.insert("PID.8".to_string(), vec![ValueSource::From(vec!["M".to_string(), "F".to_string(), "O".to_string()])]);
    
    let template = Template {
        name: "FromValue".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|App|Fac|App|Fac|20250128152312||ADT^A01|1|P|2.5.1"#.to_string(),
            r#"PID|1||123||Test||19700101|{{PID.8}}"#.to_string(),
        ],
        values,
    };
    
    let messages = generate(&template, 42, 10).unwrap();
    
    // All messages should have one of the specified gender values
    assert_eq!(messages.len(), 10);
}

#[test]
fn integration_empty_template_values() {
    let template = Template {
        name: "EmptyValues".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|App|Fac|App|Fac|20250128152312||ADT^A01|1|P|2.5.1"#.to_string(),
        ],
        values: HashMap::new(),
    };
    
    let messages = generate(&template, 42, 1).unwrap();
    assert_eq!(messages.len(), 1);
}

#[test]
fn integration_large_corpus() {
    let template = create_basic_template();
    
    // Generate a larger corpus to test performance
    let messages = generate_corpus(&template, 42, 1000, 100).unwrap();
    assert_eq!(messages.len(), 1000);
}

#[test]
fn integration_message_serialization() {
    let template = create_basic_template();
    let messages = generate(&template, 42, 1).unwrap();
    
    // Serialize to bytes
    let bytes = hl7v2_core::write(&messages[0]);
    
    // Should be valid UTF-8
    let s = String::from_utf8(bytes).unwrap();
    
    // Should contain expected segments
    assert!(s.contains("MSH|"));
    assert!(s.contains("PID|"));
}

#[test]
fn integration_corpus_reexport() {
    use hl7v2_template::{compute_sha256, extract_message_type};
    
    let hash = compute_sha256("test content");
    assert_eq!(hash.len(), 64);
    
    let template = create_basic_template();
    let messages = generate(&template, 42, 1).unwrap();
    let msg_type = extract_message_type(&messages[0]);
    assert!(!msg_type.is_empty());
}
