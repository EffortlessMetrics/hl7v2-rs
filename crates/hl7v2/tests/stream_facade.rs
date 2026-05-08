#![cfg(feature = "stream")]

use hl7v2::{Event, StreamParser};
use std::error::Error;
use std::io::{BufReader, Cursor, Read};

fn collect_events<R: Read>(
    parser: &mut StreamParser<BufReader<R>>,
) -> Result<Vec<Event>, hl7v2::Error> {
    let mut events = Vec::new();
    while let Some(event) = parser.next_event()? {
        events.push(event);
    }
    Ok(events)
}

fn require(condition: bool, message: &'static str) -> Result<(), Box<dyn Error>> {
    if condition {
        Ok(())
    } else {
        Err(std::io::Error::other(message).into())
    }
}

#[test]
fn stream_parser_is_available_through_hl7v2() -> Result<(), Box<dyn Error>> {
    let hl7 =
        b"MSH|^~\\&|SEND|FAC|RECV|FAC|202605070101||ADT^A01|CTRL|P|2.5\rPID|1||123||Doe^John\r";
    let cursor = Cursor::new(hl7.as_slice());
    let reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(reader);

    let events = collect_events(&mut parser)?;

    require(
        events
            .iter()
            .any(|event| matches!(event, Event::StartMessage { .. })),
        "expected StartMessage event",
    )?;
    require(
        events
            .iter()
            .any(|event| matches!(event, Event::EndMessage)),
        "expected EndMessage event",
    )?;
    require(
        events
            .iter()
            .any(|event| matches!(event, Event::Segment { id } if id == b"PID")),
        "expected PID segment event",
    )?;

    Ok(())
}

#[test]
fn stream_parser_handles_multiple_messages() -> Result<(), Box<dyn Error>> {
    let hl7 = b"MSH|^~\\&|A|B|C|D|202605070101||ADT^A01|ONE|P|2.5\rPID|1||1||One^Pat\rMSH|^~\\&|A|B|C|D|202605070102||ADT^A01|TWO|P|2.5\rPID|1||2||Two^Pat\r";
    let cursor = Cursor::new(hl7.as_slice());
    let reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(reader);

    let events = collect_events(&mut parser)?;
    let start_count = events
        .iter()
        .filter(|event| matches!(event, Event::StartMessage { .. }))
        .count();
    let end_count = events
        .iter()
        .filter(|event| matches!(event, Event::EndMessage))
        .count();

    require(start_count == 2, "expected two StartMessage events")?;
    require(end_count == 2, "expected two EndMessage events")?;

    Ok(())
}

#[test]
fn stream_parser_reports_message_size_limit() -> Result<(), Box<dyn Error>> {
    let hl7 =
        b"MSH|^~\\&|SEND|FAC|RECV|FAC|202605070101||ADT^A01|CTRL|P|2.5\rPID|1||123||Doe^John\r";
    let cursor = Cursor::new(hl7.as_slice());
    let reader = BufReader::new(cursor);
    let mut parser = StreamParser::with_max_message_size(reader, 4);

    let mut saw_size_error = false;
    loop {
        match parser.next_event() {
            Ok(Some(_event)) => {}
            Ok(None) => break,
            Err(err) => {
                saw_size_error = matches!(err, hl7v2::Error::InvalidFieldFormat { .. });
                break;
            }
        }
    }

    require(saw_size_error, "expected message size error")?;

    Ok(())
}

#[test]
fn stream_parser_emits_field_content_through_hl7v2() -> Result<(), Box<dyn Error>> {
    let hl7 =
        b"MSH|^~\\&|SEND|FAC|RECV|FAC|202605070101||ADT^A01|CTRL|P|2.5\rPID|1||123456^^^HOSP^MR||Doe^John\r";
    let cursor = Cursor::new(hl7.as_slice());
    let reader = BufReader::with_capacity(16, cursor);
    let mut parser = StreamParser::new(reader);

    let events = collect_events(&mut parser)?;

    require(
        events.iter().any(
            |event| matches!(event, Event::Field { raw, .. } if raw.as_slice() == b"Doe^John"),
        ),
        "expected patient name field event",
    )?;
    require(
        events.iter().any(|event| {
            matches!(event, Event::Field { raw, .. } if raw.as_slice() == b"123456^^^HOSP^MR")
        }),
        "expected MRN field event",
    )?;

    Ok(())
}

#[test]
fn stream_parser_preserves_segment_order_through_hl7v2() -> Result<(), Box<dyn Error>> {
    let hl7 = b"MSH|^~\\&|SEND|FAC|RECV|FAC|202605070101||ADT^A01|CTRL|P|2.5\rEVN|A01|202605070101\rPID|1||123456^^^HOSP^MR||Doe^John\rPV1|1|I|ICU^101\r";
    let cursor = Cursor::new(hl7.as_slice());
    let reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(reader);

    let events = collect_events(&mut parser)?;
    let segment_ids: Vec<&[u8]> = events
        .iter()
        .filter_map(|event| {
            if let Event::Segment { id } = event {
                Some(id.as_slice())
            } else {
                None
            }
        })
        .collect();

    require(
        segment_ids.as_slice() == [b"EVN".as_slice(), b"PID".as_slice(), b"PV1".as_slice()],
        "expected EVN/PID/PV1 segment order",
    )?;

    Ok(())
}
