use crate::model::{Atom, Field, Message};

pub(crate) fn field_to_text(field: &Field, delims: &crate::Delims) -> String {
    field
        .reps
        .iter()
        .map(|rep| {
            rep.comps
                .iter()
                .map(|comp| {
                    comp.subs
                        .iter()
                        .map(|atom| match atom {
                            Atom::Text(text) => text.as_str(),
                            Atom::Null => "\"\"",
                        })
                        .collect::<Vec<_>>()
                        .join(&delims.sub.to_string())
                })
                .collect::<Vec<_>>()
                .join(&delims.comp.to_string())
        })
        .collect::<Vec<_>>()
        .join(&delims.rep.to_string())
}

pub(crate) fn message_type(message: &Message) -> String {
    message
        .segments
        .iter()
        .find(|segment| segment.id_str() == "MSH")
        .and_then(|segment| segment.fields.get(7))
        .map(|field| field_to_text(field, &message.delims))
        .filter(|message_type| !message_type.is_empty())
        .unwrap_or_else(|| "UNKNOWN".to_string())
}
