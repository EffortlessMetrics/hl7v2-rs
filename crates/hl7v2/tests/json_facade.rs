#![cfg(feature = "json")]

use hl7v2::{Comp, Delims, Field, Message, Rep, Segment, to_json, to_json_string_pretty};
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
fn json_facade_preserves_charsets_segments_and_nested_fields() -> Result<(), Box<dyn Error>> {
    let message = Message {
        delims: Delims::default(),
        segments: vec![Segment {
            id: *b"PID",
            fields: vec![
                Field::from_text("1"),
                Field {
                    reps: vec![Rep {
                        comps: vec![Comp::from_text("Doe"), Comp::from_text("John")],
                    }],
                },
            ],
        }],
        charsets: vec!["ASCII".to_string(), "UTF-8".to_string()],
    };

    let json = to_json(&message);
    let segments = json
        .get("segments")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| std::io::Error::other("missing segments"))?;
    let meta = json
        .get("meta")
        .ok_or_else(|| std::io::Error::other("missing meta"))?;
    let charsets = meta
        .get("charsets")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| std::io::Error::other("missing charsets"))?;

    require_eq(segments.len(), 1, "segment count")?;
    require_eq(charsets.len(), 2, "charset count")?;
    require(
        serde_json::to_string(&json)?.contains("Doe"),
        "expected nested family name in JSON",
    )?;

    Ok(())
}

#[test]
fn json_facade_pretty_output_is_valid_json() -> Result<(), Box<dyn Error>> {
    let message = Message::with_segments(vec![Segment {
        id: *b"MSH",
        fields: vec![Field::from_text("^~\\&")],
    }]);

    let json = to_json_string_pretty(&message);
    let parsed: serde_json::Value = serde_json::from_str(&json)?;

    require(json.contains('\n'), "expected pretty JSON newlines")?;
    require(parsed.is_object(), "expected object JSON")?;

    Ok(())
}
