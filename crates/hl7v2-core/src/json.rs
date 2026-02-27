use crate::{Atom, Field, Message};

/// Convert message to canonical JSON
pub fn to_json(msg: &Message) -> serde_json::Value {
    use serde_json::json;

    let segments: Vec<serde_json::Value> = msg
        .segments
        .iter()
        .map(|segment| {
            let segment_id = String::from_utf8_lossy(&segment.id).to_string();
            let fields: serde_json::Map<String, serde_json::Value> = segment
                .fields
                .iter()
                .enumerate()
                .filter_map(|(index, field)| {
                    if field.reps.is_empty() {
                        None
                    } else {
                        let field_value = field_to_json(field);
                        Some(((index + 1).to_string(), field_value))
                    }
                })
                .collect();

            json!({
                "id": segment_id,
                "fields": fields
            })
        })
        .collect();

    json!({
        "meta": {
            "delims": {
                "field": msg.delims.field.to_string(),
                "comp": msg.delims.comp.to_string(),
                "rep": msg.delims.rep.to_string(),
                "esc": msg.delims.esc.to_string(),
                "sub": msg.delims.sub.to_string()
            },
            "charsets": msg.charsets
        },
        "segments": segments
    })
}

/// Convert a field to JSON
fn field_to_json(field: &Field) -> serde_json::Value {
    use serde_json::json;

    let reps: Vec<serde_json::Value> = field
        .reps
        .iter()
        .map(|rep| {
            let comps: Vec<serde_json::Value> = rep
                .comps
                .iter()
                .map(|comp| {
                    let subs: Vec<serde_json::Value> = comp
                        .subs
                        .iter()
                        .map(|atom| match atom {
                            Atom::Text(text) => json!(text),
                            Atom::Null => json!("__NULL__"),
                        })
                        .collect();
                    json!(subs)
                })
                .collect();
            json!(comps)
        })
        .collect();

    json!(reps)
}
