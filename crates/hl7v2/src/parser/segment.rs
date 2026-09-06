//! Segment, field, repetition, component, and atom parsing.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::uninlined_format_args,
    reason = "segment parsing preserves existing delimiter behavior while parser responsibilities are split into SRP submodules"
)]

use crate::escape::unescape_text;
use crate::model::{Atom, Comp, Delims, Error, Field, Rep, Segment};

/// Parse a single segment.
pub(super) fn parse_segment(line: &str, delims: &Delims) -> Result<Segment, Error> {
    if line.len() < 3 {
        std::hint::cold_path();
        return Err(Error::InvalidSegmentId);
    }

    let Some(id_bytes) = line.as_bytes().get(0..3) else {
        std::hint::cold_path();
        return Err(Error::InvalidSegmentId);
    };
    let mut id = [0u8; 3];
    id.copy_from_slice(id_bytes);

    for &byte in &id {
        if !(byte.is_ascii_uppercase() || byte.is_ascii_digit()) {
            std::hint::cold_path();
            return Err(Error::InvalidSegmentId);
        }
    }

    let mut field_sep_buf = [0; 4];
    let field_sep = delims.field.encode_utf8(&mut field_sep_buf);
    let fields_str = if line.len() == 3 {
        ""
    } else if field_sep.len() == 1 && line.as_bytes().get(3) == field_sep.as_bytes().first() {
        let Some(fields_str) = line.get(4..) else {
            std::hint::cold_path();
            return Err(Error::InvalidFieldFormat {
                details: "Segment field separator must end at a UTF-8 boundary".to_string(),
            });
        };
        fields_str
    } else {
        std::hint::cold_path();
        return Err(Error::InvalidFieldFormat {
            details: "Segment fields must start with the configured field separator".to_string(),
        });
    };

    let fields = if is_encoding_header(&id) {
        parse_encoding_header_fields(fields_str, delims)
    } else {
        parse_fields(fields_str, delims)
    }
    .map_err(|e| Error::ParseError {
        segment_id: String::from_utf8_lossy(&id).to_string(),
        field_index: 0,
        source: Box::new(e),
    })?;

    Ok(Segment { id, fields })
}

fn is_encoding_header(id: &[u8; 3]) -> bool {
    id == b"MSH" || id == b"BHS" || id == b"FHS"
}

fn parse_encoding_header_fields(
    fields_str: &str,
    delims: &Delims,
) -> Result<Vec<Field>, Error> {
    let encoding_chars = String::from_iter([delims.comp, delims.rep, delims.esc, delims.sub]);
    let Some(remainder) = fields_str.strip_prefix(&encoding_chars) else {
        std::hint::cold_path();
        return Err(Error::InvalidFieldFormat {
            details: "Encoding header does not declare the configured delimiters".to_string(),
        });
    };

    let mut fields = vec![encoding_field(delims)];
    if remainder.is_empty() {
        return Ok(fields);
    }

    let Some(remaining_fields) = remainder.strip_prefix(delims.field) else {
        std::hint::cold_path();
        return Err(Error::InvalidFieldFormat {
            details: "Encoding characters must be followed by the field separator".to_string(),
        });
    };

    fields.extend(parse_fields(remaining_fields, delims)?);
    Ok(fields)
}

fn encoding_field(delims: &Delims) -> Field {
    let encoding_chars = String::from_iter([delims.comp, delims.rep, delims.esc, delims.sub]);

    Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![Atom::Text(encoding_chars)],
            }],
        }],
    }
}

fn parse_fields(fields_str: &str, delims: &Delims) -> Result<Vec<Field>, Error> {
    if fields_str.is_empty() {
        return Ok(vec![]);
    }

    let field_count = fields_str.matches(delims.field).count() + 1;
    let mut fields = Vec::with_capacity(field_count);

    for (i, field_str) in fields_str.split(delims.field).enumerate() {
        let field = parse_field(field_str, delims).map_err(|e| Error::ParseError {
            segment_id: "UNKNOWN".to_string(),
            field_index: i,
            source: Box::new(e),
        })?;
        fields.push(field);
    }

    Ok(fields)
}

fn parse_field(field_str: &str, delims: &Delims) -> Result<Field, Error> {
    if field_str.contains('\n') || field_str.contains('\r') {
        return Err(Error::InvalidFieldFormat {
            details: "Field contains invalid line break characters".to_string(),
        });
    }

    let rep_count = field_str.matches(delims.rep).count() + 1;
    let mut reps = Vec::with_capacity(rep_count);

    for (i, rep_str) in field_str.split(delims.rep).enumerate() {
        let rep = parse_rep(rep_str, delims).map_err(|e| match e {
            Error::InvalidRepFormat { .. } => e,
            _ => Error::InvalidRepFormat {
                details: format!("Repetition {}: {}", i, e),
            },
        })?;
        reps.push(rep);
    }

    Ok(Field { reps })
}

fn parse_rep(rep_str: &str, delims: &Delims) -> Result<Rep, Error> {
    if rep_str == "\"\"" {
        return Ok(Rep {
            comps: vec![Comp {
                subs: vec![Atom::Null],
            }],
        });
    }

    if rep_str.contains('\n') || rep_str.contains('\r') {
        return Err(Error::InvalidRepFormat {
            details: "Repetition contains invalid line break characters".to_string(),
        });
    }

    let comp_count = rep_str.matches(delims.comp).count() + 1;
    let mut comps = Vec::with_capacity(comp_count);

    for (i, comp_str) in rep_str.split(delims.comp).enumerate() {
        let comp = parse_comp(comp_str, delims).map_err(|e| match e {
            Error::InvalidCompFormat { .. } => e,
            _ => Error::InvalidCompFormat {
                details: format!("Component {}: {}", i, e),
            },
        })?;
        comps.push(comp);
    }

    Ok(Rep { comps })
}

fn parse_comp(comp_str: &str, delims: &Delims) -> Result<Comp, Error> {
    if comp_str.contains('\n') || comp_str.contains('\r') {
        return Err(Error::InvalidCompFormat {
            details: "Component contains invalid line break characters".to_string(),
        });
    }

    let sub_count = comp_str.matches(delims.sub).count() + 1;
    let mut subs = Vec::with_capacity(sub_count);

    for (i, sub_str) in comp_str.split(delims.sub).enumerate() {
        let atom = parse_atom(sub_str, delims).map_err(|e| match e {
            Error::InvalidSubcompFormat { .. } => e,
            _ => Error::InvalidSubcompFormat {
                details: format!("Subcomponent {}: {}", i, e),
            },
        })?;
        subs.push(atom);
    }

    Ok(Comp { subs })
}

fn parse_atom(atom_str: &str, delims: &Delims) -> Result<Atom, Error> {
    if atom_str == "\"\"" {
        return Ok(Atom::Null);
    }

    if atom_str.contains('\n') || atom_str.contains('\r') {
        return Err(Error::InvalidSubcompFormat {
            details: "Subcomponent contains invalid line break characters".to_string(),
        });
    }

    let unescaped = unescape_text(atom_str, delims)?;
    Ok(Atom::Text(unescaped))
}
