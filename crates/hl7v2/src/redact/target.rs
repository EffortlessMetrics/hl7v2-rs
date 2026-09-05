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
    for_each_target(field, path, |target| match action {
        RedactionAction::Hash => target.hash(delims),
        RedactionAction::Drop => target.drop_value(),
        RedactionAction::Retain => {}
    })
}

pub(crate) fn replace_redaction_target(
    field: &mut Field,
    path: &ParsedRedactionPath,
    replacement: &str,
) -> bool {
    for_each_target(field, path, |target| {
        target.replace_with_text(replacement.to_string());
    })
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

/// Visit all targets selected by a redaction path.
///
/// A whole-field rule remains one target. A component or subcomponent
/// rule without an explicit field-repetition selector visits every
/// repetition; an explicit selector remains narrow. The boolean reports
/// one match for the containing segment field, preserving receipt counts.
fn for_each_target(
    field: &mut Field,
    path: &ParsedRedactionPath,
    mut visit: impl FnMut(RedactionTarget<'_>),
) -> bool {
    if path.field_repetition.is_none() && path.component.is_none() {
        visit(RedactionTarget::Field(field));
        return true;
    }

    if let Some(repetition) = path.field_repetition {
        let Some(index) = repetition.checked_sub(1) else {
            return false;
        };
        let Some(rep) = field.reps.get_mut(index) else {
            return false;
        };
        let Some(target) = select_rep_target(rep, path) else {
            return false;
        };
        visit(target);
        return true;
    }

    let mut matched = false;
    for rep in &mut field.reps {
        if let Some(target) = select_rep_target(rep, path) {
            visit(target);
            matched = true;
        }
    }
    matched
}

fn select_rep_target<'a>(
    rep: &'a mut Rep,
    path: &ParsedRedactionPath,
) -> Option<RedactionTarget<'a>> {
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

#[cfg(test)]
mod tests {
    use super::{field_to_text, replace_redaction_target};
    use crate::model::{Atom, Comp, Field, Rep};
    use crate::redact::path::parse_redaction_path;
    use crate::redact::{
        RedactionActionStatus, RedactionConfig, RedactionError, redact, redact_hl7_safe_analysis,
    };
    use std::io;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn require(condition: bool, message: &'static str) -> TestResult {
        if condition {
            Ok(())
        } else {
            Err(io::Error::other(message).into())
        }
    }

    fn fixture_field(value: &str) -> Field {
        Field {
            reps: value
                .split('~')
                .map(|rep| Rep {
                    comps: rep
                        .split('^')
                        .map(|comp| Comp {
                            subs: comp
                                .split('&')
                                .map(|atom| Atom::Text(atom.to_string()))
                                .collect(),
                        })
                        .collect(),
                })
                .collect(),
        }
    }

    #[test]
    fn omitted_repetition_selects_components_in_every_repetition() -> TestResult {
        let mut field = fixture_field("Alpha^one~Beta^two~Gamma^three");
        let path = parse_redaction_path("OBX.5.1")?;

        require(
            replace_redaction_target(&mut field, &path, "XXX"),
            "component path should match the field",
        )?;
        require(
            field_to_text(&field, &crate::Delims::default()) == "XXX^one~XXX^two~XXX^three",
            "component replacement must cover every repetition",
        )
    }

    #[test]
    fn explicit_repetition_remains_narrow() -> TestResult {
        let mut field = fixture_field("Alpha^one~Beta^two");
        let path = parse_redaction_path("OBX.5[2].1")?;

        require(
            replace_redaction_target(&mut field, &path, "XXX"),
            "explicit repetition should match",
        )?;
        require(
            field_to_text(&field, &crate::Delims::default()) == "Alpha^one~XXX^two",
            "explicit repetition must not alter its sibling",
        )
    }

    #[test]
    fn missing_early_component_does_not_hide_later_targets() -> TestResult {
        let mut field = fixture_field("keep~keep^Alpha~keep^Beta");
        let path = parse_redaction_path("OBX.5.2")?;

        require(
            replace_redaction_target(&mut field, &path, "XXX"),
            "later repetitions should still match",
        )?;
        require(
            field_to_text(&field, &crate::Delims::default()) == "keep~keep^XXX~keep^XXX",
            "later component targets were not all replaced",
        )
    }

    #[test]
    fn public_redaction_covers_repeated_fields_in_repeated_segments() -> TestResult {
        let mut message = crate::parse(
            b"MSH|^~\\&|SEND|FAC|RECV|FAC|202605090101||ORU^R01|CTRL1|P|2.5\rOBX|1|ST|A||Alpha^one~Beta^two\rOBX|2|ST|B||Gamma^three~Delta^four",
        )?;
        redact(
            &mut message,
            &RedactionConfig {
                fields: vec!["OBX.5.1".to_string()],
                replacement: "XXX".to_string(),
            },
        );
        let written = String::from_utf8(crate::write(&message))?;

        require(
            written.contains("OBX|1|ST|A||XXX^one~XXX^two\r"),
            "first repeated segment was not fully redacted",
        )?;
        require(
            written.contains("OBX|2|ST|B||XXX^three~XXX^four\r"),
            "second repeated segment was not fully redacted",
        )
    }

    #[test]
    fn safe_analysis_counts_segments_not_field_repetitions() -> TestResult {
        let output = redact_hl7_safe_analysis(
            "MSH|^~\\&|SEND|FAC|RECV|FAC|202605090101||ORU^R01|CTRL1|P|2.5\rOBX|1|ST|A||Alpha^one~Beta^two\rOBX|2|ST|B||Gamma^three~Delta^four",
            r#"
[[rules]]
path = "OBX.5.1"
action = "drop"
reason = "Remove every first component"
"#,
        )?;
        let action = output
            .receipt
            .actions
            .iter()
            .find(|action| action.path == "OBX.5.1")
            .ok_or_else(|| io::Error::other("missing OBX.5.1 receipt"))?;

        require(action.matched_count == 2, "receipt must count two segments")?;
        require(
            action.status == RedactionActionStatus::Applied,
            "receipt must report applied",
        )?;
        require(
            output.redacted_hl7.contains("OBX|1|ST|A||^one~^two\r"),
            "first segment repetitions were not dropped",
        )?;
        require(
            output.redacted_hl7.contains("OBX|2|ST|B||^three~^four\r"),
            "second segment repetitions were not dropped",
        )
    }

    #[test]
    fn component_rule_does_not_claim_whole_sensitive_field_coverage() -> TestResult {
        let result = redact_hl7_safe_analysis(
            "MSH|^~\\&|SEND|FAC|RECV|FAC|202605090101||ADT^A01|CTRL1|P|2.5\rPID|1||||Alpha^one~Beta^two",
            r#"
[[rules]]
path = "PID.5.1"
action = "drop"
reason = "A component rule is not whole-field protection"
"#,
        );

        match result {
            Err(RedactionError::Policy(reason))
                if reason.contains("does not protect present sensitive field(s): PID.5") =>
            {
                Ok(())
            }
            _ => Err(io::Error::other(
                "component rule must not satisfy whole-field safe-analysis coverage",
            )
            .into()),
        }
    }
}
