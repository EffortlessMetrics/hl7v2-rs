//! Property-based tests for hl7v2-ack crate using proptest
//!
//! These tests verify that ACK generation properties hold for
//! arbitrary input messages.

use hl7v2_ack::{ack, ack_with_error, AckCode};
use hl7v2_core::parse;
use proptest::prelude::*;

/// Generate a valid application name (alphanumeric with underscores)
fn app_name_strategy() -> impl Strategy<Value = String> {
    "[A-Za-z][A-Za-z0-9_]{0,19}"
}

/// Generate a valid facility name
fn facility_name_strategy() -> impl Strategy<Value = String> {
    "[A-Za-z][A-Za-z0-9_]{0,19}"
}

/// Generate a valid message control ID
fn control_id_strategy() -> impl Strategy<Value = String> {
    "[A-Za-z0-9]{1,50}"
}

/// Generate a valid HL7 version
fn version_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("2.3".to_string()),
        Just("2.3.1".to_string()),
        Just("2.4".to_string()),
        Just("2.5".to_string()),
        Just("2.5.1".to_string()),
        Just("2.6".to_string()),
        Just("2.7".to_string()),
        Just("2.8".to_string()),
    ]
}

/// Generate a valid processing ID
fn processing_id_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("P".to_string()),
        Just("T".to_string()),
        Just("D".to_string()),
    ]
}

/// Generate a basic MSH segment string
fn msh_segment_strategy() -> impl Strategy<Value = String> {
    (app_name_strategy(), facility_name_strategy(), app_name_strategy(), facility_name_strategy(),
     control_id_strategy(), version_strategy(), processing_id_strategy())
        .prop_map(|(send_app, send_fac, recv_app, recv_fac, ctrl_id, version, proc_id)| {
            format!(
                "MSH|^~\\&|{}|{}|{}|{}|20250128150000||ADT^A01|{}|{}|{}",
                send_app, send_fac, recv_app, recv_fac, ctrl_id, proc_id, version
            )
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    /// Test that ACK generation never panics for valid messages
    #[test]
    fn prop_ack_never_panics(
        send_app in app_name_strategy(),
        send_fac in facility_name_strategy(),
        recv_app in app_name_strategy(),
        recv_fac in facility_name_strategy(),
        ctrl_id in control_id_strategy(),
        version in version_strategy(),
        proc_id in processing_id_strategy()
    ) {
        let message_str = format!(
            "MSH|^~\\&|{}|{}|{}|{}|20250128150000||ADT^A01|{}|{}|{}\r",
            send_app, send_fac, recv_app, recv_fac, ctrl_id, proc_id, version
        );
        
        if let Ok(original) = parse(message_str.as_bytes()) {
            let ack_result = ack(&original, AckCode::AA);
            prop_assert!(ack_result.is_ok());
        }
    }
    
    /// Test that ACK always has exactly 2 segments (MSH and MSA)
    #[test]
    fn prop_ack_has_two_segments(
        send_app in app_name_strategy(),
        send_fac in facility_name_strategy(),
        recv_app in app_name_strategy(),
        recv_fac in facility_name_strategy(),
        ctrl_id in control_id_strategy()
    ) {
        let message_str = format!(
            "MSH|^~\\&|{}|{}|{}|{}|20250128150000||ADT^A01|{}|P|2.5.1\r",
            send_app, send_fac, recv_app, recv_fac, ctrl_id
        );
        
        if let Ok(original) = parse(message_str.as_bytes()) {
            if let Ok(ack_msg) = ack(&original, AckCode::AA) {
                prop_assert_eq!(ack_msg.segments.len(), 2);
            }
        }
    }
    
    /// Test that ACK with error has exactly 3 segments
    #[test]
    fn prop_ack_with_error_has_three_segments(
        send_app in app_name_strategy(),
        send_fac in facility_name_strategy(),
        recv_app in app_name_strategy(),
        recv_fac in facility_name_strategy(),
        ctrl_id in control_id_strategy(),
        error_msg in ".{1,100}"
    ) {
        let message_str = format!(
            "MSH|^~\\&|{}|{}|{}|{}|20250128150000||ADT^A01|{}|P|2.5.1\r",
            send_app, send_fac, recv_app, recv_fac, ctrl_id
        );
        
        if let Ok(original) = parse(message_str.as_bytes()) {
            if let Ok(ack_msg) = ack_with_error(&original, AckCode::AE, Some(&error_msg)) {
                prop_assert_eq!(ack_msg.segments.len(), 3);
            }
        }
    }
    
    /// Test that ACK preserves control ID
    #[test]
    fn prop_ack_preserves_control_id(ctrl_id in control_id_strategy()) {
        let message_str = format!(
            "MSH|^~\\&|AppA|FacA|AppB|FacB|20250128150000||ADT^A01|{}|P|2.5.1\r",
            ctrl_id
        );
        
        if let Ok(original) = parse(message_str.as_bytes()) {
            if let Ok(ack_msg) = ack(&original, AckCode::AA) {
                let msa = &ack_msg.segments[1];
                if let Some(ack_control_id) = get_field_value(msa, 2) {
                    prop_assert_eq!(ack_control_id, ctrl_id);
                }
            }
        }
    }
    
    /// Test that ACK swaps sending and receiving applications
    #[test]
    fn prop_ack_swaps_applications(
        send_app in app_name_strategy(),
        recv_app in app_name_strategy()
    ) {
        let message_str = format!(
            "MSH|^~\\&|{}|FacA|{}|FacB|20250128150000||ADT^A01|MSG123|P|2.5.1\r",
            send_app, recv_app
        );
        
        if let Ok(original) = parse(message_str.as_bytes()) {
            if let Ok(ack_msg) = ack(&original, AckCode::AA) {
                let ack_msh = &ack_msg.segments[0];
                if let Some(ack_send_app) = get_field_value(ack_msh, 2) {
                    // ACK sending app should be original receiving app
                    prop_assert_eq!(ack_send_app, recv_app);
                }
                if let Some(ack_recv_app) = get_field_value(ack_msh, 4) {
                    // ACK receiving app should be original sending app
                    prop_assert_eq!(ack_recv_app, send_app);
                }
            }
        }
    }
    
    /// Test that ACK preserves version
    #[test]
    fn prop_ack_preserves_version(version in version_strategy()) {
        let message_str = format!(
            "MSH|^~\\&|AppA|FacA|AppB|FacB|20250128150000||ADT^A01|MSG123|P|{}\r",
            version
        );
        
        if let Ok(original) = parse(message_str.as_bytes()) {
            if let Ok(ack_msg) = ack(&original, AckCode::AA) {
                let ack_msh = &ack_msg.segments[0];
                if let Some(ack_version) = get_field_value(ack_msh, 11) {
                    prop_assert_eq!(ack_version, version);
                }
            }
        }
    }
    
    /// Test that ACK preserves processing ID
    #[test]
    fn prop_ack_preserves_processing_id(proc_id in processing_id_strategy()) {
        let message_str = format!(
            "MSH|^~\\&|AppA|FacA|AppB|FacB|20250128150000||ADT^A01|MSG123|{}|2.5.1\r",
            proc_id
        );
        
        if let Ok(original) = parse(message_str.as_bytes()) {
            if let Ok(ack_msg) = ack(&original, AckCode::AA) {
                let ack_msh = &ack_msg.segments[0];
                if let Some(ack_proc_id) = get_field_value(ack_msh, 10) {
                    prop_assert_eq!(ack_proc_id, proc_id);
                }
            }
        }
    }
    
    /// Test all ACK codes work correctly
    #[test]
    fn prop_all_ack_codes_work(
        send_app in app_name_strategy(),
        ctrl_id in control_id_strategy(),
        ack_code in 0u8..6
    ) {
        let code = match ack_code {
            0 => AckCode::AA,
            1 => AckCode::AE,
            2 => AckCode::AR,
            3 => AckCode::CA,
            4 => AckCode::CE,
            _ => AckCode::CR,
        };
        
        let message_str = format!(
            "MSH|^~\\&|{}|FacA|AppB|FacB|20250128150000||ADT^A01|{}|P|2.5.1\r",
            send_app, ctrl_id
        );
        
        if let Ok(original) = parse(message_str.as_bytes()) {
            if let Ok(ack_msg) = ack(&original, code) {
                let msa = &ack_msg.segments[1];
                if let Some(ack_code_value) = get_field_value(msa, 1) {
                    prop_assert_eq!(ack_code_value, code.as_str());
                }
            }
        }
    }
    
    /// Test that ACK preserves delimiters
    #[test]
    fn prop_ack_preserves_delimiters(ctrl_id in control_id_strategy()) {
        let message_str = format!(
            "MSH|^~\\&|AppA|FacA|AppB|FacB|20250128150000||ADT^A01|{}|P|2.5.1\r",
            ctrl_id
        );
        
        if let Ok(original) = parse(message_str.as_bytes()) {
            if let Ok(ack_msg) = ack(&original, AckCode::AA) {
                prop_assert_eq!(ack_msg.delims.field, original.delims.field);
                prop_assert_eq!(ack_msg.delims.comp, original.delims.comp);
                prop_assert_eq!(ack_msg.delims.rep, original.delims.rep);
                prop_assert_eq!(ack_msg.delims.esc, original.delims.esc);
                prop_assert_eq!(ack_msg.delims.sub, original.delims.sub);
            }
        }
    }
}

/// Helper function to extract field value from a segment
fn get_field_value(segment: &hl7v2_core::Segment, field_index: usize) -> Option<String> {
    if field_index > segment.fields.len() {
        return None;
    }
    
    let field = &segment.fields[field_index - 1];
    if field.reps.is_empty() {
        return None;
    }
    
    let rep = &field.reps[0];
    if rep.comps.is_empty() {
        return None;
    }
    
    let comp = &rep.comps[0];
    if comp.subs.is_empty() {
        return None;
    }
    
    match &comp.subs[0] {
        hl7v2_core::Atom::Text(text) => Some(text.clone()),
        hl7v2_core::Atom::Null => None,
    }
}
