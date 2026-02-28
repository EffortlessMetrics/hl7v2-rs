//! HL7 v2 Message Building Example
//!
//! This example demonstrates how to:
//! - Build a message programmatically using the data model
//! - Add segments and fields
//! - Set custom delimiters
//! - Serialize the message to bytes
//!
//! Run with: cargo run --example message_building

use hl7v2_core::{
    Atom,
    Comp,
    Delims,
    Error,
    Field,
    Message,
    Rep,
    Segment,
    write, // For serializing messages
};

fn main() {
    println!("=== HL7 v2 Message Building Example ===\n");

    // Example 1: Build a message with default delimiters
    match build_adt_a01_message() {
        Ok(message) => {
            println!("✓ Successfully built ADT^A01 message\n");
            let bytes = write(&message);
            println!("Serialized message:");
            println!("{}\n", String::from_utf8_lossy(&bytes));
        }
        Err(e) => {
            eprintln!("✗ Failed to build message: {}", e);
            std::process::exit(1);
        }
    }

    // Example 2: Build a message with custom delimiters
    match build_message_with_custom_delimiters() {
        Ok(message) => {
            println!("✓ Successfully built message with custom delimiters\n");
            let bytes = write(&message);
            println!("Serialized message:");
            println!("{}\n", String::from_utf8_lossy(&bytes));
        }
        Err(e) => {
            eprintln!("✗ Failed to build message: {}", e);
            std::process::exit(1);
        }
    }

    // Example 3: Build a simple ACK message
    match build_ack_message() {
        Ok(message) => {
            println!("✓ Successfully built ACK message\n");
            let bytes = write(&message);
            println!("Serialized message:");
            println!("{}\n", String::from_utf8_lossy(&bytes));
        }
        Err(e) => {
            eprintln!("✗ Failed to build message: {}", e);
            std::process::exit(1);
        }
    }
}

/// Build an ADT^A01 (Admit) message with default delimiters
fn build_adt_a01_message() -> Result<Message, Error> {
    println!("--- Example 1: Building ADT^A01 Message ---");

    // Create default delimiters (|^~\&)
    let delims = Delims::new();

    // Build the MSH (Message Header) segment
    // MSH is special: the field separator (|) is counted as field 1
    let msh = build_msh_segment(&delims)?;

    // Build the PID (Patient Identification) segment
    let pid = build_pid_segment()?;

    // Build the PV1 (Patient Visit) segment
    let pv1 = build_pv1_segment()?;

    // Assemble the message
    let message = Message {
        delims,
        segments: vec![msh, pid, pv1],
        charsets: vec![],
    };

    println!("Message structure:");
    for (i, seg) in message.segments.iter().enumerate() {
        println!("  {}: {}", i + 1, seg.id_str());
    }
    println!();

    Ok(message)
}

/// Build the MSH (Message Header) segment
fn build_msh_segment(delims: &Delims) -> Result<Segment, Error> {
    // MSH segment fields (note: MSH-1 is the field separator itself)
    // MSH-2: Encoding Characters (^~\&)
    // MSH-3: Sending Application
    // MSH-4: Sending Facility
    // MSH-5: Receiving Application
    // MSH-6: Receiving Facility
    // MSH-7: Date/Time of Message
    // MSH-8: Security (optional)
    // MSH-9: Message Type (ADT^A01^ADT_A01)
    // MSH-10: Message Control ID
    // MSH-11: Processing ID (P = Production)
    // MSH-12: Version ID (2.5.1)

    let mut fields = Vec::new();

    // MSH-2: Encoding Characters
    // Note: MSH-1 is implicit (the field separator |)
    let encoding_chars = format!("{}{}{}{}", delims.comp, delims.rep, delims.esc, delims.sub);
    fields.push(create_simple_field(&encoding_chars));

    // MSH-3: Sending Application
    fields.push(create_simple_field("HL7V2RS"));

    // MSH-4: Sending Facility
    fields.push(create_simple_field("HOSPITAL"));

    // MSH-5: Receiving Application
    fields.push(create_simple_field("LABSYSTEM"));

    // MSH-6: Receiving Facility
    fields.push(create_simple_field("LABORATORY"));

    // MSH-7: Date/Time of Message (current timestamp)
    let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S").to_string();
    fields.push(create_simple_field(&timestamp));

    // MSH-8: Security (empty - optional)
    fields.push(create_empty_field());

    // MSH-9: Message Type (ADT^A01^ADT_A01) - composite field
    fields.push(create_composite_field(&["ADT", "A01", "ADT_A01"]));

    // MSH-10: Message Control ID (unique identifier)
    let control_id = format!("MSG{}", chrono::Utc::now().format("%Y%m%d%H%M%S"));
    fields.push(create_simple_field(&control_id));

    // MSH-11: Processing ID (P = Production, T = Training, D = Debugging)
    fields.push(create_simple_field("P"));

    // MSH-12: Version ID
    fields.push(create_simple_field("2.5.1"));

    Ok(Segment {
        id: *b"MSH",
        fields,
    })
}

/// Build the PID (Patient Identification) segment
fn build_pid_segment() -> Result<Segment, Error> {
    // PID segment fields
    // PID-1: Set ID
    // PID-2: Patient ID (External) - often empty
    // PID-3: Patient Identifier List (MRN)
    // PID-4: Alternate Patient ID - often empty
    // PID-5: Patient Name
    // PID-6: Mother's Maiden Name - often empty
    // PID-7: Date/Time of Birth
    // PID-8: Administrative Sex
    // PID-9: Patient Alias - often empty
    // PID-10: Race
    // PID-11: Patient Address
    // PID-12: County Code - often empty
    // PID-13: Phone Number - Home
    // PID-14: Phone Number - Business - often empty

    let mut fields = Vec::new();

    // PID-1: Set ID
    fields.push(create_simple_field("1"));

    // PID-2: Patient ID (External) - empty
    fields.push(create_empty_field());

    // PID-3: Patient Identifier List (MRN with check digit and assigning authority)
    // Format: ID^CheckDigit^CheckDigitScheme^AssigningAuthority^IdentifierType
    fields.push(create_composite_field(&[
        "12345678", "", "", "HOSPITAL", "MR",
    ]));

    // PID-4: Alternate Patient ID - empty
    fields.push(create_empty_field());

    // PID-5: Patient Name (Family^Given^Middle^Suffix^Prefix)
    fields.push(create_composite_field(&["DOE", "JOHN", "ROBERT", "", "MR"]));

    // PID-6: Mother's Maiden Name - empty
    fields.push(create_empty_field());

    // PID-7: Date/Time of Birth (YYYYMMDD)
    fields.push(create_simple_field("19850315"));

    // PID-8: Administrative Sex (M, F, O, U)
    fields.push(create_simple_field("M"));

    // PID-9: Patient Alias - empty
    fields.push(create_empty_field());

    // PID-10: Race (CE data type - coded element)
    fields.push(create_composite_field(&["2106-3", "White", "HL70005"]));

    // PID-11: Patient Address (Street^Street2^City^State^Zip^Country^AddressType)
    fields.push(create_composite_field(&[
        "123 MAIN ST",
        "APT 4B",
        "ANYTOWN",
        "CA",
        "12345",
        "USA",
        "H",
    ]));

    // PID-12: County Code - empty
    fields.push(create_empty_field());

    // PID-13: Phone Number - Home
    fields.push(create_simple_field("(555)123-4567"));

    // PID-14: Phone Number - Business - empty
    fields.push(create_empty_field());

    Ok(Segment {
        id: *b"PID",
        fields,
    })
}

/// Build the PV1 (Patient Visit) segment
fn build_pv1_segment() -> Result<Segment, Error> {
    // PV1 segment fields (partial)
    // PV1-1: Set ID
    // PV1-2: Patient Class (I=Inpatient, O=Outpatient, E=Emergency)
    // PV1-3: Assigned Patient Location
    // PV1-4: Admission Type
    // PV1-5: Preadmit Number - often empty
    // ... more fields

    let mut fields = Vec::new();

    // PV1-1: Set ID
    fields.push(create_simple_field("1"));

    // PV1-2: Patient Class
    fields.push(create_simple_field("I"));

    // PV1-3: Assigned Patient Location (Point of Care^Room^Bed)
    fields.push(create_composite_field(&["3N", "301", "A"]));

    // PV1-4: Admission Type (E=Emergency, U=Urgent, E=Elective)
    fields.push(create_simple_field("E"));

    // PV1-5: Preadmit Number - empty
    fields.push(create_empty_field());

    // PV1-6: Prior Patient Location - empty
    fields.push(create_empty_field());

    // PV1-7: Attending Doctor (ID^Name)
    fields.push(create_composite_field(&[
        "DR123", "SMITH", "JOHN", "", "", "MD",
    ]));

    // PV1-8: Referring Doctor - empty
    fields.push(create_empty_field());

    // PV1-9: Consulting Doctor - empty
    fields.push(create_empty_field());

    // PV1-10: Hospital Service
    fields.push(create_simple_field("MED"));

    Ok(Segment {
        id: *b"PV1",
        fields,
    })
}

/// Build a message with custom delimiters
fn build_message_with_custom_delimiters() -> Result<Message, Error> {
    println!("--- Example 2: Building Message with Custom Delimiters ---");

    // Create custom delimiters
    // Note: All delimiters must be distinct
    let delims = Delims {
        field: '|',
        comp: ':', // Custom component separator
        rep: '*',  // Custom repetition separator
        esc: '\\',
        sub: '>', // Custom subcomponent separator
    };

    let mut fields = Vec::new();

    // MSH-2: Encoding Characters (custom)
    let encoding_chars = format!("{}{}{}{}", delims.comp, delims.rep, delims.esc, delims.sub);
    fields.push(create_simple_field(&encoding_chars));

    // MSH-3: Sending Application
    fields.push(create_simple_field("CUSTOM_APP"));

    // MSH-4: Sending Facility
    fields.push(create_simple_field("CUSTOM_FAC"));

    // MSH-7: Date/Time
    let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S").to_string();
    fields.push(create_simple_field(&timestamp));

    // MSH-8: Security - empty
    fields.push(create_empty_field());

    // MSH-9: Message Type (using custom component separator :)
    fields.push(create_composite_field(&["ADT", "A01"]));

    // MSH-10: Message Control ID
    fields.push(create_simple_field("CUSTOM123"));

    // MSH-11: Processing ID
    fields.push(create_simple_field("T"));

    // MSH-12: Version ID
    fields.push(create_simple_field("2.5.1"));

    let msh = Segment {
        id: *b"MSH",
        fields,
    };

    println!("Custom delimiters:");
    println!("  Field: '{}'", delims.field);
    println!("  Component: '{}'", delims.comp);
    println!("  Repetition: '{}'", delims.rep);
    println!("  Escape: '{}'", delims.esc);
    println!("  Subcomponent: '{}'", delims.sub);
    println!();

    Ok(Message {
        delims,
        segments: vec![msh],
        charsets: vec![],
    })
}

/// Build a simple ACK (Acknowledgment) message
fn build_ack_message() -> Result<Message, Error> {
    println!("--- Example 3: Building ACK Message ---");

    let delims = Delims::new();
    let mut fields = Vec::new();

    // MSH-2: Encoding Characters
    let encoding_chars = format!("{}{}{}{}", delims.comp, delims.rep, delims.esc, delims.sub);
    fields.push(create_simple_field(&encoding_chars));

    // MSH-3: Sending Application (reversed from original)
    fields.push(create_simple_field("RECEIVER"));

    // MSH-4: Sending Facility
    fields.push(create_simple_field("RECV_FAC"));

    // MSH-5: Receiving Application
    fields.push(create_simple_field("SENDER"));

    // MSH-6: Receiving Facility
    fields.push(create_simple_field("SEND_FAC"));

    // MSH-7: Date/Time
    let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S").to_string();
    fields.push(create_simple_field(&timestamp));

    // MSH-8: Security - empty
    fields.push(create_empty_field());

    // MSH-9: Message Type (ACK)
    fields.push(create_composite_field(&["ACK", "", "ACK"]));

    // MSH-10: Message Control ID
    fields.push(create_simple_field("ACK123"));

    // MSH-11: Processing ID
    fields.push(create_simple_field("P"));

    // MSH-12: Version ID
    fields.push(create_simple_field("2.5.1"));

    let msh = Segment {
        id: *b"MSH",
        fields,
    };

    // Build MSA (Message Acknowledgment) segment
    let mut msa_fields = Vec::new();

    // MSA-1: Acknowledgment Code (AA=Application Accept)
    msa_fields.push(create_simple_field("AA"));

    // MSA-2: Message Control ID (from original message)
    msa_fields.push(create_simple_field("ORIGINAL123"));

    // MSA-3: Text Message
    msa_fields.push(create_simple_field("Message accepted"));

    let msa = Segment {
        id: *b"MSA",
        fields: msa_fields,
    };

    println!("ACK message structure:");
    println!("  1: MSH (Message Header)");
    println!("  2: MSA (Message Acknowledgment)");
    println!();

    Ok(Message {
        delims,
        segments: vec![msh, msa],
        charsets: vec![],
    })
}

// ============================================================================
// Helper Functions for Building Field Structures
// ============================================================================

/// Create a simple field with a single text value
fn create_simple_field(value: &str) -> Field {
    Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![Atom::Text(value.to_string())],
            }],
        }],
    }
}

/// Create an empty field
fn create_empty_field() -> Field {
    Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![Atom::Text(String::new())],
            }],
        }],
    }
}

/// Create a composite field with multiple components
fn create_composite_field(values: &[&str]) -> Field {
    Field {
        reps: vec![Rep {
            comps: values
                .iter()
                .map(|v| Comp {
                    subs: vec![Atom::Text(v.to_string())],
                })
                .collect(),
        }],
    }
}
