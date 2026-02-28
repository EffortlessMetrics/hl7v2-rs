//! HL7 v2 ACK Generation Example
//!
//! This example demonstrates how to:
//! - Generate ACK (Acknowledgment) messages for received HL7 messages
//! - Handle accept/reject scenarios with appropriate ACK codes
//! - Include error details in rejection ACKs
//!
//! Run with: cargo run --example ack_generation

use hl7v2_ack::{AckCode, ack, ack_with_error};
use hl7v2_core::{Message, get, parse, write};

/// A valid ADT^A01 message
const VALID_ADT_MESSAGE: &[u8] = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|MSG12345|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John||19700101|M\r";

/// A message with missing required fields
const INCOMPLETE_MESSAGE: &[u8] = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|MSG67890|P|2.5.1\rPID|1||||||\r";

/// A message with invalid data
const INVALID_DATA_MESSAGE: &[u8] = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|MSG11111|P|2.5.1\rPID|1||ABC||Doe^John||INVALID_DATE|X\r";

fn main() {
    println!("=== HL7 v2 ACK Generation Example ===\n");

    // Example 1: Generate a simple acceptance ACK
    acceptance_ack_example();

    // Example 2: Generate rejection ACK for invalid message
    rejection_ack_example();

    // Example 3: Generate error ACK with details
    error_ack_example();

    // Example 4: All ACK code types
    all_ack_codes_example();

    // Example 5: ACK generation workflow
    ack_workflow_example();
}

/// Example 1: Generate a simple acceptance ACK (AA)
fn acceptance_ack_example() {
    println!("--- Example 1: Acceptance ACK (AA) ---\n");

    // Parse the original message
    println!("Original message:");
    let original = parse(VALID_ADT_MESSAGE).expect("Message should parse");
    println!(
        "{}",
        String::from_utf8_lossy(VALID_ADT_MESSAGE).replace("\r", "\r\n")
    );
    println!();

    // Generate an acceptance ACK
    println!("Generating acceptance ACK (AA)...");
    match ack(&original, AckCode::AA) {
        Ok(ack_msg) => {
            println!("✓ ACK generated successfully\n");
            display_ack(&ack_msg);

            // Verify key fields
            println!("\nACK verification:");
            println!("  MSH-9 (Message Type): {:?}", get(&ack_msg, "MSH.9"));
            println!("  MSA-1 (ACK Code): {:?}", get(&ack_msg, "MSA.1"));
            println!("  MSA-2 (Message Control ID): {:?}", get(&ack_msg, "MSA.2"));
        }
        Err(e) => {
            eprintln!("✗ Failed to generate ACK: {:?}", e);
        }
    }
    println!();
}

/// Example 2: Generate rejection ACK (AR) for invalid message
fn rejection_ack_example() {
    println!("--- Example 2: Rejection ACK (AR) ---\n");

    // Parse the incomplete message
    println!("Incomplete message:");
    let original = parse(INCOMPLETE_MESSAGE).expect("Message should parse");
    println!(
        "{}",
        String::from_utf8_lossy(INCOMPLETE_MESSAGE).replace("\r", "\r\n")
    );
    println!();

    // Generate a rejection ACK
    println!("Generating rejection ACK (AR)...");
    match ack(&original, AckCode::AR) {
        Ok(ack_msg) => {
            println!("✓ Rejection ACK generated\n");
            display_ack(&ack_msg);

            println!("\nRejection reason:");
            println!("  Missing required patient identifier (PID.3)");
            println!("  Missing required patient name (PID.5)");
        }
        Err(e) => {
            eprintln!("✗ Failed to generate ACK: {:?}", e);
        }
    }
    println!();
}

/// Example 3: Generate error ACK (AE) with details
fn error_ack_example() {
    println!("--- Example 3: Error ACK (AE) with Details ---\n");

    // Parse the message with invalid data
    println!("Message with invalid data:");
    let original = parse(INVALID_DATA_MESSAGE).expect("Message should parse");
    println!(
        "{}",
        String::from_utf8_lossy(INVALID_DATA_MESSAGE).replace("\r", "\r\n")
    );
    println!();

    // Generate an error ACK with details
    println!("Generating error ACK (AE) with details...");

    let error_details = "PID.7: Invalid date format - expected YYYYMMDD\nPID.8: Invalid sex value - expected M, F, O, or U";

    match ack_with_error(&original, AckCode::AE, Some(error_details)) {
        Ok(ack_msg) => {
            println!("✓ Error ACK generated with details\n");
            display_ack(&ack_msg);

            println!("\nError details:");
            println!("  - PID.7: Invalid date format - expected YYYYMMDD");
            println!("  - PID.8: Invalid sex value - expected M, F, O, or U");
        }
        Err(e) => {
            eprintln!("✗ Failed to generate ACK: {:?}", e);
        }
    }
    println!();
}

/// Example 4: All ACK code types
fn all_ack_codes_example() {
    println!("--- Example 4: All ACK Code Types ---\n");

    let original = parse(VALID_ADT_MESSAGE).expect("Message should parse");

    println!("ACK Code Types:\n");

    // Application Accept
    println!("1. AA - Application Accept");
    println!("   Use: Message was accepted and processed successfully");
    if let Ok(ack_msg) = ack(&original, AckCode::AA) {
        println!("   MSA-1: {:?}", get(&ack_msg, "MSA.1"));
    }
    println!();

    // Application Error
    println!("2. AE - Application Error");
    println!("   Use: Message was accepted but processing failed");
    if let Ok(ack_msg) = ack(&original, AckCode::AE) {
        println!("   MSA-1: {:?}", get(&ack_msg, "MSA.1"));
    }
    println!();

    // Application Reject
    println!("3. AR - Application Reject");
    println!("   Use: Message was rejected (invalid format, missing data)");
    if let Ok(ack_msg) = ack(&original, AckCode::AR) {
        println!("   MSA-1: {:?}", get(&ack_msg, "MSA.1"));
    }
    println!();

    // Commit Accept (Enhanced mode)
    println!("4. CA - Commit Accept");
    println!("   Use: Enhanced mode - commit-level acknowledgment");
    if let Ok(ack_msg) = ack(&original, AckCode::CA) {
        println!("   MSA-1: {:?}", get(&ack_msg, "MSA.1"));
    }
    println!();

    // Commit Error (Enhanced mode)
    println!("5. CE - Commit Error");
    println!("   Use: Enhanced mode - commit-level error");
    if let Ok(ack_msg) = ack(&original, AckCode::CE) {
        println!("   MSA-1: {:?}", get(&ack_msg, "MSA.1"));
    }
    println!();

    // Commit Reject (Enhanced mode)
    println!("6. CR - Commit Reject");
    println!("   Use: Enhanced mode - commit-level reject");
    if let Ok(ack_msg) = ack(&original, AckCode::CR) {
        println!("   MSA-1: {:?}", get(&ack_msg, "MSA.1"));
    }
    println!();
}

/// Example 5: Complete ACK generation workflow
fn ack_workflow_example() {
    println!("--- Example 5: ACK Generation Workflow ---\n");

    println!("Typical message processing workflow with ACK generation:\n");

    println!("```rust");
    println!("fn process_message(hl7_bytes: &[u8]) -> Result<Vec<u8>, ProcessError> {{");
    println!("    // Step 1: Parse the incoming message");
    println!("    let message = parse(hl7_bytes).map_err(|e| ProcessError::Parse(e))?;");
    println!();
    println!("    // Step 2: Validate the message");
    println!("    let validation_result = validate(&message);");
    println!();
    println!("    // Step 3: Generate appropriate ACK based on validation");
    println!("    let ack_message = if !validation_result.is_valid {{");
    println!("        // Rejection - message doesn't meet requirements");
    println!("        let error_msg = format_errors(&validation_result.errors);");
    println!("        ack_with_error(&message, AckCode::AR, Some(&error_msg))?");
    println!("    }} else {{");
    println!("        // Step 4: Process the message");
    println!("        match process_business_logic(&message) {{");
    println!("            Ok(_) => ack(&message, AckCode::AA)?,  // Success");
    println!("            Err(e) => ack_with_error(&message, AckCode::AE, Some(&e.to_string()))?,");
    println!("        }}");
    println!("    }};");
    println!();
    println!("    // Step 5: Serialize and return the ACK");
    println!("    Ok(write(&ack_message))");
    println!("}}");
    println!("```\n");

    // Demonstrate the workflow
    println!("Demonstration:\n");

    // Simulate processing different scenarios
    let test_cases: Vec<(&str, &[u8], bool)> = vec![
        ("Valid Message", VALID_ADT_MESSAGE, true),
        ("Incomplete Message", INCOMPLETE_MESSAGE, false),
        ("Invalid Data", INVALID_DATA_MESSAGE, false),
    ];

    for (name, msg_bytes, should_succeed) in test_cases {
        println!("Processing: {}", name);

        match process_message_with_ack(msg_bytes) {
            Ok(ack_bytes) => {
                let ack_msg = parse(&ack_bytes).expect("ACK should parse");
                let ack_code = get(&ack_msg, "MSA.1").unwrap_or("?");
                let status = if ack_code == "AA" {
                    "✓ Accepted"
                } else {
                    "✗ Rejected"
                };
                println!("  Result: {} (ACK code: {})", status, ack_code);

                if !should_succeed && ack_code != "AA" {
                    println!("  ✓ Correctly rejected invalid message");
                }
            }
            Err(e) => {
                println!("  ✗ Processing error: {:?}", e);
            }
        }
        println!();
    }

    // Best practices
    println!("ACK Generation Best Practices:");
    println!("  1. Always include the original Message Control ID in MSA-2");
    println!("  2. Use AA only for successfully processed messages");
    println!("  3. Use AR for structural/validation failures");
    println!("  4. Use AE for processing/runtime failures");
    println!("  5. Include meaningful error messages in MSA-3");
    println!("  6. Consider adding ERR segment for detailed errors");
    println!("  7. Mirror the original message's delimiters in the ACK");
    println!("  8. Include MSH segment with swapped sender/receiver");
    println!();
}

/// Display an ACK message's key fields
fn display_ack(ack_msg: &Message) {
    println!("ACK Message:");
    let bytes = write(ack_msg);
    println!("{}", String::from_utf8_lossy(&bytes).replace("\r", "\r\n"));
}

/// Simulated message processing with ACK generation
fn process_message_with_ack(hl7_bytes: &[u8]) -> Result<Vec<u8>, String> {
    // Parse
    let message = parse(hl7_bytes).map_err(|e| format!("Parse error: {:?}", e))?;

    // Validate
    let patient_id = get(&message, "PID.3.1");
    let patient_name = get(&message, "PID.5");
    let birth_date = get(&message, "PID.7");
    let sex = get(&message, "PID.8");

    let mut errors = Vec::new();

    if patient_id.is_none() || patient_id.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
        errors.push("Missing patient identifier (PID.3)");
    }
    if patient_name.is_none() || patient_name.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
        errors.push("Missing patient name (PID.5)");
    }
    if let Some(ref bd) = birth_date {
        if !bd.is_empty() && bd.len() != 8 {
            errors.push("Invalid birth date format (expected YYYYMMDD)");
        }
    }
    if let Some(ref s) = sex {
        if !s.is_empty() {
            let valid_values = ["M", "F", "O", "U"];
            let valid = valid_values.contains(&s.as_str());
            if !valid {
                errors.push("Invalid sex value (expected M, F, O, or U)");
            }
        }
    }

    // Generate appropriate ACK
    let ack_msg = if errors.is_empty() {
        ack(&message, AckCode::AA).map_err(|e| format!("ACK generation error: {:?}", e))?
    } else {
        ack_with_error(&message, AckCode::AR, Some(&errors.join("; ")))
            .map_err(|e| format!("ACK generation error: {:?}", e))?
    };

    Ok(write(&ack_msg))
}

/// Custom error type for processing
#[derive(Debug)]
#[allow(dead_code)]
enum ProcessError {
    Parse(hl7v2_core::Error),
    Validation(Vec<String>),
    Processing(String),
}
