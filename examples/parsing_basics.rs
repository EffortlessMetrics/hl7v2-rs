//! HL7 v2 Parsing Basics Example
//!
//! This example demonstrates how to:
//! - Parse a simple HL7 message from a string
//! - Access segments and fields using path-based queries
//! - Handle parsing errors properly
//!
//! Run with: cargo run --example parsing_basics

use hl7v2_core::{parse, get, Error, Message};

/// A simple HL7 v2 ADT^A01 (Admit) message
const SAMPLE_MESSAGE: &[u8] = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John^Middle||19700101|M|||123 Main St^^Anytown^CA^12345||5551234567\r";

fn main() {
    println!("=== HL7 v2 Parsing Basics Example ===\n");

    // Example 1: Parse a simple message
    match parse_message_example() {
        Ok(message) => {
            println!("✓ Successfully parsed message with {} segments\n", message.segments.len());
        }
        Err(e) => {
            eprintln!("✗ Failed to parse message: {}", e);
            std::process::exit(1);
        }
    }

    // Example 2: Access segments and fields
    access_fields_example();

    // Example 3: Handle parsing errors
    error_handling_example();
}

/// Parse a simple HL7 message from bytes
fn parse_message_example() -> Result<Message, Error> {
    println!("--- Example 1: Parsing a Message ---");
    
    // The parse function accepts a byte slice (&[u8])
    // HL7 messages typically use \r as segment terminators
    println!("Input message (truncated):");
    println!("  {:?}", String::from_utf8_lossy(&SAMPLE_MESSAGE[..80.min(SAMPLE_MESSAGE.len())]));
    println!();

    // Parse the message - this returns Result<Message, Error>
    let message = parse(SAMPLE_MESSAGE)?;

    // Display basic message information
    println!("Parsed message details:");
    println!("  Delimiters: field='{}' component='{}' repetition='{}' escape='{}' subcomponent='{}'",
        message.delims.field,
        message.delims.comp,
        message.delims.rep,
        message.delims.esc,
        message.delims.sub,
    );
    println!("  Segment count: {}", message.segments.len());
    
    // List all segments
    for (i, segment) in message.segments.iter().enumerate() {
        let segment_id = String::from_utf8_lossy(&segment.id);
        println!("  Segment {}: {} ({} fields)", i + 1, segment_id, segment.fields.len());
    }
    println!();

    Ok(message)
}

/// Access fields using path-based queries
fn access_fields_example() {
    println!("--- Example 2: Accessing Fields ---");

    // Parse the message (we know it's valid from the previous example)
    let message = parse(SAMPLE_MESSAGE).expect("Message should parse successfully");

    // The `get` function allows path-based field access
    // Path format: SEGMENT.FIELD[.COMPONENT][.SUBCOMPONENT]
    // Examples: MSH.3, PID.5.1, PID.5.2
    
    // Access MSH segment fields
    // MSH.3 = Sending Application
    let sending_app = get(&message, "MSH.3");
    println!("MSH.3 (Sending Application): {:?}", sending_app);

    // MSH.4 = Sending Facility
    let sending_fac = get(&message, "MSH.4");
    println!("MSH.4 (Sending Facility): {:?}", sending_fac);

    // MSH.9 = Message Type (this is a composite field)
    let message_type = get(&message, "MSH.9");
    println!("MSH.9 (Message Type): {:?}", message_type);

    // MSH.9.1 = Message Code (first component)
    let msg_code = get(&message, "MSH.9.1");
    println!("MSH.9.1 (Message Code): {:?}", msg_code);

    // MSH.9.2 = Trigger Event (second component)
    let trigger = get(&message, "MSH.9.2");
    println!("MSH.9.2 (Trigger Event): {:?}", trigger);

    // MSH.10 = Message Control ID
    let control_id = get(&message, "MSH.10");
    println!("MSH.10 (Message Control ID): {:?}", control_id);

    // MSH.12 = Version ID
    let version = get(&message, "MSH.12");
    println!("MSH.12 (Version ID): {:?}", version);

    println!();

    // Access PID segment fields
    // PID.3 = Patient Identifier List
    let patient_id = get(&message, "PID.3");
    println!("PID.3 (Patient ID): {:?}", patient_id);

    // PID.3.1 = ID Number (first component of patient ID)
    let id_number = get(&message, "PID.3.1");
    println!("PID.3.1 (ID Number): {:?}", id_number);

    // PID.3.4 = Assigning Authority
    let assigning_auth = get(&message, "PID.3.4");
    println!("PID.3.4 (Assigning Authority): {:?}", assigning_auth);

    // PID.3.5 = Identifier Type
    let id_type = get(&message, "PID.3.5");
    println!("PID.3.5 (Identifier Type): {:?}", id_type);

    // PID.5 = Patient Name (composite field)
    let patient_name = get(&message, "PID.5");
    println!("PID.5 (Patient Name): {:?}", patient_name);

    // PID.5.1 = Family Name
    let family_name = get(&message, "PID.5.1");
    println!("PID.5.1 (Family Name): {:?}", family_name);

    // PID.5.2 = Given Name
    let given_name = get(&message, "PID.5.2");
    println!("PID.5.2 (Given Name): {:?}", given_name);

    // PID.7 = Date/Time of Birth
    let birth_date = get(&message, "PID.7");
    println!("PID.7 (Date of Birth): {:?}", birth_date);

    // PID.8 = Administrative Sex
    let sex = get(&message, "PID.8");
    println!("PID.8 (Administrative Sex): {:?}", sex);

    // PID.11 = Patient Address
    let address = get(&message, "PID.11");
    println!("PID.11 (Patient Address): {:?}", address);

    // PID.11.1 = Street Address
    let street = get(&message, "PID.11.1");
    println!("PID.11.1 (Street Address): {:?}", street);

    // PID.11.3 = City
    let city = get(&message, "PID.11.3");
    println!("PID.11.3 (City): {:?}", city);

    // PID.11.4 = State
    let state = get(&message, "PID.11.4");
    println!("PID.11.4 (State): {:?}", state);

    // PID.11.5 = Zip Code
    let zip = get(&message, "PID.11.5");
    println!("PID.11.5 (Zip Code): {:?}", zip);

    // PID.13 = Phone Number - Home
    let phone = get(&message, "PID.13");
    println!("PID.13 (Phone Number): {:?}", phone);

    println!();

    // Demonstrate accessing non-existent fields
    let nonexistent = get(&message, "PID.999");
    println!("PID.999 (Non-existent): {:?}", nonexistent);

    let nonexistent_component = get(&message, "PID.5.999");
    println!("PID.5.999 (Non-existent component): {:?}", nonexistent_component);

    println!();
}

/// Handle parsing errors gracefully
fn error_handling_example() {
    println!("--- Example 3: Error Handling ---");

    // Example of an invalid message (missing MSH segment)
    let invalid_message = b"PID|1||123456||Doe^John\r";
    
    match parse(invalid_message) {
        Ok(msg) => {
            println!("✗ Unexpectedly parsed invalid message: {:?}", msg);
        }
        Err(Error::InvalidSegmentId) => {
            println!("✓ Correctly detected invalid segment ID (missing MSH)");
        }
        Err(e) => {
            println!("✓ Detected error: {}", e);
        }
    }

    // Example of an empty message
    let empty_message = b"";
    
    match parse(empty_message) {
        Ok(msg) => {
            println!("✗ Unexpectedly parsed empty message: {:?}", msg);
        }
        Err(e) => {
            println!("✓ Correctly rejected empty message: {}", e);
        }
    }

    // Example of a message with invalid UTF-8
    let invalid_utf8: &[u8] = &[0xFF, 0xFE, 0xFD];
    
    match parse(invalid_utf8) {
        Ok(msg) => {
            println!("✗ Unexpectedly parsed invalid UTF-8: {:?}", msg);
        }
        Err(Error::InvalidCharset) => {
            println!("✓ Correctly detected invalid charset");
        }
        Err(e) => {
            println!("✓ Detected error: {}", e);
        }
    }

    // Example of a message with duplicate delimiters
    let duplicate_delims = b"MSH||||SendingApp|SendingFac|\r";
    
    match parse(duplicate_delims) {
        Ok(msg) => {
            println!("✗ Unexpectedly parsed message with duplicate delimiters: {:?}", msg);
        }
        Err(Error::DuplicateDelims) => {
            println!("✓ Correctly detected duplicate delimiters");
        }
        Err(e) => {
            println!("✓ Detected error: {}", e);
        }
    }

    println!();

    // Best practice: Use pattern matching for specific error handling
    println!("Best practice for error handling:");
    println!("  match parse(message_bytes) {{");
    println!("      Ok(message) => process_message(message),");
    println!("      Err(Error::InvalidCharset) => handle_encoding_error(),");
    println!("      Err(Error::InvalidSegmentId) => handle_structure_error(),");
    println!("      Err(Error::ParseError {{ segment_id, field_index, source }}) => {{");
    println!("          log_error(\"Parse error at {{}} field {{}}: {{}}\", segment_id, field_index, source);");
    println!("      }}");
    println!("      Err(e) => handle_other_error(e),");
    println!("  }}");
    println!();
}
