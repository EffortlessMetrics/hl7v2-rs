use hl7v2::{LocatedPath, Path, Presence, QueryIndex, get_located, get_presence_located, parse};

#[test]
fn programmatic_msh_zero_field_is_missing_without_panicking()
-> Result<(), Box<dyn std::error::Error>> {
    let message =
        parse(b"MSH|^~\\&|SND|SF|RCV|RF|202609050101||ADT^A01|CTRL1|P|2.5\rPID|1||12345\r")?;
    let path = LocatedPath {
        segment_repetition: None,
        path: Path::new("MSH", 0),
    };

    assert_eq!(get_located(&message, &path), None);
    assert!(matches!(
        get_presence_located(&message, &path),
        Presence::Missing
    ));

    let index = QueryIndex::new(&message);
    assert_eq!(index.get_located(&path), None);
    assert!(matches!(
        index.get_presence_located(&path),
        Presence::Missing
    ));
    Ok(())
}
