//! Comprehensive unit tests for hl7v2-batch crate
//!
//! This module contains unit tests for:
//! - Batch parsing (FHS/BHS/FTS/BTS segments)
//! - Batch creation
//! - Batch info extraction
//! - Error handling

use super::*;

// ============================================================================
// BatchType Tests
// ============================================================================

#[test]
fn test_batch_type_single() {
    let batch_type = BatchType::Single;
    assert_eq!(batch_type, BatchType::Single);
    assert_ne!(batch_type, BatchType::File);
}

#[test]
fn test_batch_type_file() {
    let batch_type = BatchType::File;
    assert_eq!(batch_type, BatchType::File);
    assert_ne!(batch_type, BatchType::Single);
}

#[test]
fn test_batch_type_debug() {
    assert!(format!("{:?}", BatchType::Single).contains("Single"));
    assert!(format!("{:?}", BatchType::File).contains("File"));
}

// ============================================================================
// BatchInfo Tests
// ============================================================================

#[test]
fn test_batch_info_default() {
    let info = BatchInfo::default();
    assert_eq!(info.batch_type, BatchType::Single);
    assert!(info.field_separator.is_none());
    assert!(info.encoding_characters.is_none());
    assert!(info.sending_application.is_none());
    assert!(info.sending_facility.is_none());
    assert!(info.receiving_application.is_none());
    assert!(info.receiving_facility.is_none());
    assert!(info.file_creation_time.is_none());
    assert!(info.security.is_none());
    assert!(info.batch_name.is_none());
    assert!(info.batch_comment.is_none());
    assert!(info.message_count.is_none());
    assert!(info.trailer_comment.is_none());
}

#[test]
fn test_batch_info_with_fields() {
    let info = BatchInfo {
        batch_type: BatchType::File,
        field_separator: Some('|'),
        encoding_characters: Some("^~\\&".to_string()),
        sending_application: Some("App".to_string()),
        sending_facility: Some("Fac".to_string()),
        receiving_application: Some("RecvApp".to_string()),
        receiving_facility: Some("RecvFac".to_string()),
        file_creation_time: Some("20250128152312".to_string()),
        security: Some("SEC".to_string()),
        batch_name: Some("Batch1".to_string()),
        batch_comment: Some("Comment".to_string()),
        message_count: Some(5),
        trailer_comment: Some("Trailer".to_string()),
    };
    
    assert_eq!(info.batch_type, BatchType::File);
    assert_eq!(info.field_separator, Some('|'));
    assert_eq!(info.encoding_characters, Some("^~\\&".to_string()));
    assert_eq!(info.sending_application, Some("App".to_string()));
    assert_eq!(info.message_count, Some(5));
}

// ============================================================================
// Batch Tests
// ============================================================================

#[test]
fn test_batch_new() {
    let batch = Batch::new();
    assert!(batch.header.is_none());
    assert!(batch.messages.is_empty());
    assert!(batch.trailer.is_none());
    assert_eq!(batch.message_count(), 0);
}

#[test]
fn test_batch_default() {
    let batch = Batch::default();
    assert!(batch.header.is_none());
    assert!(batch.messages.is_empty());
}

#[test]
fn test_batch_add_message() {
    let mut batch = Batch::new();
    
    let message = hl7v2_parser::parse(
        b"MSH|^~\\&|App|Fac|Recv|RecvFac|20250128152312||ADT^A01|MSG1|P|2.5.1\r"
    ).unwrap();
    
    batch.add_message(message);
    assert_eq!(batch.message_count(), 1);
}

#[test]
fn test_batch_add_multiple_messages() {
    let mut batch = Batch::new();
    
    for i in 0..5 {
        let msg_text = format!(
            "MSH|^~\\&|App|Fac|Recv|RecvFac|20250128152312||ADT^A01|MSG{}|P|2.5.1\r",
            i
        );
        let message = hl7v2_parser::parse(msg_text.as_bytes()).unwrap();
        batch.add_message(message);
    }
    
    assert_eq!(batch.message_count(), 5);
}

#[test]
fn test_batch_iter_messages() {
    let mut batch = Batch::new();
    
    for i in 0..3 {
        let msg_text = format!(
            "MSH|^~\\&|App|Fac|Recv|RecvFac|20250128152312||ADT^A01|MSG{}|P|2.5.1\r",
            i
        );
        let message = hl7v2_parser::parse(msg_text.as_bytes()).unwrap();
        batch.add_message(message);
    }
    
    let count = batch.iter_messages().count();
    assert_eq!(count, 3);
}

// ============================================================================
// FileBatch Tests
// ============================================================================

#[test]
fn test_file_batch_new() {
    let file_batch = FileBatch::new();
    assert!(file_batch.header.is_none());
    assert!(file_batch.batches.is_empty());
    assert!(file_batch.trailer.is_none());
    assert_eq!(file_batch.info.batch_type, BatchType::File);
}

#[test]
fn test_file_batch_default() {
    let file_batch = FileBatch::default();
    assert_eq!(file_batch.info.batch_type, BatchType::File);
}

#[test]
fn test_file_batch_add_batch() {
    let mut file_batch = FileBatch::new();
    let batch = Batch::new();
    
    file_batch.add_batch(batch);
    assert_eq!(file_batch.batches.len(), 1);
}

#[test]
fn test_file_batch_total_message_count() {
    let mut file_batch = FileBatch::new();
    
    // Add first batch with 2 messages
    let mut batch1 = Batch::new();
    let msg = hl7v2_parser::parse(
        b"MSH|^~\\&|App|Fac|Recv|RecvFac|20250128152312||ADT^A01|MSG1|P|2.5.1\r"
    ).unwrap();
    batch1.add_message(msg.clone());
    batch1.add_message(msg);
    file_batch.add_batch(batch1);
    
    // Add second batch with 3 messages
    let mut batch2 = Batch::new();
    let msg = hl7v2_parser::parse(
        b"MSH|^~\\&|App|Fac|Recv|RecvFac|20250128152312||ADT^A01|MSG2|P|2.5.1\r"
    ).unwrap();
    batch2.add_message(msg.clone());
    batch2.add_message(msg.clone());
    batch2.add_message(msg);
    file_batch.add_batch(batch2);
    
    assert_eq!(file_batch.total_message_count(), 5);
}

#[test]
fn test_file_batch_iter_all_messages() {
    let mut file_batch = FileBatch::new();
    
    // Add two batches with messages
    for _ in 0..2 {
        let mut batch = Batch::new();
        let msg = hl7v2_parser::parse(
            b"MSH|^~\\&|App|Fac|Recv|RecvFac|20250128152312||ADT^A01|MSG|P|2.5.1\r"
        ).unwrap();
        batch.add_message(msg);
        file_batch.add_batch(batch);
    }
    
    let count = file_batch.iter_all_messages().count();
    assert_eq!(count, 2);
}

// ============================================================================
// Batch Parsing Tests
// ============================================================================

#[test]
fn test_parse_simple_messages_only() {
    let data = b"MSH|^~\\&|App|Fac|App2|Fac2|20250128||ADT^A01|123|P|2.5\rPID|1||12345\r";
    let result = parse_batch(data).unwrap();
    
    assert_eq!(result.info.batch_type, BatchType::File);
    assert_eq!(result.total_message_count(), 1);
}

#[test]
fn test_parse_single_batch_with_bhs_bts() {
    let data = b"BHS|^~\\&|App|Fac\rMSH|^~\\&|App|Fac|App2|Fac2|20250128||ADT^A01|123|P|2.5\rPID|1||12345\rBTS|1\r";
    let result = parse_batch(data).unwrap();
    
    assert_eq!(result.batches.len(), 1);
    assert_eq!(result.batches[0].message_count(), 1);
    assert!(result.batches[0].header.is_some());
    assert!(result.batches[0].trailer.is_some());
}

#[test]
fn test_parse_file_batch_with_fhs_fts() {
    let data = b"FHS|^~\\&|App|Fac\rBHS|^~\\&|App|Fac\rMSH|^~\\&|App|Fac|App2|Fac2|||ADT^A01|123|P|2.5\rBTS|1\rFTS|1\r";
    let result = parse_batch(data).unwrap();
    
    assert_eq!(result.info.batch_type, BatchType::File);
    assert!(result.header.is_some());
    assert!(result.trailer.is_some());
}

#[test]
fn test_parse_batch_with_multiple_messages() {
    let data = b"BHS|^~\\&|App|Fac\r\
                MSH|^~\\&|App|Fac|App2|Fac2|20250128||ADT^A01|MSG1|P|2.5\rPID|1||12345\r\
                MSH|^~\\&|App|Fac|App2|Fac2|20250128||ADT^A01|MSG2|P|2.5\rPID|1||67890\r\
                BTS|2\r";
    let result = parse_batch(data).unwrap();
    
    assert_eq!(result.batches[0].message_count(), 2);
}

#[test]
fn test_parse_batch_with_nested_batches() {
    let data = b"FHS|^~\\&|App|Fac\r\
                BHS|^~\\&|App|Fac\rMSH|^~\\&|App|Fac|App2|Fac2|||ADT^A01|MSG1|P|2.5\rBTS|1\r\
                BHS|^~\\&|App|Fac\rMSH|^~\\&|App|Fac|App2|Fac2|||ADT^A01|MSG2|P|2.5\rBTS|1\r\
                FTS|2\r";
    let result = parse_batch(data).unwrap();
    
    assert_eq!(result.batches.len(), 2);
    assert_eq!(result.total_message_count(), 2);
}

// ============================================================================
// Batch Info Extraction Tests
// ============================================================================

#[test]
fn test_extract_batch_info_bhs() {
    let line = "BHS|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128|||BatchName|Comment";
    let info = extract_batch_info(line, "BHS").unwrap();
    
    assert_eq!(info.encoding_characters, Some("^~\\&".to_string()));
    assert_eq!(info.sending_application, Some("SendingApp".to_string()));
    assert_eq!(info.sending_facility, Some("SendingFac".to_string()));
    assert_eq!(info.receiving_application, Some("ReceivingApp".to_string()));
    assert_eq!(info.receiving_facility, Some("ReceivingFac".to_string()));
    assert_eq!(info.batch_name, Some("BatchName".to_string()));
    assert_eq!(info.batch_comment, Some("Comment".to_string()));
}

#[test]
fn test_extract_batch_info_fhs() {
    let line = "FHS|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128|||FileName|FileComment";
    let info = extract_batch_info(line, "FHS").unwrap();
    
    assert_eq!(info.sending_application, Some("SendingApp".to_string()));
    assert_eq!(info.sending_facility, Some("SendingFac".to_string()));
}

#[test]
fn test_extract_batch_info_bts() {
    let line = "BTS|5|TrailerComment";
    let info = extract_batch_info(line, "BTS").unwrap();
    
    assert_eq!(info.message_count, Some(5));
    assert_eq!(info.trailer_comment, Some("TrailerComment".to_string()));
}

#[test]
fn test_extract_batch_info_fts() {
    let line = "FTS|10|FileTrailerComment";
    let info = extract_batch_info(line, "FTS").unwrap();
    
    assert_eq!(info.message_count, Some(10));
    assert_eq!(info.trailer_comment, Some("FileTrailerComment".to_string()));
}

#[test]
fn test_extract_batch_info_minimal() {
    let line = "BHS|";
    let info = extract_batch_info(line, "BHS").unwrap();
    
    // With minimal line, encoding_characters will be empty string (Some(""))
    // because the split produces an empty first field
    // This is acceptable behavior
    assert!(info.encoding_characters.is_none() || info.encoding_characters.as_ref().map_or(false, |s| s.is_empty()));
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_parse_empty_data() {
    let result = parse_batch(b"");
    assert!(result.is_err());
}

#[test]
fn test_parse_invalid_utf8() {
    let result = parse_batch(&[0xFF, 0xFE]);
    assert!(result.is_err());
}

#[test]
fn test_parse_unknown_segment() {
    let result = parse_batch(b"XXX|unknown|segment\r");
    assert!(result.is_err());
}

#[test]
fn test_message_count_mismatch() {
    let data = b"BHS|^~\\&|App|Fac\rMSH|^~\\&|App|Fac\rMSH|^~\\&|App|Fac\rBTS|5\r";
    let result = parse_batch(data);
    assert!(matches!(result, Err(BatchError::CountMismatch { .. })));
}

#[test]
fn test_batch_error_display() {
    let error = BatchError::InvalidStructure("test error".to_string());
    assert!(error.to_string().contains("test error"));
    
    let error = BatchError::MissingSegment("MSH".to_string());
    assert!(error.to_string().contains("MSH"));
    
    let error = BatchError::MismatchedHeaders;
    assert!(error.to_string().contains("Mismatched"));
    
    let error = BatchError::CountMismatch { expected: 5, actual: 3 };
    assert!(error.to_string().contains("5"));
    assert!(error.to_string().contains("3"));
}

// ============================================================================
// Segment Parsing Tests
// ============================================================================

#[test]
fn test_parse_segment_bhs() {
    let line = "BHS|^~\\&|App|Fac";
    let segment = parse_segment(line).unwrap();
    
    assert_eq!(&segment.id, b"BHS");
    assert!(!segment.fields.is_empty());
}

#[test]
fn test_parse_segment_fhs() {
    let line = "FHS|^~\\&|App|Fac|RecvApp|RecvFac";
    let segment = parse_segment(line).unwrap();
    
    assert_eq!(&segment.id, b"FHS");
}

#[test]
fn test_parse_segment_bts() {
    let line = "BTS|5";
    let segment = parse_segment(line).unwrap();
    
    assert_eq!(&segment.id, b"BTS");
}

#[test]
fn test_parse_segment_fts() {
    let line = "FTS|10";
    let segment = parse_segment(line).unwrap();
    
    assert_eq!(&segment.id, b"FTS");
}

#[test]
fn test_parse_segment_too_short() {
    let result = parse_segment("AB");
    assert!(result.is_err());
}

// ============================================================================
// Multi-Line Message Tests
// ============================================================================

#[test]
fn test_parse_message_with_multiple_segments() {
    let data = b"MSH|^~\\&|App|Fac|Recv|RecvFac|20250128||ADT^A01|MSG|P|2.5\rPID|1||12345\rPV1|1|I\r";
    let result = parse_batch(data).unwrap();
    
    assert_eq!(result.total_message_count(), 1);
    // The message should have 3 segments (MSH, PID, PV1)
    let msg = &result.batches[0].messages[0];
    assert_eq!(msg.segments.len(), 3);
}

// ============================================================================
// Line Ending Tests
// ============================================================================

#[test]
fn test_parse_with_cr_only() {
    let data = b"MSH|^~\\&|App|Fac|Recv|RecvFac|20250128||ADT^A01|MSG|P|2.5\rPID|1||12345\r";
    let result = parse_batch(data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_with_lf_only() {
    let data = b"MSH|^~\\&|App|Fac|Recv|RecvFac|20250128||ADT^A01|MSG|P|2.5\nPID|1||12345\n";
    let result = parse_batch(data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_with_crlf() {
    let data = b"MSH|^~\\&|App|Fac|Recv|RecvFac|20250128||ADT^A01|MSG|P|2.5\r\nPID|1||12345\r\n";
    let result = parse_batch(data);
    assert!(result.is_ok());
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_parse_batch_with_empty_lines() {
    let data = b"BHS|^~\\&|App|Fac\r\rMSH|^~\\&|App|Fac|App2|Fac2|||ADT^A01|123|P|2.5\r\rBTS|1\r";
    let result = parse_batch(data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_batch_minimal_fhs_fts() {
    let data = b"FHS|\rFTS|";
    let result = parse_batch(data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_batch_with_special_characters() {
    let data = b"MSH|^~\\&|App\\F\\Test|Fac^Name|Recv~App|Fac\\E\\Esc|20250128||ADT^A01|MSG|P|2.5\r";
    let result = parse_batch(data);
    assert!(result.is_ok());
}

#[test]
fn test_messages_without_batch_wrapper() {
    // Messages without BHS/BTS should still be parsed
    let data = b"MSH|^~\\&|App|Fac|Recv|RecvFac|20250128||ADT^A01|MSG1|P|2.5\r\
                 MSH|^~\\&|App|Fac|Recv|RecvFac|20250128||ADT^A01|MSG2|P|2.5\r";
    let result = parse_batch(data).unwrap();
    
    assert_eq!(result.total_message_count(), 2);
}
