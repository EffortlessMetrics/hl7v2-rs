//! BDD tests for hl7v2-parser using Cucumber
//!
//! Run with: cargo test --test bdd_tests -p hl7v2-parser

use cucumber::{World, given, then, when};
use hl7v2_model::Error;
use hl7v2_parser::{get, parse, parse_batch, parse_file_batch, parse_mllp};

/// Test world for Parser BDD tests
#[derive(Debug, World)]
#[world(init = Self::new)]
pub struct ParserWorld {
    /// Raw input bytes
    raw_bytes: Vec<u8>,
    /// Parsed message result
    parse_result: Option<Result<hl7v2_model::Message, Error>>,
    /// Parsed message (if successful)
    message: Option<hl7v2_model::Message>,
    /// Error (if parsing failed)
    error: Option<Error>,
    /// Parsed batch result
    batch: Option<hl7v2_model::Batch>,
    /// Parsed file batch result
    file_batch: Option<hl7v2_model::FileBatch>,
}

impl ParserWorld {
    fn new() -> Self {
        Self {
            raw_bytes: Vec::new(),
            parse_result: None,
            message: None,
            error: None,
            batch: None,
            file_batch: None,
        }
    }
}

// ============================================================================
// Given Steps
// ============================================================================

#[given("a valid ADT^A01 message with MSH and PID segments")]
fn given_adt_a01(world: &mut ParserWorld) {
    world.raw_bytes = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r".to_vec();
}

#[given("a message with MSH, PID, PV1, and OBX segments")]
fn given_multi_segment(world: &mut ParserWorld) {
    world.raw_bytes = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\r\
PID|1||123456^^^HOSP^MR||Doe^John\r\
PV1|1|I|ICU\r\
OBX|1|NM|1234-5^Weight^LN||75|kg\r".to_vec();
}

#[given(regex = r#"^a message with custom delimiters "([^"]+)"$"#)]
fn given_custom_delimiters(world: &mut ParserWorld, delims: String) {
    // delims = "#$*@!"
    let chars: Vec<char> = delims.chars().collect();
    assert_eq!(chars.len(), 5, "Expected 5 delimiter characters");
    let field = chars[0]; // #
    let comp = chars[1]; // $
    let rep = chars[2]; // *
    let esc = chars[3]; // @
    let sub = chars[4]; // !

    // Build MSH with custom delimiters: MSH<field><comp><rep><esc><sub><field>...
    let msg = format!(
        "MSH{f}{c}{r}{e}{s}{f}SendingApp{f}SendingFac{f}ReceivingApp{f}ReceivingFac{f}20250128152312{f}{f}ADT{c}A01{f}MSG001{f}P{f}2.5.1\r",
        f = field,
        c = comp,
        r = rep,
        e = esc,
        s = sub
    );
    world.raw_bytes = msg.into_bytes();
}

#[given(regex = r#"^a message with repeated patient names "([^"]+)"$"#)]
fn given_repeated_names(world: &mut ParserWorld, names: String) {
    // names = "Doe^John~Smith^Jane"
    let msg = format!(
        "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||{}\r",
        names
    );
    world.raw_bytes = msg.into_bytes();
}

#[given(regex = r#"^a message with patient ID "([^"]+)"$"#)]
fn given_patient_id(world: &mut ParserWorld, pid: String) {
    let msg = format!(
        "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||{}||Doe^John\r",
        pid
    );
    world.raw_bytes = msg.into_bytes();
}

#[given(regex = r#"^a message with subcomponent value "([^"]+)" in PID-3$"#)]
fn given_subcomponent(world: &mut ParserWorld, value: String) {
    let msg = format!(
        "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||{}||Doe^John\r",
        value
    );
    world.raw_bytes = msg.into_bytes();
}

#[given("an MLLP-framed ADT^A01 message")]
fn given_mllp_message(world: &mut ParserWorld) {
    let inner = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r";
    world.raw_bytes = hl7v2_mllp::wrap_mllp(inner);
}

#[given("an empty byte input")]
fn given_empty_input(world: &mut ParserWorld) {
    world.raw_bytes = Vec::new();
}

#[given(regex = r#"^a message starting with "([^"]+)" instead of MSH$"#)]
fn given_non_msh_start(world: &mut ParserWorld, segment: String) {
    let msg = format!("{}|1||123456^^^HOSP^MR||Doe^John\r", segment);
    world.raw_bytes = msg.into_bytes();
}

#[given("a byte sequence with invalid UTF-8")]
fn given_invalid_utf8(world: &mut ParserWorld) {
    // 0xFF 0xFE are not valid UTF-8 continuation bytes
    world.raw_bytes = vec![0xFF, 0xFE, 0xFD, 0x0D];
}

#[given(regex = r#"^a message with escape sequence "([^"]*)" in a field value$"#)]
fn given_escape_sequence(world: &mut ParserWorld, _esc: String) {
    // Build a message with \F\ escape in OBX-5 (observation value)
    let msg = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ORU^R01|ABC123|P|2.5.1\rOBX|1|ST|1234-5^Test^LN||Value\\F\\More\r";
    world.raw_bytes = msg.to_vec();
}

#[given("a message with 10 OBX segments")]
fn given_many_obx(world: &mut ParserWorld) {
    let mut msg = String::from(
        "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ORU^R01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r",
    );
    for i in 1..=10 {
        msg.push_str(&format!("OBX|{}|NM|1234-{}^Test^LN||{}\r", i, i, i * 10));
    }
    world.raw_bytes = msg.into_bytes();
}

#[given(regex = r#"^a message with HL7 version "([^"]+)"$"#)]
fn given_hl7_version(world: &mut ParserWorld, version: String) {
    let msg = format!(
        "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|{}\rPID|1||123456^^^HOSP^MR||Doe^John\r",
        version
    );
    world.raw_bytes = msg.into_bytes();
}

#[given("a batch message with BHS, 2 MSH messages, and BTS")]
fn given_batch(world: &mut ParserWorld) {
    world.raw_bytes = b"BHS|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128120000|||BATCH001|Test batch\r\
MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128120001||ADT^A01|MSG001|P|2.5.1\r\
PID|1||111111^^^HOSP^MR||Doe^John\r\
MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128120002||ADT^A01|MSG002|P|2.5.1\r\
PID|1||222222^^^HOSP^MR||Smith^Jane\r\
BTS|2\r".to_vec();
}

#[given("a file batch with FHS, BHS, 1 MSH message, BTS, and FTS")]
fn given_file_batch(world: &mut ParserWorld) {
    world.raw_bytes = b"FHS|^~\\&|FileSender|FileFacility|FileReceiver|FileFacility|20250128120000|||FILE001|Test file batch\r\
BHS|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128120001|||BATCH001|Test batch\r\
MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128120002||ADT^A01|MSG001|P|2.5.1\r\
PID|1||123456^^^HOSP^MR||Doe^John\r\
BTS|1\r\
FTS|1\r".to_vec();
}

// ============================================================================
// When Steps
// ============================================================================

#[when("I parse the message")]
fn when_parse(world: &mut ParserWorld) {
    let result = parse(&world.raw_bytes);
    match &result {
        Ok(msg) => {
            world.message = Some(msg.clone());
            world.error = None;
        }
        Err(e) => {
            world.error = Some(e.clone());
            world.message = None;
        }
    }
    world.parse_result = Some(result);
}

#[when("I attempt to parse the message")]
fn when_attempt_parse(world: &mut ParserWorld) {
    when_parse(world);
}

#[when("I parse the MLLP message")]
fn when_parse_mllp(world: &mut ParserWorld) {
    let result = parse_mllp(&world.raw_bytes);
    match &result {
        Ok(msg) => {
            world.message = Some(msg.clone());
            world.error = None;
        }
        Err(e) => {
            world.error = Some(e.clone());
            world.message = None;
        }
    }
    world.parse_result = Some(result);
}

#[when("I parse the batch")]
fn when_parse_batch(world: &mut ParserWorld) {
    match parse_batch(&world.raw_bytes) {
        Ok(batch) => {
            world.batch = Some(batch);
            world.error = None;
        }
        Err(e) => {
            world.error = Some(e);
            world.batch = None;
        }
    }
}

#[when("I parse the file batch")]
fn when_parse_file_batch(world: &mut ParserWorld) {
    match parse_file_batch(&world.raw_bytes) {
        Ok(fb) => {
            world.file_batch = Some(fb);
            world.error = None;
        }
        Err(e) => {
            world.error = Some(e);
            world.file_batch = None;
        }
    }
}

// ============================================================================
// Then Steps
// ============================================================================

#[then("parsing should succeed")]
fn then_parse_success(world: &mut ParserWorld) {
    assert!(
        world.message.is_some(),
        "Expected parsing to succeed, but got error: {:?}",
        world.error
    );
}

#[then("parsing should fail")]
fn then_parse_fail(world: &mut ParserWorld) {
    assert!(
        world.error.is_some(),
        "Expected parsing to fail, but it succeeded"
    );
}

#[then(regex = r#"^the message should have (\d+) segments$"#)]
fn then_segment_count(world: &mut ParserWorld, count: usize) {
    let msg = world.message.as_ref().expect("No parsed message");
    assert_eq!(
        msg.segments.len(),
        count,
        "Expected {} segments, got {}",
        count,
        msg.segments.len()
    );
}

#[then(regex = r#"^segment (\d+) should be "([^"]+)"$"#)]
fn then_segment_id(world: &mut ParserWorld, index: usize, expected_id: String) {
    let msg = world.message.as_ref().expect("No parsed message");
    let segment = &msg.segments[index - 1];
    let actual_id = std::str::from_utf8(&segment.id).unwrap();
    assert_eq!(
        actual_id, expected_id,
        "Segment {} should be '{}', got '{}'",
        index, expected_id, actual_id
    );
}

#[then(regex = r#"^MSH\.3 should be "([^"]+)"$"#)]
fn then_msh3(world: &mut ParserWorld, expected: String) {
    let msg = world.message.as_ref().expect("No parsed message");
    let actual = get(msg, "MSH.3").expect("MSH.3 not found");
    assert_eq!(actual, expected);
}

#[then(regex = r#"^MSH\.4 should be "([^"]+)"$"#)]
fn then_msh4(world: &mut ParserWorld, expected: String) {
    let msg = world.message.as_ref().expect("No parsed message");
    let actual = get(msg, "MSH.4").expect("MSH.4 not found");
    assert_eq!(actual, expected);
}

#[then(regex = r#"^MSH\.5 should be "([^"]+)"$"#)]
fn then_msh5(world: &mut ParserWorld, expected: String) {
    let msg = world.message.as_ref().expect("No parsed message");
    let actual = get(msg, "MSH.5").expect("MSH.5 not found");
    assert_eq!(actual, expected);
}

#[then(regex = r#"^MSH\.6 should be "([^"]+)"$"#)]
fn then_msh6(world: &mut ParserWorld, expected: String) {
    let msg = world.message.as_ref().expect("No parsed message");
    let actual = get(msg, "MSH.6").expect("MSH.6 not found");
    assert_eq!(actual, expected);
}

#[then(regex = r#"^MSH\.9\.1 should be "([^"]+)"$"#)]
fn then_msh9_1(world: &mut ParserWorld, expected: String) {
    let msg = world.message.as_ref().expect("No parsed message");
    let actual = get(msg, "MSH.9.1").expect("MSH.9.1 not found");
    assert_eq!(actual, expected);
}

#[then(regex = r#"^MSH\.9\.2 should be "([^"]+)"$"#)]
fn then_msh9_2(world: &mut ParserWorld, expected: String) {
    let msg = world.message.as_ref().expect("No parsed message");
    let actual = get(msg, "MSH.9.2").expect("MSH.9.2 not found");
    assert_eq!(actual, expected);
}

#[then(regex = r#"^MSH\.10 should be "([^"]+)"$"#)]
fn then_msh10(world: &mut ParserWorld, expected: String) {
    let msg = world.message.as_ref().expect("No parsed message");
    let actual = get(msg, "MSH.10").expect("MSH.10 not found");
    assert_eq!(actual, expected);
}

#[then(regex = r#"^MSH\.11 should be "([^"]+)"$"#)]
fn then_msh11(world: &mut ParserWorld, expected: String) {
    let msg = world.message.as_ref().expect("No parsed message");
    let actual = get(msg, "MSH.11").expect("MSH.11 not found");
    assert_eq!(actual, expected);
}

#[then(regex = r#"^MSH\.12 should be "([^"]+)"$"#)]
fn then_msh12(world: &mut ParserWorld, expected: String) {
    let msg = world.message.as_ref().expect("No parsed message");
    let actual = get(msg, "MSH.12").expect("MSH.12 not found");
    assert_eq!(actual, expected);
}

#[then(regex = r#"^the field delimiter should be "([^"]+)"$"#)]
fn then_field_delim(world: &mut ParserWorld, expected: String) {
    let msg = world.message.as_ref().expect("No parsed message");
    let expected_char = expected.chars().next().unwrap();
    assert_eq!(msg.delims.field, expected_char);
}

#[then(regex = r#"^the component delimiter should be "([^"]+)"$"#)]
fn then_comp_delim(world: &mut ParserWorld, expected: String) {
    let msg = world.message.as_ref().expect("No parsed message");
    let expected_char = expected.chars().next().unwrap();
    assert_eq!(msg.delims.comp, expected_char);
}

#[then(regex = r#"^the repetition delimiter should be "([^"]+)"$"#)]
fn then_rep_delim(world: &mut ParserWorld, expected: String) {
    let msg = world.message.as_ref().expect("No parsed message");
    let expected_char = expected.chars().next().unwrap();
    assert_eq!(msg.delims.rep, expected_char);
}

#[then(regex = r#"^the escape delimiter should be "([^"]+)"$"#)]
fn then_esc_delim(world: &mut ParserWorld, expected: String) {
    let msg = world.message.as_ref().expect("No parsed message");
    let expected_char = expected.chars().next().unwrap();
    assert_eq!(msg.delims.esc, expected_char);
}

#[then(regex = r#"^the subcomponent delimiter should be "([^"]+)"$"#)]
fn then_sub_delim(world: &mut ParserWorld, expected: String) {
    let msg = world.message.as_ref().expect("No parsed message");
    let expected_char = expected.chars().next().unwrap();
    assert_eq!(msg.delims.sub, expected_char);
}

#[then(regex = r#"^PID\.5\[1\]\.1 should be "([^"]+)"$"#)]
fn then_pid5_rep1_comp1(world: &mut ParserWorld, expected: String) {
    let msg = world.message.as_ref().expect("No parsed message");
    let actual = get(msg, "PID.5[1].1").expect("PID.5[1].1 not found");
    assert_eq!(actual, expected);
}

#[then(regex = r#"^PID\.5\[1\]\.2 should be "([^"]+)"$"#)]
fn then_pid5_rep1_comp2(world: &mut ParserWorld, expected: String) {
    let msg = world.message.as_ref().expect("No parsed message");
    let actual = get(msg, "PID.5[1].2").expect("PID.5[1].2 not found");
    assert_eq!(actual, expected);
}

#[then(regex = r#"^PID\.5\[2\]\.1 should be "([^"]+)"$"#)]
fn then_pid5_rep2_comp1(world: &mut ParserWorld, expected: String) {
    let msg = world.message.as_ref().expect("No parsed message");
    let actual = get(msg, "PID.5[2].1").expect("PID.5[2].1 not found");
    assert_eq!(actual, expected);
}

#[then(regex = r#"^PID\.5\[2\]\.2 should be "([^"]+)"$"#)]
fn then_pid5_rep2_comp2(world: &mut ParserWorld, expected: String) {
    let msg = world.message.as_ref().expect("No parsed message");
    let actual = get(msg, "PID.5[2].2").expect("PID.5[2].2 not found");
    assert_eq!(actual, expected);
}

#[then(regex = r#"^PID\.3\.1 should be "([^"]+)"$"#)]
fn then_pid3_comp1(world: &mut ParserWorld, expected: String) {
    let msg = world.message.as_ref().expect("No parsed message");
    let actual = get(msg, "PID.3.1").expect("PID.3.1 not found");
    assert_eq!(actual, expected);
}

#[then(regex = r#"^PID\.3\.4 should be "([^"]+)"$"#)]
fn then_pid3_comp4(world: &mut ParserWorld, expected: String) {
    let msg = world.message.as_ref().expect("No parsed message");
    let actual = get(msg, "PID.3.4").expect("PID.3.4 not found");
    assert_eq!(actual, expected);
}

#[then(regex = r#"^PID\.3\.5 should be "([^"]+)"$"#)]
fn then_pid3_comp5(world: &mut ParserWorld, expected: String) {
    let msg = world.message.as_ref().expect("No parsed message");
    let actual = get(msg, "PID.3.5").expect("PID.3.5 not found");
    assert_eq!(actual, expected);
}

#[then(regex = r#"^PID\.3\.1 should contain subcomponents "([^"]+)" and "([^"]+)"$"#)]
fn then_pid3_comp1_subcomponents(world: &mut ParserWorld, sub1: String, sub2: String) {
    use hl7v2_model::Atom;
    let msg = world.message.as_ref().expect("No parsed message");
    let pid = msg
        .segments
        .iter()
        .find(|s| &s.id == b"PID")
        .expect("PID not found");
    // PID-3 is field index 2 (0-based)
    let comp = &pid.fields[2].reps[0].comps[0];
    assert!(
        comp.subs.len() >= 2,
        "Expected at least 2 subcomponents, got {}",
        comp.subs.len()
    );
    assert_eq!(comp.subs[0], Atom::Text(sub1));
    assert_eq!(comp.subs[1], Atom::Text(sub2));
}

#[then("the error should indicate an invalid segment")]
fn then_error_invalid_segment(world: &mut ParserWorld) {
    let err = world.error.as_ref().expect("Expected an error");
    assert!(
        matches!(err, Error::InvalidSegmentId),
        "Expected InvalidSegmentId error, got: {:?}",
        err
    );
}

#[then("the error should indicate an invalid charset")]
fn then_error_invalid_charset(world: &mut ParserWorld) {
    let err = world.error.as_ref().expect("Expected an error");
    assert!(
        matches!(err, Error::InvalidCharset),
        "Expected InvalidCharset error, got: {:?}",
        err
    );
}

#[then(regex = r#"^the unescaped field value should contain "([^"]+)"$"#)]
fn then_unescaped_contains(world: &mut ParserWorld, expected: String) {
    let msg = world.message.as_ref().expect("No parsed message");
    // OBX-5 contains the escape-sequence value
    let value = get(msg, "OBX.5").expect("OBX.5 not found");
    assert!(
        value.contains(&expected),
        "Expected OBX.5 ('{}') to contain '{}'",
        value,
        expected
    );
}

#[then("batch parsing should succeed")]
fn then_batch_success(world: &mut ParserWorld) {
    assert!(
        world.batch.is_some(),
        "Expected batch parsing to succeed, but got error: {:?}",
        world.error
    );
}

#[then(regex = r#"^the batch should contain (\d+) messages$"#)]
fn then_batch_message_count(world: &mut ParserWorld, count: usize) {
    let batch = world.batch.as_ref().expect("No parsed batch");
    assert_eq!(
        batch.messages.len(),
        count,
        "Expected {} messages in batch, got {}",
        count,
        batch.messages.len()
    );
}

#[then("file batch parsing should succeed")]
fn then_file_batch_success(world: &mut ParserWorld) {
    assert!(
        world.file_batch.is_some(),
        "Expected file batch parsing to succeed, but got error: {:?}",
        world.error
    );
}

#[then(regex = r#"^the file batch should contain (\d+) batch$"#)]
fn then_file_batch_count(world: &mut ParserWorld, count: usize) {
    let fb = world.file_batch.as_ref().expect("No parsed file batch");
    assert_eq!(
        fb.batches.len(),
        count,
        "Expected {} batches in file batch, got {}",
        count,
        fb.batches.len()
    );
}

// ============================================================================
// Cucumber Main
// ============================================================================

#[tokio::main]
async fn main() {
    ParserWorld::run("features/parser.feature").await;
}
