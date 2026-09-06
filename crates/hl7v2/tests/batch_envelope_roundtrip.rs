//! End-to-end batch envelope delimiter regression tests.

#![expect(
    clippy::panic_in_result_fn,
    reason = "round-trip tests propagate parser errors and use assertions for decoded contract checks"
)]

use hl7v2::{
    Batch, Delims, Error, Field, FileBatch, Message, Segment, parse_batch, parse_file_batch,
    write_batch, write_file_batch,
};

fn encoding_text(segment: Option<&Segment>) -> Option<&str> {
    segment?.fields.first()?.first_text()
}

#[test]
fn batch_round_trips_default_encoding_header() -> Result<(), Box<dyn std::error::Error>> {
    let batch = Batch {
        header: Some(Segment {
            id: *b"BHS",
            fields: vec![Field::from_text("ignored"), Field::from_text("SENDAPP")],
        }),
        messages: vec![Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"MSH",
                fields: vec![Field::from_text("ignored"), Field::from_text("APP")],
            }],
            charsets: vec![],
        }],
        trailer: Some(Segment {
            id: *b"BTS",
            fields: vec![Field::from_text("1")],
        }),
    };

    let written = write_batch(&batch);
    let parsed = parse_batch(&written)?;

    assert_eq!(encoding_text(parsed.header.as_ref()), Some("^~\\&"));
    assert_eq!(
        parsed
            .messages
            .first()
            .map(|message| message.delims.clone()),
        Some(Delims::default())
    );
    assert_eq!(
        parsed
            .trailer
            .as_ref()
            .and_then(|segment| segment.fields.first())
            .and_then(Field::first_text),
        Some("1")
    );
    Ok(())
}

#[test]
fn file_round_trips_custom_envelope_after_empty_batch() -> Result<(), Box<dyn std::error::Error>> {
    let delims = Delims {
        field: '!',
        comp: '*',
        rep: '~',
        esc: '\\',
        sub: '&',
    };
    let file_batch = FileBatch {
        header: Some(Segment {
            id: *b"FHS",
            fields: vec![Field::from_text("ignored"), Field::from_text("FILE")],
        }),
        batches: vec![
            Batch {
                header: Some(Segment {
                    id: *b"BHS",
                    fields: vec![Field::from_text("ignored"), Field::from_text("EMPTY")],
                }),
                messages: vec![],
                trailer: Some(Segment {
                    id: *b"BTS",
                    fields: vec![Field::from_text("0")],
                }),
            },
            Batch {
                header: Some(Segment {
                    id: *b"BHS",
                    fields: vec![Field::from_text("ignored"), Field::from_text("FULL")],
                }),
                messages: vec![Message {
                    delims: delims.clone(),
                    segments: vec![Segment {
                        id: *b"MSH",
                        fields: vec![Field::from_text("ignored"), Field::from_text("APP")],
                    }],
                    charsets: vec![],
                }],
                trailer: Some(Segment {
                    id: *b"BTS",
                    fields: vec![Field::from_text("1")],
                }),
            },
        ],
        trailer: Some(Segment {
            id: *b"FTS",
            fields: vec![Field::from_text("2")],
        }),
    };

    let written = write_file_batch(&file_batch);
    let parsed = parse_file_batch(&written)?;

    assert_eq!(encoding_text(parsed.header.as_ref()), Some("*~\\&"));
    assert_eq!(parsed.batches.len(), 2);
    assert!(
        parsed
            .batches
            .first()
            .is_some_and(|batch| batch.messages.is_empty())
    );
    assert_eq!(
        parsed
            .batches
            .get(1)
            .and_then(|batch| batch.messages.first())
            .map(|message| message.delims.clone()),
        Some(delims)
    );
    assert_eq!(
        parsed
            .batches
            .get(1)
            .and_then(|batch| encoding_text(batch.header.as_ref())),
        Some("*~\\&")
    );
    Ok(())
}

#[test]
fn file_parser_rejects_nested_batch_with_conflicting_envelope_delimiters() {
    let result = parse_file_batch(b"FHS!*~\\&!FILE\rBHS|^~\\&|BATCH\rBTS|0\rFTS!1\r");

    assert!(matches!(result, Err(Error::InvalidBatchHeader { .. })));
}
