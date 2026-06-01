//! Unit tests for message normalization.

use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn ensure(condition: bool, message: &'static str) -> TestResult {
    if condition {
        Ok(())
    } else {
        Err(std::io::Error::other(message).into())
    }
}

#[test]
fn normalize_basic_message() -> TestResult {
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\r";

    let normalized = normalize(hl7, false)?;

    ensure(normalized.starts_with(b"MSH|^~\\&|"), "MSH not normalized")
}

#[test]
fn normalize_with_canonical_delimiters() -> TestResult {
    let hl7 = b"MSH*%$!?*SendingApp*SendingFac*ReceivingApp*ReceivingFac*20250128152312**ADT%A01*ABC123*P*2.5.1\r";

    let normalized = normalize(hl7, true)?;
    let normalized_str = String::from_utf8(normalized)?;

    ensure(
        normalized_str.starts_with("MSH|^~\\&|"),
        "canonical delimiters not used",
    )
}

#[test]
fn normalize_preserves_custom_delimiters_when_not_canonical() -> TestResult {
    let hl7 = b"MSH*%$!?*SendingApp*SendingFac*ReceivingApp*ReceivingFac*20250128152312**ADT%A01*ABC123*P*2.5.1\r";

    let normalized = normalize(hl7, false)?;

    ensure(
        normalized.starts_with(b"MSH*%$!?*"),
        "custom delimiters not preserved",
    )
}

#[test]
fn normalize_multi_segment_with_canonical() -> TestResult {
    let hl7 = b"MSH*%$!?*SendingApp*SendingFac*ReceivingApp*ReceivingFac*20250128152312**ADT%A01*ABC123*P*2.5.1\rPID*1**123456%%%HOSP%MR**Doe%John\r";

    let normalized = normalize(hl7, true)?;
    let normalized_str = String::from_utf8(normalized)?;

    ensure(
        normalized_str.starts_with("MSH|^~\\&|"),
        "canonical delimiters not used",
    )?;
    ensure(
        normalized_str.contains("PID|1||123456^^^HOSP^MR||Doe^John"),
        "PID segment not normalized",
    )
}

#[test]
fn normalize_rejects_invalid_message() -> TestResult {
    let invalid = b"PID|1||12345\r";

    ensure(
        matches!(normalize(invalid, true), Err(Error::InvalidSegmentId)),
        "invalid message did not return InvalidSegmentId",
    )
}

#[test]
fn normalize_roundtrips_with_canonical_delimiters() -> TestResult {
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r";

    let normalized = normalize(hl7, true)?;
    let reparsed = crate::parser::parse(&normalized)?;

    ensure(reparsed.segments.len() == 2, "unexpected segment count")?;
    ensure(
        reparsed.delims.field == '|',
        "field delimiter not canonical",
    )?;
    ensure(
        reparsed.delims.comp == '^',
        "component delimiter not canonical",
    )?;
    ensure(
        reparsed.delims.rep == '~',
        "repetition delimiter not canonical",
    )?;
    ensure(
        reparsed.delims.esc == '\\',
        "escape delimiter not canonical",
    )?;
    ensure(
        reparsed.delims.sub == '&',
        "subcomponent delimiter not canonical",
    )
}

#[test]
fn normalize_preserves_escape_sequences() -> TestResult {
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||12345||O\\F\\Brien^John\r";

    let normalized = normalize(hl7, false)?;
    let normalized_str = String::from_utf8(normalized)?;

    ensure(
        normalized_str.contains("O\\F\\Brien"),
        "escape sequence not preserved",
    )
}

#[test]
fn normalize_without_canonical_delimiters_preserves_wire_bytes() -> TestResult {
    let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\r\nPID|1||12345||\\H\\O\\N\\Brien^John||||\r\n";

    let normalized = normalize(hl7, false)?;

    ensure(
        normalized == hl7,
        "non-canonical normalization changed validated wire bytes",
    )
}

#[test]
fn normalize_preserves_unicode_text() -> TestResult {
    let hl7 = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||12345||Müller^Jöhn\r".as_bytes();

    let normalized = normalize(hl7, false)?;
    let normalized_str = String::from_utf8(normalized)?;

    ensure(normalized_str.contains("Müller"), "last name not preserved")?;
    ensure(normalized_str.contains("Jöhn"), "first name not preserved")
}
