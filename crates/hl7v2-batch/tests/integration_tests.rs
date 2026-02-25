//! Integration tests for hl7v2-batch crate
//!
//! These tests verify batch parsing and creation works correctly with
//! real-world HL7 batch scenarios.

use hl7v2_batch::{parse_batch, Batch, BatchType, FileBatch, BatchInfo};
use hl7v2_parser::parse;

// ============================================================================
// Real-World Batch File Tests
// ============================================================================

#[test]
fn test_parse_hl7_file_batch() {
    // Typical file batch with FHS/FTS
    let batch_data = b"FHS|^~\\&|HIS|HOSPITAL|LIS|LABHOST|20250128150000\r\
                       BHS|^~\\&|HIS|HOSPITAL|LIS|LABHOST|20250128150000\r\
                       MSH|^~\\&|HIS|HOSPITAL|LIS|LABHOST|20250128150001||ORU^R01|MSG001|P|2.5.1\r\
                       PID|1||12345^^^HOSP^MR||SMITH^JOHN\r\
                       OBR|1|ORD001||GLUCOSE|||20250128150000\r\
                       OBX|1|NM|GLUCOSE||120|mg/dL|70-110|H|||F\r\
                       MSH|^~\\&|HIS|HOSPITAL|LIS|LABHOST|20250128150002||ORU^R01|MSG002|P|2.5.1\r\
                       PID|1||12346^^^HOSP^MR||DOE^JANE\r\
                       OBR|1|ORD002||GLUCOSE|||20250128150000\r\
                       OBX|1|NM|GLUCOSE||95|mg/dL|70-110|N|||F\r\
                       BTS|2\r\
                       FTS|2\r";
    
    let result = parse_batch(batch_data).unwrap();
    
    assert_eq!(result.info.batch_type, BatchType::File);
    assert_eq!(result.batches.len(), 1);
    assert_eq!(result.total_message_count(), 2);
}

#[test]
fn test_parse_lab_results_batch() {
    // Lab results batch
    let batch_data = b"BHS|^~\\&|LAB|LABHOST|HIS|HISHOST|20250128160000|||LAB_RESULTS_20250128\r\
                       MSH|^~\\&|LAB|LABHOST|HIS|HISHOST|20250128160100||ORU^R01|LAB001|P|2.5.1\r\
                       PID|1||MRN001^^^HOSP^MR||TEST^PATIENT^ONE||19800101|M\r\
                       ORC|RE|ORD001|ORD001|||CM\r\
                       OBR|1|ORD001|ORD001|CBC^Complete Blood Count|||20250128150000\r\
                       OBX|1|NM|WBC^White Blood Cells||7.5|10*3/uL|4.5-11.0|N|||F\r\
                       OBX|2|NM|RBC^Red Blood Cells||5.0|10*6/uL|4.5-5.5|N|||F\r\
                       OBX|3|NM|HGB^Hemoglobin||14.5|g/dL|13.5-17.5|N|||F\r\
                       MSH|^~\\&|LAB|LABHOST|HIS|HISHOST|20250128160200||ORU^R01|LAB002|P|2.5.1\r\
                       PID|1||MRN002^^^HOSP^MR||TEST^PATIENT^TWO||19750101|F\r\
                       ORC|RE|ORD002|ORD002|||CM\r\
                       OBR|1|ORD002|ORD002|CMP^Comprehensive Metabolic Panel|||20250128151000\r\
                       OBX|1|NM|GLUCOSE^Glucose||110|mg/dL|70-110|H|||F\r\
                       OBX|2|NM|SODIUM^Sodium||140|mEq/L|136-145|N|||F\r\
                       BTS|2\r";
    
    let result = parse_batch(batch_data).unwrap();
    
    assert_eq!(result.batches.len(), 1);
    assert_eq!(result.batches[0].message_count(), 2);
    
    // Verify batch info
    assert_eq!(result.batches[0].info.sending_application, Some("LAB".to_string()));
    assert_eq!(result.batches[0].info.batch_name, Some("LAB_RESULTS_20250128".to_string()));
}

#[test]
fn test_parse_adt_batch() {
    // ADT event batch
    let batch_data = b"BHS|^~\\&|ADT|HOSPITAL|HIS|HISHOST|20250128120000\r\
                       MSH|^~\\&|ADT|HOSPITAL|HIS|HISHOST|20250128120100||ADT^A01|ADT001|P|2.5.1\r\
                       EVN|A01|20250128120100\r\
                       PID|1||MRN001^^^HOSP^MR||ADMIT^PATIENT^ONE||19800101|M\r\
                       PV1|1|I|ICU^101^^HOSP||||123456^ATTENDING^DOCTOR^^^MD\r\
                       MSH|^~\\&|ADT|HOSPITAL|HIS|HISHOST|20250128120200||ADT^A03|ADT002|P|2.5.1\r\
                       EVN|A03|20250128120200\r\
                       PID|1||MRN002^^^HOSP^MR||DISCHARGE^PATIENT^TWO||19750101|F\r\
                       PV1|1|I|MED^202^^HOSP||||789012^ATTENDING^DOCTOR^^^MD\r\
                       MSH|^~\\&|ADT|HOSPITAL|HIS|HISHOST|20250128120300||ADT^A08|ADT003|P|2.5.1\r\
                       EVN|A08|20250128120300\r\
                       PID|1||MRN003^^^HOSP^MR||UPDATE^PATIENT^THREE||19900101|M\r\
                       PV1|1|O|ER^^^HOSP||||345678^ER^DOCTOR^^^MD\r\
                       BTS|3\r";
    
    let result = parse_batch(batch_data).unwrap();
    
    assert_eq!(result.total_message_count(), 3);
}

// ============================================================================
// Multi-Batch File Tests
// ============================================================================

#[test]
fn test_parse_multi_batch_file() {
    // File with multiple batches
    let batch_data = b"FHS|^~\\&|HIS|HOSPITAL|||20250128120000\r\
                       BHS|^~\\&|HIS|HOSPITAL|LAB|LABHOST|20250128120000|||LAB_BATCH\r\
                       MSH|^~\\&|HIS|HOSPITAL|LAB|LABHOST|20250128120100||ORM^O01|ORD001|P|2.5.1\r\
                       PID|1||MRN001^^^HOSP^MR||PATIENT^ONE\r\
                       ORC|NW|ORD001\r\
                       OBR|1|ORD001||CBC\r\
                       MSH|^~\\&|HIS|HOSPITAL|LAB|LABHOST|20250128120200||ORM^O01|ORD002|P|2.5.1\r\
                       PID|1||MRN002^^^HOSP^MR||PATIENT^TWO\r\
                       ORC|NW|ORD002\r\
                       OBR|1|ORD002||CMP\r\
                       BTS|2\r\
                       BHS|^~\\&|HIS|HOSPITAL|RAD|RADHOST|20250128130000|||RAD_BATCH\r\
                       MSH|^~\\&|HIS|HOSPITAL|RAD|RADHOST|20250128130100||ORM^O01|RAD001|P|2.5.1\r\
                       PID|1||MRN003^^^HOSP^MR||PATIENT^THREE\r\
                       ORC|NW|RAD001\r\
                       OBR|1|RAD001||XRAY_CHEST\r\
                       BTS|1\r\
                       FTS|3\r";
    
    let result = parse_batch(batch_data).unwrap();
    
    assert_eq!(result.batches.len(), 2);
    assert_eq!(result.total_message_count(), 3);
    
    // Verify first batch
    assert_eq!(result.batches[0].info.batch_name, Some("LAB_BATCH".to_string()));
    assert_eq!(result.batches[0].message_count(), 2);
    
    // Verify second batch
    assert_eq!(result.batches[1].info.batch_name, Some("RAD_BATCH".to_string()));
    assert_eq!(result.batches[1].message_count(), 1);
}

// ============================================================================
// Message-Only Batch Tests
// ============================================================================

#[test]
fn test_parse_messages_without_batch_headers() {
    // Messages without BHS/BTS
    let messages_data = b"MSH|^~\\&|APP|FAC|RECV|RECVFAC|20250128120000||ADT^A01|MSG001|P|2.5.1\r\
                          PID|1||MRN001^^^HOSP^MR||PATIENT^ONE\r\
                          MSH|^~\\&|APP|FAC|RECV|RECVFAC|20250128120100||ADT^A01|MSG002|P|2.5.1\r\
                          PID|1||MRN002^^^HOSP^MR||PATIENT^TWO\r\
                          MSH|^~\\&|APP|FAC|RECV|RECVFAC|20250128120200||ADT^A01|MSG003|P|2.5.1\r\
                          PID|1||MRN003^^^HOSP^MR||PATIENT^THREE\r";
    
    let result = parse_batch(messages_data).unwrap();
    
    assert_eq!(result.total_message_count(), 3);
}

// ============================================================================
// Batch Creation Tests
// ============================================================================

#[test]
fn test_create_batch_from_messages() {
    let mut batch = Batch::new();
    
    // Create and add messages
    let msg1 = parse(b"MSH|^~\\&|APP|FAC|RECV|RECVFAC|20250128120000||ADT^A01|MSG001|P|2.5.1\rPID|1||12345\r").unwrap();
    let msg2 = parse(b"MSH|^~\\&|APP|FAC|RECV|RECVFAC|20250128120100||ADT^A01|MSG002|P|2.5.1\rPID|1||67890\r").unwrap();
    
    batch.add_message(msg1);
    batch.add_message(msg2);
    
    assert_eq!(batch.message_count(), 2);
}

#[test]
fn test_create_file_batch_from_batches() {
    let mut file_batch = FileBatch::new();
    
    // Create first batch
    let mut batch1 = Batch::new();
    let msg = parse(b"MSH|^~\\&|APP|FAC|RECV|RECVFAC|20250128120000||ADT^A01|MSG001|P|2.5.1\r").unwrap();
    batch1.add_message(msg);
    
    // Create second batch
    let mut batch2 = Batch::new();
    let msg = parse(b"MSH|^~\\&|APP|FAC|RECV|RECVFAC|20250128120100||ADT^A01|MSG002|P|2.5.1\r").unwrap();
    batch2.add_message(msg);
    
    file_batch.add_batch(batch1);
    file_batch.add_batch(batch2);
    
    assert_eq!(file_batch.batches.len(), 2);
    assert_eq!(file_batch.total_message_count(), 2);
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_empty_batch() {
    let batch = Batch::new();
    assert_eq!(batch.message_count(), 0);
    assert!(batch.iter_messages().next().is_none());
}

#[test]
fn test_empty_file_batch() {
    let file_batch = FileBatch::new();
    assert_eq!(file_batch.total_message_count(), 0);
    assert!(file_batch.iter_all_messages().next().is_none());
}

#[test]
fn test_batch_with_single_message() {
    let data = b"MSH|^~\\&|APP|FAC|RECV|RECVFAC|20250128120000||ADT^A01|MSG001|P|2.5.1\r";
    let result = parse_batch(data).unwrap();
    
    assert_eq!(result.total_message_count(), 1);
}

#[test]
fn test_batch_with_large_message_count() {
    let mut batch_data = Vec::new();
    batch_data.extend_from_slice(b"BHS|^~\\&|APP|FAC|RECV|RECVFAC|20250128120000\r");
    
    for i in 0..100 {
        let msg = format!("MSH|^~\\&|APP|FAC|RECV|RECVFAC|20250128120000||ADT^A01|MSG{:03}|P|2.5.1\r", i);
        batch_data.extend_from_slice(msg.as_bytes());
    }
    
    batch_data.extend_from_slice(b"BTS|100\r");
    
    let result = parse_batch(&batch_data).unwrap();
    assert_eq!(result.total_message_count(), 100);
}

// ============================================================================
// Batch Info Extraction Tests
// ============================================================================

#[test]
fn test_file_batch_info_extraction() {
    let data = b"FHS|^~\\&|SENDING_APP|SENDING_FAC|RECEIVING_APP|RECEIVING_FAC|20250128150000|||FILE_NAME|FILE_COMMENT\r\
                 BHS|^~\\&|SENDING_APP|SENDING_FAC|RECEIVING_APP|RECEIVING_FAC|20250128150000\r\
                 MSH|^~\\&|SENDING_APP|SENDING_FAC|RECEIVING_APP|RECEIVING_FAC|20250128150000||ADT^A01|MSG001|P|2.5.1\r\
                 BTS|1\r\
                 FTS|1|TRAILER_COMMENT\r";
    
    let result = parse_batch(data).unwrap();
    
    assert_eq!(result.info.sending_application, Some("SENDING_APP".to_string()));
    assert_eq!(result.info.sending_facility, Some("SENDING_FAC".to_string()));
    assert_eq!(result.info.receiving_application, Some("RECEIVING_APP".to_string()));
    assert_eq!(result.info.receiving_facility, Some("RECEIVING_FAC".to_string()));
    assert_eq!(result.info.batch_name, Some("FILE_NAME".to_string()));
    assert_eq!(result.info.batch_comment, Some("FILE_COMMENT".to_string()));
    assert_eq!(result.info.message_count, Some(1));
    assert_eq!(result.info.trailer_comment, Some("TRAILER_COMMENT".to_string()));
}

// ============================================================================
// Complex Message Structure Tests
// ============================================================================

#[test]
fn test_batch_with_complex_messages() {
    let data = b"BHS|^~\\&|APP|FAC|RECV|RECVFAC|20250128120000\r\
                 MSH|^~\\&|APP|FAC|RECV|RECVFAC|20250128120100||ADT^A01^ADT_A01|MSG001|P|2.5.1\r\
                 PID|1||12345^^^HOSP^MR||SMITH^JOHN^Q||19650101|M|||123 MAIN ST^^ANYTOWN^CA^12345||(555)123-4567|||M||123456789\r\
                 PD1|||HOSP||||\r\
                 PV1|1|I|ICU^101^^HOSP||||123456^DOCTOR^JANE^A^^MD||||||||ADM|A0|||||||||||||||||||HOSP||20250128100000|||\r\
                 PV2||||||||||||||||||||||||\r\
                 DB1|1||123456789|||\r\
                 OBX|1|ST|NOTE^ADMISSION NOTE||Patient admitted for observation.||||||F\r\
                 MSH|^~\\&|APP|FAC|RECV|RECVFAC|20250128120200||ADT^A01^ADT_A01|MSG002|P|2.5.1\r\
                 PID|1||67890^^^HOSP^MR||DOE^JANE||19750101|F\r\
                 PV1|1|I|MED^202^^HOSP||||789012^DOCTOR^JOHN^B^^MD\r\
                 BTS|2\r";
    
    let result = parse_batch(data).unwrap();
    
    assert_eq!(result.total_message_count(), 2);
    
    // Verify first message has multiple segments
    let msg1 = &result.batches[0].messages[0];
    assert!(msg1.segments.len() >= 5);
}

// ============================================================================
// Iterator Tests
// ============================================================================

#[test]
fn test_batch_iterator() {
    let mut batch = Batch::new();
    
    for i in 0..5 {
        let msg = parse(format!("MSH|^~\\&|APP|FAC|RECV|RECVFAC|20250128120000||ADT^A01|MSG{}|P|2.5.1\r", i).as_bytes()).unwrap();
        batch.add_message(msg);
    }
    
    let mut count = 0;
    for msg in batch.iter_messages() {
        assert_eq!(&msg.segments[0].id, b"MSH");
        count += 1;
    }
    assert_eq!(count, 5);
}

#[test]
fn test_file_batch_iterator() {
    let mut file_batch = FileBatch::new();
    
    // Add two batches
    for _ in 0..2 {
        let mut batch = Batch::new();
        let msg = parse(b"MSH|^~\\&|APP|FAC|RECV|RECVFAC|20250128120000||ADT^A01|MSG|P|2.5.1\r").unwrap();
        batch.add_message(msg);
        file_batch.add_batch(batch);
    }
    
    let mut count = 0;
    for _ in file_batch.iter_all_messages() {
        count += 1;
    }
    assert_eq!(count, 2);
}

// ============================================================================
// Different Delimiter Tests
// ============================================================================

#[test]
fn test_batch_with_custom_delimiters() {
    // Using # as field separator, : as component, @ as repetition, * as escape, % as subcomponent
    let data = b"FHS#:@*%APP#FAC#RECV#RECVFAC#20250128120000\r\
                 BHS#:@*%APP#FAC#RECV#RECVFAC#20250128120000\r\
                 MSH#:@*%APP#FAC#RECV#RECVFAC#20250128120000##ADT:A01#MSG001#P#2.5.1\r\
                 BTS#1\r\
                 FTS#1\r";
    
    let result = parse_batch(data);
    assert!(result.is_ok());
}

// ============================================================================
// Validation Tests
// ============================================================================

#[test]
fn test_message_count_validation() {
    // Correct count
    let valid_data = b"BHS|^~\\&|APP|FAC\rMSH|^~\\&|APP|FAC|RECV|RECVFAC|||ADT^A01|MSG|P|2.5.1\rBTS|1\r";
    let valid_result = parse_batch(valid_data);
    assert!(valid_result.is_ok());
    
    // Incorrect count
    let invalid_data = b"BHS|^~\\&|APP|FAC\rMSH|^~\\&|APP|FAC|RECV|RECVFAC|||ADT^A01|MSG|P|2.5.1\rBTS|5\r";
    let invalid_result = parse_batch(invalid_data);
    assert!(invalid_result.is_err());
}
