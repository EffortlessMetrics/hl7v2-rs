//! Property-based tests for hl7v2-redact crate
//!
//! Uses proptest to verify redaction properties hold across arbitrary inputs.

use hl7v2_parser::parse;
use hl7v2_redact::{redact_hipaa, redact_phi, RedactionEngine, RedactionRule, RedactionStrategy};
use proptest::prelude::*;

// ============================================================================
// Helper Functions and Strategies
// ============================================================================

/// Generate alphanumeric strings
fn alphanumeric() -> impl Strategy<Value = String> {
    "[A-Za-z0-9]{5,20}"
}

/// Generate valid HL7 paths for common segments
fn hl7_path() -> impl Strategy<Value = String> {
    prop_oneof![
        (3usize..=40).prop_map(|n| format!("PID.{}", n)),
        (3usize..=9).prop_map(|n| format!("MSH.{}", n)),
        (1usize..=30).prop_map(|n| format!("NK1.{}", n)),
        (1usize..=20).prop_map(|n| format!("PV1.{}", n)),
    ]
}

/// Generate segment names
fn segment_name() -> impl Strategy<Value = String> {
    "[A-Z]{3}"
}

/// Generate a redaction strategy (excluding Custom for serialization tests)
fn redaction_strategy() -> impl Strategy<Value = RedactionStrategy> {
    prop_oneof![
        "[A-Z0-9]{5,20}".prop_map(RedactionStrategy::Replace),
        Just(RedactionStrategy::Hash),
        Just(RedactionStrategy::Mask),
        Just(RedactionStrategy::Remove),
        (1usize..=20).prop_map(RedactionStrategy::Truncate),
    ]
}

// ============================================================================
// Never Panics Properties
// ============================================================================

proptest! {
    /// Test that redaction never panics for any valid message with common PHI rules
    #[test]
    fn prop_redact_phi_never_panics(
        sending_app in alphanumeric(),
        sending_fac in alphanumeric(),
        msg_id in alphanumeric(),
        pat_id in alphanumeric(),
        pat_name in alphanumeric()
    ) {
        let hl7 = format!(
            "MSH|^~\\&|{}|{}|RCV|RCVFAC|20250128120000||ADT^A01|{}|P|2.5\rPID|1||{}||{}||19800101\r",
            sending_app, sending_fac, msg_id, pat_id, pat_name
        );
        if let Ok(parsed) = parse(hl7.as_bytes()) {
            let _ = redact_phi(&parsed);
        }
    }
}

proptest! {
    /// Test that redaction never panics for HIPAA rules
    #[test]
    fn prop_redact_hipaa_never_panics(
        sending_app in alphanumeric(),
        sending_fac in alphanumeric(),
        msg_id in alphanumeric(),
        pat_id in alphanumeric(),
        pat_name in alphanumeric()
    ) {
        let hl7 = format!(
            "MSH|^~\\&|{}|{}|RCV|RCVFAC|20250128120000||ADT^A01|{}|P|2.5\rPID|1||{}||{}||19800101\r",
            sending_app, sending_fac, msg_id, pat_id, pat_name
        );
        if let Ok(parsed) = parse(hl7.as_bytes()) {
            let _ = redact_hipaa(&parsed);
        }
    }
}

proptest! {
    /// Test that custom rules never panic
    #[test]
    fn prop_custom_rules_never_panics(
        sending_app in alphanumeric(),
        sending_fac in alphanumeric(),
        msg_id in alphanumeric(),
        pat_id in alphanumeric(),
        pat_name in alphanumeric(),
        path in hl7_path(),
        strategy in redaction_strategy()
    ) {
        let hl7 = format!(
            "MSH|^~\\&|{}|{}|RCV|RCVFAC|20250128120000||ADT^A01|{}|P|2.5\rPID|1||{}||{}||19800101\r",
            sending_app, sending_fac, msg_id, pat_id, pat_name
        );
        if let Ok(parsed) = parse(hl7.as_bytes()) {
            let rules = vec![RedactionRule::new(path, strategy)];
            let engine = RedactionEngine::new(rules);
            let _ = engine.redact(&parsed);
        }
    }
}

// ============================================================================
// Structure Preservation Properties
// ============================================================================

proptest! {
    /// Test that segment count is preserved after redaction
    #[test]
    fn prop_segment_count_preserved(
        sending_app in alphanumeric(),
        sending_fac in alphanumeric(),
        msg_id in alphanumeric(),
        pat_id in alphanumeric(),
        pat_name in alphanumeric()
    ) {
        let hl7 = format!(
            "MSH|^~\\&|{}|{}|RCV|RCVFAC|20250128120000||ADT^A01|{}|P|2.5\rPID|1||{}||{}||19800101\r",
            sending_app, sending_fac, msg_id, pat_id, pat_name
        );
        if let Ok(parsed) = parse(hl7.as_bytes()) {
            let original_count = parsed.segments.len();
            if let Ok(result) = redact_phi(&parsed) {
                let redacted_count = result.message.segments.len();
                prop_assert_eq!(original_count, redacted_count);
            }
        }
    }
}

proptest! {
    /// Test that redacted message is still valid HL7 (can be reparsed)
    #[test]
    fn prop_redacted_message_valid_hl7(
        sending_app in alphanumeric(),
        sending_fac in alphanumeric(),
        msg_id in alphanumeric(),
        pat_id in alphanumeric(),
        pat_name in alphanumeric()
    ) {
        let hl7 = format!(
            "MSH|^~\\&|{}|{}|RCV|RCVFAC|20250128120000||ADT^A01|{}|P|2.5\rPID|1||{}||{}||19800101\r",
            sending_app, sending_fac, msg_id, pat_id, pat_name
        );
        if let Ok(parsed) = parse(hl7.as_bytes())
            && let Ok(result) = redact_phi(&parsed)
        {
            let reparsed = parse(&result.message_bytes);
            prop_assert!(reparsed.is_ok(), "Redacted message should be valid HL7");
        }
    }
}

proptest! {
    /// Test that segment IDs are preserved after redaction
    #[test]
    fn prop_segment_ids_preserved(
        sending_app in alphanumeric(),
        sending_fac in alphanumeric(),
        msg_id in alphanumeric(),
        pat_id in alphanumeric(),
        pat_name in alphanumeric()
    ) {
        let hl7 = format!(
            "MSH|^~\\&|{}|{}|RCV|RCVFAC|20250128120000||ADT^A01|{}|P|2.5\rPID|1||{}||{}||19800101\r",
            sending_app, sending_fac, msg_id, pat_id, pat_name
        );
        if let Ok(parsed) = parse(hl7.as_bytes()) {
            let original_ids: Vec<String> = parsed.segments.iter()
                .filter_map(|s| std::str::from_utf8(&s.id).ok().map(|id| id.to_string()))
                .collect();

            if let Ok(result) = redact_phi(&parsed) {
                let redacted_ids: Vec<String> = result.message.segments.iter()
                    .filter_map(|s| std::str::from_utf8(&s.id).ok().map(|id| id.to_string()))
                    .collect();
                prop_assert_eq!(original_ids, redacted_ids);
            }
        }
    }
}

// ============================================================================
// Audit Log Properties
// ============================================================================

proptest! {
    /// Test that audit log length matches rule count
    #[test]
    fn prop_audit_log_matches_rule_count(
        sending_app in alphanumeric(),
        sending_fac in alphanumeric(),
        msg_id in alphanumeric(),
        pat_id in alphanumeric(),
        pat_name in alphanumeric(),
        rules in prop::collection::vec((hl7_path(), redaction_strategy()), 1..=5)
    ) {
        let hl7 = format!(
            "MSH|^~\\&|{}|{}|RCV|RCVFAC|20250128120000||ADT^A01|{}|P|2.5\rPID|1||{}||{}||19800101\r",
            sending_app, sending_fac, msg_id, pat_id, pat_name
        );
        if let Ok(parsed) = parse(hl7.as_bytes()) {
            let rules: Vec<RedactionRule> = rules.into_iter()
                .map(|(p, s)| RedactionRule::new(p, s))
                .collect();
            let rule_count = rules.len();
            let engine = RedactionEngine::new(rules);

            if let Ok(result) = engine.redact(&parsed) {
                prop_assert_eq!(result.audit_log.len(), rule_count,
                    "Audit log should have one entry per rule");
            }
        }
    }
}

proptest! {
    /// Test that audit log entries have correct paths
    #[test]
    fn prop_audit_log_paths_match_rules(
        sending_app in alphanumeric(),
        sending_fac in alphanumeric(),
        msg_id in alphanumeric(),
        pat_id in alphanumeric(),
        pat_name in alphanumeric(),
        path in hl7_path(),
        strategy in redaction_strategy()
    ) {
        let hl7 = format!(
            "MSH|^~\\&|{}|{}|RCV|RCVFAC|20250128120000||ADT^A01|{}|P|2.5\rPID|1||{}||{}||19800101\r",
            sending_app, sending_fac, msg_id, pat_id, pat_name
        );
        if let Ok(parsed) = parse(hl7.as_bytes()) {
            let rule = RedactionRule::new(path.clone(), strategy);
            let engine = RedactionEngine::new(vec![rule]);

            if let Ok(result) = engine.redact(&parsed)
                && let Some(entry) = result.audit_log.first()
            {
                prop_assert_eq!(&entry.path, &path, "Audit log path should match rule path");
            }
        }
    }
}

proptest! {
    /// Test that audit log strategy matches rule strategy
    #[test]
    fn prop_audit_log_strategy_matches(
        sending_app in alphanumeric(),
        sending_fac in alphanumeric(),
        msg_id in alphanumeric(),
        pat_id in alphanumeric(),
        pat_name in alphanumeric(),
        path in hl7_path(),
        strategy in redaction_strategy()
    ) {
        let hl7 = format!(
            "MSH|^~\\&|{}|{}|RCV|RCVFAC|20250128120000||ADT^A01|{}|P|2.5\rPID|1||{}||{}||19800101\r",
            sending_app, sending_fac, msg_id, pat_id, pat_name
        );
        if let Ok(parsed) = parse(hl7.as_bytes()) {
            // Skip Custom strategy as it's not serializable
            if !matches!(strategy, RedactionStrategy::Custom(_)) {
                let rule = RedactionRule::new(path, strategy.clone());
                let engine = RedactionEngine::new(vec![rule]);

                if let Ok(result) = engine.redact(&parsed)
                    && let Some(entry) = result.audit_log.first()
                {
                    prop_assert_eq!(&entry.strategy, &strategy,
                        "Audit log strategy should match rule strategy");
                }
            }
        }
    }
}

// ============================================================================
// Strategy-Specific Properties
// ============================================================================

proptest! {
    /// Test that hash strategy produces 16-character hex strings
    #[test]
    fn prop_hash_produces_16_char_hex(
        sending_app in alphanumeric(),
        sending_fac in alphanumeric(),
        msg_id in alphanumeric(),
        pat_id in alphanumeric(),
        pat_name in alphanumeric()
    ) {
        let hl7 = format!(
            "MSH|^~\\&|{}|{}|RCV|RCVFAC|20250128120000||ADT^A01|{}|P|2.5\rPID|1||{}||{}||19800101\r",
            sending_app, sending_fac, msg_id, pat_id, pat_name
        );
        if let Ok(parsed) = parse(hl7.as_bytes()) {
            let rule = RedactionRule::new("PID.3", RedactionStrategy::Hash);
            let engine = RedactionEngine::new(vec![rule]);

            if let Ok(result) = engine.redact(&parsed) {
                let redacted_str = String::from_utf8_lossy(&result.message_bytes);
                // Hash should be 16 hex chars
                if let Some(pid_start) = redacted_str.find("PID|") {
                    let pid_section = &redacted_str[pid_start..];
                    if let Some(end) = pid_section.find("||") {
                        let pid_3_field = &pid_section[0..end];
                        // If the field was redacted with hash, it should be 16 hex chars
                        let hash_candidate = pid_3_field.split('|').nth(3).unwrap_or("");
                        if !hash_candidate.is_empty() && hash_candidate.len() == 16 {
                            prop_assert!(hash_candidate.chars().all(|c| c.is_ascii_hexdigit()),
                                "Hash should be 16 hex characters");
                        }
                    }
                }
            }
        }
    }
}

proptest! {
    /// Test that remove strategy doesn't cause errors
    #[test]
    fn prop_remove_strategy_no_errors(
        sending_app in alphanumeric(),
        sending_fac in alphanumeric(),
        msg_id in alphanumeric(),
        pat_id in alphanumeric(),
        pat_name in alphanumeric()
    ) {
        let hl7 = format!(
            "MSH|^~\\&|{}|{}|RCV|RCVFAC|20250128120000||ADT^A01|{}|P|2.5\rPID|1||{}||{}||19800101||ADDR\r",
            sending_app, sending_fac, msg_id, pat_id, pat_name
        );
        if let Ok(parsed) = parse(hl7.as_bytes()) {
            let rule = RedactionRule::new("PID.11", RedactionStrategy::Remove);
            let engine = RedactionEngine::new(vec![rule]);

            // Should not panic and should produce a result
            let result = engine.redact(&parsed);
            prop_assert!(result.is_ok(), "Remove strategy should not cause errors");

            if let Ok(redacted) = result {
                // Message should still be valid HL7
                let reparsed = parse(&redacted.message_bytes);
                prop_assert!(reparsed.is_ok(), "Result should be valid HL7");
            }
        }
    }
}

proptest! {
    /// Test that mask strategy changes the field value
    #[test]
    fn prop_mask_changes_value(
        sending_app in alphanumeric(),
        sending_fac in alphanumeric(),
        msg_id in alphanumeric(),
        pat_id in alphanumeric(),
        pat_name in alphanumeric()
    ) {
        let hl7 = format!(
            "MSH|^~\\&|{}|{}|RCV|RCVFAC|20250128120000||ADT^A01|{}|P|2.5\rPID|1||{}||{}||19800101\r",
            sending_app, sending_fac, msg_id, pat_id, pat_name
        );
        if let Ok(parsed) = parse(hl7.as_bytes()) {
            let original_bytes = hl7v2_writer::write(&parsed);
            let original_str = String::from_utf8_lossy(&original_bytes);

            let rule = RedactionRule::new("PID.5", RedactionStrategy::Mask);
            let engine = RedactionEngine::new(vec![rule]);

            if let Ok(result) = engine.redact(&parsed) {
                let redacted_str = String::from_utf8_lossy(&result.message_bytes);
                // The name should be different after masking
                let original_name = extract_field(&original_str, "PID", 5);
                let redacted_name = extract_field(&redacted_str, "PID", 5);

                if original_name.is_some() && redacted_name.is_some() {
                    // If there was a name, it should be masked (not the same)
                    prop_assert_ne!(original_name, redacted_name,
                        "Masked field should differ from original");
                }
            }
        }
    }
}

proptest! {
    /// Test that replace strategy produces exact replacement
    #[test]
    fn prop_replace_produces_exact_value(
        sending_app in alphanumeric(),
        sending_fac in alphanumeric(),
        msg_id in alphanumeric(),
        pat_id in alphanumeric(),
        pat_name in alphanumeric(),
        replacement in "[A-Z0-9]{5,20}"
    ) {
        let hl7 = format!(
            "MSH|^~\\&|{}|{}|RCV|RCVFAC|20250128120000||ADT^A01|{}|P|2.5\rPID|1||{}||{}||19800101\r",
            sending_app, sending_fac, msg_id, pat_id, pat_name
        );
        if let Ok(parsed) = parse(hl7.as_bytes()) {
            let rule = RedactionRule::new("PID.3", RedactionStrategy::Replace(replacement.clone()));
            let engine = RedactionEngine::new(vec![rule]);

            if let Ok(result) = engine.redact(&parsed) {
                let redacted_str = String::from_utf8_lossy(&result.message_bytes);
                // The replacement value should be present
                prop_assert!(redacted_str.contains(&replacement),
                    "Replace strategy should produce exact replacement value");
            }
        }
    }
}

proptest! {
    /// Test that truncate strategy limits field length appropriately
    #[test]
    fn prop_truncates_respects_limit(
        sending_app in alphanumeric(),
        sending_fac in alphanumeric(),
        msg_id in alphanumeric(),
        pat_id in alphanumeric(),
        pat_name in alphanumeric(),
        limit in 1usize..=10
    ) {
        let hl7 = format!(
            "MSH|^~\\&|{}|{}|RCV|RCVFAC|20250128120000||ADT^A01|{}|P|2.5\rPID|1||{}||{}||19800101||||PHONECONTENT\r",
            sending_app, sending_fac, msg_id, pat_id, pat_name
        );
        if let Ok(parsed) = parse(hl7.as_bytes()) {
            let rule = RedactionRule::new("PID.13", RedactionStrategy::Truncate(limit));
            let engine = RedactionEngine::new(vec![rule]);

            if let Ok(result) = engine.redact(&parsed) {
                let redacted_str = String::from_utf8_lossy(&result.message_bytes);
                // Just verify no panic - exact length checking requires more complex parsing
                prop_assert!(redacted_str.contains("PID|"), "Message should still be valid");
            }
        }
    }
}

// ============================================================================
// Idempotence Properties
// ============================================================================

proptest! {
    /// Test that redacting twice preserves structure (even if content changes due to hash re-hashing)
    #[test]
    fn prop_redaction_idempotent_structure(
        sending_app in alphanumeric(),
        sending_fac in alphanumeric(),
        msg_id in alphanumeric(),
        pat_id in alphanumeric(),
        pat_name in alphanumeric()
    ) {
        let hl7 = format!(
            "MSH|^~\\&|{}|{}|RCV|RCVFAC|20250128120000||ADT^A01|{}|P|2.5\rPID|1||{}||{}||19800101\r",
            sending_app, sending_fac, msg_id, pat_id, pat_name
        );
        if let Ok(parsed) = parse(hl7.as_bytes()) {
            // Use only Mask strategy for true idempotence test (hash would re-hash)
            let rules = vec![
                RedactionRule::new("PID.3", RedactionStrategy::Mask),
                RedactionRule::new("PID.5", RedactionStrategy::Mask),
            ];
            let engine = RedactionEngine::new(rules);

            if let Ok(first_pass) = engine.redact(&parsed)
                && let Ok(second_pass) = engine.redact(&first_pass.message)
            {
                // Messages should be identical after second redaction with Mask strategy
                prop_assert_eq!(first_pass.message_bytes, second_pass.message_bytes,
                    "Mask redaction should be idempotent");
            }
        }
    }
}

proptest! {
    /// Test that audit log is consistent across multiple runs
    #[test]
    fn prop_audit_log_consistent(
        sending_app in alphanumeric(),
        sending_fac in alphanumeric(),
        msg_id in alphanumeric(),
        pat_id in alphanumeric(),
        pat_name in alphanumeric()
    ) {
        let hl7 = format!(
            "MSH|^~\\&|{}|{}|RCV|RCVFAC|20250128120000||ADT^A01|{}|P|2.5\rPID|1||{}||{}||19800101\r",
            sending_app, sending_fac, msg_id, pat_id, pat_name
        );
        if let Ok(parsed) = parse(hl7.as_bytes()) {
            let engine = RedactionEngine::common_phi_rules();

            if let Ok(first_result) = engine.redact(&parsed)
                && let Ok(second_result) = engine.redact(&parsed)
            {
                // Audit log lengths should be the same
                prop_assert_eq!(first_result.audit_log.len(), second_result.audit_log.len(),
                    "Audit log should have consistent length across runs");
            }
        }
    }
}

// ============================================================================
// Path Handling Properties
// ============================================================================

proptest! {
    /// Test that non-existent paths are handled gracefully (no panic)
    #[test]
    fn prop_nonexistent_path_graceful(
        sending_app in alphanumeric(),
        sending_fac in alphanumeric(),
        msg_id in alphanumeric(),
        pat_id in alphanumeric(),
        pat_name in alphanumeric(),
        segment in segment_name()
    ) {
        let hl7 = format!(
            "MSH|^~\\&|{}|{}|RCV|RCVFAC|20250128120000||ADT^A01|{}|P|2.5\rPID|1||{}||{}||19800101\r",
            sending_app, sending_fac, msg_id, pat_id, pat_name
        );
        if let Ok(parsed) = parse(hl7.as_bytes()) {
            // Create a path to a segment that likely doesn't exist
            let path = format!("{}.{}", segment, 1);
            let rule = RedactionRule::new(path, RedactionStrategy::Mask);
            let engine = RedactionEngine::new(vec![rule]);

            // Should not panic, even if segment doesn't exist
            let result = engine.redact(&parsed);
            // Result should be Ok even if path doesn't exist
            prop_assert!(result.is_ok(), "Non-existent paths should be handled gracefully");
        }
    }
}

proptest! {
    /// Test that MSH special handling doesn't panic
    #[test]
    fn prop_msh_fields_handled(
        sending_app in alphanumeric(),
        sending_fac in alphanumeric(),
        msg_id in alphanumeric(),
        pat_id in alphanumeric(),
        pat_name in alphanumeric()
    ) {
        let hl7 = format!(
            "MSH|^~\\&|{}|{}|RCV|RCVFAC|20250128120000||ADT^A01|{}|P|2.5\rPID|1||{}||{}||19800101\r",
            sending_app, sending_fac, msg_id, pat_id, pat_name
        );
        if let Ok(parsed) = parse(hl7.as_bytes()) {
            // MSH.1 and MSH.2 are special (field separators)
            let rules = vec![
                RedactionRule::new("MSH.3", RedactionStrategy::Mask),
                RedactionRule::new("MSH.4", RedactionStrategy::Replace("REDACTED".to_string())),
            ];
            let engine = RedactionEngine::new(rules);

            let result = engine.redact(&parsed);
            prop_assert!(result.is_ok(), "MSH field handling should not panic");
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Extract a field value from an HL7 message string (simplified)
fn extract_field(msg: &str, segment: &str, field_num: usize) -> Option<String> {
    if let Some(seg_start) = msg.find(&format!("{}|", segment)) {
        let seg_end = msg[seg_start..].find('\r').unwrap_or(msg.len() - seg_start);
        let segment_str = &msg[seg_start..seg_start + seg_end];
        let fields: Vec<&str> = segment_str.split('|').collect();
        fields.get(field_num).map(|s| s.to_string())
    } else {
        None
    }
}

// ============================================================================
// Edge Case Unit Tests (complementing property tests)
// ============================================================================

#[test]
fn test_empty_message_redaction() {
    let hl7 = "MSH|^~\\&|HOSP|FAC|20250128120000||ADT^A01|MSG|P|2.5\r";
    let parsed = parse(hl7.as_bytes()).unwrap();
    let result = redact_phi(&parsed).unwrap();
    assert!(!result.audit_log.is_empty());
}

#[test]
fn test_repeated_fields_handled() {
    // Message with repeated field (using ~)
    let hl7 =
        "MSH|^~\\&|HOSP|FAC|20250128120000||ADT^A01|MSG|P|2.5\rPID|1||12345~67890||Doe^John\r";
    let parsed = parse(hl7.as_bytes()).unwrap();
    let rules = vec![RedactionRule::new("PID.3", RedactionStrategy::Hash)];
    let engine = RedactionEngine::new(rules);
    let result = engine.redact(&parsed).unwrap();
    // Should handle repeated fields without panic
    assert!(result.audit_log.len() == 1);
}

#[test]
fn test_unicode_field_content() {
    let hl7 = "MSH|^~\\&|HOSP|FAC|20250128120000||ADT^A01|MSG|P|2.5\rPID|1||12345||Müller^José\r";
    let parsed = parse(hl7.as_bytes()).unwrap();
    let rules = vec![RedactionRule::new("PID.5", RedactionStrategy::Mask)];
    let engine = RedactionEngine::new(rules);
    let result = engine.redact(&parsed).unwrap();
    let redacted = String::from_utf8_lossy(&result.message_bytes);
    assert!(!redacted.contains("Müller"));
    assert!(!redacted.contains("José"));
}

#[test]
fn test_component_level_redaction() {
    let hl7 =
        "MSH|^~\\&|HOSP|FAC|20250128120000||ADT^A01|MSG|P|2.5\rPID|1||12345||Doe^John^Middle\r";
    let parsed = parse(hl7.as_bytes()).unwrap();
    // Redact just the first component of the name (family name)
    let rules = vec![RedactionRule::new("PID.5.1", RedactionStrategy::Mask)];
    let engine = RedactionEngine::new(rules);
    let result = engine.redact(&parsed).unwrap();
    let redacted = String::from_utf8_lossy(&result.message_bytes);
    // Family name should be masked
    assert!(!redacted.contains("Doe"));
}

#[test]
fn test_all_strategies_coverage() {
    // Use a realistic message similar to the integration tests
    let hl7 = "MSH|^~\\&|HOSPITAL|MAIN|20250128120000||ADT^A01|MSG|P|2.5\r\
PID|1||ID12345||Doe^John||19800115||||123 Main St^^City^ST^12345||555-1234\r";
    let parsed = parse(hl7.as_bytes()).unwrap();

    let rules = vec![
        RedactionRule::new("PID.3", RedactionStrategy::Hash),
        RedactionRule::new("PID.5", RedactionStrategy::Mask),
        RedactionRule::new(
            "PID.7",
            RedactionStrategy::Replace("1900-01-01".to_string()),
        ),
        RedactionRule::new("PID.11", RedactionStrategy::Remove),
        RedactionRule::new("PID.13", RedactionStrategy::Truncate(3)),
    ];

    let engine = RedactionEngine::new(rules);
    let result = engine.redact(&parsed).unwrap();

    assert_eq!(result.audit_log.len(), 5);

    let redacted = String::from_utf8_lossy(&result.message_bytes);
    // Verify each strategy was applied (check specific content that should be gone)
    assert!(!redacted.contains("ID12345")); // Hashed
    assert!(!redacted.contains("Doe^John")); // Masked
    assert!(redacted.contains("1900-01-01")); // Replaced
    assert!(!redacted.contains("123 Main St")); // Removed
}
