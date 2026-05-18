//! Character-set extraction helpers for parsed messages.

use crate::model::{Atom, Segment};

/// Extract character sets from MSH-18 field.
pub(super) fn extract_charsets(segments: &[Segment]) -> Vec<String> {
    const MSH_18_INDEX: usize = 16;

    let Some(msh_segment) = segments.first() else {
        return vec![];
    };
    if &msh_segment.id != b"MSH" {
        return vec![];
    }

    let Some(field_18) = msh_segment.fields.get(MSH_18_INDEX) else {
        return vec![];
    };

    field_18
        .reps
        .iter()
        .filter_map(|rep| {
            rep.comps
                .first()
                .and_then(|comp| comp.subs.first())
                .and_then(|atom| match atom {
                    Atom::Text(text) if !text.is_empty() => Some(text.clone()),
                    _ => None,
                })
        })
        .collect()
}
