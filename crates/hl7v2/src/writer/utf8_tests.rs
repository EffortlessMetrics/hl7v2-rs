use super::*;

fn utf8_delims() -> Delims {
    Delims {
        field: '§',
        comp: '¶',
        rep: '※',
        esc: '¤',
        sub: '¦',
    }
}

fn utf8_message(delims: &Delims) -> Message {
    Message {
        delims: delims.clone(),
        segments: vec![
            Segment {
                id: *b"MSH",
                fields: vec![Field::from_text("ignored"), Field::from_text("APP")],
            },
            Segment {
                id: *b"PID",
                fields: vec![
                    Field {
                        reps: vec![
                            Rep {
                                comps: vec![
                                    Comp {
                                        subs: vec![
                                            Atom::Text("A".to_string()),
                                            Atom::Text("B".to_string()),
                                        ],
                                    },
                                    Comp::from_text("C"),
                                ],
                            },
                            Rep::from_text("D"),
                        ],
                    },
                    Field::from_text("left§right"),
                ],
            },
        ],
        charsets: vec![],
    }
}

#[test]
fn message_writer_encodes_utf8_delimiters_with_exact_capacity() {
    let delims = utf8_delims();
    let message = utf8_message(&delims);
    let expected = "MSH§¶※¤¦§APP\rPID§A¦B¶C※D§left¤F¤right\r";

    let written = write(&message);

    assert_eq!(written, expected.as_bytes());
    assert_eq!(message_capacity(&message), written.len());
}

#[test]
fn batch_and_file_writers_encode_utf8_envelope_delimiters_with_exact_capacity() {
    let delims = utf8_delims();
    let message = utf8_message(&delims);
    let batch = Batch {
        header: Some(Segment {
            id: *b"BHS",
            fields: vec![Field::from_text("ignored"), Field::from_text("BATCH")],
        }),
        messages: vec![message],
        trailer: Some(Segment {
            id: *b"BTS",
            fields: vec![Field::from_text("1")],
        }),
    };
    let expected_batch = concat!(
        "BHS§¶※¤¦§BATCH\r",
        "MSH§¶※¤¦§APP\r",
        "PID§A¦B¶C※D§left¤F¤right\r",
        "BTS§1\r",
    );

    let written_batch = write_batch(&batch);

    assert_eq!(written_batch, expected_batch.as_bytes());
    assert_eq!(batch_capacity(&batch), written_batch.len());

    let file_batch = FileBatch {
        header: Some(Segment {
            id: *b"FHS",
            fields: vec![Field::from_text("ignored"), Field::from_text("FILE")],
        }),
        batches: vec![batch],
        trailer: Some(Segment {
            id: *b"FTS",
            fields: vec![Field::from_text("1")],
        }),
    };
    let expected_file = concat!(
        "FHS§¶※¤¦§FILE\r",
        "BHS§¶※¤¦§BATCH\r",
        "MSH§¶※¤¦§APP\r",
        "PID§A¦B¶C※D§left¤F¤right\r",
        "BTS§1\r",
        "FTS§1\r",
    );

    let written_file = write_file_batch(&file_batch);

    assert_eq!(written_file, expected_file.as_bytes());
    assert_eq!(file_batch_capacity(&file_batch), written_file.len());
}
