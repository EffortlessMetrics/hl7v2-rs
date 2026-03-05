//! HL7 v2 Batch Processing Example
//!
//! This example demonstrates how to:
//! - Read a batch file with multiple HL7 messages
//! - Process each message in the batch
//! - Write results to output
//!
//! Run with: cargo run --example batch_processing

use hl7v2_batch::{Batch, FileBatch, parse_batch};
use hl7v2_core::{Message, get, parse, write};

/// Sample batch file with FHS/BHS headers and multiple messages
const SAMPLE_BATCH: &[u8] = b"FHS|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128120000||BATCH001\rBHS|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128120000||BATCH001\rMSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128120001||ADT^A01^ADT_A01|MSG001|P|2.5.1\rPID|1||PAT001^^^HOSP^MR||Smith^John||19800101|M\rMSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128120002||ADT^A01^ADT_A01|MSG002|P|2.5.1\rPID|1||PAT002^^^HOSP^MR||Jones^Jane||19850215|F\rMSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128120003||ADT^A01^ADT_A01|MSG003|P|2.5.1\rPID|1||PAT003^^^HOSP^MR||Brown^Bob||19900320|M\rBTS|3\rFTS|3\r";

/// Sample simple batch (BHS/BTS only, no FHS/FTS)
const SIMPLE_BATCH: &[u8] = b"BHS|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128120000\rMSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128120001||ADT^A01|MSG001|P|2.5.1\rPID|1||PAT001^^^HOSP^MR||Doe^John\rMSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128120002||ADT^A01|MSG002|P|2.5.1\rPID|1||PAT002^^^HOSP^MR||Doe^Jane\rBTS|2\r";

fn main() {
    println!("=== HL7 v2 Batch Processing Example ===\n");

    // Example 1: Parse a file batch (FHS/FTS)
    file_batch_example();

    // Example 2: Parse a simple batch (BHS/BTS)
    simple_batch_example();

    // Example 3: Process each message in a batch
    process_batch_messages_example();

    // Example 4: Create and write a new batch
    create_batch_example();

    // Example 5: Error handling
    batch_error_handling_example();
}

/// Example 1: Parse a file batch with FHS/FTS
fn file_batch_example() {
    println!("--- Example 1: File Batch (FHS/FTS) ---\n");

    println!("Parsing file batch...");
    match parse_batch(SAMPLE_BATCH) {
        Ok(file_batch) => {
            println!("✓ Successfully parsed file batch\n");
            display_file_batch_info(&file_batch);
        }
        Err(e) => {
            eprintln!("✗ Failed to parse batch: {}", e);
        }
    }
    println!();
}

/// Example 2: Parse a simple batch with BHS/BTS only
fn simple_batch_example() {
    println!("--- Example 2: Simple Batch (BHS/BTS) ---\n");

    println!("Parsing simple batch...");
    match parse_batch(SIMPLE_BATCH) {
        Ok(file_batch) => {
            println!("✓ Successfully parsed simple batch\n");
            display_file_batch_info(&file_batch);
        }
        Err(e) => {
            eprintln!("✗ Failed to parse batch: {}", e);
        }
    }
    println!();
}

/// Display file batch information
fn display_file_batch_info(file_batch: &FileBatch) {
    println!("File Batch Information:");

    if let Some(ref header) = file_batch.header {
        println!("  Has FHS header: {}", header.id_str());
    }

    println!("  Number of batches: {}", file_batch.batches.len());

    for (i, batch) in file_batch.batches.iter().enumerate() {
        println!("\n  Batch {}:", i + 1);
        println!("    Messages: {}", batch.messages.len());

        for (j, msg) in batch.messages.iter().enumerate() {
            let msg_type = get(msg, "MSH.9").unwrap_or("Unknown");
            let control_id = get(msg, "MSH.10").unwrap_or("Unknown");
            println!(
                "      Message {}: {} (Control ID: {})",
                j + 1,
                msg_type,
                control_id
            );
        }
    }

    if let Some(ref trailer) = file_batch.trailer {
        println!("\n  Has FTS trailer: {}", trailer.id_str());
    }
}

/// Example 3: Process each message in a batch
fn process_batch_messages_example() {
    println!("--- Example 3: Processing Batch Messages ---\n");

    // Parse the batch
    let file_batch = match parse_batch(SAMPLE_BATCH) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("✗ Failed to parse batch: {}", e);
            return;
        }
    };

    // Collect all messages from all batches
    let mut all_messages: Vec<&Message> = Vec::new();
    for batch in &file_batch.batches {
        for msg in &batch.messages {
            all_messages.push(msg);
        }
    }

    println!("Processing {} messages...\n", all_messages.len());

    // Process each message
    let mut results = Vec::new();
    for (i, message) in all_messages.iter().enumerate() {
        let result = process_message(message, i + 1);
        results.push(result);
    }

    // Summary
    let successful: Vec<_> = results.iter().filter(|r| r.success).collect();
    let failed: Vec<_> = results.iter().filter(|r| !r.success).collect();

    println!("\nProcessing Summary:");
    println!("  Total messages: {}", results.len());
    println!("  Successful: {}", successful.len());
    println!("  Failed: {}", failed.len());

    if !failed.is_empty() {
        println!("\nFailed messages:");
        for result in failed {
            let error_msg = result.error.as_deref().unwrap_or("Unknown error");
            println!("  Message {}: {}", result.message_num, error_msg);
        }
    }
    println!();
}

/// Process result structure
struct ProcessResult {
    message_num: usize,
    success: bool,
    #[allow(dead_code)]
    patient_id: Option<String>,
    #[allow(dead_code)]
    patient_name: Option<String>,
    #[allow(dead_code)]
    error: Option<String>,
}

/// Process a single message
fn process_message(message: &Message, num: usize) -> ProcessResult {
    // Extract patient information
    let patient_id = get(message, "PID.3.1").map(|s| s.to_string());
    let patient_name = get(message, "PID.5").map(|s| s.to_string());

    println!("Message {}:", num);
    if let Some(ref id) = patient_id {
        println!("  Patient ID: {}", id);
    }
    if let Some(ref name) = patient_name {
        println!("  Patient Name: {}", name);
    }

    // Simulate processing logic
    let success = patient_id.is_some();

    ProcessResult {
        message_num: num,
        success,
        patient_id,
        patient_name,
        error: if success {
            None
        } else {
            Some("Missing patient ID".to_string())
        },
    }
}

/// Example 4: Create and write a new batch
fn create_batch_example() {
    println!("--- Example 4: Creating a New Batch ---\n");

    // Create a new batch with messages
    let mut batch = Batch::default();

    // Add messages to the batch
    for i in 1..=3 {
        let message = create_sample_message(i);
        batch.messages.push(message);
    }

    println!("Created batch with {} messages", batch.messages.len());

    // Write the batch to bytes
    let output = write_batch_manually(&batch);

    println!("\nBatch output:");
    println!("{}", String::from_utf8_lossy(&output).replace("\r", "\r\n"));
    println!();
}

/// Create a sample message for the batch
fn create_sample_message(num: usize) -> Message {
    let message_str = format!(
        "MSH|^~\\&|App|Fac|RecvApp|RecvFac|2025012812000{}||ADT^A01|MSG{:03}|P|2.5.1\rPID|1||PAT{:03}^^^HOSP^MR||Patient^{}||19900101|M\r",
        num, num, num, num
    );
    parse(message_str.as_bytes()).expect("Message should parse")
}

/// Manually write batch to bytes (demonstration)
fn write_batch_manually(batch: &Batch) -> Vec<u8> {
    let mut output = Vec::new();

    // Write BHS
    output.extend_from_slice(b"BHS|^~\\&|App|Fac|RecvApp|RecvFac|20250128120000\r");

    // Write messages
    for message in &batch.messages {
        let msg_bytes = write(message);
        output.extend_from_slice(&msg_bytes);
    }

    // Write BTS
    output.extend_from_slice(format!("BTS|{}\r", batch.messages.len()).as_bytes());

    output
}

/// Example 5: Error handling for batch processing
fn batch_error_handling_example() {
    println!("--- Example 5: Batch Error Handling ---\n");

    // Test various error conditions

    // 1. Invalid batch structure
    let invalid_structure: &[u8] = b"MSH|^~\\&|App|Fac\rPID|1||123||Doe^John\r";
    println!("Testing invalid structure (no batch headers):");
    match parse_batch(invalid_structure) {
        Ok(_) => println!("  Unexpectedly succeeded"),
        Err(e) => println!("  ✓ Expected error: {}", e),
    }

    // 2. Mismatched headers/trailers
    let mismatched: &[u8] = b"BHS|^~\\&|App|Fac\rMSH|^~\\&|App|Fac\rFTS|1\r";
    println!("\nTesting mismatched headers/trailers:");
    match parse_batch(mismatched) {
        Ok(batch) => {
            // Some implementations may not enforce count validation
            println!("  Batch parsed with {} batches", batch.batches.len());
        }
        Err(e) => println!("  ✓ Expected error: {}", e),
    }

    println!();

    // Best practices for batch processing
    println!("Best practices for batch processing:");
    println!("  1. Always validate batch structure before processing");
    println!("  2. Check message counts match declared counts");
    println!("  3. Process messages individually with error isolation");
    println!("  4. Log progress for large batches");
    println!("  5. Consider memory usage for very large batches");
    println!("  6. Use streaming for files that don't fit in memory");
    println!();

    // Demonstrate safe batch processing pattern
    println!("Safe batch processing pattern:");
    println!("  ```rust");
    println!("  fn safe_batch_process(bytes: &[u8]) -> Result<Vec<ProcessResult>, BatchError> {{");
    println!("      let file_batch = parse_batch(bytes)?;");
    println!("      ");
    println!("      // Collect all messages from all batches");
    println!("      let mut results = Vec::new();");
    println!("      for batch in &file_batch.batches {{");
    println!("          for (i, msg) in batch.messages.iter().enumerate() {{");
    println!("              results.push(process_message_safe(msg, i));");
    println!("          }}");
    println!("      }}");
    println!("      ");
    println!("      Ok(results)");
    println!("  }}");
    println!("  ```");
    println!();
}

/// Safe message processing with error handling
#[allow(dead_code)]
fn process_message_safe(message: &Message, index: usize) -> ProcessResult {
    // Use pattern matching for safe extraction
    let patient_id = get(message, "PID.3.1").map(|s| s.to_string());
    let patient_name = get(message, "PID.5").map(|s| s.to_string());

    // Validate required fields
    if patient_id.is_none() {
        return ProcessResult {
            message_num: index + 1,
            success: false,
            patient_id: None,
            patient_name,
            error: Some("Missing required patient ID".to_string()),
        };
    }

    // Process successfully
    ProcessResult {
        message_num: index + 1,
        success: true,
        patient_id,
        patient_name,
        error: None,
    }
}
