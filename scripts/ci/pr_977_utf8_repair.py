#!/usr/bin/env python3
"""Apply the current-source UTF-8 delimiter writer repair without line-ending churn."""

from pathlib import Path

WRITER = Path("crates/hl7v2/src/writer/mod.rs")
TESTS = Path("crates/hl7v2/src/writer/utf8_tests.rs")


def replace_exact(data: bytes, old: bytes, new: bytes, expected: int = 1) -> bytes:
    found = data.count(old)
    if found != expected:
        raise SystemExit(f"expected {expected} occurrences of {old!r}, found {found}")
    return data.replace(old, new)


def replace_between(
    data: bytes,
    start: bytes,
    end: bytes,
    old: bytes,
    new: bytes,
    expected: int = 1,
) -> bytes:
    start_at = data.index(start)
    end_at = data.index(end, start_at)
    block = data[start_at:end_at]
    found = block.count(old)
    if found != expected:
        raise SystemExit(
            f"expected {expected} occurrences of {old!r} between {start!r} "
            f"and {end!r}, found {found}"
        )
    block = block.replace(old, new)
    return data[:start_at] + block + data[end_at:]


def main() -> None:
    data = WRITER.read_bytes()

    data = replace_exact(
        data,
        b"buf.push(msg.delims.field as u8);",
        b"push_delimiter(buf, msg.delims.field);",
    )
    data = replace_exact(
        data,
        b"result.push(delims.field as u8);",
        b"push_delimiter(result, delims.field);",
        expected=2,
    )
    data = replace_exact(
        data,
        b"output.push(delims.field as u8);",
        b"push_delimiter(output, delims.field);",
        expected=3,
    )
    data = replace_exact(
        data,
        b"output.push(delims.comp as u8);",
        b"push_delimiter(output, delims.comp);",
        expected=2,
    )
    data = replace_exact(
        data,
        b"output.push(delims.rep as u8);",
        b"push_delimiter(output, delims.rep);",
        expected=2,
    )
    data = replace_exact(
        data,
        b"output.push(delims.esc as u8);",
        b"push_delimiter(output, delims.esc);",
    )
    data = replace_exact(
        data,
        b"output.push(delims.sub as u8);",
        b"push_delimiter(output, delims.sub);",
        expected=2,
    )

    data = replace_exact(
        data,
        b"capacity = capacity.saturating_add(5);",
        b"capacity = capacity.saturating_add(encoding_header_delimiters_capacity(delims));",
    )
    data = replace_between(
        data,
        b"fn segment_capacity(",
        b"fn segment_fields_capacity(",
        b".saturating_add(1)\n                .saturating_add(field_capacity(field, delims));",
        b".saturating_add(delims.field.len_utf8())\n                .saturating_add(field_capacity(field, delims));",
        expected=2,
    )
    data = replace_between(
        data,
        b"fn segment_fields_capacity(",
        b"fn field_capacity(",
        b".saturating_add(if index > 0 { 1 } else { 0 })",
        b".saturating_add(if index > 0 {\n                        delims.field.len_utf8()\n                    } else {\n                        0\n                    })",
    )
    data = replace_between(
        data,
        b"fn segment_fields_capacity(",
        b"fn field_capacity(",
        b".saturating_add(1)\n        .saturating_add(fields_capacity)",
        b".saturating_add(delims.field.len_utf8())\n        .saturating_add(fields_capacity)",
    )
    data = replace_between(
        data,
        b"fn field_capacity(",
        b"fn rep_capacity(",
        b".saturating_add(if index > 0 { 1 } else { 0 })",
        b".saturating_add(if index > 0 {\n                    delims.rep.len_utf8()\n                } else {\n                    0\n                })",
    )
    data = replace_between(
        data,
        b"fn rep_capacity(",
        b"fn comp_capacity(",
        b".saturating_add(if index > 0 { 1 } else { 0 })",
        b".saturating_add(if index > 0 {\n                    delims.comp.len_utf8()\n                } else {\n                    0\n                })",
    )
    data = replace_between(
        data,
        b"fn comp_capacity(",
        b"fn atom_capacity(",
        b".saturating_add(if index > 0 { 1 } else { 0 })",
        b".saturating_add(if index > 0 {\n                    delims.sub.len_utf8()\n                } else {\n                    0\n                })",
    )

    helper = """fn encoding_header_delimiters_capacity(delims: &Delims) -> usize {
    [
        delims.field,
        delims.comp,
        delims.rep,
        delims.esc,
        delims.sub,
    ]
    .into_iter()
    .fold(0usize, |capacity, delimiter| {
        capacity.saturating_add(delimiter.len_utf8())
    })
}

fn push_delimiter(output: &mut Vec<u8>, delimiter: char) {
    let mut encoded = [0; 4];
    output.extend_from_slice(delimiter.encode_utf8(&mut encoded).as_bytes());
}

""".replace("\n", "\r\n").encode()
    data = replace_exact(
        data,
        b"/// Write a field to bytes (with escaping)",
        helper + b"/// Write a field to bytes (with escaping)",
    )
    data = replace_exact(
        data,
        b"#[cfg(test)]\r\nmod tests;\r\n",
        b"#[cfg(test)]\r\nmod tests;\r\n\r\n#[cfg(test)]\r\nmod utf8_tests;\r\n",
    )

    stale_casts = [
        b"delims.field as u8",
        b"delims.comp as u8",
        b"delims.rep as u8",
        b"delims.esc as u8",
        b"delims.sub as u8",
    ]
    remaining = [needle.decode() for needle in stale_casts if needle in data]
    if remaining:
        raise SystemExit(f"UTF-8 delimiter casts remain: {remaining}")

    WRITER.write_bytes(data)
    TESTS.write_text(
        r'''use super::*;

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
''',
        encoding="utf-8",
    )


if __name__ == "__main__":
    main()
