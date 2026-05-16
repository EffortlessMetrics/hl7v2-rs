#[derive(Debug)]
pub(crate) struct ParsedRedactionPath {
    pub(crate) segment_id: String,
    pub(crate) field_index: usize,
}

pub(crate) fn parse_segment_field_path(path: &str) -> Option<(&str, usize)> {
    let (segment_id, field_part) = path.split_once('.')?;
    if segment_id.is_empty() || field_part.contains('.') {
        return None;
    }

    field_part
        .parse::<usize>()
        .ok()
        .map(|field_index| (segment_id, field_index))
}

pub(crate) fn parse_redaction_path(path: &str) -> Result<ParsedRedactionPath, String> {
    let (segment_id, field_part) = path
        .split_once('.')
        .ok_or_else(|| format!("redaction path '{path}' must use SEG.field syntax"))?;
    if segment_id.len() != 3
        || !segment_id
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit())
    {
        return Err(format!(
            "redaction path '{path}' must start with a three-character uppercase segment id"
        ));
    }
    if field_part.contains('.') {
        return Err(format!(
            "redaction path '{path}' must target a field, not a component"
        ));
    }

    let field_index = field_part.parse::<usize>().map_err(|_err| {
        format!("redaction path '{path}' must use a positive numeric field index")
    })?;
    if field_index == 0 {
        return Err(format!(
            "redaction path '{path}' must use a one-based field index"
        ));
    }
    if segment_id == "MSH" && field_index < 3 {
        return Err(format!(
            "redaction path '{path}' targets MSH.1/MSH.2, which are delimiter metadata and not redacted by this command"
        ));
    }

    Ok(ParsedRedactionPath {
        segment_id: segment_id.to_string(),
        field_index,
    })
}

pub(crate) fn modeled_field_index(segment_id: &str, field_index: usize) -> Option<usize> {
    if segment_id == "MSH" {
        field_index.checked_sub(2)
    } else {
        field_index.checked_sub(1)
    }
}
