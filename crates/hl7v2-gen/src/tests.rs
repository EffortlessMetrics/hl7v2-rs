//! Unit tests for hl7v2-gen

use super::*;
use std::collections::HashMap;

// =============================================================================
// Basic Generation Tests
// =============================================================================

#[test]
fn test_generate_simple_message() {
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
    assert_eq!(std::str::from_utf8(&message.segments[0].id).unwrap(), "MSH");
    assert_eq!(std::str::from_utf8(&message.segments[1].id).unwrap(), "PID");
}

#[test]
fn test_generate_multiple_messages() {
    let template = Template {
        name: "test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1"#.to_string(),
            r#"PID|1||123456^^^HOSP^MR||Doe^John"#.to_string(),
        ],
        values: HashMap::new(),
    };

    let messages = generate(&template, 42, 3).unwrap();
    assert_eq!(messages.len(), 3);

    // All messages should have the same structure
    for message in &messages {
        assert_eq!(message.segments.len(), 2);
    }
}

#[test]
fn test_generate_deterministic() {
    let template = Template {
        name: "test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1"#.to_string(),
            r#"PID|1||123456^^^HOSP^MR||Doe^John"#.to_string(),
        ],
        values: HashMap::new(),
    };

    // Generate messages with the same seed
    let messages1 = generate(&template, 42, 3).unwrap();
    let messages2 = generate(&template, 42, 3).unwrap();

    // Results should be identical
    assert_eq!(messages1.len(), messages2.len());
    for i in 0..messages1.len() {
        assert_eq!(messages1[i].segments.len(), messages2[i].segments.len());
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
            r#"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1"#.to_string(),
            r#"PID|1||123456^^^HOSP^MR||Doe^John"#.to_string(),
        ],
        values,
    };

    // Generate messages with different seeds
    let messages1 = generate(&template, 42, 1).unwrap();
    let messages2 = generate(&template, 43, 1).unwrap();

    // Results should be different (because of UUID generation)
    assert_ne!(
        hl7v2_core::write(&messages1[0]),
        hl7v2_core::write(&messages2[0])
    );
}

// =============================================================================
// Value Source Tests
// =============================================================================

#[test]
fn test_generate_with_uuid() {
    let mut values = HashMap::new();
    values.insert("PID.3".to_string(), vec![ValueSource::UuidV4]);

    let template = Template {
        name: "test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1"#.to_string(),
            r#"PID|1||123456^^^HOSP^MR||Doe^John"#.to_string(),
        ],
        values,
    };

    let messages = generate(&template, 42, 1).unwrap();
    assert_eq!(messages.len(), 1);
}

#[test]
fn test_generate_with_date() {
    let mut values = HashMap::new();
    values.insert(
        "PID.7".to_string(),
        vec![ValueSource::Date {
            start: "20200101".to_string(),
            end: "20251231".to_string(),
        }],
    );

    let template = Template {
        name: "test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1"#.to_string(),
            r#"PID|1||123456^^^HOSP^MR||Doe^John|||M||||"#.to_string(),
        ],
        values,
    };

    let messages = generate(&template, 42, 1).unwrap();
    assert_eq!(messages.len(), 1);
}

#[test]
fn test_generate_with_gaussian() {
    let mut values = HashMap::new();
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
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1"#.to_string(),
            r#"PID|1||123456^^^HOSP^MR||Doe^John|||M||||"#.to_string(),
        ],
        values,
    };

    let messages = generate(&template, 42, 1).unwrap();
    assert_eq!(messages.len(), 1);
}

#[test]
fn test_generate_with_fixed_value() {
    let mut values = HashMap::new();
    values.insert(
        "PID.5".to_string(),
        vec![ValueSource::Fixed(r#"Smith^John"#.to_string())],
    );

    let template = Template {
        name: "test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1"#.to_string(),
            r#"PID|1||123456^^^HOSP^MR||Doe^John"#.to_string(),
        ],
        values,
    };

    let messages = generate(&template, 42, 1).unwrap();
    assert_eq!(messages.len(), 1);
}

#[test]
fn test_generate_with_from_value() {
    let mut values = HashMap::new();
    values.insert(
        "PID.5".to_string(),
        vec![ValueSource::From(vec![
            r#"Smith^John"#.to_string(),
            r#"Doe^Jane"#.to_string(),
            r#"Brown^Bob"#.to_string(),
        ])],
    );

    let template = Template {
        name: "test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1"#.to_string(),
            r#"PID|1||123456^^^HOSP^MR||Doe^John"#.to_string(),
        ],
        values,
    };

    let messages = generate(&template, 42, 10).unwrap();
    assert_eq!(messages.len(), 10);
}

#[test]
fn test_generate_with_numeric() {
    let mut values = HashMap::new();
    values.insert(
        "PID.3".to_string(),
        vec![ValueSource::Numeric { digits: 6 }],
    );

    let template = Template {
        name: "test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1"#.to_string(),
            r#"PID|1||123456^^^HOSP^MR||Doe^John"#.to_string(),
        ],
        values,
    };

    let messages = generate(&template, 42, 1).unwrap();
    assert_eq!(messages.len(), 1);
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
            r#"INVALID|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1"#.to_string(),
        ],
        values: HashMap::new(),
    };

    let result = generate(&template, 42, 1);
    assert!(result.is_err());
}

#[test]
fn test_generate_invalid_delimiters() {
    let template = Template {
        name: "test".to_string(),
        delims: "^^".to_string(), // Too short
        segments: vec![
            r#"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1"#.to_string(),
        ],
        values: HashMap::new(),
    };

    let result = generate(&template, 42, 1);
    assert!(result.is_err());
}

#[test]
fn test_generate_duplicate_delimiters() {
    let template = Template {
        name: "test".to_string(),
        delims: "^^^^".to_string(), // Duplicate characters
        segments: vec![
            r#"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1"#.to_string(),
        ],
        values: HashMap::new(),
    };

    let result = generate(&template, 42, 1);
    assert!(result.is_err());
}

// =============================================================================
// ACK Generation Tests
// =============================================================================

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
fn test_ack_with_error() {
    let original_message = hl7v2_core::parse(
        b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r"
    ).unwrap();

    let ack_message = ack_with_error(
        &original_message,
        AckCode::AE,
        Some("Segment sequence error"),
    )
    .unwrap();

    assert_eq!(ack_message.segments.len(), 3);
    assert_eq!(
        std::str::from_utf8(&ack_message.segments[0].id).unwrap(),
        "MSH"
    );
    assert_eq!(
        std::str::from_utf8(&ack_message.segments[1].id).unwrap(),
        "MSA"
    );
    assert_eq!(
        std::str::from_utf8(&ack_message.segments[2].id).unwrap(),
        "ERR"
    );
}

// =============================================================================
// Faker Re-export Tests
// =============================================================================

#[test]
fn test_faker_generation() {
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);

    let name = faker.name(Some("M"));
    assert!(name.contains('^'));

    let address = faker.address();
    assert!(address.contains("USA"));

    let phone = faker.phone();
    assert!(phone.starts_with('('));
}

#[test]
fn test_faker_value_generation() {
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);

    let value = FakerValue::Fixed("test".to_string());
    let result = value.generate(&mut faker).unwrap();
    assert_eq!(result, "test");

    let value = FakerValue::Numeric { digits: 5 };
    let result = value.generate(&mut faker).unwrap();
    assert_eq!(result.len(), 5);
}

// =============================================================================
// Template Structure Tests
// =============================================================================

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
    assert_eq!(template.segments.len(), cloned.segments.len());
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
// Zero Count Test
// =============================================================================

#[test]
fn test_generate_zero_messages() {
    let template = Template {
        name: "test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1"#.to_string(),
        ],
        values: HashMap::new(),
    };

    let messages = generate(&template, 42, 0).unwrap();
    assert_eq!(messages.len(), 0);
}

// =============================================================================
// Large Count Test
// =============================================================================

#[test]
fn test_generate_many_messages() {
    let template = Template {
        name: "test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1"#.to_string(),
            r#"PID|1||123456^^^HOSP^MR||Doe^John"#.to_string(),
        ],
        values: HashMap::new(),
    };

    let messages = generate(&template, 42, 100).unwrap();
    assert_eq!(messages.len(), 100);
}

// =============================================================================
// Multiple Value Sources Test
// =============================================================================

#[test]
fn test_generate_with_multiple_value_sources() {
    let mut values = HashMap::new();
    values.insert(
        "PID.3".to_string(),
        vec![
            ValueSource::Numeric { digits: 3 },
            ValueSource::Fixed(r#"^^^HOSP^MR"#.to_string()),
        ],
    );

    let template = Template {
        name: "test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1"#.to_string(),
            r#"PID|1||123456^^^HOSP^MR||Doe^John"#.to_string(),
        ],
        values,
    };

    let messages = generate(&template, 42, 1).unwrap();
    assert_eq!(messages.len(), 1);
}

// =============================================================================
// Different Message Types Tests
// =============================================================================

#[test]
fn test_generate_adt_a04() {
    let template = Template {
        name: "adt_a04".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A04^ADT_A04|ABC123|P|2.5.1"#.to_string(),
            r#"PID|1||123456^^^HOSP^MR||Doe^John"#.to_string(),
        ],
        values: HashMap::new(),
    };

    let messages = generate(&template, 42, 1).unwrap();
    assert_eq!(messages.len(), 1);

    let written = hl7v2_core::write(&messages[0]);
    let written_str = String::from_utf8(written).unwrap();
    assert!(written_str.contains("ADT^A04"));
}

#[test]
fn test_generate_oru_r01() {
    let template = Template {
        name: "oru_r01".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ORU^R01|ABC123|P|2.5.1"#.to_string(),
            r#"PID|1||123456^^^HOSP^MR||Doe^John"#.to_string(),
            r#"OBR|1|||1234^Test"#.to_string(),
            r#"OBX|1|NM|1234^Result||120|mg/dL"#.to_string(),
        ],
        values: HashMap::new(),
    };

    let messages = generate(&template, 42, 1).unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].segments.len(), 4);
}
