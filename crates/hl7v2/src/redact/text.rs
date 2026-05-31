use crate::model::{Atom, Comp, Field, Message, Rep};

pub(crate) fn field_to_text(field: &Field, delims: &crate::Delims) -> String {
    field
        .reps
        .iter()
        .map(|rep| rep_to_text(rep, delims))
        .collect::<Vec<_>>()
        .join(&delims.rep.to_string())
}

pub(crate) fn rep_to_text(rep: &Rep, delims: &crate::Delims) -> String {
    rep.comps
        .iter()
        .map(|comp| comp_to_text(comp, delims))
        .collect::<Vec<_>>()
        .join(&delims.comp.to_string())
}

pub(crate) fn comp_to_text(comp: &Comp, delims: &crate::Delims) -> String {
    comp.subs
        .iter()
        .map(atom_to_text)
        .collect::<Vec<_>>()
        .join(&delims.sub.to_string())
}

pub(crate) fn atom_to_text(atom: &Atom) -> &str {
    match atom {
        Atom::Text(text) => text.as_str(),
        Atom::Null => "\"\"",
    }
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
