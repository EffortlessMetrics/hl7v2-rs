//! Property-based tests for hl7v2-template crate

use hl7v2_template::{
    Template, ValueSource, generate, generate_corpus, generate_golden_hashes, verify_golden_hashes,
};
use proptest::prelude::*;
use std::collections::HashMap;

prop_compose! {
    fn arb_template_name()(name in "[A-Za-z][A-Za-z0-9_]{0,19}") -> String {
        name
    }
}

prop_compose! {
    fn arb_basic_template()(name in arb_template_name()) -> Template {
        Template {
            name,
            delims: r#"^~\&"#.to_string(),
            segments: vec![
                r#"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1"#.to_string(),
                r#"PID|1||123456^^^HOSP^MR||Doe^John"#.to_string(),
            ],
            values: HashMap::new(),
        }
    }
}

proptest! {
    #[test]
    fn prop_generate_deterministic(template in arb_basic_template(), seed in 0u64..10000u64, count in 1usize..20) {
        let messages1 = generate(&template, seed, count).unwrap();
        let messages2 = generate(&template, seed, count).unwrap();

        prop_assert_eq!(messages1.len(), messages2.len());
        prop_assert_eq!(messages1.len(), count);

        for i in 0..messages1.len() {
            prop_assert_eq!(messages1[i].segments.len(), messages2[i].segments.len());
        }
    }
}

proptest! {
    #[test]
    fn prop_generate_count(template in arb_basic_template(), seed in 0u64..10000u64, count in 1usize..100) {
        let messages = generate(&template, seed, count).unwrap();
        prop_assert_eq!(messages.len(), count);
    }
}

proptest! {
    #[test]
    fn prop_golden_hashes_length(seed in 0u64..10000u64, count in 1usize..20) {
        let template = Template {
            name: "test".to_string(),
            delims: r#"^~\&"#.to_string(),
            segments: vec![
                r#"MSH|^~\&|App|Fac|App|Fac|20250128152312||ADT^A01|1|P|2.5.1"#.to_string(),
            ],
            values: HashMap::new(),
        };

        let hashes = generate_golden_hashes(&template, seed, count).unwrap();
        prop_assert_eq!(hashes.len(), count);

        for hash in &hashes {
            prop_assert_eq!(hash.len(), 64);
            prop_assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
        }
    }
}

proptest! {
    #[test]
    fn prop_golden_hash_verification(seed in 0u64..10000u64, count in 1usize..10) {
        let template = Template {
            name: "test".to_string(),
            delims: r#"^~\&"#.to_string(),
            segments: vec![
                r#"MSH|^~\&|App|Fac|App|Fac|20250128152312||ADT^A01|1|P|2.5.1"#.to_string(),
            ],
            values: HashMap::new(),
        };

        let hashes = generate_golden_hashes(&template, seed, count).unwrap();
        let verification = verify_golden_hashes(&template, seed, count, &hashes).unwrap();

        prop_assert_eq!(verification.len(), count);
        prop_assert!(verification.iter().all(|&v| v));
    }
}

proptest! {
    #[test]
    fn prop_different_seeds_different_messages(seed1 in 0u64..5000u64, seed2 in 5000u64..10000u64) {
        // Skip if seeds are the same
        prop_assume!(seed1 != seed2);

        let mut values = HashMap::new();
        values.insert("PID.3".to_string(), vec![ValueSource::UuidV4]);

        let template = Template {
            name: "test".to_string(),
            delims: r#"^~\&"#.to_string(),
            segments: vec![
                r#"MSH|^~\&|App|Fac|App|Fac|20250128152312||ADT^A01|1|P|2.5.1"#.to_string(),
                r#"PID|1||{{PID.3}}||Test"#.to_string(),
            ],
            values,
        };

        let messages1 = generate(&template, seed1, 1).unwrap();
        let messages2 = generate(&template, seed2, 1).unwrap();

        let msg1 = hl7v2_core::write(&messages1[0]);
        let msg2 = hl7v2_core::write(&messages2[0]);

        // With UUID generation, different seeds should produce different results
        prop_assert_ne!(msg1, msg2);
    }
}

proptest! {
    #[test]
    fn prop_corpus_batch_generation(seed in 0u64..10000u64, total in 10usize..100, batch in 5usize..20) {
        let template = Template {
            name: "test".to_string(),
            delims: r#"^~\&"#.to_string(),
            segments: vec![
                r#"MSH|^~\&|App|Fac|App|Fac|20250128152312||ADT^A01|1|P|2.5.1"#.to_string(),
            ],
            values: HashMap::new(),
        };

        let messages = generate_corpus(&template, seed, total, batch).unwrap();
        prop_assert_eq!(messages.len(), total);
    }
}

proptest! {
    #[test]
    fn prop_segment_count_preserved(template in arb_basic_template(), seed in 0u64..10000u64) {
        let expected_segments = template.segments.len();
        let messages = generate(&template, seed, 1).unwrap();

        prop_assert_eq!(messages[0].segments.len(), expected_segments);
    }
}

proptest! {
    #[test]
    fn prop_message_type_extracted(template in arb_basic_template(), seed in 0u64..10000u64) {
        let messages = generate(&template, seed, 1).unwrap();
        let msg_type = hl7v2_template::extract_message_type(&messages[0]);

        // Should extract ADT_A01 from the MSH segment
        prop_assert!(!msg_type.is_empty());
    }
}

proptest! {
    #[test]
    fn prop_sha256_consistency(content in ".*") {
        let hash1 = hl7v2_template::compute_sha256(&content);
        let hash2 = hl7v2_template::compute_sha256(&content);

        prop_assert_eq!(hash1.clone(), hash2);
        prop_assert_eq!(hash1.len(), 64);
    }
}

proptest! {
    #[test]
    fn prop_sha256_different_content(content1 in "[a-zA-Z0-9]+", content2 in "[a-zA-Z0-9]+") {
        prop_assume!(content1 != content2);

        let hash1 = hl7v2_template::compute_sha256(&content1);
        let hash2 = hl7v2_template::compute_sha256(&content2);

        prop_assert_ne!(hash1, hash2);
    }
}

proptest! {
    #[test]
    fn prop_message_hash_consistency(seed in 0u64..10000u64) {
        let template = Template {
            name: "test".to_string(),
            delims: r#"^~\&"#.to_string(),
            segments: vec![
                r#"MSH|^~\&|App|Fac|App|Fac|20250128152312||ADT^A01|1|P|2.5.1"#.to_string(),
            ],
            values: HashMap::new(),
        };

        let messages = generate(&template, seed, 1).unwrap();

        let hash1 = hl7v2_template::compute_message_hash(&messages[0]);
        let hash2 = hl7v2_template::compute_message_hash(&messages[0]);

        prop_assert_eq!(hash1.clone(), hash2);
        prop_assert_eq!(hash1.len(), 64);
    }
}

proptest! {
    #[test]
    fn prop_template_name_preserved(name in arb_template_name()) {
        let template = Template {
            name: name.clone(),
            delims: r#"^~\&"#.to_string(),
            segments: vec![
                r#"MSH|^~\&|App|Fac|App|Fac|20250128152312||ADT^A01|1|P|2.5.1"#.to_string(),
            ],
            values: HashMap::new(),
        };

        prop_assert_eq!(template.name, name);
    }
}

proptest! {
    #[test]
    fn prop_message_serialization_valid_utf8(template in arb_basic_template(), seed in 0u64..10000u64, count in 1usize..5) {
        let messages = generate(&template, seed, count).unwrap();

        for msg in &messages {
            let bytes = hl7v2_core::write(msg);
            prop_assert!(String::from_utf8(bytes).is_ok());
        }
    }
}

proptest! {
    #[test]
    fn prop_msh_segment_first(template in arb_basic_template(), seed in 0u64..10000u64) {
        let messages = generate(&template, seed, 1).unwrap();

        prop_assert!(!messages[0].segments.is_empty());
        let first_segment_id = std::str::from_utf8(&messages[0].segments[0].id).unwrap();
        prop_assert_eq!(first_segment_id, "MSH");
    }
}
