use hl7v2::{Presence, QueryIndex, get, get_located, get_presence, get_presence_located, parse};
use std::error::Error;
use std::fmt::Debug;

fn require_eq<T>(actual: T, expected: T, label: &str) -> Result<(), Box<dyn Error>>
where
    T: PartialEq + Debug,
{
    if actual == expected {
        Ok(())
    } else {
        Err(std::io::Error::other(format!("{label}: expected {expected:?}, got {actual:?}")).into())
    }
}

fn require(condition: bool, message: &'static str) -> Result<(), Box<dyn Error>> {
    if condition {
        Ok(())
    } else {
        Err(std::io::Error::other(message).into())
    }
}

#[test]
fn query_facade_reads_real_adt_components_and_repetitions() -> Result<(), Box<dyn Error>> {
    let message = parse(
        b"MSH|^~\\&|SEND|FAC|RECV|RF|202605030101||ADT^A01|CTRL123|P|2.5\r\
PID|1||123456^^^HOSP^MR~ALT999^^^ALT^MR||Doe^John^A||19700101|M\r",
    )?;

    require_eq(get(&message, "MSH.10"), Some("CTRL123"), "control id")?;
    require_eq(get(&message, "MSH.12"), Some("2.5"), "version id")?;
    require_eq(get(&message, "MSH.13"), None, "missing field after version")?;
    require(
        matches!(get_presence(&message, "MSH.12"), Presence::Value(value) if value == "2.5"),
        "expected MSH-12 version presence",
    )?;
    require(
        matches!(get_presence(&message, "MSH.13"), Presence::Missing),
        "expected missing MSH-13",
    )?;
    require_eq(get(&message, "PID.3.1"), Some("123456"), "primary MRN")?;
    require_eq(get(&message, "PID.3[2].1"), Some("ALT999"), "alternate MRN")?;
    require_eq(get(&message, "PID.5.1"), Some("Doe"), "family name")?;
    require_eq(get(&message, "PID.5.2"), Some("John"), "given name")?;

    Ok(())
}

#[test]
fn query_facade_reads_target_dash_paths_and_segment_repetitions() -> Result<(), Box<dyn Error>> {
    let message = parse(
        b"MSH|^~\\&|LAB|FAC|EHR|RF|202605030101||ORU^R01|CTRL456|P|2.5\r\
PID|1||123456^^^HOSP^MR~ALT999^^^ALT^MR||Doe^John^A||19700101|M\r\
OBR|1|ORD1|FILL1|CBC^Complete blood count\r\
OBX|1|ST|NOTE^First note||Alpha\r\
OBX|2|ST|NOTE^Second note||Beta\r\
OBX|3|ST|NOTE^Third note||Gamma\r\
NTE|1|L|operator note\r",
    )?;

    require_eq(get(&message, "MSH-9.1"), Some("ORU"), "message code")?;
    require_eq(
        get(&message, "PID-3[2].4"),
        Some("ALT"),
        "alternate assigning authority",
    )?;
    require_eq(get(&message, "OBX[3]-5"), Some("Gamma"), "third OBX value")?;
    require(
        matches!(
            get_presence(&message, "NTE[1]-3"),
            Presence::Value(value) if value == "operator note"
        ),
        "expected first NTE comment",
    )?;

    Ok(())
}

#[test]
fn query_facade_distinguishes_empty_and_missing_presence() -> Result<(), Box<dyn Error>> {
    let message =
        parse(b"MSH|^~\\&|SEND|FAC|RECV|RF|202605030101||ADT^A01|CTRL123|P|2.5\rPID|1||\r")?;

    require(
        matches!(get_presence(&message, "PID.3"), Presence::Empty),
        "expected empty PID-3",
    )?;
    require(
        matches!(get_presence(&message, "PID.9"), Presence::Missing),
        "expected missing PID-9",
    )?;

    Ok(())
}

#[test]
fn query_facade_returns_msh_field_separator_value() -> Result<(), Box<dyn Error>> {
    let message = parse(b"MSH*^~\\&*SEND*FAC*RECV*RF*202605030101**ADT^A01*CTRL123*P*2.5\r")?;

    require_eq(get(&message, "MSH.1"), Some("*"), "MSH-1 value")?;
    require(
        matches!(get_presence(&message, "MSH.1"), Presence::Value(value) if value == "*"),
        "expected MSH-1 presence value",
    )?;

    Ok(())
}

#[test]
fn query_facade_reuses_parsed_located_paths() -> Result<(), Box<dyn Error>> {
    let message = parse(
        b"MSH|^~\\&|LAB|FAC|EHR|RF|202605030101||ORU^R01|CTRL789|P|2.5\r\
PID|1||123456^^^HOSP^MR~ALT999^^^ALT^MR||Doe^John^A||19700101|M\r\
OBR|1|ORD1|FILL1|CBC^Complete blood count\r\
OBX|1|ST|NOTE^First note||Alpha\r\
NTE|1|L|first note\r\
OBX|2|ST|NOTE^Second note||Beta\r\
NTE|2|L|second note\r",
    )?;

    let message_code = hl7v2::parse_located_path("MSH-9.1")?;
    let alternate_authority = hl7v2::parse_located_path("PID-3[2].4")?;
    let second_obx_value = hl7v2::parse_located_path("OBX[2]-5")?;
    let second_nte_comment = hl7v2::parse_located_path("NTE[2]-3")?;
    let missing_obx_value = hl7v2::parse_located_path("OBX[3]-5")?;

    require_eq(
        get_located(&message, &message_code),
        get(&message, "MSH-9.1"),
        "parsed MSH path matches string query",
    )?;
    require_eq(
        get_located(&message, &alternate_authority),
        Some("ALT"),
        "parsed field repetition path",
    )?;
    require_eq(
        get_located(&message, &second_obx_value),
        Some("Beta"),
        "parsed segment repetition path",
    )?;
    require(
        matches!(
            get_presence_located(&message, &second_nte_comment),
            Presence::Value(value) if value == "second note"
        ),
        "expected parsed presence path value",
    )?;
    require(
        matches!(
            get_presence_located(&message, &missing_obx_value),
            Presence::Missing
        ),
        "expected missing parsed segment repetition",
    )?;

    Ok(())
}

#[test]
fn query_facade_reuses_indexed_segment_paths() -> Result<(), Box<dyn Error>> {
    let message = parse(
        b"MSH|^~\\&|LAB|FAC|EHR|RF|202605030101||ORU^R01|CTRL999|P|2.5\r\
OBX|1|ST|NOTE^First note||Alpha\r\
NTE|1|L|first note\r\
OBX|2|ST|NOTE^Second note||Beta\r\
NTE|2|L|\r",
    )?;
    let index = QueryIndex::new(&message);
    let second_obx_value = hl7v2::parse_located_path("OBX[2]-5")?;
    let second_nte_comment = hl7v2::parse_located_path("NTE[2]-3")?;

    require_eq(
        index.get("MSH-1"),
        get(&message, "MSH-1"),
        "indexed MSH delimiter path matches string query",
    )?;
    require_eq(
        index.get_located(&second_obx_value),
        Some("Beta"),
        "indexed segment repetition path",
    )?;
    require(
        matches!(
            index.get_presence_located(&second_nte_comment),
            Presence::Empty
        ),
        "expected indexed empty presence",
    )?;
    require(
        matches!(index.get_presence("OBX[3]-5"), Presence::Missing),
        "expected indexed missing segment repetition",
    )?;

    Ok(())
}
