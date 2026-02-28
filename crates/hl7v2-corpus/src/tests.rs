//! Unit tests for hl7v2-corpus

use super::*;
use hl7v2_core::{Atom, Comp, Delims, Field, Message, Rep, Segment};

/// Helper to create a minimal MSH segment for testing
fn create_msh_segment() -> Segment {
    Segment {
        id: *b"MSH",
        fields: vec![
            Field {
                reps: vec![Rep {
                    comps: vec![Comp {
                        subs: vec![Atom::Text("^~\\&".to_string())],
                    }],
                }],
            },
            Field {
                reps: vec![Rep {
                    comps: vec![Comp {
                        subs: vec![Atom::Text("SendingApp".to_string())],
                    }],
                }],
            },
            Field {
                reps: vec![Rep {
                    comps: vec![Comp {
                        subs: vec![Atom::Text("SendingFac".to_string())],
                    }],
                }],
            },
            Field {
                reps: vec![Rep {
                    comps: vec![Comp {
                        subs: vec![Atom::Text("ReceivingApp".to_string())],
                    }],
                }],
            },
            Field {
                reps: vec![Rep {
                    comps: vec![Comp {
                        subs: vec![Atom::Text("ReceivingFac".to_string())],
                    }],
                }],
            },
            Field {
                reps: vec![Rep {
                    comps: vec![Comp {
                        subs: vec![Atom::Text("20250128152312".to_string())],
                    }],
                }],
            },
            Field {
                reps: vec![Rep {
                    comps: vec![Comp {
                        subs: vec![Atom::Text("".to_string())],
                    }],
                }],
            },
            Field {
                reps: vec![Rep {
                    comps: vec![
                        Comp {
                            subs: vec![Atom::Text("ADT".to_string())],
                        },
                        Comp {
                            subs: vec![Atom::Text("A01".to_string())],
                        },
                        Comp {
                            subs: vec![Atom::Text("ADT_A01".to_string())],
                        },
                    ],
                }],
            },
        ],
    }
}

// =============================================================================
// CorpusConfig Tests
// =============================================================================

#[test]
fn test_corpus_config_default_values() {
    let config = CorpusConfig::default();
    assert_eq!(config.seed, 42);
    assert_eq!(config.count, 100);
    assert_eq!(config.batch_size, 50);
    assert!(config.output_dir.is_none());
    assert!(!config.create_splits);
    assert_eq!(config.split_ratios, Some((0.7, 0.15, 0.15)));
}

#[test]
fn test_corpus_config_custom_values() {
    let config = CorpusConfig {
        seed: 12345,
        count: 500,
        batch_size: 100,
        output_dir: Some("/tmp/corpus".to_string()),
        create_splits: true,
        split_ratios: Some((0.8, 0.1, 0.1)),
    };

    assert_eq!(config.seed, 12345);
    assert_eq!(config.count, 500);
    assert_eq!(config.batch_size, 100);
    assert_eq!(config.output_dir, Some("/tmp/corpus".to_string()));
    assert!(config.create_splits);
    assert_eq!(config.split_ratios, Some((0.8, 0.1, 0.1)));
}

#[test]
fn test_corpus_config_clone() {
    let config = CorpusConfig::default();
    let cloned = config.clone();
    assert_eq!(config.seed, cloned.seed);
    assert_eq!(config.count, cloned.count);
}

// =============================================================================
// CorpusManifest Tests
// =============================================================================

#[test]
fn test_corpus_manifest_new() {
    let manifest = CorpusManifest::new(42);
    assert_eq!(manifest.seed, 42);
    assert_eq!(manifest.version, "1.0.0");
    assert!(manifest.templates.is_empty());
    assert!(manifest.profiles.is_empty());
    assert!(manifest.messages.is_empty());
    assert!(manifest.splits.train.is_empty());
    assert!(manifest.splits.validation.is_empty());
    assert!(manifest.splits.test.is_empty());
}

#[test]
fn test_corpus_manifest_add_template() {
    let mut manifest = CorpusManifest::new(42);
    manifest.add_template("test.yaml", "template content");

    assert_eq!(manifest.templates.len(), 1);
    assert_eq!(manifest.templates[0].path, "test.yaml");
    assert_eq!(manifest.templates[0].sha256.len(), 64); // SHA-256 hex string
}

#[test]
fn test_corpus_manifest_add_multiple_templates() {
    let mut manifest = CorpusManifest::new(42);
    manifest.add_template("adt_a01.yaml", "adt content");
    manifest.add_template("oru_r01.yaml", "oru content");

    assert_eq!(manifest.templates.len(), 2);
    assert_ne!(manifest.templates[0].sha256, manifest.templates[1].sha256);
}

#[test]
fn test_corpus_manifest_add_profile() {
    let mut manifest = CorpusManifest::new(42);
    manifest.add_profile("profile.yaml", "profile content");

    assert_eq!(manifest.profiles.len(), 1);
    assert_eq!(manifest.profiles[0].path, "profile.yaml");
    assert_eq!(manifest.profiles[0].sha256.len(), 64);
}

#[test]
fn test_corpus_manifest_add_message() {
    let mut manifest = CorpusManifest::new(42);
    manifest.add_message("msg001.hl7", "MSH|^~\\&|...", "ADT^A01", 0);

    assert_eq!(manifest.messages.len(), 1);
    assert_eq!(manifest.messages[0].path, "msg001.hl7");
    assert_eq!(manifest.messages[0].message_type, "ADT^A01");
    assert_eq!(manifest.messages[0].template_index, 0);
    assert_eq!(manifest.messages[0].sha256.len(), 64);
}

#[test]
fn test_corpus_manifest_message_count() {
    let mut manifest = CorpusManifest::new(42);
    assert_eq!(manifest.message_count(), 0);

    manifest.add_message("msg001.hl7", "content1", "ADT^A01", 0);
    assert_eq!(manifest.message_count(), 1);

    manifest.add_message("msg002.hl7", "content2", "ORU^R01", 1);
    assert_eq!(manifest.message_count(), 2);
}

#[test]
fn test_corpus_manifest_message_type_counts() {
    let mut manifest = CorpusManifest::new(42);
    manifest.add_message("msg001.hl7", "content1", "ADT^A01", 0);
    manifest.add_message("msg002.hl7", "content2", "ADT^A01", 0);
    manifest.add_message("msg003.hl7", "content3", "ORU^R01", 0);
    manifest.add_message("msg004.hl7", "content4", "ADT^A04", 0);

    let counts = manifest.message_type_counts();
    assert_eq!(*counts.get("ADT^A01").unwrap(), 2);
    assert_eq!(*counts.get("ORU^R01").unwrap(), 1);
    assert_eq!(*counts.get("ADT^A04").unwrap(), 1);
    assert_eq!(counts.len(), 3);
}

// =============================================================================
// JSON Serialization Tests
// =============================================================================

#[test]
fn test_corpus_manifest_json_roundtrip() {
    let mut manifest = CorpusManifest::new(42);
    manifest.add_template("test.yaml", "content");
    manifest.add_profile("profile.yaml", "profile content");
    manifest.add_message("msg001.hl7", "MSH|...", "ADT^A01", 0);

    let json = manifest.to_json().unwrap();
    let parsed = CorpusManifest::from_json(&json).unwrap();

    assert_eq!(parsed.seed, manifest.seed);
    assert_eq!(parsed.version, manifest.version);
    assert_eq!(parsed.templates.len(), manifest.templates.len());
    assert_eq!(parsed.profiles.len(), manifest.profiles.len());
    assert_eq!(parsed.messages.len(), manifest.messages.len());
}

#[test]
fn test_corpus_manifest_json_invalid() {
    let result = CorpusManifest::from_json("invalid json");
    assert!(result.is_err());

    if let Err(CorpusError::SerializationError(msg)) = result {
        assert!(msg.contains("expected") || msg.contains("invalid"));
    } else {
        panic!("Expected SerializationError");
    }
}

#[test]
fn test_corpus_manifest_json_structure() {
    let manifest = CorpusManifest::new(42);
    let json = manifest.to_json().unwrap();

    // Verify JSON structure contains expected fields
    assert!(json.contains("\"version\""));
    assert!(json.contains("\"tool_version\""));
    assert!(json.contains("\"seed\""));
    assert!(json.contains("\"templates\""));
    assert!(json.contains("\"messages\""));
    assert!(json.contains("\"generated_at\""));
}

// =============================================================================
// Split Tests
// =============================================================================

#[test]
fn test_corpus_manifest_create_splits() {
    let mut manifest = CorpusManifest::new(42);

    // Add 100 messages
    for i in 0..100 {
        manifest.add_message(
            &format!("msg{:03}.hl7", i),
            &format!("content{}", i),
            "ADT^A01",
            0,
        );
    }

    manifest.create_splits((0.7, 0.15, 0.15));

    // Check that splits were created
    assert!(!manifest.splits.train.is_empty());
    assert!(!manifest.splits.validation.is_empty());
    assert!(!manifest.splits.test.is_empty());

    // Total should equal original count
    let total =
        manifest.splits.train.len() + manifest.splits.validation.len() + manifest.splits.test.len();
    assert_eq!(total, 100);

    // Check approximate ratios (70/15/15)
    assert!(manifest.splits.train.len() >= 65 && manifest.splits.train.len() <= 75);
    assert!(manifest.splits.validation.len() >= 10 && manifest.splits.validation.len() <= 20);
    assert!(manifest.splits.test.len() >= 10 && manifest.splits.test.len() <= 20);
}

#[test]
fn test_corpus_manifest_create_splits_empty() {
    let mut manifest = CorpusManifest::new(42);
    manifest.create_splits((0.7, 0.15, 0.15));

    // Should not panic on empty manifest
    assert!(manifest.splits.train.is_empty());
    assert!(manifest.splits.validation.is_empty());
    assert!(manifest.splits.test.is_empty());
}

#[test]
fn test_corpus_manifest_create_splits_single_message() {
    let mut manifest = CorpusManifest::new(42);
    manifest.add_message("msg001.hl7", "content", "ADT^A01", 0);

    manifest.create_splits((0.7, 0.15, 0.15));

    let total =
        manifest.splits.train.len() + manifest.splits.validation.len() + manifest.splits.test.len();
    assert_eq!(total, 1);
}

#[test]
fn test_corpus_manifest_splits_reproducible() {
    let seed = 12345;

    let mut manifest1 = CorpusManifest::new(seed);
    for i in 0..20 {
        manifest1.add_message(
            &format!("msg{:03}.hl7", i),
            &format!("content{}", i),
            "ADT^A01",
            0,
        );
    }
    manifest1.create_splits((0.7, 0.15, 0.15));

    let mut manifest2 = CorpusManifest::new(seed);
    for i in 0..20 {
        manifest2.add_message(
            &format!("msg{:03}.hl7", i),
            &format!("content{}", i),
            "ADT^A01",
            0,
        );
    }
    manifest2.create_splits((0.7, 0.15, 0.15));

    // Same seed should produce same splits
    assert_eq!(manifest1.splits.train, manifest2.splits.train);
    assert_eq!(manifest1.splits.validation, manifest2.splits.validation);
    assert_eq!(manifest1.splits.test, manifest2.splits.test);
}

#[test]
fn test_corpus_manifest_splits_different_seeds() {
    let mut manifest1 = CorpusManifest::new(111);
    for i in 0..20 {
        manifest1.add_message(
            &format!("msg{:03}.hl7", i),
            &format!("content{}", i),
            "ADT^A01",
            0,
        );
    }
    manifest1.create_splits((0.7, 0.15, 0.15));

    let mut manifest2 = CorpusManifest::new(222);
    for i in 0..20 {
        manifest2.add_message(
            &format!("msg{:03}.hl7", i),
            &format!("content{}", i),
            "ADT^A01",
            0,
        );
    }
    manifest2.create_splits((0.7, 0.15, 0.15));

    // Different seeds should produce different splits (very likely)
    assert_ne!(manifest1.splits.train, manifest2.splits.train);
}

// =============================================================================
// SHA-256 Hash Tests
// =============================================================================

#[test]
fn test_compute_sha256_deterministic() {
    let hash1 = compute_sha256("test content");
    let hash2 = compute_sha256("test content");
    assert_eq!(hash1, hash2);
}

#[test]
fn test_compute_sha256_length() {
    let hash = compute_sha256("test content");
    assert_eq!(hash.len(), 64); // SHA-256 produces 256 bits = 64 hex chars
}

#[test]
fn test_compute_sha256_different_inputs() {
    let hash1 = compute_sha256("content1");
    let hash2 = compute_sha256("content2");
    assert_ne!(hash1, hash2);
}

#[test]
fn test_compute_sha256_empty_string() {
    let hash = compute_sha256("");
    assert_eq!(hash.len(), 64);
    // SHA-256 of empty string is known
    assert_eq!(
        hash,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn test_compute_sha256_known_value() {
    // SHA-256 of "hello" is known
    let hash = compute_sha256("hello");
    assert_eq!(
        hash,
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
    );
}

#[test]
fn test_compute_sha256_unicode() {
    let hash = compute_sha256("Hello, 世界!");
    assert_eq!(hash.len(), 64);
}

#[test]
fn test_compute_sha256_long_input() {
    let long_content = "x".repeat(10000);
    let hash = compute_sha256(&long_content);
    assert_eq!(hash.len(), 64);
}

// =============================================================================
// Message Hash Tests
// =============================================================================

#[test]
fn test_compute_message_hash() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![create_msh_segment()],
        charsets: vec![],
    };

    let hash = compute_message_hash(&message);
    assert_eq!(hash.len(), 64);
}

#[test]
fn test_compute_message_hash_deterministic() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![create_msh_segment()],
        charsets: vec![],
    };

    let hash1 = compute_message_hash(&message);
    let hash2 = compute_message_hash(&message);
    assert_eq!(hash1, hash2);
}

// =============================================================================
// Extract Message Type Tests
// =============================================================================

#[test]
fn test_extract_message_type_valid() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![create_msh_segment()],
        charsets: vec![],
    };

    let msg_type = extract_message_type(&message);
    assert_eq!(msg_type, "ADT^A01^ADT_A01");
}

#[test]
fn test_extract_message_type_no_msh() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![],
        charsets: vec![],
    };

    let msg_type = extract_message_type(&message);
    assert_eq!(msg_type, "UNKNOWN");
}

#[test]
fn test_extract_message_type_empty_fields() {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"MSH",
            fields: vec![],
        }],
        charsets: vec![],
    };

    let msg_type = extract_message_type(&message);
    assert_eq!(msg_type, "UNKNOWN");
}

// =============================================================================
// CorpusSplits Tests
// =============================================================================

#[test]
fn test_corpus_splits_default() {
    let splits = CorpusSplits::default();
    assert!(splits.train.is_empty());
    assert!(splits.validation.is_empty());
    assert!(splits.test.is_empty());
}

#[test]
fn test_corpus_splits_clone() {
    let splits = CorpusSplits {
        train: vec!["msg001.hl7".to_string()],
        validation: vec!["msg002.hl7".to_string()],
        test: vec!["msg003.hl7".to_string()],
    };

    let cloned = splits.clone();
    assert_eq!(splits.train, cloned.train);
    assert_eq!(splits.validation, cloned.validation);
    assert_eq!(splits.test, cloned.test);
}

// =============================================================================
// TemplateInfo Tests
// =============================================================================

#[test]
fn test_template_info_serialization() {
    let info = TemplateInfo {
        path: "test.yaml".to_string(),
        sha256: "abc123def456".to_string(),
    };

    let json = serde_json::to_string(&info).unwrap();
    let parsed: TemplateInfo = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.path, info.path);
    assert_eq!(parsed.sha256, info.sha256);
}

#[test]
fn test_template_info_clone() {
    let info = TemplateInfo {
        path: "test.yaml".to_string(),
        sha256: "abc123".to_string(),
    };

    let cloned = info.clone();
    assert_eq!(info.path, cloned.path);
    assert_eq!(info.sha256, cloned.sha256);
}

// =============================================================================
// ProfileInfo Tests
// =============================================================================

#[test]
fn test_profile_info_serialization() {
    let info = ProfileInfo {
        path: "profile.yaml".to_string(),
        sha256: "def789".to_string(),
    };

    let json = serde_json::to_string(&info).unwrap();
    let parsed: ProfileInfo = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.path, info.path);
    assert_eq!(parsed.sha256, info.sha256);
}

// =============================================================================
// MessageInfo Tests
// =============================================================================

#[test]
fn test_message_info_serialization() {
    let info = MessageInfo {
        path: "msg001.hl7".to_string(),
        sha256: "hash123".to_string(),
        message_type: "ADT^A01".to_string(),
        template_index: 0,
    };

    let json = serde_json::to_string(&info).unwrap();
    let parsed: MessageInfo = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.path, info.path);
    assert_eq!(parsed.sha256, info.sha256);
    assert_eq!(parsed.message_type, info.message_type);
    assert_eq!(parsed.template_index, info.template_index);
}

#[test]
fn test_message_info_clone() {
    let info = MessageInfo {
        path: "msg001.hl7".to_string(),
        sha256: "hash123".to_string(),
        message_type: "ADT^A01".to_string(),
        template_index: 2,
    };

    let cloned = info.clone();
    assert_eq!(info.path, cloned.path);
    assert_eq!(info.sha256, cloned.sha256);
    assert_eq!(info.message_type, cloned.message_type);
    assert_eq!(info.template_index, cloned.template_index);
}

// =============================================================================
// CorpusError Tests
// =============================================================================

#[test]
fn test_corpus_error_display() {
    let err = CorpusError::SerializationError("test error".to_string());
    assert!(err.to_string().contains("test error"));
    assert!(err.to_string().contains("Serialization error"));

    let err = CorpusError::IoError("file not found".to_string());
    assert!(err.to_string().contains("file not found"));
    assert!(err.to_string().contains("IO error"));

    let err = CorpusError::InvalidConfig("bad config".to_string());
    assert!(err.to_string().contains("bad config"));
    assert!(err.to_string().contains("Invalid configuration"));

    let err = CorpusError::InvalidSplitRatios;
    assert!(err.to_string().contains("Invalid split ratios"));
}

#[test]
fn test_corpus_error_clone() {
    let err = CorpusError::SerializationError("test".to_string());
    let cloned = err.clone();
    assert!(matches!(cloned, CorpusError::SerializationError(_)));
}

// =============================================================================
// Edge Cases and Boundary Tests
// =============================================================================

#[test]
fn test_corpus_manifest_large_number_of_messages() {
    let mut manifest = CorpusManifest::new(42);

    for i in 0..10000 {
        manifest.add_message(
            &format!("msg{:06}.hl7", i),
            &format!("content{}", i),
            "ADT^A01",
            0,
        );
    }

    assert_eq!(manifest.message_count(), 10000);

    // JSON serialization should still work
    let json = manifest.to_json().unwrap();
    let parsed = CorpusManifest::from_json(&json).unwrap();
    assert_eq!(parsed.message_count(), 10000);
}

#[test]
fn test_corpus_manifest_special_characters_in_path() {
    let mut manifest = CorpusManifest::new(42);

    // Paths with special characters
    manifest.add_template("path/with/slashes.yaml", "content");
    manifest.add_message("path/with/slashes/msg.hl7", "content", "ADT^A01", 0);

    let json = manifest.to_json().unwrap();
    let parsed = CorpusManifest::from_json(&json).unwrap();

    assert_eq!(parsed.templates[0].path, "path/with/slashes.yaml");
    assert_eq!(parsed.messages[0].path, "path/with/slashes/msg.hl7");
}

#[test]
fn test_corpus_manifest_unicode_in_content() {
    let mut manifest = CorpusManifest::new(42);

    manifest.add_template("test.yaml", "模板内容"); // Chinese characters
    manifest.add_message("msg.hl7", "MSH|^~\\&|医院|...", "ADT^A01", 0);

    let json = manifest.to_json().unwrap();
    let parsed = CorpusManifest::from_json(&json).unwrap();

    assert_eq!(parsed.templates[0].sha256, manifest.templates[0].sha256);
    assert_eq!(parsed.messages[0].sha256, manifest.messages[0].sha256);
}

#[test]
fn test_corpus_manifest_empty_content() {
    let mut manifest = CorpusManifest::new(42);

    manifest.add_template("empty.yaml", "");
    manifest.add_message("empty.hl7", "", "ADT^A01", 0);

    // Should still compute hash (non-empty)
    assert_eq!(manifest.templates[0].sha256.len(), 64);
    assert_eq!(manifest.messages[0].sha256.len(), 64);
}

#[test]
fn test_corpus_splits_no_overlap() {
    let mut manifest = CorpusManifest::new(42);

    for i in 0..100 {
        manifest.add_message(
            &format!("msg{:03}.hl7", i),
            &format!("content{}", i),
            "ADT^A01",
            0,
        );
    }

    manifest.create_splits((0.7, 0.15, 0.15));

    // Check that no message appears in multiple splits
    let train_set: std::collections::HashSet<_> = manifest.splits.train.iter().cloned().collect();
    let val_set: std::collections::HashSet<_> =
        manifest.splits.validation.iter().cloned().collect();
    let test_set: std::collections::HashSet<_> = manifest.splits.test.iter().cloned().collect();

    // Check no overlap
    for path in &train_set {
        assert!(
            !val_set.contains(path),
            "Path {} in both train and validation",
            path
        );
        assert!(
            !test_set.contains(path),
            "Path {} in both train and test",
            path
        );
    }
    for path in &val_set {
        assert!(
            !test_set.contains(path),
            "Path {} in both validation and test",
            path
        );
    }
}
