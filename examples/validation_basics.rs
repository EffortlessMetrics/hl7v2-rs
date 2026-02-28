//! HL7 v2 Validation Basics Example
//!
//! This example demonstrates how to:
//! - Load a validation profile from YAML
//! - Validate a message against the profile
//! - Handle and interpret validation errors
//!
//! Run with: cargo run --example validation_basics

use hl7v2_core::parse;
use hl7v2_prof::{Issue, Profile, Severity, load_profile, validate};

/// Sample ADT^A01 message for validation
const SAMPLE_MESSAGE: &[u8] = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John||19700101|M\r";

/// A minimal validation profile in YAML format
const MINIMAL_PROFILE_YAML: &str = r#"
message_structure: ADT_A01
version: "2.5.1"
segments:
  - id: MSH
  - id: PID
constraints:
  - path: MSH.9
    required: true
    message: "Message Type is required"
  - path: MSH.10
    required: true
    message: "Message Control ID is required"
  - path: MSH.12
    required: true
    message: "Version ID is required"
  - path: PID.3
    required: true
    message: "Patient Identifier is required"
  - path: PID.5
    required: true
    message: "Patient Name is required"
datatypes:
  - path: PID.7
    datatype: DT
    message: "Date of Birth must be a valid date (YYYYMMDD)"
  - path: PID.8
    datatype: ID
    allowed_values: ["M", "F", "O", "U"]
    message: "Sex must be M, F, O, or U"
"#;

/// A stricter profile with more constraints
const STRICT_PROFILE_YAML: &str = r#"
message_structure: ADT_A01
version: "2.5.1"
segments:
  - id: MSH
  - id: PID
  - id: PV1
constraints:
  - path: MSH.3
    required: true
    message: "Sending Application is required"
  - path: MSH.4
    required: true
    message: "Sending Facility is required"
  - path: MSH.5
    required: true
    message: "Receiving Application is required"
  - path: MSH.6
    required: true
    message: "Receiving Facility is required"
  - path: MSH.7
    required: true
    message: "Date/Time of Message is required"
  - path: MSH.9
    required: true
    pattern: "^ADT\\^A01"
    message: "Message Type must be ADT^A01"
  - path: MSH.10
    required: true
    min_length: 1
    max_length: 199
    message: "Message Control ID must be 1-199 characters"
  - path: MSH.11
    required: true
    allowed_values: ["P", "T", "D"]
    message: "Processing ID must be P, T, or D"
  - path: MSH.12
    required: true
    allowed_values: ["2.3", "2.3.1", "2.4", "2.5", "2.5.1", "2.6", "2.7", "2.8"]
    message: "Unsupported HL7 version"
  - path: PID.3
    required: true
    message: "Patient Identifier is required"
  - path: PID.3.1
    required: true
    min_length: 1
    message: "Patient ID Number is required"
  - path: PID.5
    required: true
    message: "Patient Name is required"
  - path: PID.5.1
    required: true
    message: "Patient Family Name is required"
  - path: PID.5.2
    required: true
    message: "Patient Given Name is required"
  - path: PID.7
    required: true
    datatype: DT
    message: "Date of Birth is required and must be valid"
  - path: PID.8
    required: true
    datatype: ID
    allowed_values: ["M", "F", "O", "U"]
    message: "Sex is required and must be M, F, O, or U"
  - path: PID.11
    required: false
    message: "Patient Address is optional"
lengths:
  - path: PID.3.1
    min: 1
    max: 20
    message: "Patient ID must be 1-20 characters"
  - path: PID.5.1
    min: 1
    max: 100
    message: "Family name must be 1-100 characters"
  - path: PID.5.2
    min: 1
    max: 50
    message: "Given name must be 1-50 characters"
"#;

fn main() {
    println!("=== HL7 v2 Validation Basics Example ===\n");

    // Example 1: Load and use a minimal profile
    minimal_validation_example();

    // Example 2: Strict validation with more constraints
    strict_validation_example();

    // Example 3: Validate an invalid message
    invalid_message_example();

    // Example 4: Working with validation results
    working_with_results_example();
}

/// Example 1: Load and use a minimal validation profile
fn minimal_validation_example() {
    println!("--- Example 1: Minimal Profile Validation ---\n");

    // Load the profile from YAML
    println!("Loading minimal validation profile...");
    let profile: Profile = match load_profile(MINIMAL_PROFILE_YAML) {
        Ok(p) => {
            println!("✓ Profile loaded successfully");
            println!("  Message Structure: {}", p.message_structure);
            println!("  Version: {}", p.version);
            println!(
                "  Segments: {:?}",
                p.segments.iter().map(|s| &s.id).collect::<Vec<_>>()
            );
            println!("  Constraints: {}", p.constraints.len());
            p
        }
        Err(e) => {
            eprintln!("✗ Failed to load profile: {}", e);
            return;
        }
    };
    println!();

    // Parse the message
    println!("Parsing message...");
    let message = match parse(SAMPLE_MESSAGE) {
        Ok(m) => {
            println!("✓ Message parsed successfully");
            m
        }
        Err(e) => {
            eprintln!("✗ Failed to parse message: {}", e);
            return;
        }
    };
    println!();

    // Validate the message against the profile
    println!("Validating message against profile...");
    let issues = validate(&message, &profile);

    if issues.is_empty() {
        println!("✓ Message is valid - no issues found");
    } else {
        println!("✗ Validation completed with {} issue(s):", issues.len());
        for issue in &issues {
            print_issue(issue);
        }
    }
    println!();
}

/// Example 2: Strict validation with more constraints
fn strict_validation_example() {
    println!("--- Example 2: Strict Profile Validation ---\n");

    // Load the strict profile
    println!("Loading strict validation profile...");
    let profile: Profile = match load_profile(STRICT_PROFILE_YAML) {
        Ok(p) => {
            println!("✓ Profile loaded with {} constraints", p.constraints.len());
            p
        }
        Err(e) => {
            eprintln!("✗ Failed to load profile: {}", e);
            return;
        }
    };
    println!();

    // Parse the message
    let message = match parse(SAMPLE_MESSAGE) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("✗ Failed to parse message: {}", e);
            return;
        }
    };

    // Validate
    println!("Validating against strict profile...");
    let issues = validate(&message, &profile);

    // Categorize issues by severity
    let errors: Vec<_> = issues
        .iter()
        .filter(|i| i.severity == Severity::Error)
        .collect();
    let warnings: Vec<_> = issues
        .iter()
        .filter(|i| i.severity == Severity::Warning)
        .collect();

    println!("Validation Results:");
    println!("  Errors: {}", errors.len());
    println!("  Warnings: {}", warnings.len());
    println!();

    if !errors.is_empty() {
        println!("Errors found:");
        for issue in errors {
            print_issue(issue);
        }
    }

    if !warnings.is_empty() {
        println!("Warnings found:");
        for issue in warnings {
            print_issue(issue);
        }
    }

    if errors.is_empty() && warnings.is_empty() {
        println!("✓ Message passes all validation checks");
    }
    println!();
}

/// Example 3: Validate an invalid message
fn invalid_message_example() {
    println!("--- Example 3: Invalid Message Validation ---\n");

    // Create an invalid message (missing required fields, invalid data)
    let invalid_message: &[u8] =
        b"MSH|^~\\&|||||20250128152312||ADT^A01||X|99.9\rPID|1||||||XYZ||Q\r";

    println!("Invalid message (missing fields, invalid values):");
    println!(
        "{}",
        String::from_utf8_lossy(invalid_message).replace("\r", "\r\n")
    );
    println!();

    // Parse the message
    let message = match parse(invalid_message) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("✗ Failed to parse message: {}", e);
            return;
        }
    };

    // Load minimal profile
    let profile: Profile = match load_profile(MINIMAL_PROFILE_YAML) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("✗ Failed to load profile: {}", e);
            return;
        }
    };

    // Validate
    println!("Validating invalid message...");
    let issues = validate(&message, &profile);

    println!("Found {} validation issue(s):", issues.len());
    for (i, issue) in issues.iter().enumerate() {
        println!("\n  Issue {}:", i + 1);
        print_issue(issue);
    }
    println!();
}

/// Example 4: Working with validation results programmatically
fn working_with_results_example() {
    println!("--- Example 4: Working with Validation Results ---\n");

    // Parse message and validate
    let message = parse(SAMPLE_MESSAGE).expect("Message should parse");
    let profile = load_profile(STRICT_PROFILE_YAML).expect("Profile should load");
    let issues = validate(&message, &profile);

    // Demonstrate different ways to work with results

    // 1. Check if message is valid (no errors)
    let has_errors = issues.iter().any(|i| i.severity == Severity::Error);
    println!("Message has errors: {}", has_errors);

    // 2. Count by severity
    let error_count = issues
        .iter()
        .filter(|i| i.severity == Severity::Error)
        .count();
    let warning_count = issues
        .iter()
        .filter(|i| i.severity == Severity::Warning)
        .count();
    println!("Error count: {}", error_count);
    println!("Warning count: {}", warning_count);
    println!();

    // 3. Group issues by path
    println!("Issues grouped by path:");
    use std::collections::HashMap;
    let mut by_path: HashMap<Option<&str>, Vec<&Issue>> = HashMap::new();
    for issue in &issues {
        let path = issue.path.as_deref();
        by_path.entry(path).or_default().push(issue);
    }
    for (path, path_issues) in by_path {
        println!("  {:?}: {} issue(s)", path, path_issues.len());
    }
    println!();

    // 4. Group issues by code
    println!("Issues grouped by code:");
    let mut by_code: HashMap<&str, Vec<&Issue>> = HashMap::new();
    for issue in &issues {
        by_code.entry(&issue.code).or_default().push(issue);
    }
    for (code, code_issues) in by_code {
        println!("  {}: {} occurrence(s)", code, code_issues.len());
    }
    println!();

    // 5. Filter to specific segment
    println!("Issues in PID segment:");
    let pid_issues: Vec<_> = issues
        .iter()
        .filter(|i| {
            i.path
                .as_ref()
                .map(|p| p.starts_with("PID"))
                .unwrap_or(false)
        })
        .collect();
    for issue in pid_issues {
        print_issue(issue);
    }
    println!();

    // 6. Convert to JSON for logging/API responses
    println!("Issues as JSON:");
    let json =
        serde_json::to_string_pretty(&issues).unwrap_or_else(|e| format!("JSON error: {}", e));
    println!("{}", json);
    println!();
}

/// Print a validation issue with formatting
fn print_issue(issue: &Issue) {
    let severity_icon = match issue.severity {
        Severity::Error => "✗",
        Severity::Warning => "⚠",
    };

    let path = issue.path.as_deref().unwrap_or("(unknown)");

    println!(
        "    {} [{}] {} - {}",
        severity_icon, issue.code, path, issue.detail
    );
}
