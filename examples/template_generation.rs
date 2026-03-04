//! HL7 v2 Template Generation Example
//!
//! This example demonstrates how to:
//! - Create message templates programmatically
//! - Generate messages with dynamic values
//! - Use various value sources (fixed, random, UUID, etc.)
//!
//! Run with: cargo run --example template_generation

use hl7v2_core::{get, write};
use hl7v2_template::{Template, ValueSource, generate};
use std::collections::HashMap;

fn main() {
    println!("=== HL7 v2 Template Generation Example ===\n");

    // Example 1: Create template programmatically
    programmatic_template_example();

    // Example 2: Batch generation with deterministic seeds
    batch_generation_example();

    // Example 3: Value source types
    value_source_examples();
}

/// Example 1: Create template programmatically
fn programmatic_template_example() {
    println!("--- Example 1: Programmatic Template Creation ---\n");

    // Create value mappings using the actual ValueSource enum
    let mut values: HashMap<String, Vec<ValueSource>> = HashMap::new();

    // Fixed values
    values.insert(
        "sending_app".to_string(),
        vec![ValueSource::Fixed("HL7V2RS".to_string())],
    );
    values.insert(
        "sending_fac".to_string(),
        vec![ValueSource::Fixed("HOSPITAL".to_string())],
    );
    values.insert(
        "receiving_app".to_string(),
        vec![ValueSource::Fixed("LABSYSTEM".to_string())],
    );
    values.insert(
        "receiving_fac".to_string(),
        vec![ValueSource::Fixed("LABORATORY".to_string())],
    );
    values.insert(
        "version".to_string(),
        vec![ValueSource::Fixed("2.5.1".to_string())],
    );

    // Random choice from list
    values.insert(
        "patient_name".to_string(),
        vec![ValueSource::From(vec![
            "SMITH^JOHN".to_string(),
            "JONES^JANE".to_string(),
            "BROWN^BOB".to_string(),
            "WILSON^MARY".to_string(),
            "TAYLOR^DAVID".to_string(),
        ])],
    );

    // Random numeric (6 digits)
    values.insert("patient_id".to_string(), vec![ValueSource::Numeric { digits: 6 }]);

    // UUID
    values.insert("control_id".to_string(), vec![ValueSource::UuidV4]);

    // Current timestamp
    values.insert("timestamp".to_string(), vec![ValueSource::DtmNowUtc]);

    // Random date
    values.insert(
        "birth_date".to_string(),
        vec![ValueSource::Date {
            start: "19500101".to_string(),
            end: "20051231".to_string(),
        }],
    );

    // Random choice for sex
    values.insert(
        "sex".to_string(),
        vec![ValueSource::From(vec!["M".to_string(), "F".to_string()])],
    );

    // Create the template
    let template = Template {
        name: "ADT_A01_Programmatic".to_string(),
        delims: "^~\\&".to_string(),
        segments: vec![
            "MSH|^~\\&|{{sending_app}}|{{sending_fac}}|{{receiving_app}}|{{receiving_fac}}|{{timestamp}}||ADT^A01^ADT_A01|{{control_id}}|P|{{version}}".to_string(),
            "PID|1||{{patient_id}}^^^HOSP^MR||{{patient_name}}||{{birth_date}}|{{sex}}".to_string(),
        ],
        values,
    };

    println!("Created template programmatically:");
    println!("  Name: {}", template.name);
    println!("  Segments: {}", template.segments.len());
    println!("  Value mappings: {}", template.values.len());
    println!();

    // Generate a message
    println!("Generating message with seed 42...");
    match generate(&template, 42, 1) {
        Ok(messages) => {
            println!("✓ Generated {} message(s)\n", messages.len());

            if let Some(msg) = messages.first() {
                println!("Generated Message:");
                let bytes = write(msg);
                println!("{}", String::from_utf8_lossy(&bytes).replace("\r", "\r\n"));

                // Show extracted values
                println!("\nExtracted values:");
                println!("  Control ID: {:?}", get(msg, "MSH.10"));
                println!("  Patient ID: {:?}", get(msg, "PID.3.1"));
                println!("  Patient Name: {:?}", get(msg, "PID.5"));
                println!("  Birth Date: {:?}", get(msg, "PID.7"));
                println!("  Sex: {:?}", get(msg, "PID.8"));
            }
        }
        Err(e) => {
            eprintln!("✗ Failed to generate: {:?}", e);
        }
    }
    println!();
}

/// Example 2: Batch generation with deterministic seeds
fn batch_generation_example() {
    println!("--- Example 2: Batch Generation ---\n");

    // Create a simple template
    let mut values: HashMap<String, Vec<ValueSource>> = HashMap::new();
    values.insert(
        "sending_app".to_string(),
        vec![ValueSource::Fixed("HL7V2RS".to_string())],
    );
    values.insert("timestamp".to_string(), vec![ValueSource::DtmNowUtc]);
    values.insert("control_id".to_string(), vec![ValueSource::UuidV4]);
    values.insert("patient_id".to_string(), vec![ValueSource::Numeric { digits: 6 }]);
    values.insert(
        "patient_name".to_string(),
        vec![ValueSource::From(vec![
            "DOE^JOHN".to_string(),
            "SMITH^JANE".to_string(),
            "JONES^BOB".to_string(),
        ])],
    );

    let template = Template {
        name: "SimpleBatch".to_string(),
        delims: "^~\\&".to_string(),
        segments: vec![
            "MSH|^~\\&|{{sending_app}}|FAC|RECV|RECVFAC|{{timestamp}}||ADT^A01|{{control_id}}|P|2.5.1".to_string(),
            "PID|1||{{patient_id}}^^^HOSP^MR||{{patient_name}}||19800101|M".to_string(),
        ],
        values,
    };

    // Generate a batch of messages
    let batch_size = 5;
    println!("Generating batch of {} messages...\n", batch_size);

    // Using different seeds for each message
    let base_seed = 1000u64;
    let mut all_messages = Vec::new();

    for i in 0..batch_size {
        let seed = base_seed + i;
        match generate(&template, seed, 1) {
            Ok(mut msgs) => {
                all_messages.append(&mut msgs);
            }
            Err(e) => {
                eprintln!("Failed to generate message {}: {:?}", i, e);
            }
        }
    }

    println!("Generated {} messages:\n", all_messages.len());
    for (i, msg) in all_messages.iter().enumerate() {
        let control_id = get(msg, "MSH.10").unwrap_or("N/A");
        let patient_id = get(msg, "PID.3.1").unwrap_or("N/A");
        let patient_name = get(msg, "PID.5").unwrap_or("N/A");
        println!(
            "  {}: ControlID={}, Patient={} (ID={})",
            i + 1,
            control_id,
            patient_name,
            patient_id
        );
    }

    // Demonstrate reproducibility
    println!("\nReproducibility test:");
    println!("  Generating with same seed (42) twice...");

    let first = generate(&template, 42, 1).unwrap();
    let second = generate(&template, 42, 1).unwrap();

    let first_id = get(&first[0], "PID.3.1");
    let second_id = get(&second[0], "PID.3.1");

    println!("  First Patient ID: {:?}", first_id);
    println!("  Second Patient ID: {:?}", second_id);
    println!("  Match: {}", first_id == second_id);
    println!();
}

/// Example 3: Demonstrate all value source types
fn value_source_examples() {
    println!("--- Example 3: Value Source Types ---\n");

    println!("Available ValueSource types:\n");

    // Fixed value
    println!("1. Fixed - Constant value");
    println!("   ValueSource::Fixed(\"VALUE\".to_string())");
    println!("   Result: Always generates \"VALUE\"\n");

    // From (random choice)
    println!("2. From - Random choice from list");
    println!("   ValueSource::From(vec![\"A\".to_string(), \"B\".to_string()])");
    println!("   Result: Randomly selects A or B\n");

    // Numeric
    println!("3. Numeric - Random number with specified digits");
    println!("   ValueSource::Numeric {{ digits: 6 }}");
    println!("   Result: Random 6-digit number\n");

    // Date
    println!("4. Date - Random date in range");
    println!("   ValueSource::Date {{");
    println!("       start: \"19500101\".to_string(),");
    println!("       end: \"20051231\".to_string(),");
    println!("   }}");
    println!("   Result: Random date formatted as YYYYMMDD\n");

    // Gaussian
    println!("5. Gaussian - Gaussian-distributed value");
    println!("   ValueSource::Gaussian {{ mean: 50.0, sd: 10.0, precision: 2 }}");
    println!("   Result: Value around mean with normal distribution\n");

    // UUID v4
    println!("6. UuidV4 - Random UUID");
    println!("   ValueSource::UuidV4");
    println!("   Result: e.g., \"550e8400-e29b-41d4-a716-446655440000\"\n");

    // DtmNowUtc
    println!("7. DtmNowUtc - Current UTC timestamp");
    println!("   ValueSource::DtmNowUtc");
    println!("   Result: Current time as YYYYMMDDHHMMSS\n");

    // Map
    println!("8. Map - Mapped value");
    println!("   ValueSource::Map(HashMap::from([(\"KEY\", \"VALUE\")]))");
    println!("   Result: Looks up KEY in provided map\n");

    // Realistic data generators
    println!("9. Realistic data generators:");
    println!("   - RealisticName {{ gender: Some(\"M\".to_string()) }}");
    println!("   - RealisticAddress");
    println!("   - RealisticPhone");
    println!("   - RealisticSsn");
    println!("   - RealisticMrn");
    println!("   - RealisticIcd10");
    println!("   - RealisticLoinc");
    println!("   - RealisticMedication");
    println!("   - RealisticBloodType");
    println!("   - RealisticRace");
    println!("   - RealisticEthnicity\n");

    // Error injection
    println!("10. Error injection (for testing):");
    println!("    - InvalidSegmentId");
    println!("    - InvalidFieldFormat");
    println!("    - InvalidRepFormat");
    println!("    - InvalidCompFormat");
    println!("    - InvalidSubcompFormat");
    println!("    - DuplicateDelims");
    println!("    - BadDelimLength");
    println!();
}
