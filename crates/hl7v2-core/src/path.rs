use crate::{Atom, Message, Presence};

/// Get value at path (e.g., "PID.5[1].1")
pub fn get<'a>(msg: &'a Message, path: &str) -> Option<&'a str> {
    // Parse the path
    // Format: SEGMENT.FIELD[REP].COMPONENT
    // Examples: "PID.5.1", "PID.5[1].1", "MSH.9"

    let mut parts = path.split('.');
    let segment_id = parts.next()?;

    // Find the segment
    let segment = msg.segments.iter().find(|s| {
        std::str::from_utf8(&s.id).map_or(false, |id| id == segment_id)
    })?;

    // Parse field index (1-based)
    let field_part = parts.next()?;
    let (field_index, rep_index) = parse_field_and_rep(field_part)?;

    // Special handling for MSH segments
    if segment_id == "MSH" {
        if field_index == 1 {
            // MSH-1 is the field separator character
            // We can't return a reference to a temporary string, so we don't support this case
            // Users should access msg.delims.field directly for the field separator
            return None;
        } else if field_index == 2 {
            // MSH-2 is the encoding characters
            // This should be the first parsed field (index 0)
            if segment.fields.is_empty() {
                return None;
            }
            let field = &segment.fields[0];
            // Get the repetition
            if rep_index == 0 || rep_index > field.reps.len() {
                return None;
            }
            let rep = &field.reps[rep_index - 1];
            // Get the component
            let comp_index = if let Some(comp_part) = parts.next() {
                comp_part.parse::<usize>().ok()?
            } else {
                1
            };
            if comp_index == 0 || comp_index > rep.comps.len() {
                return None;
            }
            let comp = &rep.comps[comp_index - 1];
            // Get the subcomponent
            if comp.subs.is_empty() {
                return None;
            }
            match &comp.subs[0] {
                Atom::Text(text) => Some(text.as_str()),
                Atom::Null => None,
            }
        } else {
            // MSH-3 and beyond
            // Adjust index: MSH-3 maps to parsed field 1, MSH-4 to parsed field 2, etc.
            let adjusted_field_index = field_index - 2;
            if adjusted_field_index >= segment.fields.len() {
                return None;
            }
            let field = &segment.fields[adjusted_field_index];
            // Get the repetition
            if rep_index == 0 || rep_index > field.reps.len() {
                return None;
            }
            let rep = &field.reps[rep_index - 1];
            // Get the component
            let comp_index = if let Some(comp_part) = parts.next() {
                comp_part.parse::<usize>().ok()?
            } else {
                1
            };
            if comp_index == 0 || comp_index > rep.comps.len() {
                return None;
            }
            let comp = &rep.comps[comp_index - 1];
            // Get the subcomponent
            if comp.subs.is_empty() {
                return None;
            }
            match &comp.subs[0] {
                Atom::Text(text) => Some(text.as_str()),
                Atom::Null => None,
            }
        }
    } else {
        // For non-MSH segments, convert directly to 0-based indexing
        if field_index == 0 {
            return None;
        }
        let zero_based_field_index = field_index - 1;

        // Get the field
        if zero_based_field_index >= segment.fields.len() {
            return None;
        }
        let field = &segment.fields[zero_based_field_index];

        // Get the repetition (convert to 0-based indexing)
        if rep_index == 0 || rep_index > field.reps.len() {
            return None;
        }
        let rep = &field.reps[rep_index - 1];

        // Parse component index if provided
        let comp_index = if let Some(comp_part) = parts.next() {
            comp_part.parse::<usize>().ok()?
        } else {
            1 // Default to first component
        };

        // Get the component (convert to 0-based indexing)
        if comp_index == 0 || comp_index > rep.comps.len() {
            return None;
        }
        let comp = &rep.comps[comp_index - 1];

        // Get the first subcomponent as text
        if comp.subs.is_empty() {
            return None;
        }

        match &comp.subs[0] {
            Atom::Text(text) => Some(text.as_str()),
            Atom::Null => None,
        }
    }
}

/// Get presence semantics for a field at path (e.g., "PID.5[1].1")
pub fn get_presence(msg: &Message, path: &str) -> Presence {
    // Parse the path
    // Format: SEGMENT.FIELD[REP].COMPONENT
    // Examples: "PID.5.1", "PID.5[1].1", "MSH.9"

    let mut parts = path.split('.');
    let segment_id = match parts.next() {
        Some(id) => id,
        None => return Presence::Missing,
    };

    // Find the segment
    let segment = match msg.segments.iter().find(|s| {
        std::str::from_utf8(&s.id).map_or(false, |id| id == segment_id)
    }) {
        Some(seg) => seg,
        None => return Presence::Missing,
    };

    // Parse field index (1-based)
    let field_part = match parts.next() {
        Some(part) => part,
        None => return Presence::Missing,
    };

    let (field_index, rep_index) = match parse_field_and_rep(field_part) {
        Some(indices) => indices,
        None => return Presence::Missing,
    };

    // Special handling for MSH segments
    if segment_id == "MSH" {
        if field_index == 1 {
            // MSH-1 is the field separator character
            // We treat this as a special case - present with the field separator value
            return Presence::Value(msg.delims.field.to_string());
        } else if field_index == 2 {
            // MSH-2 is the encoding characters
            // This should be the first parsed field (index 0)
            if segment.fields.is_empty() {
                return Presence::Missing;
            }
            let field = &segment.fields[0];
            // Check repetition bounds
            if rep_index == 0 || rep_index > field.reps.len() {
                return Presence::Missing;
            }
            let rep = &field.reps[rep_index - 1];
            // Get the component
            let comp_index = if let Some(comp_part) = parts.next() {
                match comp_part.parse::<usize>() {
                    Ok(index) => index,
                    Err(_) => return Presence::Missing,
                }
            } else {
                1
            };
            if comp_index == 0 || comp_index > rep.comps.len() {
                return Presence::Missing;
            }
            let comp = &rep.comps[comp_index - 1];
            // Get the subcomponent
            if comp.subs.is_empty() {
                return Presence::Missing;
            }
            match &comp.subs[0] {
                Atom::Text(text) => {
                    if text.is_empty() {
                        Presence::Empty
                    } else {
                        Presence::Value(text.clone())
                    }
                }
                Atom::Null => Presence::Null,
            }
        } else {
            // MSH-3 and beyond
            // Adjust index: MSH-3 maps to parsed field 1, MSH-4 to parsed field 2, etc.
            let adjusted_field_index = field_index - 2;
            if adjusted_field_index >= segment.fields.len() {
                return Presence::Missing;
            }
            let field = &segment.fields[adjusted_field_index];
            // Check repetition bounds
            if rep_index == 0 || rep_index > field.reps.len() {
                return Presence::Missing;
            }
            let rep = &field.reps[rep_index - 1];
            // Get the component
            let comp_index = if let Some(comp_part) = parts.next() {
                match comp_part.parse::<usize>() {
                    Ok(index) => index,
                    Err(_) => return Presence::Missing,
                }
            } else {
                1
            };
            if comp_index == 0 || comp_index > rep.comps.len() {
                return Presence::Missing;
            }
            let comp = &rep.comps[comp_index - 1];
            // Get the subcomponent
            if comp.subs.is_empty() {
                return Presence::Missing;
            }
            match &comp.subs[0] {
                Atom::Text(text) => {
                    if text.is_empty() {
                        Presence::Empty
                    } else {
                        Presence::Value(text.clone())
                    }
                }
                Atom::Null => Presence::Null,
            }
        }
    } else {
        // For non-MSH segments, convert directly to 0-based indexing
        if field_index == 0 {
            return Presence::Missing;
        }
        let zero_based_field_index = field_index - 1;

        // Check field bounds
        if zero_based_field_index >= segment.fields.len() {
            return Presence::Missing;
        }
        let field = &segment.fields[zero_based_field_index];

        // Check repetition bounds
        if rep_index == 0 || rep_index > field.reps.len() {
            return Presence::Missing;
        }
        let rep = &field.reps[rep_index - 1];

        // Parse component index if provided
        let comp_index = if let Some(comp_part) = parts.next() {
            match comp_part.parse::<usize>() {
                Ok(index) => index,
                Err(_) => return Presence::Missing,
            }
        } else {
            1 // Default to first component
        };

        // Check component bounds
        if comp_index == 0 || comp_index > rep.comps.len() {
            return Presence::Missing;
        }
        let comp = &rep.comps[comp_index - 1];

        // Get the first subcomponent
        if comp.subs.is_empty() {
            return Presence::Missing;
        }

        match &comp.subs[0] {
            Atom::Text(text) => {
                if text.is_empty() {
                    Presence::Empty
                } else {
                    Presence::Value(text.clone())
                }
            }
            Atom::Null => Presence::Null,
        }
    }
}

/// Parse field and repetition indices from a string like "5" or "5[1]"
fn parse_field_and_rep(field_str: &str) -> Option<(usize, usize)> {
    if let Some(bracket_pos) = field_str.find('[') {
        // Has repetition index
        let field_index = field_str[..bracket_pos].parse::<usize>().ok()?;
        let rep_part = &field_str[bracket_pos + 1..];
        if let Some(end_bracket) = rep_part.find(']') {
            let rep_index = rep_part[..end_bracket].parse::<usize>().ok()?;
            Some((field_index, rep_index))
        } else {
            None
        }
    } else {
        // No repetition index, default to 1
        let field_index = field_str.parse::<usize>().ok()?;
        Some((field_index, 1))
    }
}
