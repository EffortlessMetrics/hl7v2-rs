use crate::model::{Atom, Comp, Field, Rep};

use super::digest::compute_sha256;
use super::path::ParsedRedactionPath;
use super::text::{atom_to_text, comp_to_text, field_to_text, rep_to_text};
use super::types::RedactionAction;

pub(crate) fn apply_redaction_target(
    field: &mut Field,
    path: &ParsedRedactionPath,
    action: RedactionAction,
    delims: &crate::Delims,
) -> bool {
    let Some(target) = select_target(field, path) else {
        return false;
    };

    match action {
        RedactionAction::Hash => target.hash(delims),
        RedactionAction::Drop => target.drop_value(),
        RedactionAction::Retain => {}
    }

    true
}

pub(crate) fn replace_redaction_target(
    field: &mut Field,
    path: &ParsedRedactionPath,
    replacement: &str,
) -> bool {
    let Some(target) = select_target(field, path) else {
        return false;
    };

    target.replace_with_text(replacement.to_string());
    true
}

enum RedactionTarget<'a> {
    Field(&'a mut Field),
    Rep(&'a mut Rep),
    Comp(&'a mut Comp),
    Atom(&'a mut Atom),
}

impl RedactionTarget<'_> {
    fn hash(self, delims: &crate::Delims) {
        let value = match &self {
            Self::Field(field) => field_to_text(field, delims),
            Self::Rep(rep) => rep_to_text(rep, delims),
            Self::Comp(comp) => comp_to_text(comp, delims),
            Self::Atom(atom) => atom_to_text(atom).to_string(),
        };
        self.replace_with_text(format!("hash:sha256:{}", compute_sha256(&value)));
    }

    fn drop_value(self) {
        self.replace_with_text(String::new());
    }

    fn replace_with_text(self, replacement: String) {
        match self {
            Self::Field(field) => {
                *field = Field::from_text(replacement);
            }
            Self::Rep(rep) => {
                *rep = Rep::from_text(replacement);
            }
            Self::Comp(comp) => {
                *comp = Comp::from_text(replacement);
            }
            Self::Atom(atom) => {
                *atom = Atom::Text(replacement);
            }
        }
    }
}

fn select_target<'a>(
    field: &'a mut Field,
    path: &ParsedRedactionPath,
) -> Option<RedactionTarget<'a>> {
    if path.field_repetition.is_none() && path.component.is_none() {
        return Some(RedactionTarget::Field(field));
    }

    let rep_index = path.field_repetition.unwrap_or(1).checked_sub(1)?;
    let rep = field.reps.get_mut(rep_index)?;
    let Some(component) = path.component else {
        return Some(RedactionTarget::Rep(rep));
    };

    let component_index = component.checked_sub(1)?;
    let comp = rep.comps.get_mut(component_index)?;
    let Some(subcomponent) = path.subcomponent else {
        return Some(RedactionTarget::Comp(comp));
    };

    let subcomponent_index = subcomponent.checked_sub(1)?;
    comp.subs
        .get_mut(subcomponent_index)
        .map(RedactionTarget::Atom)
}
