use hl7v2::model::{Batch, FileBatch};
use hl7v2::{Atom, Comp, Delims, Field, Message, Rep, Segment};
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

fn require_some<T>(value: Option<T>, label: &str) -> Result<T, Box<dyn Error>> {
    value.ok_or_else(|| std::io::Error::other(format!("missing {label}")).into())
}

#[test]
fn model_roundtrips_complex_message_through_public_facade() -> Result<(), Box<dyn Error>> {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"PID",
            fields: vec![
                Field::from_text("1"),
                Field {
                    reps: vec![
                        Rep {
                            comps: vec![
                                Comp::from_text("123456"),
                                Comp::from_text(""),
                                Comp::from_text(""),
                                Comp::from_text("HOSP"),
                                Comp::from_text("MR"),
                            ],
                        },
                        Rep {
                            comps: vec![Comp::from_text("ALT999"), Comp::from_text("ALT")],
                        },
                    ],
                },
                Field {
                    reps: vec![Rep {
                        comps: vec![Comp {
                            subs: vec![Atom::text("Doe"), Atom::text("John")],
                        }],
                    }],
                },
            ],
        }],
        charsets: vec!["ASCII".to_string()],
    };

    let json = serde_json::to_string(&message)?;
    let decoded: Message = serde_json::from_str(&json)?;
    let pid = require_some(decoded.segments.first(), "PID segment")?;

    require_eq(pid.id, *b"PID", "segment id")?;
    require_eq(pid.fields.len(), 3, "field count")?;
    require_eq(decoded.charsets, vec!["ASCII".to_string()], "charsets")?;

    let repeating_identifier = require_some(pid.fields.get(1), "PID-2 repeating identifier")?;
    require_eq(
        repeating_identifier.reps.len(),
        2,
        "identifier repetition count",
    )?;

    Ok(())
}

#[test]
fn model_file_batch_preserves_nested_batch_counts() -> Result<(), Box<dyn Error>> {
    let mut file_batch = FileBatch::default();
    let mut first_batch = Batch::default();
    let mut second_batch = Batch::default();

    first_batch
        .messages
        .push(Message::with_segments(vec![Segment {
            id: *b"MSH",
            fields: vec![Field::from_text("^~\\&")],
        }]));
    second_batch
        .messages
        .push(Message::with_segments(vec![Segment {
            id: *b"MSH",
            fields: vec![Field::from_text("^~\\&")],
        }]));
    second_batch
        .messages
        .push(Message::with_segments(vec![Segment {
            id: *b"MSH",
            fields: vec![Field::from_text("^~\\&")],
        }]));

    file_batch.batches.push(first_batch);
    file_batch.batches.push(second_batch);

    require_eq(file_batch.batches.len(), 2, "batch count")?;
    require_eq(
        file_batch
            .batches
            .iter()
            .map(|batch| batch.messages.len())
            .sum::<usize>(),
        3,
        "nested message count",
    )?;

    Ok(())
}
