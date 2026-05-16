//! Character-set extraction helpers for parsed messages.

use crate::model::{Atom, Segment};

/// Extract character sets from MSH-18 field.
pub(super) fn extract_charsets(segments: &[Segment]) -> Vec<String> {
    if let Some(msh_segment) = segments.first()
        && &msh_segment.id == b"MSH"
        && let Some(field_18) = msh_segment.fields.get(17)
        && let Some(rep) = field_18.reps.first()
    {
        let mut charsets = Vec::new();
        for comp in &rep.comps {
            if let Some(Atom::Text(text)) = comp.subs.first()
                && !text.is_empty()
            {
                charsets.push(text.clone());
            }
        }

        return charsets;
    }
    vec![]
}
