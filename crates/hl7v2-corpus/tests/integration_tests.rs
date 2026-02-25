//! Integration tests for hl7v2-corpus

use hl7v2_corpus::*;
use hl7v2_core::{Message, Segment, Field, Rep, Comp, Atom, Delims};

/// Helper to create a test message
fn create_test_message(message_type: &str) -> Message {
    let parts: Vec<&str> = message_type.split('^').collect();
    
    Message {
        delims: Delims::default(),
        segments: vec![
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
                            comps: parts.iter().map(|p| Comp {
                                subs: vec![Atom::Text(p.to_string())],
                            }).collect(),
                        }],
                    },
                ],
            },
            Segment {
                id: *b"PID",
                fields: vec![
                    Field {
                        reps: vec![Rep {
                            comps: vec![Comp {
                                subs: vec![Atom::Text("1".to_string())],
                            }],
                        }],
                    },
                    Field {
                        reps: vec![Rep {
                            comps: vec![Comp {
                                subs: vec![Atom::Text("12345".to_string())],
                            }],
                        }],
                    },
                ],
            },
        ],
        charsets: vec![],
    }
}

// =============================================================================
// Corpus Manifest Integration Tests
// =============================================================================

#[test]
fn test_full_corpus_workflow() {
    // Create a manifest with seed
    let mut manifest = CorpusManifest::new(42);
    
    // Add templates
    manifest.add_template("templates/adt_a01.yaml", "ADT^A01 template content");
    manifest.add_template("templates/oru_r01.yaml", "ORU^R01 template content");
    
    // Add profiles
    manifest.add_profile("profiles/adt_a01.yaml", "ADT^A01 profile");
    
    // Add messages
    for i in 0..50 {
        let msg_type = if i % 2 == 0 { "ADT^A01" } else { "ORU^R01" };
        manifest.add_message(
            &format!("messages/msg{:04}.hl7", i),
            &format!("MSH|^~\\&|...content{}...", i),
            msg_type,
            i % 2
        );
    }
    
    // Create splits
    manifest.create_splits((0.7, 0.15, 0.15));
    
    // Serialize to JSON
    let json = manifest.to_json().unwrap();
    
    // Deserialize
    let parsed = CorpusManifest::from_json(&json).unwrap();
    
    // Verify
    assert_eq!(parsed.seed, 42);
    assert_eq!(parsed.templates.len(), 2);
    assert_eq!(parsed.profiles.len(), 1);
    assert_eq!(parsed.messages.len(), 50);
    assert_eq!(parsed.message_type_counts().get("ADT^A01"), Some(&25));
    assert_eq!(parsed.message_type_counts().get("ORU^R01"), Some(&25));
    
    // Verify splits
    let total_split: usize = parsed.splits.train.len() 
        + parsed.splits.validation.len() 
        + parsed.splits.test.len();
    assert_eq!(total_split, 50);
}

#[test]
fn test_corpus_config_integration() {
    let config = CorpusConfig {
        seed: 12345,
        count: 1000,
        batch_size: 100,
        output_dir: Some("/tmp/corpus".to_string()),
        create_splits: true,
        split_ratios: Some((0.8, 0.1, 0.1)),
    };
    
    // Create manifest based on config
    let mut manifest = CorpusManifest::new(config.seed);
    
    // Simulate adding messages based on config
    for i in 0..config.count {
        manifest.add_message(
            &format!("msg{:04}.hl7", i),
            &format!("content{}", i),
            "ADT^A01",
            0
        );
    }
    
    if config.create_splits {
        if let Some(ratios) = config.split_ratios {
            manifest.create_splits(ratios);
        }
    }
    
    assert_eq!(manifest.message_count(), 1000);
    assert!(!manifest.splits.train.is_empty());
}

// =============================================================================
// Message Hash Integration Tests
// =============================================================================

#[test]
fn test_message_hash_consistency() {
    let message = create_test_message("ADT^A01");
    
    // Hash should be consistent
    let hash1 = compute_message_hash(&message);
    let hash2 = compute_message_hash(&message);
    
    assert_eq!(hash1, hash2);
    assert_eq!(hash1.len(), 64);
}

#[test]
fn test_different_messages_different_hashes() {
    let msg1 = create_test_message("ADT^A01");
    let msg2 = create_test_message("ORU^R01");
    
    let hash1 = compute_message_hash(&msg1);
    let hash2 = compute_message_hash(&msg2);
    
    // Different messages should have different hashes
    assert_ne!(hash1, hash2);
}

// =============================================================================
// Extract Message Type Integration Tests
// =============================================================================

#[test]
fn test_extract_message_type_various() {
    let test_cases = vec![
        ("ADT^A01", "ADT^A01"),
        ("ORU^R01", "ORU^R01"),
        ("DFT^P03", "DFT^P03"),
        ("ACK^A01", "ACK^A01"),
    ];
    
    for (msg_type, expected) in test_cases {
        let message = create_test_message(msg_type);
        let extracted = extract_message_type(&message);
        assert_eq!(extracted, expected, "Failed for message type: {}", msg_type);
    }
}

// =============================================================================
// Split Integration Tests
// =============================================================================

#[test]
fn test_splits_maintain_message_coverage() {
    let mut manifest = CorpusManifest::new(42);
    
    // Add messages with different types
    for i in 0..100 {
        let msg_type = match i % 4 {
            0 => "ADT^A01",
            1 => "ADT^A04",
            2 => "ORU^R01",
            _ => "DFT^P03",
        };
        manifest.add_message(
            &format!("msg{:03}.hl7", i),
            &format!("content{}", i),
            msg_type,
            0
        );
    }
    
    manifest.create_splits((0.7, 0.15, 0.15));
    
    // Verify all messages are in exactly one split
    let mut all_paths: std::collections::HashSet<String> = std::collections::HashSet::new();
    for msg in &manifest.messages {
        all_paths.insert(msg.path.clone());
    }
    
    let mut split_paths: std::collections::HashSet<String> = std::collections::HashSet::new();
    for path in &manifest.splits.train {
        assert!(split_paths.insert(path.clone()), "Duplicate path in splits: {}", path);
    }
    for path in &manifest.splits.validation {
        assert!(split_paths.insert(path.clone()), "Duplicate path in splits: {}", path);
    }
    for path in &manifest.splits.test {
        assert!(split_paths.insert(path.clone()), "Duplicate path in splits: {}", path);
    }
    
    assert_eq!(all_paths.len(), split_paths.len());
    assert_eq!(all_paths, split_paths);
}

#[test]
fn test_splits_different_seeds_different_distributions() {
    let mut manifests: Vec<CorpusManifest> = Vec::new();
    
    for seed in [42, 100, 200, 300] {
        let mut manifest = CorpusManifest::new(seed);
        for i in 0..50 {
            manifest.add_message(
                &format!("msg{:03}.hl7", i),
                &format!("content{}", i),
                "ADT^A01",
                0
            );
        }
        manifest.create_splits((0.7, 0.15, 0.15));
        manifests.push(manifest);
    }
    
    // At least some splits should be different
    let train_sets: Vec<Vec<String>> = manifests.iter().map(|m| m.splits.train.clone()).collect();
    
    // Not all train sets should be identical
    let all_same = train_sets.windows(2).all(|w| w[0] == w[1]);
    assert!(!all_same, "All splits are identical despite different seeds");
}

// =============================================================================
// JSON Persistence Integration Tests
// =============================================================================

#[test]
fn test_json_persistence_with_all_fields() {
    let mut manifest = CorpusManifest::new(999);
    
    // Add all types of data
    manifest.add_template("template1.yaml", "template content 1");
    manifest.add_template("template2.yaml", "template content 2");
    manifest.add_profile("profile1.yaml", "profile content");
    manifest.add_message("msg001.hl7", "message 1 content", "ADT^A01", 0);
    manifest.add_message("msg002.hl7", "message 2 content", "ORU^R01", 1);
    manifest.create_splits((0.5, 0.25, 0.25));
    
    // Serialize
    let json = manifest.to_json().unwrap();
    
    // Verify JSON is valid and contains expected fields
    assert!(json.contains("\"seed\": 999"));
    assert!(json.contains("\"template1.yaml\""));
    assert!(json.contains("\"profile1.yaml\""));
    assert!(json.contains("\"msg001.hl7\""));
    assert!(json.contains("\"ADT^A01\""));
    
    // Deserialize and verify all fields match
    let parsed = CorpusManifest::from_json(&json).unwrap();
    
    assert_eq!(parsed.seed, manifest.seed);
    assert_eq!(parsed.templates.len(), manifest.templates.len());
    assert_eq!(parsed.profiles.len(), manifest.profiles.len());
    assert_eq!(parsed.messages.len(), manifest.messages.len());
    assert_eq!(parsed.splits.train.len(), manifest.splits.train.len());
    assert_eq!(parsed.splits.validation.len(), manifest.splits.validation.len());
    assert_eq!(parsed.splits.test.len(), manifest.splits.test.len());
    
    // Verify hashes match
    for (orig, parsed) in manifest.templates.iter().zip(parsed.templates.iter()) {
        assert_eq!(orig.sha256, parsed.sha256);
    }
    for (orig, parsed) in manifest.messages.iter().zip(parsed.messages.iter()) {
        assert_eq!(orig.sha256, parsed.sha256);
    }
}

// =============================================================================
// SHA-256 Hash Integration Tests
// =============================================================================

#[test]
fn test_sha256_with_realistic_hl7_content() {
    let hl7_content = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John^Robert||19700101|M\r";
    
    let hash = compute_sha256(hl7_content);
    
    // Should be 64 hex characters
    assert_eq!(hash.len(), 64);
    
    // Should be deterministic
    assert_eq!(hash, compute_sha256(hl7_content));
    
    // Should be lowercase hex
    for c in hash.chars() {
        assert!(c.is_ascii_lowercase() || c.is_ascii_digit());
    }
}

#[test]
fn test_sha256_different_encodings() {
    // Same content with different line endings should produce different hashes
    let content_cr = "MSH|...\rPID|...\r";
    let content_crlf = "MSH|...\r\nPID|...\r\n";
    let content_lf = "MSH|...\nPID|...\n";
    
    let hash_cr = compute_sha256(content_cr);
    let hash_crlf = compute_sha256(content_crlf);
    let hash_lf = compute_sha256(content_lf);
    
    assert_ne!(hash_cr, hash_crlf);
    assert_ne!(hash_crlf, hash_lf);
    assert_ne!(hash_cr, hash_lf);
}

// =============================================================================
// Error Handling Integration Tests
// =============================================================================

#[test]
fn test_invalid_json_handling() {
    let invalid_json_cases = vec![
        "",
        "not json",
        "{invalid}",
        r#"{"seed": "not a number"}"#,
        r#"{"version": 123}"#, // wrong type
    ];
    
    for invalid in invalid_json_cases {
        let result = CorpusManifest::from_json(invalid);
        assert!(result.is_err(), "Should fail for: {}", invalid);
    }
}

// =============================================================================
// Large Corpus Integration Tests
// =============================================================================

#[test]
fn test_large_corpus_performance() {
    let mut manifest = CorpusManifest::new(42);
    
    // Add a large number of messages
    let message_count = 10000;
    for i in 0..message_count {
        manifest.add_message(
            &format!("messages/{:06}.hl7", i),
            &format!("MSH|^~\\&|...content{}...", i),
            if i % 2 == 0 { "ADT^A01" } else { "ORU^R01" },
            i % 2
        );
    }
    
    // Verify count
    assert_eq!(manifest.message_count(), message_count);
    
    // Create splits (should be efficient)
    manifest.create_splits((0.7, 0.15, 0.15));
    
    // Verify splits
    let total: usize = manifest.splits.train.len() 
        + manifest.splits.validation.len() 
        + manifest.splits.test.len();
    assert_eq!(total, message_count);
    
    // JSON serialization should work
    let json = manifest.to_json().unwrap();
    assert!(!json.is_empty());
}

// =============================================================================
// Cross-Crate Integration Tests
// =============================================================================

#[test]
fn test_corpus_with_parsed_message() {
    // Parse a real HL7 message using hl7v2-core
    let hl7_bytes = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r";
    
    let message = hl7v2_core::parse(hl7_bytes).unwrap();
    
    // Extract message type
    let msg_type = extract_message_type(&message);
    assert_eq!(msg_type, "ADT^A01^ADT_A01");
    
    // Compute hash
    let hash = compute_message_hash(&message);
    assert_eq!(hash.len(), 64);
    
    // Add to manifest
    let mut manifest = CorpusManifest::new(42);
    let content = String::from_utf8_lossy(hl7_bytes);
    manifest.add_message("parsed.hl7", &content, &msg_type, 0);
    
    assert_eq!(manifest.message_count(), 1);
}
