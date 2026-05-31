#[derive(Debug)]
pub(crate) struct ParsedRedactionPath {
    pub(crate) segment_id: String,
    pub(crate) segment_repetition: Option<usize>,
    pub(crate) field_index: usize,
    pub(crate) field_repetition: Option<usize>,
    pub(crate) component: Option<usize>,
    pub(crate) subcomponent: Option<usize>,
    pub(crate) canonical_path: String,
}

pub(crate) fn parse_redaction_path(path: &str) -> Result<ParsedRedactionPath, String> {
    let located = crate::query::path::parse_located_path(path).map_err(|error| {
        if !path.contains('.') && !path.contains('-') {
            format!("redaction path '{path}' must use SEG.field or SEG-FIELD syntax")
        } else {
            format!("redaction path '{path}' is invalid: {error}")
        }
    })?;

    if located.path.segment == "MSH" && located.path.field < 3 {
        return Err(format!(
            "redaction path '{path}' targets MSH.1/MSH.2, which are delimiter metadata and not redacted by this command"
        ));
    }

    let canonical_path = located.to_path_string();

    Ok(ParsedRedactionPath {
        segment_id: located.path.segment,
        segment_repetition: located.segment_repetition,
        field_index: located.path.field,
        field_repetition: located.path.repetition,
        component: located.path.component,
        subcomponent: located.path.subcomponent,
        canonical_path,
    })
}

pub(crate) fn modeled_field_index(segment_id: &str, field_index: usize) -> Option<usize> {
    if segment_id == "MSH" {
        field_index.checked_sub(2)
    } else {
        field_index.checked_sub(1)
    }
}
