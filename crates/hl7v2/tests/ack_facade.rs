#![cfg(feature = "ack")]

use hl7v2::{AckCode, Message, Segment, ack, ack_with_error, get, parse, write};
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

fn sample_message() -> Result<Message, hl7v2::Error> {
    parse(
        b"MSH|^~\\&|SENDAPP|SENDFAC|RECVAPP|RECVFAC|202605030101||ADT^A01|CTRL123|P|2.5\r\
PID|1||123456^^^HOSP^MR||Doe^John||19700101|M\r",
    )
}

#[test]
fn ack_facade_preserves_control_id_and_swaps_applications() -> Result<(), Box<dyn Error>> {
    let original = sample_message()?;
    let response = ack(&original, AckCode::AA)?;

    require_eq(get(&response, "MSH.3"), Some("RECVAPP"), "ack sender app")?;
    require_eq(get(&response, "MSH.5"), Some("SENDAPP"), "ack receiver app")?;
    require_eq(get(&response, "MSA.1"), Some("AA"), "ack code")?;
    require_eq(get(&response, "MSA.2"), Some("CTRL123"), "ack control id")?;

    Ok(())
}

#[test]
fn ack_with_error_roundtrips_with_err_segment() -> Result<(), Box<dyn Error>> {
    let original = sample_message()?;
    let response = ack_with_error(&original, AckCode::AE, Some("Application failure"))?;
    let reparsed = parse(&write(&response))?;

    require_eq(reparsed.segments.len(), 3, "segment count")?;
    require_eq(get(&reparsed, "MSA.1"), Some("AE"), "ack code")?;
    require_eq(
        get(&reparsed, "ERR.3"),
        Some("Application failure"),
        "ERR message",
    )?;

    Ok(())
}

#[test]
fn ack_facade_supports_application_and_commit_codes() -> Result<(), Box<dyn Error>> {
    let original = sample_message()?;

    for code in [
        AckCode::AA,
        AckCode::AE,
        AckCode::AR,
        AckCode::CA,
        AckCode::CE,
        AckCode::CR,
    ] {
        let response = ack(&original, code)?;
        require_eq(get(&response, "MSA.1"), Some(code.as_str()), "ack code")?;
    }

    Ok(())
}

#[test]
fn ack_facade_rejects_messages_without_msh() -> Result<(), Box<dyn Error>> {
    let invalid = Message::with_segments(vec![Segment {
        id: *b"PID",
        fields: Vec::new(),
    }]);

    let result = ack(&invalid, AckCode::AA);
    require_eq(result.is_err(), true, "invalid message rejection")?;

    Ok(())
}
