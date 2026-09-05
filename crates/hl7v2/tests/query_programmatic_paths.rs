use hl7v2::{LocatedPath, Path, Presence, QueryIndex, get_located, get_presence_located, parse};
use std::io;

#[test]
fn programmatic_msh_zero_field_is_missing_without_panicking()
-> Result<(), Box<dyn std::error::Error>> {
    let message =
        parse(b"MSH|^~\\&|SND|SF|RCV|RF|202609050101||ADT^A01|CTRL1|P|2.5\rPID|1||12345\r")?;
    let path = LocatedPath {
        segment_repetition: None,
        path: Path::new("MSH", 0),
    };

    if get_located(&message, &path).is_some() {
        return Err(io::Error::other("unindexed MSH.0 value lookup must return None").into());
    }
    if !matches!(get_presence_located(&message, &path), Presence::Missing) {
        return Err(io::Error::other("unindexed MSH.0 presence must be Missing").into());
    }

    let index = QueryIndex::new(&message);
    if index.get_located(&path).is_some() {
        return Err(io::Error::other("indexed MSH.0 value lookup must return None").into());
    }
    if !matches!(index.get_presence_located(&path), Presence::Missing) {
        return Err(io::Error::other("indexed MSH.0 presence must be Missing").into());
    }
    Ok(())
}
