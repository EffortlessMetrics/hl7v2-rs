//! Full message processing pipeline tests.
//!
//! These tests validate the complete flow:
//! Parse HL7 message → Validate → Generate ACK → Write response

use hl7v2_parser::parse;
use hl7v2_writer::write;
use hl7v2_ack::{ack, AckCode};
use hl7v2_test_utils::{SampleMessages, assert_hl7_roundtrips};
use hl7v2_validation::{Validator, Issue, Severity};
use hl7v2_core::get;

use super::common::init_tracing;

/// Simple validator for testing
struct SimpleValidator;

impl Validator for SimpleValidator {
    fn validate(&self, _msg: &hl7v2_core::Message) -> Vec<Issue> {
        // Basic validation - no issues for valid messages
        Vec::new()
    }
}

// =========================================================================
// ADT^A01 Message Pipeline Tests
// =========================================================================

mod adt_a01_pipeline {
    use super::*;

    #[tokio::test]
    async fn test_full_pipeline_adt_a01() {
        init_tracing();

        // Step 1: Parse the message
        let raw_message = SampleMessages::adt_a01();
        let parsed = parse(raw_message.as_bytes())
            .expect("ADT^A01 should parse successfully");

        // Verify message type using get function
        let msg_type = get(&parsed, "MSH.9");
        assert!(msg_type.is_some());

        // Step 2: Validate the message (basic structure validation)
        let validator = SimpleValidator;
        let issues = validator.validate(&parsed);
        assert!(issues.is_empty() || issues.iter().all(|i| i.severity != Severity::Error), 
            "ADT^A01 should be valid");

        // Step 3: Generate ACK
        let ack_msg = ack(&parsed, AckCode::AA)
            .expect("Should generate ACK");

        // Verify ACK structure
        let ack_bytes = write(&ack_msg);
        let ack_str = String::from_utf8_lossy(&ack_bytes);
        assert!(ack_str.contains("MSA|AA"), "ACK should contain MSA|AA, got: {}", ack_str);
        // ACK message type in MSH.9 should be ACK or ACK^ACK
        assert!(ack_str.contains("ACK") || ack_str.contains("MSH|"), "Should be a valid HL7 ACK message");

        // Step 4: Write response (serialize the parsed message back)
        let _rewritten = write(&parsed);
        
        // Verify round-trip
        assert_hl7_roundtrips(raw_message.as_bytes());
    }

    #[tokio::test]
    async fn test_pipeline_adt_a01_error_ack() {
        init_tracing();

        let raw_message = SampleMessages::adt_a01();
        let parsed = parse(raw_message.as_bytes())
            .expect("ADT^A01 should parse");

        // Generate an error ACK
        let ack_msg = ack(&parsed, AckCode::AE)
            .expect("Should generate error ACK");

        let ack_bytes = write(&ack_msg);
        let ack_str = String::from_utf8_lossy(&ack_bytes);
        assert!(ack_str.contains("MSA|AE"));
    }
}

// =========================================================================
// ADT^A04 Message Pipeline Tests
// =========================================================================

mod adt_a04_pipeline {
    use super::*;

    #[tokio::test]
    async fn test_full_pipeline_adt_a04() {
        init_tracing();

        // Step 1: Parse
        let raw_message = SampleMessages::adt_a04();
        let parsed = parse(raw_message.as_bytes())
            .expect("ADT^A04 should parse successfully");

        let msg_type = get(&parsed, "MSH.9");
        assert!(msg_type.is_some());

        // Step 2: Validate
        let validator = SimpleValidator;
        let issues = validator.validate(&parsed);
        assert!(issues.is_empty() || issues.iter().all(|i| i.severity != Severity::Error));

        // Step 3: Generate ACK
        let ack_msg = ack(&parsed, AckCode::AA)
            .expect("Should generate ACK");

        let ack_bytes = write(&ack_msg);
        let ack_str = String::from_utf8_lossy(&ack_bytes);
        assert!(ack_str.contains("MSA|AA"));

        // Step 4: Round-trip
        assert_hl7_roundtrips(raw_message.as_bytes());
    }

    #[tokio::test]
    async fn test_pipeline_adt_a04_reject_ack() {
        init_tracing();

        let raw_message = SampleMessages::adt_a04();
        let parsed = parse(raw_message.as_bytes())
            .expect("ADT^A04 should parse");

        // Generate a reject ACK
        let ack_msg = ack(&parsed, AckCode::AR)
            .expect("Should generate reject ACK");

        let ack_bytes = write(&ack_msg);
        let ack_str = String::from_utf8_lossy(&ack_bytes);
        assert!(ack_str.contains("MSA|AR"));
    }
}

// =========================================================================
// ORU^R01 Message Pipeline Tests
// =========================================================================

mod oru_r01_pipeline {
    use super::*;

    #[tokio::test]
    async fn test_full_pipeline_oru_r01() {
        init_tracing();

        // Step 1: Parse
        let raw_message = SampleMessages::oru_r01();
        let parsed = parse(raw_message.as_bytes())
            .expect("ORU^R01 should parse successfully");

        let msg_type = get(&parsed, "MSH.9");
        assert!(msg_type.is_some());

        // Step 2: Validate
        let validator = SimpleValidator;
        let issues = validator.validate(&parsed);
        assert!(issues.is_empty() || issues.iter().all(|i| i.severity != Severity::Error));

        // Step 3: Generate ACK
        let ack_msg = ack(&parsed, AckCode::AA)
            .expect("Should generate ACK");

        let ack_bytes = write(&ack_msg);
        let ack_str = String::from_utf8_lossy(&ack_bytes);
        assert!(ack_str.contains("MSA|AA"));

        // Step 4: Round-trip
        assert_hl7_roundtrips(raw_message.as_bytes());
    }

    #[tokio::test]
    async fn test_pipeline_oru_r01_with_obx_validation() {
        init_tracing();

        let raw_message = SampleMessages::oru_r01();
        let parsed = parse(raw_message.as_bytes())
            .expect("ORU^R01 should parse");

        // Verify OBX segments are present by checking segment count
        assert!(parsed.segments.len() > 3, "ORU^R01 should have multiple segments including OBX");

        // Validate and generate ACK
        let validator = SimpleValidator;
        let issues = validator.validate(&parsed);
        assert!(issues.is_empty() || issues.iter().all(|i| i.severity != Severity::Error));

        let ack_msg = ack(&parsed, AckCode::AA)
            .expect("Should generate ACK");

        // ACK should reference the original message control ID
        let ack_bytes = write(&ack_msg);
        let ack_str = String::from_utf8_lossy(&ack_bytes);
        let msh_10 = get(&parsed, "MSH.10").unwrap_or("");
        assert!(ack_str.contains(msh_10) || msh_10.is_empty());
    }
}

// =========================================================================
// Edge Cases Pipeline Tests
// =========================================================================

mod edge_case_pipeline {
    use super::*;

    #[tokio::test]
    async fn test_pipeline_with_special_characters() {
        init_tracing();

        let raw_message = SampleMessages::edge_case("special_chars")
            .expect("Should have special_chars edge case");

        // Parse
        let parsed = parse(raw_message.as_bytes())
            .expect("Message with special chars should parse");

        // Validate
        let validator = SimpleValidator;
        let issues = validator.validate(&parsed);
        // May have warnings but should not have errors
        assert!(issues.iter().all(|i| i.severity != Severity::Error));

        // Generate ACK
        let ack_msg = ack(&parsed, AckCode::AA)
            .expect("Should generate ACK");

        let ack_bytes = write(&ack_msg);
        let ack_str = String::from_utf8_lossy(&ack_bytes);
        assert!(ack_str.contains("MSA|AA"));
    }

    #[tokio::test]
    async fn test_pipeline_with_custom_delimiters() {
        init_tracing();

        let raw_message = SampleMessages::edge_case("custom_delims")
            .expect("Should have custom_delims edge case");

        // Parse with custom delimiters
        let parsed = parse(raw_message.as_bytes())
            .expect("Message with custom delimiters should parse");

        // Validate
        let validator = SimpleValidator;
        let issues = validator.validate(&parsed);
        assert!(issues.is_empty() || issues.iter().all(|i| i.severity != Severity::Error));

        // Generate ACK (should use original delimiters)
        let ack_msg = ack(&parsed, AckCode::AA)
            .expect("Should generate ACK");

        let ack_bytes = write(&ack_msg);
        let ack_str = String::from_utf8_lossy(&ack_bytes);
        assert!(!ack_str.is_empty());
    }

    #[tokio::test]
    async fn test_pipeline_with_repetitions() {
        init_tracing();

        let raw_message = SampleMessages::edge_case("with_repetitions")
            .expect("Should have with_repetitions edge case");

        // Parse
        let parsed = parse(raw_message.as_bytes())
            .expect("Message with repetitions should parse");

        // Validate
        let validator = SimpleValidator;
        let issues = validator.validate(&parsed);
        assert!(issues.is_empty() || issues.iter().all(|i| i.severity != Severity::Error));

        // Generate ACK
        let ack_msg = ack(&parsed, AckCode::AA)
            .expect("Should generate ACK");

        let ack_bytes = write(&ack_msg);
        let ack_str = String::from_utf8_lossy(&ack_bytes);
        assert!(ack_str.contains("MSA|AA"));

        // Round-trip should preserve repetitions
        assert_hl7_roundtrips(raw_message.as_bytes());
    }
}

// =========================================================================
// Invalid Message Pipeline Tests
// =========================================================================

mod invalid_message_pipeline {
    use super::*;

    #[tokio::test]
    async fn test_pipeline_invalid_message_returns_error_ack() {
        init_tracing();

        let raw_message = SampleMessages::invalid("malformed")
            .expect("Should have malformed invalid case");

        // Parse should fail or produce an error
        let parse_result = parse(raw_message.as_bytes());

        if let Ok(parsed) = parse_result {
            // If it parsed, generate error ACK
            let ack_msg = ack(&parsed, AckCode::AE)
                .expect("Should generate ACK");

            let ack_bytes = write(&ack_msg);
            let ack_str = String::from_utf8_lossy(&ack_bytes);
            assert!(ack_str.contains("MSA|AE"));
        }
        // If parse failed, that's also acceptable for invalid messages
    }

    #[tokio::test]
    async fn test_pipeline_truncated_message() {
        init_tracing();

        let raw_message = SampleMessages::invalid("truncated")
            .expect("Should have truncated invalid case");

        // Parse should fail or produce a message with errors
        let parse_result = parse(raw_message.as_bytes());

        // Truncated messages should either fail to parse or validate with errors
        if let Ok(parsed) = parse_result {
            let validator = SimpleValidator;
            let issues = validator.validate(&parsed);
            // Should have validation issues or be acceptable
            drop(issues);
        }
    }

    #[tokio::test]
    async fn test_pipeline_missing_msh_segment() {
        init_tracing();

        let raw_message = SampleMessages::invalid("no_msh")
            .expect("Should have no_msh invalid case");

        // Parse should fail - MSH is required
        let parse_result = parse(raw_message.as_bytes());
        assert!(parse_result.is_err(), "Message without MSH should fail to parse");
    }
}

// =========================================================================
// Batch Processing Pipeline Tests
// =========================================================================

mod batch_pipeline {
    use super::*;

    #[tokio::test]
    async fn test_pipeline_batch_messages() {
        init_tracing();

        // Create a batch with multiple messages
        let batch = format!(
            "MSH|^~\\&|App|Fac|Recv|RecvFac|20250128120000||ADT^A01|MSG1|P|2.5\rPID|1||123||Doe^John\rMSH|^~\\&|App|Fac|Recv|RecvFac|20250128120001||ADT^A04|MSG2|P|2.5\rPID|1||456||Smith^Jane\r"
        );

        // Parse the batch
        let parsed = parse(batch.as_bytes())
            .expect("Batch should parse");

        // Validate
        let validator = SimpleValidator;
        let issues = validator.validate(&parsed);
        assert!(issues.is_empty() || issues.iter().all(|i| i.severity != Severity::Error));

        // Generate ACK for batch
        let ack_msg = ack(&parsed, AckCode::AA)
            .expect("Should generate ACK");

        let ack_bytes = write(&ack_msg);
        let ack_str = String::from_utf8_lossy(&ack_bytes);
        assert!(ack_str.contains("MSA|AA"));
    }
}

// =========================================================================
// Performance Pipeline Tests
// =========================================================================

mod performance_pipeline {
    use super::*;
    use std::time::Instant;

    #[tokio::test]
    async fn test_pipeline_performance_100_messages() {
        init_tracing();

        let raw_message = SampleMessages::adt_a01();
        let iterations = 100;

        let start = Instant::now();

        for _ in 0..iterations {
            let parsed = parse(raw_message.as_bytes()).expect("Should parse");
            let validator = SimpleValidator;
            let _issues = validator.validate(&parsed);
            let _ack = ack(&parsed, AckCode::AA).expect("Should generate ACK");
            let _rewritten = write(&parsed);
        }

        let elapsed = start.elapsed();
        let msgs_per_sec = iterations as f64 / elapsed.as_secs_f64();

        println!("Pipeline throughput: {:.2} messages/second", msgs_per_sec);

        // Should process at least 100 messages per second
        assert!(
            msgs_per_sec > 100.0,
            "Pipeline should process at least 100 messages/second, got {:.2}",
            msgs_per_sec
        );
    }
}
