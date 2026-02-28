//! HL7 v2 Streaming Parser Example
//!
//! This example demonstrates how to:
//! - Parse large files incrementally using the streaming parser
//! - Handle backpressure with async streaming
//! - Process messages as they're parsed without loading everything into memory
//!
//! Run with: cargo run --example streaming_parser

use hl7v2_stream::{Event, StreamParser};
use std::io::{BufReader, Cursor};

/// A moderately sized HL7 message for demonstration
const SAMPLE_MESSAGE: &[u8] = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John^Middle||19700101|M|||123 Main St^^Anytown^CA^12345||5551234567\rPV1|1|I|3N^301^A|||||||MED||||||||ADM|A0|||||||||||||||||||HOSPITAL||20250128120000|||\r";

/// Multiple messages for batch demonstration
fn create_multi_message_data(count: usize) -> Vec<u8> {
    let mut data = Vec::new();
    for i in 0..count {
        let msg = format!(
            "MSH|^~\\&|App{}|Fac|RecvApp|RecvFac|202501281200{:02}||ADT^A01|MSG{:03}|P|2.5.1\rPID|1||PAT{:03}^^^HOSP^MR||Patient^{}||19900101|M\r",
            i % 10,
            i % 60,
            i,
            i,
            i
        );
        data.extend_from_slice(msg.as_bytes());
    }
    data
}

fn main() {
    println!("=== HL7 v2 Streaming Parser Example ===\n");

    // Example 1: Basic synchronous streaming
    sync_streaming_example();

    // Example 2: Process events as they arrive
    event_processing_example();

    // Example 3: Memory-efficient processing
    memory_efficient_example();

    // Example 4: Async streaming pattern
    demonstrate_async_pattern();

    // Example 5: Error handling
    error_handling_example();
}

/// Example 1: Basic synchronous streaming
fn sync_streaming_example() {
    println!("--- Example 1: Basic Synchronous Streaming ---\n");

    // Create a reader from the sample data
    let cursor = Cursor::new(SAMPLE_MESSAGE);
    let buf_reader = BufReader::new(cursor);

    // Create the streaming parser
    let mut parser = StreamParser::new(buf_reader);

    println!("Parsing message with streaming parser...\n");

    // Track parsing state
    let mut segment_count = 0;
    let mut field_count = 0;
    let mut current_segment = String::new();

    // Process events as they arrive
    while let Ok(Some(event)) = parser.next_event() {
        match event {
            Event::StartMessage { delims } => {
                println!("Message started");
                println!(
                    "  Delimiters: field='{}' comp='{}' rep='{}' esc='{}' sub='{}'",
                    delims.field, delims.comp, delims.rep, delims.esc, delims.sub
                );
            }
            Event::Segment { id } => {
                current_segment = String::from_utf8_lossy(&id).to_string();
                segment_count += 1;
                println!("\nSegment {}: {}", segment_count, current_segment);
            }
            Event::Field { num, raw } => {
                field_count += 1;
                let value = String::from_utf8_lossy(&raw);
                if !value.is_empty() {
                    println!("  {}.{} = {}", current_segment, num, value);
                }
            }
            Event::EndMessage => {
                println!("\nMessage ended");
                println!("  Total segments: {}", segment_count);
                println!("  Total fields: {}", field_count);
            }
        }
    }
    println!();
}

/// Example 2: Process events with specific handling
fn event_processing_example() {
    println!("--- Example 2: Event Processing ---\n");

    // Create sample data with multiple messages
    let data = create_multi_message_data(3);
    let cursor = Cursor::new(&data);
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    // Collect specific information
    let mut messages: Vec<MessageInfo> = Vec::new();
    let mut current_msg: Option<MessageInfo> = None;
    let mut current_segment: String = String::new();

    println!("Processing {} messages...\n", 3);

    while let Ok(Some(event)) = parser.next_event() {
        match event {
            Event::StartMessage { .. } => {
                current_msg = Some(MessageInfo::default());
            }
            Event::Segment { id } => {
                current_segment = String::from_utf8_lossy(&id).to_string();
                if let Some(ref mut msg) = current_msg {
                    msg.segments.push(current_segment.clone());
                }
            }
            Event::Field { num, raw } => {
                let value = String::from_utf8_lossy(&raw);
                if let Some(ref mut msg) = current_msg {
                    // Extract key fields
                    match (current_segment.as_str(), num) {
                        ("MSH", 9) => msg.message_type = Some(value.to_string()),
                        ("MSH", 10) => msg.control_id = Some(value.to_string()),
                        ("MSH", 12) => msg.version = Some(value.to_string()),
                        ("PID", 3) if value.contains("^^^") => {
                            // Extract patient ID (first component)
                            msg.patient_id = value.split('^').next().map(|s| s.to_string());
                        }
                        ("PID", 5) => msg.patient_name = Some(value.to_string()),
                        _ => {}
                    }
                }
            }
            Event::EndMessage => {
                if let Some(msg) = current_msg.take() {
                    messages.push(msg);
                }
            }
        }
    }

    // Display collected information
    println!("Extracted message information:");
    for (i, msg) in messages.iter().enumerate() {
        println!("\nMessage {}:", i + 1);
        println!("  Type: {:?}", msg.message_type);
        println!("  Control ID: {:?}", msg.control_id);
        println!("  Version: {:?}", msg.version);
        println!("  Patient ID: {:?}", msg.patient_id);
        println!("  Patient Name: {:?}", msg.patient_name);
        println!("  Segments: {:?}", msg.segments);
    }
    println!();
}

/// Message information extracted during streaming
#[derive(Debug, Default)]
struct MessageInfo {
    message_type: Option<String>,
    control_id: Option<String>,
    version: Option<String>,
    patient_id: Option<String>,
    patient_name: Option<String>,
    segments: Vec<String>,
}

/// Example 3: Memory-efficient processing for large files
fn memory_efficient_example() {
    println!("--- Example 3: Memory-Efficient Processing ---\n");

    // Simulate a large file with many messages
    let message_count = 100;
    let data = create_multi_message_data(message_count);

    println!(
        "Processing {} messages from {} bytes of data...\n",
        message_count,
        data.len()
    );

    let cursor = Cursor::new(&data);
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    // Process without storing full messages
    let mut stats = ProcessingStats::default();
    let mut current_segment = String::new();

    while let Ok(Some(event)) = parser.next_event() {
        match event {
            Event::StartMessage { .. } => {
                stats.messages_started += 1;
            }
            Event::Segment { id } => {
                stats.segments += 1;
                current_segment = String::from_utf8_lossy(&id).to_string();
            }
            Event::Field { num, raw } => {
                stats.fields += 1;
                let value = String::from_utf8_lossy(&raw);

                // Process only specific fields without storing everything
                if current_segment == "MSH" && num == 10 {
                    stats.control_ids.push(value.to_string());
                }
            }
            Event::EndMessage => {
                stats.messages_completed += 1;
            }
        }
    }

    println!("Processing Statistics:");
    println!("  Messages started: {}", stats.messages_started);
    println!("  Messages completed: {}", stats.messages_completed);
    println!("  Total segments: {}", stats.segments);
    println!("  Total fields: {}", stats.fields);
    println!("  Control IDs collected: {}", stats.control_ids.len());

    // Memory note
    println!("\nMemory Efficiency Note:");
    println!("  - Streaming parser processes one event at a time");
    println!("  - Only accumulated data is stored (stats, specific fields)");
    println!("  - Full messages are never loaded into memory");
    println!("  - Suitable for files of any size");
    println!();
}

/// Statistics collected during processing
#[derive(Debug, Default)]
struct ProcessingStats {
    messages_started: usize,
    messages_completed: usize,
    segments: usize,
    fields: usize,
    control_ids: Vec<String>,
}

/// Demonstrate async streaming pattern
fn demonstrate_async_pattern() {
    println!("--- Example 4: Async Streaming Pattern ---\n");

    println!("Async streaming with backpressure:");
    println!("```rust");
    println!("use hl7v2_stream::{{StreamParserBuilder, AsyncStreamParser, Event}};");
    println!("use tokio::sync::mpsc;");
    println!();
    println!("#[tokio::main]");
    println!("async fn main() {{");
    println!("    // Configure the parser with memory limits");
    println!("    let builder = StreamParserBuilder::new()");
    println!("        .buffer_size(100)           // Event buffer size");
    println!("        .max_message_size(1024 * 1024); // 1MB max message");
    println!();
    println!("    let data = std::fs::read(\"large_file.hl7\").unwrap();");
    println!("    let mut parser = builder.build_async(data);");
    println!();
    println!("    // Process events as they arrive");
    println!("    while let Some(result) = parser.next().await {{");
    println!("        match result {{");
    println!("            Ok(event) => handle_event(event),");
    println!("            Err(StreamError::MessageTooLarge {{ actual, max }}) => {{");
    println!("                eprintln!(\"Message too large: {{}} > {{}}\", actual, max);");
    println!("                break;");
    println!("            }}");
    println!("            Err(e) => eprintln!(\"Error: {{:?}}\", e),");
    println!("        }}");
    println!("    }}");
    println!("}}");
    println!("```\n");

    println!("Backpressure handling with channels:");
    println!("```rust");
    println!("async fn process_with_backpressure(data: Vec<u8>) {{");
    println!("    // Create a bounded channel for backpressure");
    println!("    let (tx, mut rx) = mpsc::channel(100);");
    println!();
    println!("    // Spawn parser task");
    println!("    let parser_handle = tokio::spawn(async move {{");
    println!("        let mut parser = StreamParserBuilder::new()");
    println!("            .buffer_size(50)");
    println!("            .build_async(data);");
    println!();
    println!("        while let Some(result) = parser.next().await {{");
    println!("            if tx.send(result).await.is_err() {{");
    println!("                break; // Receiver dropped");
    println!("            }}");
    println!("        }}");
    println!("    }});");
    println!();
    println!("    // Process events at your own pace");
    println!("    while let Some(result) = rx.recv().await {{");
    println!("        match result {{");
    println!("            Ok(event) => process_event(event),");
    println!("            Err(e) => handle_error(e),");
    println!("        }}");
    println!("        // Backpressure is automatic - parser waits if channel is full");
    println!("    }}");
    println!("}}");
    println!("```\n");
}

/// Example 5: Error handling in streaming
fn error_handling_example() {
    println!("--- Example 5: Error Handling ---\n");

    // Test with valid data
    println!("Parsing valid message:");
    let cursor = Cursor::new(SAMPLE_MESSAGE);
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let mut event_count = 0;
    while let Ok(Some(event)) = parser.next_event() {
        event_count += 1;
        let _ = event; // Process event
    }
    println!("  ✓ Successfully processed {} events", event_count);
    println!();

    // Test with truncated data
    println!("Parsing truncated message:");
    let truncated: &[u8] = b"MSH|^~\\&|App\rPID|1"; // No segment terminator
    let cursor = Cursor::new(truncated);
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    loop {
        match parser.next_event() {
            Ok(Some(event)) => {
                println!("  Event: {:?}", event);
            }
            Ok(None) => {
                println!("  ✓ Stream ended (EOF reached)");
                break;
            }
            Err(e) => {
                println!("  ✗ Error: {:?}", e);
                break;
            }
        }
    }
    println!();

    // Error handling best practices
    println!("Error handling best practices:");
    println!("  1. Always handle potential errors in next_event()");
    println!("  2. Check for None to detect end of stream");
    println!("  3. For async, handle StreamError::MessageTooLarge");
    println!("  4. Log errors with context for debugging");
    println!("  5. Consider retry/resume strategies for partial failures");
    println!();

    // Demonstrate robust error handling pattern
    println!("Robust error handling pattern:");
    println!("```rust");
    println!("fn robust_parse(data: &[u8]) -> Result<ParseSummary, StreamError> {{");
    println!("    let cursor = Cursor::new(data);");
    println!("    let buf_reader = BufReader::new(cursor);");
    println!("    let mut parser = StreamParser::new(buf_reader);");
    println!();
    println!("    let mut summary = ParseSummary::default();");
    println!();
    println!("    loop {{");
    println!("        match parser.next_event() {{");
    println!("            Ok(Some(event)) => summary.process_event(event),");
    println!("            Ok(None) => break, // Normal end");
    println!("            Err(StreamError::ParseError(msg)) => {{");
    println!("                summary.errors.push(msg);");
    println!("                // Continue parsing if possible");
    println!("            }}");
    println!("            Err(e) => return Err(e),");
    println!("        }}");
    println!("    }}");
    println!();
    println!("    Ok(summary)");
    println!("}}");
    println!("```\n");
}

/// Summary of parsing results
#[allow(dead_code)]
#[derive(Debug, Default)]
struct ParseSummary {
    messages: usize,
    segments: usize,
    fields: usize,
    errors: Vec<String>,
}

#[allow(dead_code)]
impl ParseSummary {
    fn process_event(&mut self, event: Event) {
        match event {
            Event::StartMessage { .. } => self.messages += 1,
            Event::Segment { .. } => self.segments += 1,
            Event::Field { .. } => self.fields += 1,
            Event::EndMessage => {}
        }
    }
}
