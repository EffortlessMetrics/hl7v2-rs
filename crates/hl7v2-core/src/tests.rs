#[cfg(test)]
mod tests {
    use crate::{parse, write, Atom};

    #[test]
    fn test_parse_simple_message() {
        let hl7_text = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r";
        let message = parse(hl7_text.as_bytes()).unwrap();
        
        assert_eq!(message.delims.field, '|');
        assert_eq!(message.delims.comp, '^');
        assert_eq!(message.delims.rep, '~');
        assert_eq!(message.delims.esc, '\\');
        assert_eq!(message.delims.sub, '&');
        
        assert_eq!(message.segments.len(), 2);
        
        // Check MSH segment
        assert_eq!(String::from_utf8_lossy(&message.segments[0].id), "MSH");
        assert_eq!(message.segments[0].fields.len(), 11); // MSH has 11 fields (not counting the field separator)
        
        // Check PID segment
        assert_eq!(String::from_utf8_lossy(&message.segments[1].id), "PID");
        assert_eq!(message.segments[1].fields.len(), 5); // PID has 5 fields
    }

    #[test]
    fn test_null_values() {
        let hl7_text = "MSH|^~\\&|SendingApp|SendingFac\rPID|1||\"\"||Doe^John\r";
        let message = parse(hl7_text.as_bytes()).unwrap();
        
        // Check that NULL values are properly parsed
        let pid_segment = &message.segments[1];
        let null_field = &pid_segment.fields[2]; // PID-3
        let null_rep = &null_field.reps[0];
        let null_comp = &null_rep.comps[0];
        
        match &null_comp.subs[0] {
            Atom::Null => {}, // Correct
            _ => panic!("Expected NULL atom"),
        }
    }
}