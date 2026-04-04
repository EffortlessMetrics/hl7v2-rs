#[cfg(test)]
#[test]
fn test_parse_simple_message() {
    let hl7 = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r";
    let result = hl7v2_parser::parse(hl7.as_bytes());
    assert!(result.is_ok());

    let message = result.unwrap();
    assert_eq!(message.segments.len(), 2);
}

#[test]
fn test_message_path_access() {
    let hl7 = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r";
    let message = hl7v2_parser::parse(hl7.as_bytes()).unwrap();

    // Test MSH fields
    assert_eq!(hl7v2_parser::get(&message, "MSH.3"), Some("SendingApp"));
    assert_eq!(hl7v2_parser::get(&message, "MSH.9.1"), Some("ADT"));
    assert_eq!(hl7v2_parser::get(&message, "MSH.9.2"), Some("A01"));
    assert_eq!(hl7v2_parser::get(&message, "MSH.12"), Some("2.5.1"));

    // Test PID fields
    assert_eq!(hl7v2_parser::get(&message, "PID.5.1"), Some("Doe"));
    assert_eq!(hl7v2_parser::get(&message, "PID.5.2"), Some("John"));
}

#[test]
fn test_generate_message() {
    let hl7 = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r";
    let message = hl7v2_parser::parse(hl7.as_bytes()).unwrap();
    let generated = hl7v2_writer::write(&message);
    let _generated_str = String::from_utf8_lossy(&generated);

    // The generated message should be parseable
    let reparsed = hl7v2_parser::parse(&generated).unwrap();
    assert_eq!(reparsed.segments.len(), message.segments.len());
}

#[test]
fn test_normalize_message() {
    let hl7 = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r";
    let _message = hl7v2_parser::parse(hl7.as_bytes()).unwrap();
    let normalized = hl7v2_normalize::normalize(hl7.as_bytes(), true).unwrap();

    // Normalized message should start with canonical delimiters
    let normalized_str = String::from_utf8_lossy(&normalized);
    assert!(normalized_str.starts_with("MSH|^~\\&|"));
}

#[test]
fn test_batch_parsing() {
    // Simple batch with multiple messages
    let batch = "BHS|^~\\&|SendingApp|SendingFac|20250128||Batch001\rMSH|^~\\&|App1|Fac1|App2|Fac2|20250128152312||ADT^A01|MSG001|P|2.5.1\rPID|1||12345||Doe^John\rBTS|1|Batch001\r";

    let result = hl7v2_parser::parse_batch(batch.as_bytes());
    assert!(result.is_ok());

    let batch_result = result.unwrap();
    assert!(batch_result.header.is_some());
    assert!(batch_result.trailer.is_some());
}

#[test]
fn test_validation() {
    let hl7 = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r";
    let message = hl7v2_parser::parse(hl7.as_bytes()).unwrap();

    // Check version
    let version = hl7v2_parser::get(&message, "MSH.12");
    assert_eq!(version, Some("2.5.1"));

    // Check required fields exist
    assert!(hl7v2_parser::get(&message, "MSH.3").is_some());
    assert!(hl7v2_parser::get(&message, "MSH.4").is_some());
    assert!(hl7v2_parser::get(&message, "MSH.5").is_some());
    assert!(hl7v2_parser::get(&message, "MSH.6").is_some());
}
