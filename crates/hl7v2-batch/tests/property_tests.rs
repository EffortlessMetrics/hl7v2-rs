//! Property-based tests for hl7v2-batch crate using proptest
//!
//! These tests verify batch parsing properties hold for arbitrary inputs.

use hl7v2_batch::{parse_batch, Batch, BatchType};
use hl7v2_parser::parse;
use proptest::prelude::*;

/// Generate a valid application name
fn app_name_strategy() -> impl Strategy<Value = String> {
    "[A-Za-z][A-Za-z0-9_]{0,19}"
}

/// Generate a valid facility name
fn facility_name_strategy() -> impl Strategy<Value = String> {
    "[A-Za-z][A-Za-z0-9_]{0,19}"
}

/// Generate a valid message control ID
fn control_id_strategy() -> impl Strategy<Value = String> {
    "[A-Za-z0-9]{1,20}"
}

/// Generate a valid batch name
fn batch_name_strategy() -> impl Strategy<Value = String> {
    "[A-Za-z][A-Za-z0-9_]{0,29}"
}

proptest! {
    /// Test that parsing valid messages never panics
    #[test]
    fn prop_parse_messages_never_panics(
        send_app in app_name_strategy(),
        send_fac in facility_name_strategy(),
        recv_app in app_name_strategy(),
        recv_fac in facility_name_strategy(),
        ctrl_id in control_id_strategy()
    ) {
        let message = format!(
            "MSH|^~\\&|{}|{}|{}|{}|20250128120000||ADT^A01|{}|P|2.5.1\r",
            send_app, send_fac, recv_app, recv_fac, ctrl_id
        );
        
        let result = parse_batch(message.as_bytes());
        prop_assert!(result.is_ok());
    }
}

proptest! {
    /// Test that batch with valid BHS/BTS never panics
    #[test]
    fn prop_parse_batch_never_panics(
        send_app in app_name_strategy(),
        send_fac in facility_name_strategy(),
        recv_app in app_name_strategy(),
        recv_fac in facility_name_strategy(),
        ctrl_id in control_id_strategy(),
        batch_name in batch_name_strategy()
    ) {
        let batch_data = format!(
            "BHS|^~\\&|{}|{}|{}|{}|20250128120000|||{}\r\
             MSH|^~\\&|{}|{}|{}|{}|20250128120000||ADT^A01|{}|P|2.5.1\r\
             BTS|1\r",
            send_app, send_fac, recv_app, recv_fac, batch_name,
            send_app, send_fac, recv_app, recv_fac, ctrl_id
        );
        
        let result = parse_batch(batch_data.as_bytes());
        prop_assert!(result.is_ok());
    }
}

proptest! {
    /// Test that file batch with valid FHS/FTS never panics
    #[test]
    fn prop_parse_file_batch_never_panics(
        send_app in app_name_strategy(),
        send_fac in facility_name_strategy(),
        recv_app in app_name_strategy(),
        recv_fac in facility_name_strategy(),
        ctrl_id in control_id_strategy()
    ) {
        let batch_data = format!(
            "FHS|^~\\&|{}|{}|{}|{}|20250128120000\r\
             BHS|^~\\&|{}|{}|{}|{}|20250128120000\r\
             MSH|^~\\&|{}|{}|{}|{}|20250128120000||ADT^A01|{}|P|2.5.1\r\
             BTS|1\r\
             FTS|1\r",
            send_app, send_fac, recv_app, recv_fac,
            send_app, send_fac, recv_app, recv_fac,
            send_app, send_fac, recv_app, recv_fac, ctrl_id
        );
        
        let result = parse_batch(batch_data.as_bytes());
        prop_assert!(result.is_ok());
    }
}

proptest! {
    /// Test that message count in batch is correct
    #[test]
    fn prop_batch_message_count_correct(
        ctrl_id1 in control_id_strategy(),
        ctrl_id2 in control_id_strategy()
    ) {
        // Ensure different control IDs
        prop_assume!(ctrl_id1 != ctrl_id2);
        
        let batch_data = format!(
            "BHS|^~\\&|APP|FAC|RECV|RECVFAC|20250128120000\r\
             MSH|^~\\&|APP|FAC|RECV|RECVFAC|20250128120000||ADT^A01|{}|P|2.5.1\r\
             MSH|^~\\&|APP|FAC|RECV|RECVFAC|20250128120000||ADT^A01|{}|P|2.5.1\r\
             BTS|2\r",
            ctrl_id1, ctrl_id2
        );
        
        let result = parse_batch(batch_data.as_bytes()).unwrap();
        prop_assert_eq!(result.total_message_count(), 2);
    }
}

proptest! {
    /// Test that batch preserves application names
    #[test]
    fn prop_batch_preserves_app_names(
        send_app in app_name_strategy(),
        recv_app in app_name_strategy()
    ) {
        let batch_data = format!(
            "BHS|^~\\&|{}|FAC1|{}|FAC2|20250128120000\r\
             MSH|^~\\&|{}|FAC1|{}|FAC2|20250128120000||ADT^A01|MSG|P|2.5.1\r\
             BTS|1\r",
            send_app, recv_app, send_app, recv_app
        );
        
        let result = parse_batch(batch_data.as_bytes()).unwrap();
        prop_assert_eq!(&result.batches[0].info.sending_application, &Some(send_app));
        prop_assert_eq!(&result.batches[0].info.receiving_application, &Some(recv_app));
    }
}

proptest! {
    /// Test that batch preserves facility names
    #[test]
    fn prop_batch_preserves_facility_names(
        send_fac in facility_name_strategy(),
        recv_fac in facility_name_strategy()
    ) {
        let batch_data = format!(
            "BHS|^~\\&|APP1|{}|APP2|{}|20250128120000\r\
             MSH|^~\\&|APP1|{}|APP2|{}|20250128120000||ADT^A01|MSG|P|2.5.1\r\
             BTS|1\r",
            send_fac, recv_fac, send_fac, recv_fac
        );
        
        let result = parse_batch(batch_data.as_bytes()).unwrap();
        prop_assert_eq!(&result.batches[0].info.sending_facility, &Some(send_fac));
        prop_assert_eq!(&result.batches[0].info.receiving_facility, &Some(recv_fac));
    }
}

proptest! {
    /// Test that batch name is preserved
    #[test]
    fn prop_batch_name_preserved(batch_name in batch_name_strategy()) {
        let batch_data = format!(
            "BHS|^~\\&|APP|FAC|RECV|RECVFAC|20250128120000|||{}\r\
             MSH|^~\\&|APP|FAC|RECV|RECVFAC|20250128120000||ADT^A01|MSG|P|2.5.1\r\
             BTS|1\r",
            batch_name
        );
        
        let result = parse_batch(batch_data.as_bytes()).unwrap();
        prop_assert_eq!(&result.batches[0].info.batch_name, &Some(batch_name));
    }
}

proptest! {
    /// Test that creating batches manually works
    #[test]
    fn prop_manual_batch_creation(ctrl_id in control_id_strategy()) {
        let mut batch = Batch::new();
        let msg_text = format!(
            "MSH|^~\\&|APP|FAC|RECV|RECVFAC|20250128120000||ADT^A01|{}|P|2.5.1\r",
            ctrl_id
        );
        
        if let Ok(msg) = parse(msg_text.as_bytes()) {
            batch.add_message(msg);
            prop_assert_eq!(batch.message_count(), 1);
        }
    }
}

proptest! {
    /// Test that file batch type is correct for FHS/FTS
    #[test]
    fn prop_file_batch_type_correct(ctrl_id in control_id_strategy()) {
        let batch_data = format!(
            "FHS|^~\\&|APP|FAC|RECV|RECVFAC|20250128120000\r\
             MSH|^~\\&|APP|FAC|RECV|RECVFAC|20250128120000||ADT^A01|{}|P|2.5.1\r\
             FTS|1\r",
            ctrl_id
        );
        
        let result = parse_batch(batch_data.as_bytes()).unwrap();
        prop_assert_eq!(result.info.batch_type, BatchType::File);
    }
}

proptest! {
    /// Test that multiple messages are correctly counted
    #[test]
    fn prop_multiple_messages_count(
        ctrl_id1 in control_id_strategy(),
        ctrl_id2 in control_id_strategy(),
        ctrl_id3 in control_id_strategy()
    ) {
        // Ensure all control IDs are different
        prop_assume!(ctrl_id1 != ctrl_id2);
        prop_assume!(ctrl_id2 != ctrl_id3);
        prop_assume!(ctrl_id1 != ctrl_id3);
        
        let batch_data = format!(
            "BHS|^~\\&|APP|FAC|RECV|RECVFAC|20250128120000\r\
             MSH|^~\\&|APP|FAC|RECV|RECVFAC|20250128120000||ADT^A01|{}|P|2.5.1\r\
             MSH|^~\\&|APP|FAC|RECV|RECVFAC|20250128120000||ADT^A01|{}|P|2.5.1\r\
             MSH|^~\\&|APP|FAC|RECV|RECVFAC|20250128120000||ADT^A01|{}|P|2.5.1\r\
             BTS|3\r",
            ctrl_id1, ctrl_id2, ctrl_id3
        );
        
        let result = parse_batch(batch_data.as_bytes()).unwrap();
        prop_assert_eq!(result.total_message_count(), 3);
    }
}

#[test]
fn test_empty_data_returns_error() {
    let result = parse_batch(b"");
    assert!(result.is_err());
}

proptest! {
    /// Test that invalid count is detected
    #[test]
    fn prop_invalid_count_detected(ctrl_id in control_id_strategy()) {
        // BTS says 5 messages but only 1 exists
        let batch_data = format!(
            "BHS|^~\\&|APP|FAC|RECV|RECVFAC|20250128120000\r\
             MSH|^~\\&|APP|FAC|RECV|RECVFAC|20250128120000||ADT^A01|{}|P|2.5.1\r\
             BTS|5\r",
            ctrl_id
        );
        
        let result = parse_batch(batch_data.as_bytes());
        prop_assert!(result.is_err());
    }
}
