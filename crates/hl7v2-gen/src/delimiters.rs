use hl7v2_core::{Delims, Error};

/// Parse delimiters from a string
pub(crate) fn parse_delimiters(delims_str: &str) -> Result<Delims, Error> {
    if delims_str.len() != 4 {
        return Err(Error::BadDelimLength);
    }

    let chars: Vec<char> = delims_str.chars().collect();

    // Check that all delimiters are distinct
    let delimiters = [chars[0], chars[1], chars[2], chars[3]];
    for i in 0..delimiters.len() {
        for j in (i + 1)..delimiters.len() {
            if delimiters[i] == delimiters[j] {
                return Err(Error::DuplicateDelims);
            }
        }
    }

    Ok(Delims {
        field: '|', // Field separator is always |
        comp: chars[0],
        rep: chars[1],
        esc: chars[2],
        sub: chars[3],
    })
}
