//! Property-based tests for hl7v2-corpus using proptest

use hl7v2_corpus::*;
use proptest::prelude::*;

// =============================================================================
// SHA-256 Property Tests
// =============================================================================

proptest! {
    #[test]
    fn test_sha256_deterministic(s in ".*") {
        let hash1 = compute_sha256(&s);
        let hash2 = compute_sha256(&s);
        prop_assert_eq!(hash1, hash2);
    }
    
    #[test]
    fn test_sha256_length(s in ".*") {
        let hash = compute_sha256(&s);
        prop_assert_eq!(hash.len(), 64);
    }
    
    #[test]
    fn test_sha256_hex_characters(s in ".*") {
        let hash = compute_sha256(&s);
        for c in hash.chars() {
            prop_assert!(c.is_ascii_hexdigit());
        }
    }
    
    #[test]
    fn test_sha256_different_inputs_different_hashes(s1 in ".*", s2 in ".*") {
        // Only test when inputs are actually different
        prop_assume!(s1 != s2);
        let hash1 = compute_sha256(&s1);
        let hash2 = compute_sha256(&s2);
        prop_assert_ne!(hash1, hash2);
    }
}

// =============================================================================
// JSON Serialization Property Tests
// =============================================================================

proptest! {
    #[test]
    fn test_manifest_json_roundtrip_seed(seed: u64) {
        let manifest = CorpusManifest::new(seed);
        let json = manifest.to_json().unwrap();
        let parsed = CorpusManifest::from_json(&json).unwrap();
        prop_assert_eq!(parsed.seed, seed);
    }
    
    #[test]
    fn test_manifest_json_roundtrip_with_template(
        seed: u64,
        path in "[a-zA-Z0-9_/]+\\.yaml",
        content in ".*"
    ) {
        let mut manifest = CorpusManifest::new(seed);
        manifest.add_template(&path, &content);
        
        let json = manifest.to_json().unwrap();
        let parsed = CorpusManifest::from_json(&json).unwrap();
        
        prop_assert_eq!(parsed.templates.len(), 1);
        prop_assert_eq!(&parsed.templates[0].path, &path);
        prop_assert_eq!(&parsed.templates[0].sha256, &compute_sha256(&content));
    }
    
    #[test]
    fn test_manifest_json_roundtrip_with_message(
        seed: u64,
        path in "[a-zA-Z0-9_/]+\\.hl7",
        content in ".*",
        message_type in "[A-Z]{3}\\^[A-Z][0-9]{2}",
        template_index: usize
    ) {
        let mut manifest = CorpusManifest::new(seed);
        manifest.add_message(&path, &content, &message_type, template_index);
        
        let json = manifest.to_json().unwrap();
        let parsed = CorpusManifest::from_json(&json).unwrap();
        
        prop_assert_eq!(parsed.messages.len(), 1);
        prop_assert_eq!(&parsed.messages[0].path, &path);
        prop_assert_eq!(&parsed.messages[0].message_type, &message_type);
        prop_assert_eq!(parsed.messages[0].template_index, template_index);
    }
}

// =============================================================================
// Split Property Tests
// =============================================================================

proptest! {
    #[test]
    fn test_splits_cover_all_messages(
        seed: u64,
        num_messages in 10usize..100,
        train_ratio in 0.1f64..0.8,
        val_ratio in 0.1f64..0.3
    ) {
        let test_ratio = 1.0 - train_ratio - val_ratio;
        prop_assume!(test_ratio > 0.0 && test_ratio < 0.5);
        
        let mut manifest = CorpusManifest::new(seed);
        
        for i in 0..num_messages {
            manifest.add_message(
                &format!("msg{:03}.hl7", i),
                &format!("content{}", i),
                "ADT^A01",
                0
            );
        }
        
        manifest.create_splits((train_ratio, val_ratio, test_ratio));
        
        let total: usize = manifest.splits.train.len() 
            + manifest.splits.validation.len() 
            + manifest.splits.test.len();
        
        prop_assert_eq!(total, num_messages);
    }
    
    #[test]
    fn test_splits_no_overlap(seed: u64, num_messages in 10usize..50) {
        let mut manifest = CorpusManifest::new(seed);
        
        for i in 0..num_messages {
            manifest.add_message(
                &format!("msg{:03}.hl7", i),
                &format!("content{}", i),
                "ADT^A01",
                0
            );
        }
        
        manifest.create_splits((0.7, 0.15, 0.15));
        
        use std::collections::HashSet;
        let train: HashSet<_> = manifest.splits.train.iter().cloned().collect();
        let val: HashSet<_> = manifest.splits.validation.iter().cloned().collect();
        let test: HashSet<_> = manifest.splits.test.iter().cloned().collect();
        
        // Check no overlap
        let train_val: HashSet<_> = train.intersection(&val).collect();
        let train_test: HashSet<_> = train.intersection(&test).collect();
        let val_test: HashSet<_> = val.intersection(&test).collect();
        
        prop_assert!(train_val.is_empty());
        prop_assert!(train_test.is_empty());
        prop_assert!(val_test.is_empty());
    }
    
    #[test]
    fn test_splits_reproducible(seed: u64, num_messages in 10usize..30) {
        let mut manifest1 = CorpusManifest::new(seed);
        let mut manifest2 = CorpusManifest::new(seed);
        
        for i in 0..num_messages {
            let path = format!("msg{:03}.hl7", i);
            let content = format!("content{}", i);
            manifest1.add_message(&path, &content, "ADT^A01", 0);
            manifest2.add_message(&path, &content, "ADT^A01", 0);
        }
        
        manifest1.create_splits((0.7, 0.15, 0.15));
        manifest2.create_splits((0.7, 0.15, 0.15));
        
        prop_assert_eq!(manifest1.splits.train, manifest2.splits.train);
        prop_assert_eq!(manifest1.splits.validation, manifest2.splits.validation);
        prop_assert_eq!(manifest1.splits.test, manifest2.splits.test);
    }
}

// =============================================================================
// Message Type Counts Property Tests
// =============================================================================

proptest! {
    #[test]
    fn test_message_type_counts_sum(
        seed: u64,
        adt_count in 0usize..50,
        oru_count in 0usize..50,
        dft_count in 0usize..50
    ) {
        let mut manifest = CorpusManifest::new(seed);
        
        for i in 0..adt_count {
            manifest.add_message(&format!("adt{:03}.hl7", i), &format!("adt{}", i), "ADT^A01", 0);
        }
        for i in 0..oru_count {
            manifest.add_message(&format!("oru{:03}.hl7", i), &format!("oru{}", i), "ORU^R01", 1);
        }
        for i in 0..dft_count {
            manifest.add_message(&format!("dft{:03}.hl7", i), &format!("dft{}", i), "DFT^P03", 2);
        }
        
        let counts = manifest.message_type_counts();
        
        prop_assert_eq!(*counts.get("ADT^A01").unwrap_or(&0), adt_count);
        prop_assert_eq!(*counts.get("ORU^R01").unwrap_or(&0), oru_count);
        prop_assert_eq!(*counts.get("DFT^P03").unwrap_or(&0), dft_count);
        
        let total: usize = counts.values().sum();
        prop_assert_eq!(total, adt_count + oru_count + dft_count);
    }
}

// =============================================================================
// Template/Profile Info Property Tests
// =============================================================================

proptest! {
    #[test]
    fn test_template_info_json_roundtrip(
        path in "[a-zA-Z0-9_/]+\\.yaml",
        sha256 in "[a-f0-9]{64}"
    ) {
        let info = TemplateInfo {
            path: path.clone(),
            sha256: sha256.clone(),
        };
        
        let json = serde_json::to_string(&info).unwrap();
        let parsed: TemplateInfo = serde_json::from_str(&json).unwrap();
        
        prop_assert_eq!(parsed.path, path);
        prop_assert_eq!(parsed.sha256, sha256);
    }
    
    #[test]
    fn test_profile_info_json_roundtrip(
        path in "[a-zA-Z0-9_/]+\\.yaml",
        sha256 in "[a-f0-9]{64}"
    ) {
        let info = ProfileInfo {
            path: path.clone(),
            sha256: sha256.clone(),
        };
        
        let json = serde_json::to_string(&info).unwrap();
        let parsed: ProfileInfo = serde_json::from_str(&json).unwrap();
        
        prop_assert_eq!(parsed.path, path);
        prop_assert_eq!(parsed.sha256, sha256);
    }
    
    #[test]
    fn test_message_info_json_roundtrip(
        path in "[a-zA-Z0-9_/]+\\.hl7",
        sha256 in "[a-f0-9]{64}",
        message_type in "[A-Z]{3}\\^[A-Z][0-9]{2}",
        template_index: usize
    ) {
        let info = MessageInfo {
            path: path.clone(),
            sha256: sha256.clone(),
            message_type: message_type.clone(),
            template_index,
        };
        
        let json = serde_json::to_string(&info).unwrap();
        let parsed: MessageInfo = serde_json::from_str(&json).unwrap();
        
        prop_assert_eq!(parsed.path, path);
        prop_assert_eq!(parsed.sha256, sha256);
        prop_assert_eq!(parsed.message_type, message_type);
        prop_assert_eq!(parsed.template_index, template_index);
    }
}

// =============================================================================
// Corpus Config Property Tests
// =============================================================================

proptest! {
    #[test]
    fn test_corpus_config_clone(
        seed: u64,
        count: usize,
        batch_size: usize,
        create_splits: bool
    ) {
        let config = CorpusConfig {
            seed,
            count,
            batch_size,
            output_dir: None,
            create_splits,
            split_ratios: Some((0.7, 0.15, 0.15)),
        };
        
        let cloned = config.clone();
        
        prop_assert_eq!(config.seed, cloned.seed);
        prop_assert_eq!(config.count, cloned.count);
        prop_assert_eq!(config.batch_size, cloned.batch_size);
        prop_assert_eq!(config.create_splits, cloned.create_splits);
    }
}

// =============================================================================
// Unicode and Special Characters Property Tests
// =============================================================================

proptest! {
    #[test]
    fn test_sha256_unicode(content in ".*") {
        // SHA-256 should handle any UTF-8 content
        let hash = compute_sha256(&content);
        prop_assert_eq!(hash.len(), 64);
        
        // Should be deterministic
        let hash2 = compute_sha256(&content);
        prop_assert_eq!(hash, hash2);
    }
    
    #[test]
    fn test_manifest_unicode_paths(
        seed: u64,
        unicode_path in "[\\p{L}\\p{N}_/]+\\.[a-z]{3,4}"
    ) {
        let mut manifest = CorpusManifest::new(seed);
        manifest.add_template(&unicode_path, "content");
        
        let json = manifest.to_json().unwrap();
        let parsed = CorpusManifest::from_json(&json).unwrap();
        
        prop_assert_eq!(&parsed.templates[0].path, &unicode_path);
    }
}
