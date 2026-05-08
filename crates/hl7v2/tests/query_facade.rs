use hl7v2::{Presence, get, get_presence, parse};
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
    require_eq(get(&message, "PID.3.1"), Some("123456"), "primary MRN")?;
    require_eq(get(&message, "PID.3[2].1"), Some("ALT999"), "alternate MRN")?;
    require_eq(get(&message, "PID.5.1"), Some("Doe"), "family name")?;
    require_eq(get(&message, "PID.5.2"), Some("John"), "given name")?;

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
