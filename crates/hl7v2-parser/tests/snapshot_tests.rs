//! Snapshot tests for the hl7v2-parser crate using insta.
//!
//! Snapshot tests capture the parsed structure of HL7 messages and
//! compare them against reference snapshots. This helps detect
//! unexpected changes in parsing behavior.

use hl7v2_parser::{parse, parse_batch};
use hl7v2_test_utils::SampleMessages;
use insta::{assert_debug_snapshot, assert_yaml_snapshot};

// =============================================================================
// Standard Message Snapshots
// =============================================================================

#[test]
fn snapshot_adt_a01_message() {
    let hl7 = SampleMessages::adt_a01();
    let message = parse(hl7.as_bytes()).unwrap();

    // Snapshot the parsed message structure
    assert_yaml_snapshot!("adt_a01_message", message);
}

#[test]
fn snapshot_adt_a04_message() {
    let hl7 = SampleMessages::adt_a04();
    let message = parse(hl7.as_bytes()).unwrap();

    assert_yaml_snapshot!("adt_a04_message", message);
}

#[test]
fn snapshot_oru_r01_message() {
    let hl7 = SampleMessages::oru_r01();
    let message = parse(hl7.as_bytes()).unwrap();

    assert_yaml_snapshot!("oru_r01_message", message);
}

// =============================================================================
// Edge Case Snapshots
// =============================================================================

#[test]
fn snapshot_empty_fields_message() {
    let hl7 = SampleMessages::edge_case("empty_fields").unwrap();
    let message = parse(hl7.as_bytes()).unwrap();

    assert_yaml_snapshot!("empty_fields_message", message);
}

#[test]
fn snapshot_special_chars_message() {
    let hl7 = SampleMessages::edge_case("special_chars").unwrap();
    let message = parse(hl7.as_bytes()).unwrap();

    assert_yaml_snapshot!("special_chars_message", message);
}

#[test]
fn snapshot_custom_delims_message() {
    let hl7 = SampleMessages::edge_case("custom_delims").unwrap();
    let message = parse(hl7.as_bytes()).unwrap();

    assert_yaml_snapshot!("custom_delims_message", message);
}

#[test]
fn snapshot_repetitions_message() {
    let hl7 = SampleMessages::edge_case("with_repetitions").unwrap();
    let message = parse(hl7.as_bytes()).unwrap();

    assert_yaml_snapshot!("repetitions_message", message);
}

#[test]
fn snapshot_fully_populated_message() {
    let hl7 = SampleMessages::edge_case("fully_populated").unwrap();
    let message = parse(hl7.as_bytes()).unwrap();

    assert_yaml_snapshot!("fully_populated_message", message);
}

// =============================================================================
// Error Message Snapshots
// =============================================================================

#[test]
fn snapshot_truncated_error() {
    let hl7 = SampleMessages::invalid("truncated").unwrap();
    let result = parse(hl7.as_bytes());

    assert_debug_snapshot!("truncated_error", result);
}

#[test]
fn snapshot_no_msh_error() {
    let hl7 = SampleMessages::invalid("no_msh").unwrap();
    let result = parse(hl7.as_bytes());

    assert_debug_snapshot!("no_msh_error", result);
}

#[test]
fn snapshot_bad_encoding_error() {
    let hl7 = SampleMessages::invalid("bad_encoding").unwrap();
    let result = parse(hl7.as_bytes());

    assert_debug_snapshot!("bad_encoding_error", result);
}

// =============================================================================
// Segment Snapshots
// =============================================================================

#[test]
fn snapshot_msh_segment() {
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1|||AL|NE|ASCII\r";
    let message = parse(hl7.as_slice()).unwrap();

    // Snapshot just the MSH segment
    assert_yaml_snapshot!("msh_segment", &message.segments[0]);
}

#[test]
fn snapshot_pid_segment() {
    let hl7 = b"MSH|^~\\&|App|Fac|Recv|RecvFac|20250128120000||ADT^A01|MSG123|P|2.5\rPID|1||123456^^^HOSP^MR||Doe^John^A||19800101|M|||C|\r";
    let message = parse(hl7.as_slice()).unwrap();

    // Snapshot just the PID segment
    assert_yaml_snapshot!("pid_segment", &message.segments[1]);
}

#[test]
fn snapshot_pv1_segment() {
    let hl7 = b"MSH|^~\\&|App|Fac|Recv|RecvFac|20250128120000||ADT^A01|MSG123|P|2.5\rPV1|1|I|ICU^101^01||||DOC123^Smith^Jane||||||||V123456\r";
    let message = parse(hl7.as_slice()).unwrap();

    // Snapshot just the PV1 segment
    assert_yaml_snapshot!("pv1_segment", &message.segments[1]);
}

#[test]
fn snapshot_obx_segment() {
    let hl7 = b"MSH|^~\\&|App|Fac|Recv|RecvFac|20250128120000||ADT^A01|MSG123|P|2.5\rOBX|1|NM|WBC^White Blood Count||7.5|10^9/L|4.0-11.0|N|||F\r";
    let message = parse(hl7.as_slice()).unwrap();

    // Snapshot just the OBX segment
    assert_yaml_snapshot!("obx_segment", &message.segments[1]);
}

// =============================================================================
// Complex Structure Snapshots
// =============================================================================

#[test]
fn snapshot_complex_field_structure() {
    // A message with complex field structures
    let hl7 = b"MSH|^~\\&|App|Fac|Recv|RecvFac|20250128120000||ADT^A01|MSG123|P|2.5\rPID|1||123&MR&HOSP^^^HOSP^MR||Doe^John^Adam^III^Sr.~Smith^Jane^Marie^Jr.^Dr.||19800101|M\r";
    let message = parse(hl7.as_slice()).unwrap();

    assert_yaml_snapshot!("complex_field_structure", message);
}

#[test]
fn snapshot_batch_structure() {
    let batch = concat!(
        "BHS|^~\\&|App|Fac|||20250128120000\r",
        "MSH|^~\\&|App|Fac|Recv|RecvFac|20250128120000||ADT^A01|MSG1|P|2.5\r",
        "PID|1||123||Patient1\r",
        "MSH|^~\\&|App|Fac|Recv|RecvFac|20250128120001||ADT^A01|MSG2|P|2.5\r",
        "PID|1||456||Patient2\r",
        "BTS|2\r"
    );

    let parsed_batch = parse_batch(batch.as_bytes()).unwrap();

    assert_yaml_snapshot!("batch_structure", parsed_batch);
}

// =============================================================================
// Delimiter Snapshots
// =============================================================================

#[test]
fn snapshot_standard_delimiters() {
    let delims = hl7v2_model::Delims::default();

    assert_yaml_snapshot!("standard_delimiters", delims);
}

#[test]
fn snapshot_custom_delimiters() {
    let hl7 = b"MSH#$*@!App#Fac#Rec#RecFac#20250128120000##ADT$A01#1#P#2.5\r";
    let message = parse(hl7.as_slice()).unwrap();

    assert_yaml_snapshot!("custom_delimiters", message.delims);
}

// =============================================================================
// Real-World Message Snapshots
// =============================================================================

#[test]
fn snapshot_real_world_adt() {
    let hl7 = concat!(
        "MSH|^~\\&|ADT|GOOD_HEALTH_HOSPITAL|PACS|IMAGE_ARCHIVE|20250128152312-0500||ADT^A01|MSG00001|P|2.5.1|||||ASCII\r",
        "EVN|A01|20250128152312-0500|20250128160000||JOHNSON^MIKE^A^^DR^^MD^^&MD&&PHG^^^^PHYS\r",
        "PID|1||PATID5421^^^GOOD_HEALTH_HOSPITAL^MR||TEST^PATIENT^A||19550505|M||C|12345 MAIN ST^^NEW YORK^NY^10001^USA||(212)555-1212|(212)555-1234||E|S||123456789|987654^NC||\r",
        "PV1|1|I|ICU^101^01^GOOD_HEALTH_HOSPITAL^^^^GH||||JOHNSON^MIKE^A^^DR^^MD^^&MD&&PHG^^^^PHYS||||||||ADMITTED|||||||||||||||||||||||||20250128152312-0500\r"
    );

    let message = parse(hl7.as_bytes()).unwrap();

    assert_yaml_snapshot!("real_world_adt", message);
}

#[test]
fn snapshot_real_world_oru() {
    let hl7 = concat!(
        "MSH|^~\\&|LAB|GOOD_HEALTH_HOSPITAL|HIS|GOOD_HEALTH_HOSPITAL|20250128150000||ORU^R01|LAB00001|P|2.5.1|||||ASCII\r",
        "PID|1||PATID5421^^^GOOD_HEALTH_HOSPITAL^MR||TEST^PATIENT^A||19550505|M||C|12345 MAIN ST^^NEW YORK^NY^10001^USA||(212)555-1212\r",
        "OBR|1|ORDER0001|RESULT0001|CBC^COMPLETE BLOOD COUNT^L|||20250128120000|||||||||||||||F\r",
        "OBX|1|NM|WBC^WHITE BLOOD COUNT^L||7.5|10*9/L|4.0-11.0|N|||F|||20250128150000\r",
        "OBX|2|NM|RBC^RED BLOOD COUNT^L||4.5|10*12/L|4.0-5.5|N|||F|||20250128150000\r"
    );

    let message = parse(hl7.as_bytes()).unwrap();

    assert_yaml_snapshot!("real_world_oru", message);
}

// =============================================================================
// Escape Sequence Snapshots
// =============================================================================

#[test]
fn snapshot_escape_sequences() {
    // Message with various escape sequences
    let hl7 = b"MSH|^~\\&|App|Fac|||20250128120000||ADT^A01|1|P|2.5\rPID|1||123||Test\\F\\Pipe\\S\\Amp\\R\\Tilde\\E\\Backslash\r";
    let message = parse(hl7.as_slice()).unwrap();

    assert_yaml_snapshot!("escape_sequences", message);
}

// =============================================================================
// Null Value Snapshots
// =============================================================================

#[test]
fn snapshot_null_values() {
    let hl7 = b"MSH|^~\\&|App|Fac|||20250128120000||ADT^A01|1|P|2.5\rPID|1||\"\"||\"\"\r";
    let message = parse(hl7.as_slice()).unwrap();

    assert_yaml_snapshot!("null_values", message);
}

// =============================================================================
// Large Message Snapshots (truncated for readability)
// =============================================================================

#[test]
fn snapshot_multi_segment_message() {
    let hl7 = concat!(
        "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\r",
        "EVN|A01|20250128152312|||\r",
        "PID|1||123456^^^HOSP^MR||Doe^John^A||19800101|M|||C|\r",
        "PD1|||Practice||||\r",
        "NK1|1|Doe^Jane|SPO|\r",
        "PV1|1|I|ICU^101^01||||DOC123^Smith^Jane||||||||V123456\r",
        "OBX|1|NM|WBC||7.5|10^9/L|4.0-11.0|N\r",
        "AL1|1||PEN^Penicillin||Rash\r"
    );

    let message = parse(hl7.as_bytes()).unwrap();

    assert_yaml_snapshot!("multi_segment_message", message);
}
